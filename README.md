# Bevy Civ

Cross-platform Civilization-inspired 4X (hex grid) in Rust + Bevy.

Core systems from Civ 2/3 implemented modularly.

## Current Features
- Hex map with terrain types and yields
- Cities with production queues, completion, and visuals
- Units with attack/defense/movement, selection, movement, and combat
- **A* pathfinding** (hexx algorithms) with terrain costs — units can move multi-hex paths limited by movement points
- **Path visualization** — computed path shown with colored markers (cyan start, yellow path, red destination)
- Settler city founding
- Tech tree with prerequisites and research progress
- Governments with production/science/support modifiers
- Combat resolution on unit collision (attack vs defense + terrain)
- Buildings & wonders (costs, tech requirements)
- Multiple civilizations + AI unit movement / expansion
- Fog of war (explored / visible)
- Diplomacy attitudes and happiness components

## Controls
- **WASD** — pan camera
- **Space** — end turn (processes production, research, AI, resets movement)
- **Left click** — select unit, then click destination (path is visualized, then unit moves along it)

## Roadmap
- [x] Hex map + terrain
- [x] Cities + units + production completion
- [x] Tech tree + research
- [x] Governments + modifiers
- [x] Combat resolution
- [x] Unit movement & selection
- [x] **Pathfinding (A*)**
- [x] **Visual path preview**
- [x] City founding (settlers)
- [x] Buildings & wonders
- [x] Multiple civs + basic AI
- [x] Fog of war
- [x] Diplomacy & happiness
- [ ] Building effects (granary growth, barracks veterans, etc.)
- [ ] Advanced AI (tech, war, diplomacy)
- [ ] UI panels for city management / tech tree
- [ ] Step-by-step A* animation (open/closed sets)

## Running
```bash
cargo run
```

Note: Some Bevy 0.15 API details (Text, queries) may need minor local fixes on first compile. The structure is solid.

## License
MIT OR Apache-2.0
