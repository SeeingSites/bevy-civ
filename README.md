# Bevy Civ

Cross-platform Civilization-inspired 4X (hex grid) in Rust + Bevy.

Core systems from Civ 2/3 implemented modularly. Goal: exceed FreeCiv. **All major systems complete.**

## Current Features
- Hex map with terrain, yields, improvements, and resources
- Luxury resources give happiness bonuses
- Strategic resource unit gating
- Workers build Farm / Mine / Road / terrain transform
- Cities with production, growth, building effects, and specialists
- Happiness system with civil disorder
- Units with pathfinding, combat, and selection (expanded unit roster)
- Unique units with special abilities
- Tech tree with prerequisites
- Full governments with production/science modifiers, corruption, free unit support, and upkeep
- Corruption based on distance from capital
- Unit upkeep
- Advanced war AI (aggression-based attack/retreat, targets cities, uses A* pathfinding)
- Computer players and barbarians attack units and capture undefended cities; aggressive civilizations declare war after turn 25 unless you hold a peace treaty, and units inside a city defend as if on hills
- AI that researches, founds cities, and improves tiles
- Fog of war and diplomacy treaties (Peace / Alliance / War)
- Victory conditions (Domination / Democracy / Space Race / Culture)
- Score system
- Barbarians
- City capture
- Trade routes (gold + science)
- Sprite-based map: textured terrain, animated water, shores, borders, roads, farms and mines
- Animated units, cities that grow visually, fog of war, minimap
- Interface with status bar, unit and city panels, science and foreign advisors, message log
- Wonder effects
- Save / load (expanded)
- Culture and borders (per-turn generation from population + buildings)
- Great Leaders (elite victories → armies)
- **Naval units** (Galley through Battleship + Transport) with cargo load/unload and naval combat
- **Air units** (Fighter, Bomber, Helicopter, Jet Fighter, Stealth Bomber) with fuel, air combat, and bombing damage
- Espionage (steal tech, sabotage, steal gold, incite unrest)
- Economy (inflation, building maintenance, trade gold, government modifiers)
- **Religion** (Polytheism / Monotheism / Philosophy) with happiness bonuses and science modifiers applied per city
- Unit promotions (Veteran, Elite, Combat I/II, Mobility, Siege) — applied in combat + XP awards
- Turn integration (`src/turn.rs`) — helpers to run culture, economy, and promotion-aware combat
- Hotseat multiplayer scaffolding
- Headless game simulator for balance evaluation

## Controls
- **Click / tap** a unit or city to select it (click again to cycle what is on the tile)
- **Click a tile** (or **right-click**) to move the selected unit; moving onto an enemy attacks
- **Drag** to pan, **mouse wheel / pinch** to zoom, **WASD / arrows** to pan, click the minimap to jump
- **Space / Enter** or the **End Turn** button — end turn
- **B** — settler founds a city
- **F / M / R / X** — worker builds Farm / Mine / Road / clears terrain
- **Q** — science advisor (choose research), **E** — foreign advisor (treaties, trade)
- **P / L / K** — Peace / Alliance / War with Rome, **T** — trade routes, **F2** — save
- **Esc** — close a window or deselect

Every order also has a button, so the game is playable with a mouse or touch screen alone.
City production is chosen from the city panel.

## Art
All sprites are original and generated from code: `tools/art/*.cjs` describe each sprite as SVG
and `node tools/art/generate.cjs` rasterises them into the atlases in `assets/sprites/`
(needs Playwright with Chromium). Fonts are Cinzel and Alegreya Sans (SIL Open Font License,
see `assets/fonts/`).

## Simulator
Standalone pure-Rust simulator (no Bevy dependency):

```bash
cd simulate && cargo run -- --games 100 --max-turns 120
```

### CI
GitHub Actions workflow (`.github/workflows/simulate.yml`) runs on push/PR.

## Running (native)
```bash
cargo run
```

## Play in the browser (Cloudflare)
The game compiles to WebAssembly and is served by a static-assets Cloudflare Worker (`wrangler.jsonc`).

One-time tooling: `rustup target add wasm32-unknown-unknown`, a `wasm-bindgen` CLI matching the
version in `Cargo.lock`, and `npm install`.

```bash
npm run build    # builds ./dist (scripts/build-web.sh)
npm run dev      # serve locally at http://localhost:8787
npm run deploy   # build + wrangler deploy (needs `wrangler login` or an API token)
```

`.github/workflows/deploy.yml` builds on every PR and deploys on every push to `main`. It needs two
repository secrets: `CLOUDFLARE_API_TOKEN` (the "Edit Cloudflare Workers" token template) and
`CLOUDFLARE_ACCOUNT_ID`. The game is then live at `https://bevy-civ.<your-subdomain>.workers.dev`.
In the browser, **S** saves to localStorage.

## License
MIT OR Apache-2.0
