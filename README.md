# Bevy Civ

Cross-platform Civilization-inspired 4X (hex grid) in Rust + Bevy.

Core systems from Civ 2/3 implemented modularly.

## Current Features
- Hex map with terrain, yields, improvements, and resources
- Luxury resources give happiness bonuses
- **Strategic resource unit gating**: Horseman requires Horses, Spearman/Archer require Iron
- Workers build Farm / Mine / Road
- Cities with production, growth, and building effects
- Happiness system with civil disorder
- Units with pathfinding, combat, and selection
- Tech tree with prerequisites
- Governments with modifiers
- AI that researches and founds cities
- Fog of war and diplomacy
- **Victory conditions**:
  - Domination: control 5+ cities
  - Science/Culture: research Democracy

## Controls
- **WASD** — pan camera
- **Space** — end turn
- **Left click** — select unit, then click destination (path visualized)
- **F / M / R** — worker builds Farm / Mine / Road

## Roadmap toward Civ 2/3 parity
- [x] Hex map + terrain + resources
- [x] Cities + growth + production + buildings
- [x] Happiness / civil disorder (with luxury effects)
- [x] Tile improvements + workers
- [x] Tech tree + research
- [x] Governments
- [x] Combat + pathfinding + visualization
- [x] City founding
- [x] Multiple civs + AI (research + founding)
- [x] Fog of war + diplomacy
- [x] **Strategic resource unit gating**
- [x] **Victory conditions**
- [ ] City / tech UI panels
- [ ] Unique units
- [ ] Advanced war AI

## Running
```bash
cargo run
```

## License
MIT OR Apache-2.0
