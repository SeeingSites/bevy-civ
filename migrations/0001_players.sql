-- Players, sign-ups and play statistics for the web game (Cloudflare D1: bevy-civ-players).
-- Apply with: npx wrangler d1 migrations apply bevy-civ-players --remote

-- An anonymous player: a random id kept in their browser, with how they first arrived.
CREATE TABLE IF NOT EXISTS players (
  player        TEXT PRIMARY KEY,
  first_seen    TEXT NOT NULL,
  last_seen     TEXT NOT NULL,
  country       TEXT,
  utm_source    TEXT,
  utm_medium    TEXT,
  utm_campaign  TEXT,
  utm_content   TEXT,
  utm_term      TEXT,
  referrer      TEXT,
  stats_opt_in  INTEGER NOT NULL DEFAULT 0
);

-- Someone who asked to hear about future games, with what they said they were interested in.
CREATE TABLE IF NOT EXISTS signups (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  email           TEXT NOT NULL UNIQUE COLLATE NOCASE,
  player          TEXT,
  interests       TEXT,            -- JSON array, e.g. ["strategy","sci-fi"]
  consent         TEXT NOT NULL,   -- the exact consent wording they agreed to
  utm_source      TEXT,
  utm_medium      TEXT,
  utm_campaign    TEXT,
  utm_content     TEXT,
  utm_term        TEXT,
  referrer        TEXT,
  country         TEXT,
  created_at      TEXT NOT NULL,
  unsubscribed_at TEXT
);

-- Play statistics from players who opted in (game started, turn milestones, game over).
CREATE TABLE IF NOT EXISTS events (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  player     TEXT NOT NULL,
  kind       TEXT NOT NULL,
  scenario   TEXT,
  turn       INTEGER,
  detail     TEXT,                 -- small JSON object
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS events_player ON events(player);
CREATE INDEX IF NOT EXISTS events_kind ON events(kind, created_at);
