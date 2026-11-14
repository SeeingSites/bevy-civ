# Bevy Civ

Cross-platform Civilization-inspired 4X (hex grid) in Rust + Bevy.

Core systems from Civ 2/3 implemented modularly.

## Current Features
- Hex map with terrain types and yields
- **Tile improvements**: Farm (+food), Mine (+production), Road (movement)
- Cities with production, growth from food, and visuals
- Building effects (Granary, Library, Barracks, Marketplace, Wonders)
- **Happiness system** with civil disorder risk (updates each turn)
- Units with attack/defense/movement, selection, pathfinding, and combat
- A* pathfinding with visual path markers
- Settler city founding
- Tech tree with prerequisites
- Governments with modifiers
- Multiple civilizations + basic AI
- Fog of war
- Diplomacy attitudes

## Controls
- **WASD** — pan camera
- **Space** — end turn (growth, production, happiness, research, AI)
- **Left click** — select unit, then click destination (path visualized)

## Roadmap toward Civ 2/3 parity
- [x] Hex map + terrain
- [x] Cities + units + production completion
- [x] City growth from food
- [x] Building effects
- [x] **Happiness / civil disorder**
- [x] **Tile improvements**
- [x] Tech tree + research
- [x] Governments + modifiers
- [x] Combat resolution
- [x] Unit movement & selection + pathfinding + visualization
- [x] City founding (settlers)
- [x] Buildings & wonders
- [x] Multiple civs + basic AI
- [x] Fog of war
- [x] Diplomacy
- [ ] Resources (strategic/luxury)
- [ ] Workers building improvements
- [ ] Advanced AI
- [ ] City / tech UI panels
- [ ] Unique units
- [ ] Victory conditions

## Running
```bash
cargo run
```

## License
MIT OR Apache-2.0
