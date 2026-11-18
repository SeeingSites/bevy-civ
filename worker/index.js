// The Worker in front of the game: serves the static files in ./dist, and a small API that keeps
// players' sign-ups and (opt-in) play statistics in the D1 database bound as DB.
//
//   POST /api/hello      {player, utm_*, referrer}                 -> notes an anonymous visit
//   POST /api/subscribe  {email, player, interests[], consent, utm_*, referrer, stats}
//   POST /api/stats      {player, events: [{kind, scenario, turn, detail}]}   (opted-in players only)
//   POST /api/forget     {player}                                  -> deletes that player's data
//
// Nothing here identifies a player except an email they chose to give. The player id is a random
// UUID kept in their browser.

const UTM = ["utm_source", "utm_medium", "utm_campaign", "utm_content", "utm_term"];
const INTERESTS = new Set(["strategy", "history", "sci-fi", "fantasy", "mobile", "multiplayer"]);
const EVENT_KINDS = new Set(["game_start", "turn", "game_over", "victory"]);
const MAX_BODY = 8 * 1024;

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

async function readBody(request) {
  const raw = await request.text();
  if (raw.length > MAX_BODY) throw new Error("too large");
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
      body = await readBody(request);
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
        default:
          return json({ error: "not found" }, 404);
      }
    } catch (error) {
      console.error(error);
      return json({ error: "Something went wrong; please try again later." }, 500);
    }
  },
};
