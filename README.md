# Bevy Civ

Cross-platform Civilization-inspired 4X (hex grid) in Rust + Bevy.

Core systems from Civ 2/3 implemented modularly.

## Current Features
- Hex map with terrain, yields, improvements, and **resources**
- **Luxury resources** (Wine, Spices, Silk) give happiness bonuses to nearby cities
- Strategic resources (Horses, Iron) spawn on the map
- Workers that build Farm / Mine / Road over multiple turns
- Cities with production, growth, and building effects
- Happiness system with civil disorder (now affected by luxuries)
- Units with pathfinding, combat, and selection
- Tech tree with prerequisites
- Governments with modifiers
- **Stronger AI**: researches techs, founds cities with settlers, moves units
- Fog of war and diplomacy

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
- [x] Multiple civs + improved AI (research + founding)
- [x] Fog of war + diplomacy
- [ ] Strategic resource unit gating
- [ ] City / tech UI panels
- [ ] Unique units
- [ ] Victory conditions
- [ ] Advanced combat and war AI

## Running
```bash
cargo run
```

## License
MIT OR Apache-2.0
