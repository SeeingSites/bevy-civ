// The Worker in front of the game: serves the static files in ./dist, and a small API that keeps
// players' sign-ups and (opt-in) play statistics in the D1 database bound as DB.
//
//   POST /api/hello      {player, utm_*, referrer}                 -> notes an anonymous visit
//   POST /api/subscribe  {email, player, interests[], consent, utm_*, referrer, stats}
//   POST /api/stats      {player, events: [{kind, scenario, turn, detail}]}   (opted-in players only)
//   POST /api/forget     {player}                                  -> deletes that player's data
//
// Online games (see migrations/0002_online_games.sql). A save travels as base64 of its gzipped JSON.
//   POST /api/game/create {seats: [ids], next_seat, turn, title, save} -> {game, tokens: {seat: token}}
//   POST /api/game/state  {game, token}  -> {turn, next_seat, seat, title, updated, your_move}
//   POST /api/game/fetch  {game, token}  -> {turn, next_seat, seat, save}
//   POST /api/game/turn   {game, token, turn, next_seat, save}  (only by the seat whose move it is)
//
// Nothing here identifies a player except an email they chose to give. The player id is a random
// UUID kept in their browser.

const UTM = ["utm_source", "utm_medium", "utm_campaign", "utm_content", "utm_term"];
const INTERESTS = new Set(["strategy", "history", "sci-fi", "fantasy", "mobile", "multiplayer"]);
const EVENT_KINDS = new Set(["game_start", "turn", "game_over", "victory"]);
const MAX_BODY = 8 * 1024;
// A gzipped save of a large late game is a few MB; D1 rows stay well under their 2 MB limit.
const MAX_SAVE_BODY = 24 * 1024 * 1024;
const CHUNK = 1024 * 1024;
const MAX_SEATS = 8;

const json = (body, status = 200) =>
  new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json; charset=utf-8", "cache-control": "no-store" },
  });

// A short, trimmed string, or null.
const text = (value, max = 200) => {
  if (typeof value !== "string") return null;
  const t = value.trim().slice(0, max);
  return t.length ? t : null;
};

const playerId = (value) => {
  const t = text(value, 64);
  return t && /^[A-Za-z0-9-]{8,64}$/.test(t) ? t : null;
};

const validEmail = (value) => {
  const t = text(value, 254);
  return t && /^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/.test(t) ? t.toLowerCase() : null;
};

async function readBody(request, max = MAX_BODY) {
  const raw = await request.text();
  if (raw.length > max) throw new Error("too large");
  return JSON.parse(raw || "{}");
}

const now = () => new Date().toISOString();

async function hello(env, body, country) {
  const player = playerId(body.player);
  if (!player) return json({ error: "player" }, 400);
  const t = now();
  const utm = UTM.map((k) => text(body[k], 120));
  await env.DB.prepare(
    `INSERT INTO players (player, first_seen, last_seen, country, utm_source, utm_medium, utm_campaign, utm_content, utm_term, referrer, stats_opt_in)
     VALUES (?1, ?2, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
     ON CONFLICT(player) DO UPDATE SET last_seen = ?2, stats_opt_in = ?10`,
  )
    .bind(player, t, country, ...utm, text(body.referrer, 300), body.stats ? 1 : 0)
    .run();
  return json({ ok: true });
}

async function subscribe(env, body, country) {
  const email = validEmail(body.email);
  const consent = text(body.consent, 500);
  if (!email) return json({ error: "Please enter a valid email address." }, 400);
  if (!consent) return json({ error: "Please tick the box to agree to hear from us." }, 400);
  const interests = Array.isArray(body.interests) ? body.interests.filter((i) => INTERESTS.has(i)).slice(0, 6) : [];
  const utm = UTM.map((k) => text(body[k], 120));
  await env.DB.prepare(
    `INSERT INTO signups (email, player, interests, consent, utm_source, utm_medium, utm_campaign, utm_content, utm_term, referrer, country, created_at)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
     ON CONFLICT(email) DO UPDATE SET interests = ?3, consent = ?4, player = COALESCE(?2, player), unsubscribed_at = NULL`,
  )
    .bind(email, playerId(body.player), JSON.stringify(interests), consent, ...utm, text(body.referrer, 300), country, now())
    .run();
  return json({ ok: true });
}

async function stats(env, body) {
  const player = playerId(body.player);
  if (!player || !Array.isArray(body.events)) return json({ error: "bad request" }, 400);
  const row = await env.DB.prepare("SELECT stats_opt_in FROM players WHERE player = ?1").bind(player).first();
  if (!row || !row.stats_opt_in) return json({ ok: false, reason: "not opted in" });
  const t = now();
  const statements = body.events
    .slice(0, 20)
    .filter((e) => e && EVENT_KINDS.has(e.kind))
    .map((e) =>
      env.DB.prepare("INSERT INTO events (player, kind, scenario, turn, detail, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)").bind(
        player,
        e.kind,
        text(e.scenario, 40),
        Number.isInteger(e.turn) ? e.turn : null,
        e.detail ? JSON.stringify(e.detail).slice(0, 500) : null,
        t,
      ),
    );
  if (statements.length) await env.DB.batch(statements);
  return json({ ok: true, stored: statements.length });
}

async function forget(env, body) {
  const player = playerId(body.player);
  if (!player) return json({ error: "player" }, 400);
  await env.DB.batch([
    env.DB.prepare("DELETE FROM events WHERE player = ?1").bind(player),
    env.DB.prepare("DELETE FROM players WHERE player = ?1").bind(player),
    env.DB.prepare("UPDATE signups SET player = NULL WHERE player = ?1").bind(player),
  ]);
  return json({ ok: true });
}

const seatId = (v) => (Number.isInteger(v) && v >= 0 && v < 1000 ? v : null);
const secret = () => crypto.randomUUID().replaceAll("-", "");
const fromBase64 = (b64) => Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
const toBase64 = (bytes) => {
  let s = "";
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
};

// The save's statements: replace every chunk of a game's save.
function storeSave(env, game, b64) {
  let bytes;
  try {
    bytes = fromBase64(b64);
  } catch {
    return null;
  }
  // gzip magic: refuse anything that is not a compressed save.
  if (bytes.length < 18 || bytes[0] !== 0x1f || bytes[1] !== 0x8b) return null;
  const parts = [env.DB.prepare("DELETE FROM game_saves WHERE game = ?").bind(game)];
  let idx = 0;
  for (let i = 0; i < bytes.length; i += CHUNK, idx++)
    parts.push(env.DB.prepare("INSERT INTO game_saves (game, idx, data) VALUES (?, ?, ?)").bind(game, idx, bytes.subarray(i, i + CHUNK)));
  return { parts, chunks: idx };
}

async function seatOf(env, body) {
  const game = text(body.game, 40);
  const token = text(body.token, 64);
  if (!game || !token) return null;
  return env.DB.prepare(
    "SELECT g.game, g.turn, g.next_seat, g.title, g.updated, s.seat FROM games g JOIN game_seats s ON s.game = g.game WHERE g.game = ? AND s.token = ?",
  ).bind(game, token).first();
}

async function createGame(env, body) {
  const seats = Array.isArray(body.seats) ? [...new Set(body.seats.map(seatId))] : [];
  const next = seatId(body.next_seat);
  if (seats.length < 2 || seats.length > MAX_SEATS || seats.includes(null) || !seats.includes(next) || !Number.isInteger(body.turn))
    return json({ error: "Choose two to eight human seats." }, 400);
  const game = secret().slice(0, 12);
  const save = storeSave(env, game, body.save);
  if (!save) return json({ error: "bad save" }, 400);
  const tokens = Object.fromEntries(seats.map((s) => [s, secret()]));
  const t = now();
  await env.DB.batch([
    env.DB.prepare("INSERT INTO games (game, created, updated, turn, next_seat, title, chunks) VALUES (?, ?, ?, ?, ?, ?, ?)")
      .bind(game, t, t, body.turn, next, text(body.title, 80), save.chunks),
    ...seats.map((s) => env.DB.prepare("INSERT INTO game_seats (game, seat, token) VALUES (?, ?, ?)").bind(game, s, tokens[s])),
    ...save.parts,
  ]);
  return json({ game, tokens });
}

async function gameState(env, body) {
  const row = await seatOf(env, body);
  if (!row) return json({ error: "No such game, or not your invitation." }, 404);
  return json({ turn: row.turn, next_seat: row.next_seat, seat: row.seat, title: row.title, updated: row.updated, your_move: row.next_seat === row.seat });
}

async function fetchGame(env, body) {
  const row = await seatOf(env, body);
  if (!row) return json({ error: "No such game, or not your invitation." }, 404);
  const { results } = await env.DB.prepare("SELECT data FROM game_saves WHERE game = ? ORDER BY idx").bind(row.game).all();
  const parts = results.map((r) => new Uint8Array(r.data));
  const all = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let at = 0;
  for (const p of parts) all.set(p, (at += p.length) - p.length);
  return json({ turn: row.turn, next_seat: row.next_seat, seat: row.seat, title: row.title, save: toBase64(all) });
}

async function uploadTurn(env, body) {
  const row = await seatOf(env, body);
  if (!row) return json({ error: "No such game, or not your invitation." }, 404);
  if (row.next_seat !== row.seat) return json({ error: "It is not your move." }, 409);
  const next = seatId(body.next_seat);
  const seat = next === null ? null : await env.DB.prepare("SELECT seat FROM game_seats WHERE game = ? AND seat = ?").bind(row.game, next).first();
  if (!seat || !Number.isInteger(body.turn) || body.turn < row.turn) return json({ error: "bad turn" }, 400);
  const save = storeSave(env, row.game, body.save);
  if (!save) return json({ error: "bad save" }, 400);
  // The guard on next_seat keeps two uploads of the same move from both landing.
  const [moved] = await env.DB.batch([
    env.DB.prepare("UPDATE games SET turn = ?, next_seat = ?, updated = ?, chunks = ? WHERE game = ? AND next_seat = ?")
      .bind(body.turn, next, now(), save.chunks, row.game, row.seat),
    ...save.parts.map((p) => p),
  ]);
  return json({ ok: moved.meta.changes === 1 });
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (!url.pathname.startsWith("/api/")) return env.ASSETS.fetch(request);
    if (request.method !== "POST") return json({ error: "method" }, 405);
    // Only the game's own page may call the API.
    const origin = request.headers.get("origin");
    if (origin && new URL(origin).host !== url.host) return json({ error: "origin" }, 403);
    let body;
    try {
      body = await readBody(request, url.pathname.startsWith("/api/game/") ? MAX_SAVE_BODY : MAX_BODY);
    } catch {
      return json({ error: "bad request" }, 400);
    }
    const country = request.cf?.country ?? null;
    try {
      switch (url.pathname) {
        case "/api/hello":
          return await hello(env, body, country);
        case "/api/subscribe":
          return await subscribe(env, body, country);
        case "/api/stats":
          return await stats(env, body);
        case "/api/forget":
          return await forget(env, body);
        case "/api/game/create":
          return await createGame(env, body);
        case "/api/game/state":
          return await gameState(env, body);
        case "/api/game/fetch":
          return await fetchGame(env, body);
        case "/api/game/turn":
          return await uploadTurn(env, body);
        default:
          return json({ error: "not found" }, 404);
      }
    } catch (error) {
      console.error(error);
      return json({ error: "Something went wrong; please try again later." }, 500);
    }
  },
};
