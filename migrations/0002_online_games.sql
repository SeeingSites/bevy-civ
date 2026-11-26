-- Online games, played as correspondence chess is: each human uploads their turn and the game
-- passes to the next. Apply with: npx wrangler d1 migrations apply bevy-civ-players --remote

CREATE TABLE IF NOT EXISTS games (
  game        TEXT PRIMARY KEY,
  created     TEXT NOT NULL,
  updated     TEXT NOT NULL,
  turn        INTEGER NOT NULL,
  -- The seat (nation id, in the host's numbering) whose move it is.
  next_seat   INTEGER NOT NULL,
  title       TEXT,
  chunks      INTEGER NOT NULL
);

-- Each human seat and the secret that lets it move (handed out in invite links).
CREATE TABLE IF NOT EXISTS game_seats (
  game        TEXT NOT NULL,
  seat        INTEGER NOT NULL,
  token       TEXT NOT NULL UNIQUE,
  PRIMARY KEY (game, seat)
);

-- The latest save, gzipped and split to stay under D1's row size.
CREATE TABLE IF NOT EXISTS game_saves (
  game        TEXT NOT NULL,
  idx         INTEGER NOT NULL,
  data        BLOB NOT NULL,
  PRIMARY KEY (game, idx)
);
