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
- Advanced war AI (aggression-based attack/retreat, targets cities)
- AI that researches, founds cities, and improves tiles
- Fog of war and diplomacy treaties (Peace / Alliance / War)
- Victory conditions (Domination / Democracy / Space Race / Culture)
- Score system
- Barbarians
- City capture
- Trade routes (gold + science)
- City UI panel and Tech UI panel
- Wonder effects
- Save / load (basic)
- Culture and borders
- Great Leaders (elite victories → armies)
- Naval units (Galley through Battleship + Transport)
- Air units (Fighter, Bomber, Helicopter, Jet Fighter, Stealth Bomber) with fuel
- Espionage (steal tech, sabotage, steal gold, incite unrest)
- Economy (inflation, building maintenance, government modifiers)
- **Religion** (Polytheism / Monotheism / Philosophy with happiness and science effects)
- **Unit promotions** (Veteran, Elite, Combat I/II, Mobility, Siege via XP)
- Headless game simulator for balance evaluation

## Controls
- **WASD** — pan camera
- **Space** — end turn
- **Left click** — select unit or city, then click destination (path visualized)
- **F / M / R / X** — worker builds Farm / Mine / Road / Transform terrain
- **P / L / K** — Peace / Alliance / War with AI
- **T** — establish trade route
- **S** — save game

## Simulator
Standalone pure-Rust simulator (no Bevy dependency):

```bash
cd simulate && cargo run -- --games 100 --max-turns 120
```

### CI
GitHub Actions workflow (`.github/workflows/simulate.yml`) runs on push/PR.

## Running
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
