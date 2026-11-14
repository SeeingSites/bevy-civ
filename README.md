# Bevy Civ

A cross-platform Civilization-inspired 4X strategy game written in Rust + Bevy.

Targeting the core gameplay systems of **Civilization 2 / 3** (cities, units, tech, production, turns) on a modern hex grid.

## Current Status
- Hex map with terrain types and yields (food/production)
- City component (population, production queue, progress)
- Unit types with attack/defense/movement (Settler, Warrior, Spearman, Archer, Horseman, Worker)
- Basic turn processing (Space to end turn)
- Starting capital + settler

## Roadmap (Civ 2/3 systems)

### Phase 1 - Core Loop (in progress)
- [x] Hex map + terrain
- [x] Cities + production
- [x] Units with stats
- [ ] Unit movement + selection
- [ ] City founding by settlers
- [ ] Tile yields + citizen assignment
- [ ] Production completion (units/buildings)

### Phase 2 - Economy & Research
- [ ] Tech tree (prerequisites, eras)
- [ ] Gold / science generation
- [ ] Buildings (Granary, Barracks, Library, etc.)
- [ ] Terrain improvements (farm, mine, road)

### Phase 3 - Combat & Expansion
- [ ] Combat resolution (attack vs defense + modifiers)
- [ ] Multiple civilizations + basic AI
- [ ] Fog of war
- [ ] City capture

### Phase 4 - Depth
- [ ] Wonders
- [ ] Governments
- [ ] Happiness / luxuries
- [ ] Diplomacy
- [ ] Victory conditions (conquest, space race, score)

## Running
```bash
cargo run
```

Controls: WASD pan, Space end turn, click tiles (logs for now).

## Notes
Full Civ 2/3 feature parity is a large project. This is built incrementally so the core 4X loop works first. Hex grid is used (more modern than Civ 2/3 squares).

## License
MIT OR Apache-2.0
