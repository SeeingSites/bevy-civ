# Bevy Civ

Cross-platform Civilization-inspired 4X (hex grid) in Rust + Bevy.

Core systems from Civ 2/3: cities, units, tech tree, governments, combat.

## Current Features
- Hex map with terrain types and yields
- Cities with production queues
- Units with attack/defense/movement
- **Tech tree** with prerequisites and research progress
- **Governments** (Despotism, Monarchy, Republic, Democracy) with modifiers
- Basic combat resolution (attack vs defense + terrain)
- Turn processing (Space to end turn)

## Controls
- **WASD** — pan camera
- **Space** — end turn (processes production + research)
- Click — log hex (selection/movement next)

## Roadmap
- [x] Hex map + terrain
- [x] Cities + units
- [x] Tech tree + research
- [x] Governments + modifiers
- [x] Combat resolution
- [ ] Unit movement & selection
- [ ] City founding (settlers)
- [ ] Citizen assignment + real tile yields
- [ ] Buildings & wonders
- [ ] Multiple civs + AI
- [ ] Fog of war
- [ ] Diplomacy & happiness

## Running
```bash
cargo run
```

## License
MIT OR Apache-2.0
