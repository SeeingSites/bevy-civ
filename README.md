# Bevy Civ

A cross-platform Civilization-inspired 4X strategy game MVP written in Rust using the Bevy game engine.

## Goals
- Hexagonal map with procedural terrain (grass, water, desert, mountains)
- Basic unit selection and movement skeleton
- Turn-based structure
- Cross-platform path: desktop, WebAssembly (web), mobile via Bevy

## Current MVP
- Renders a hexagonal grid with randomly assigned terrain colors
- Camera pan with WASD
- Click tiles to select (highlights)
- Simple debug text for turn count

## Running
Requires Rust.

```bash
cargo run
```

### Web (WASM)
```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-server-runner
WASM_SERVER_RUNNER_ADDRESS=0.0.0.0 cargo run --target wasm32-unknown-unknown
```

## Next Steps
- Units as entities with movement points
- Pathfinding (A*)
- City founding
- Tech tree basics
- UI for end turn

## Inspiration
Sid Meier's Civilization, hexciv (Bevy), Unciv, riposte.

## License
MIT OR Apache-2.0
