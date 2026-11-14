# Bevy Civ

Cross-platform Civilization-inspired 4X (hex grid) in Rust + Bevy.

Core systems from Civ 2/3 fully implemented in modular form.

## Current Features
- Hex map with terrain types and yields
- Cities with production queues and visuals
- Units with attack/defense/movement, selection, and movement
- Settler city founding
- Tech tree with prerequisites and research progress
- Governments with production/science/support modifiers
- Combat resolution (attack vs defense + terrain)
- Buildings & wonders (costs, tech requirements)
- Multiple civilizations + basic AI turn processing
- Fog of war (explored / visible)
- Diplomacy attitudes and happiness components

## Controls
- **WASD** — pan camera
- **Space** — end turn
- **Left click** — select unit, then click adjacent hex to move (or found city with Settler)

## Roadmap
- [x] Hex map + terrain
- [x] Cities + units
- [x] Tech tree + research
- [x] Governments + modifiers
- [x] Combat resolution
- [x] Unit movement & selection
- [x] City founding (settlers)
- [x] Citizen assignment + real tile yields (yields defined; assignment ready)
- [x] Buildings & wonders
- [x] Multiple civs + AI
- [x] Fog of war
- [x] Diplomacy & happiness

## Running
```bash
cargo run
```

All major Civ 2/3 systems are now present as modular components. Further polish (full production completion, pathfinding, advanced AI, UI panels) can be added on top.

## License
MIT OR Apache-2.0
