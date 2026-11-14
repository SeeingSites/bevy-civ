# Bevy Civ

Cross-platform Civilization-inspired 4X (hex grid) in Rust + Bevy.

Core systems from Civ 2/3 implemented modularly. Goal: exceed FreeCiv.

## Current Features
- Hex map with terrain, yields, improvements, and resources
- Luxury resources give happiness bonuses
- Strategic resource unit gating
- Workers build Farm / Mine / Road
- Cities with production, growth, and building effects
- Happiness system with civil disorder
- Units with pathfinding, combat, and selection
- Unique units with special abilities (Legion, War Chariot, Immortal)
- Tech tree with prerequisites
- Governments with modifiers
- Advanced war AI (advance when stronger, retreat when weaker)
- AI that researches and founds cities
- Fog of war and diplomacy
- Victory conditions (Domination / Democracy)
- City UI panel and Tech UI panel

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
- [x] Governments
- [x] Combat + pathfinding + visualization
- [x] City founding
- [x] Multiple civs + AI
- [x] Fog of war + diplomacy
- [x] Strategic resource unit gating
- [x] Victory conditions
- [x] Unique units with abilities
- [x] City / tech UI panels
- [x] Advanced war AI

### To exceed FreeCiv
- [ ] **Full government system**: Republic, Communism, Fundamentalism with unique effects (corruption, unit upkeep, science rate caps, celebration bonuses)
- [ ] **City specialists**: Entertainer, Tax Collector, Scientist (convert citizens from tile work)
- [ ] **Unit upkeep & city support**: Military units consume food/production based on government
- [ ] **Corruption system**: Distance from capital reduces city output (varies by government)
- [ ] **Trade routes / caravans**: Establish trade between cities for gold and science
- [ ] **Space race victory**: Research future tech and build spaceship components
- [ ] **Barbarians**: Random hostile units that spawn and raid
- [ ] **Diplomacy treaties**: Peace, alliance, war declaration with AI responses
- [ ] **Score system**: Points from population, techs, wonders, wonders for ranking
- [ ] **More unit types & eras**: Phalanx, Catapult, Knight, Cannon, Rifleman, Tank, etc.
- [ ] **Wonder effects**: Global bonuses (Pyramids food, Great Library science, etc.)
- [ ] **Terrain transformation**: Workers can change terrain types (forest to plains, etc.)
- [ ] **City capture**: Units can capture enemy cities instead of just destroying units
- [ ] **Save / load game state**

## Running
```bash
cargo run
```

## License
MIT OR Apache-2.0
