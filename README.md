# Bevy Civ

Cross-platform Civilization-inspired 4X (hex grid) in Rust + Bevy.

Core systems from Civ 2/3 implemented modularly.

## Current Features
- Hex map with terrain, yields, improvements, and resources
- Luxury resources give happiness bonuses
- Strategic resource unit gating
- Workers build Farm / Mine / Road
- Cities with production, growth, and building effects
- Happiness system with civil disorder
- Units with pathfinding, combat, and selection
- Unique units with special abilities (Legion, War Chariot, Immortal)
- Tech tree with prerequisites
- Governments with modifiers
- AI that researches and founds cities
- Fog of war and diplomacy
- Victory conditions (Domination / Democracy)
- **City UI panel** (click a city to see population, food, production, buildings, happiness)
- **Tech UI panel** (current research, researched techs, available techs)

## Controls
- **WASD** — pan camera
- **Space** — end turn
- **Left click** — select unit or city, then click destination (path visualized)
- **F / M / R** — worker builds Farm / Mine / Road

## Roadmap toward Civ 2/3 parity
- [x] Hex map + terrain + resources
- [x] Cities + growth + production + buildings
- [x] Happiness / civil disorder
- [x] Tile improvements + workers
- [x] Tech tree + research
- [x] Governments
- [x] Combat + pathfinding + visualization
- [x] City founding
- [x] Multiple civs + AI
- [x] Fog of war + diplomacy
- [x] Strategic resource unit gating
- [x] Victory conditions
- [x] Unique units with abilities
- [x] **City / tech UI panels**
- [ ] Advanced war AI

## Running
```bash
cargo run
```

## License
MIT OR Apache-2.0
