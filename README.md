# Bevy Civ

Cross-platform Civilization-inspired 4X (hex grid) in Rust + Bevy.

Core systems from Civ 2/3 implemented modularly. Goal: exceed FreeCiv.

## Current Features
- Hex map with terrain, yields, improvements, and resources
- Luxury resources give happiness bonuses
- Strategic resource unit gating
- Workers build Farm / Mine / Road
- Cities with production, growth, building effects, and **specialists**
- Happiness system with civil disorder
- Units with pathfinding, combat, and selection (expanded unit roster)
- Unique units with special abilities
- Tech tree with prerequisites
- **Full governments** with production/science modifiers, corruption, free unit support, and upkeep
- **Corruption** based on distance from capital (varies by government)
- **Unit upkeep** (military units consume gold beyond free support)
- Advanced war AI
- AI that researches and founds cities
- Fog of war and diplomacy
- Victory conditions (Domination / Democracy)
- **Score system** (population + techs + wonders)
- **Barbarians** that spawn on empty land
- **City capture** by military units
- City UI panel and Tech UI panel
- Wonder effects (Pyramids, Great Library, Hanging Gardens, Colossus, etc.)

## Controls
- **WASD** — pan camera
- **Space** — end turn
- **Left click** — select unit or city, then click destination (path visualized)
- **F / M / R** — worker builds Farm / Mine / Road

## Roadmap (exceed FreeCiv)
### Done
- [x] Hex map + terrain + resources
- [x] Cities + growth + production + buildings
- [x] Happiness / civil disorder
- [x] Tile improvements + workers
- [x] Tech tree + research
- [x] **Full government system**
- [x] Combat + pathfinding + visualization
- [x] City founding
- [x] Multiple civs + AI
- [x] Fog of war + diplomacy
- [x] Strategic resource unit gating
- [x] Victory conditions
- [x] Unique units with abilities
- [x] City / tech UI panels
- [x] Advanced war AI
- [x] **City specialists**
- [x] **Unit upkeep & city support**
- [x] **Corruption system**
- [x] **Score system**
- [x] **Barbarians**
- [x] **More unit types & eras**
- [x] **Wonder effects**
- [x] **City capture**

### Remaining
- [ ] Trade routes / caravans
- [ ] Space race victory
- [ ] Diplomacy treaties (peace/alliance/war)
- [ ] Terrain transformation
- [ ] Save / load game state

## Running
```bash
cargo run
```

## License
MIT OR Apache-2.0
