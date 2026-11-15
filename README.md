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
- AI that researches, founds cities, and improves tiles
- Fog of war and diplomacy treaties (Peace / Alliance / War)
- Victory conditions (Domination / Democracy / Space Race / Culture)
- Score system
- Barbarians
- City capture
- Trade routes (gold + science)
- City UI panel and Tech UI panel
- Wonder effects
- Save / load (expanded)
- Culture and borders (per-turn generation from population + buildings)
- Great Leaders (elite victories → armies)
- Naval units (Galley through Battleship + Transport)
- Air units (Fighter, Bomber, Helicopter, Jet Fighter, Stealth Bomber) with fuel
- Espionage (steal tech, sabotage, steal gold, incite unrest)
- Economy (inflation, building maintenance, trade gold, government modifiers)
- Religion (Polytheism / Monotheism / Philosophy)
- Unit promotions (Veteran, Elite, Combat I/II, Mobility, Siege) — applied in combat + XP awards
- **Turn integration** (`src/turn.rs`) — helpers to run culture generation, economy, and promotion-aware combat from the main turn loop
- Hotseat multiplayer scaffolding
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

## Web / Browser (Cloudflare)
See the `cloudflare-web-deploy` branch for the WASM build, Cloudflare Worker static hosting, and GitHub Actions deploy workflow. That branch also contains the compilation fixes that make the game buildable.

```bash
npm run build    # builds ./dist
npm run dev      # local serve
npm run deploy   # deploy to Cloudflare
```

## Running (native)
```bash
cargo run
```

## License
MIT OR Apache-2.0
