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
- Advanced war AI
- AI that researches and founds cities
- Fog of war and diplomacy treaties (Peace / Alliance / War)
- Victory conditions (Domination / Democracy / Space Race)
- Score system
- Barbarians
- City capture
- Trade routes (gold + science)
- City UI panel and Tech UI panel
- Wonder effects
- Save / load (basic)
- **Culture and borders** (cities expand influence with culture points)
- **Great Leaders** (generated on elite victories; form armies)
- **Naval units** (Galley, Trireme, Caravel, Frigate, Battleship, Transport)
- **Headless game simulator** for balance evaluation

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

Outputs average turns, scores, cities, population, techs, and victory distribution.

### CI
A GitHub Actions workflow (`.github/workflows/simulate.yml`) runs the simulator automatically on every push and pull request.

## Running
```bash
cargo run
```

## License
MIT OR Apache-2.0
