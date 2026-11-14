# Bevy Civ

Cross-platform Civilization-inspired 4X (hex grid) in Rust + Bevy.

Core systems from Civ 2/3 implemented modularly.

## Current Features
- Hex map with terrain types and yields
- Cities with production queues, completion, growth from food, and visuals
- **Building effects**: Granary (food), Library (science), Barracks (production), Marketplace (gold), Wonders
- Units with attack/defense/movement, selection, movement, and combat
- A* pathfinding with terrain costs and visual path markers
- Settler city founding
- Tech tree with prerequisites and research progress
- Governments with production/science/support modifiers
- Combat resolution on unit collision
- Multiple civilizations + basic AI
- Fog of war
- Diplomacy attitudes

## Controls
- **WASD** — pan camera
- **Space** — end turn (growth, production, research, AI, movement reset)
- **Left click** — select unit, then click destination (path visualized)

## Roadmap toward Civ 2/3 parity
- [x] Hex map + terrain
- [x] Cities + units + production completion
- [x] City growth from food
- [x] Building effects
- [x] Tech tree + research
- [x] Governments + modifiers
- [x] Combat resolution
- [x] Unit movement & selection + pathfinding + visualization
- [x] City founding (settlers)
- [x] Buildings & wonders
- [x] Multiple civs + basic AI
- [x] Fog of war
- [x] Diplomacy
- [ ] Happiness / civil disorder
- [ ] Resources and tile improvements (roads, farms, mines)
- [ ] Advanced AI (research, war, expansion)
- [ ] City / tech UI panels
- [ ] Unique units per civilization
- [ ] Victory conditions

## Running
```bash
cargo run
```

## License
MIT OR Apache-2.0
