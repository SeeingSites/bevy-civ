# Bevy Civ

Cross-platform Civilization-inspired 4X (hex grid) in Rust + Bevy.

Core systems from Civ 2/3 implemented modularly.

## Current Features
- Hex map with terrain types, yields, and **resources** (Horses, Iron, Wine, Spices, Silk)
- Tile improvements: Farm (+food), Mine (+production), Road
- **Workers** that can build improvements (select worker, press F/M/R, wait turns)
- Cities with production, growth from food, and visuals
- Building effects (Granary, Library, Barracks, Marketplace, Wonders)
- Happiness system with civil disorder risk
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
- **Space** — end turn (growth, production, happiness, research, AI, worker builds)
- **Left click** — select unit, then click destination (path visualized)
- **F / M / R** — with worker selected, start building Farm / Mine / Road

## Roadmap toward Civ 2/3 parity
- [x] Hex map + terrain
- [x] Cities + units + production completion
- [x] City growth from food
- [x] Building effects
- [x] Happiness / civil disorder
- [x] Tile improvements
- [x] **Resources**
- [x] **Workers building improvements**
- [x] Tech tree + research
- [x] Governments + modifiers
- [x] Combat resolution
- [x] Unit movement & selection + pathfinding + visualization
- [x] City founding (settlers)
- [x] Buildings & wonders
- [x] Multiple civs + basic AI
- [x] Fog of war
- [x] Diplomacy
- [ ] Resource effects on happiness / units
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
