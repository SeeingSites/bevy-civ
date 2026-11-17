#!/usr/bin/env bash
# Builds the browser version of the game into ./dist (what Cloudflare serves).
# Needs: the wasm32-unknown-unknown Rust target and a wasm-bindgen CLI matching Cargo.lock.
set -euo pipefail
cd "$(dirname "$0")/.."

TARGET=wasm32-unknown-unknown
PROFILE=wasm-release

cargo build --locked --profile "$PROFILE" --target "$TARGET" --bin bevy-civ

rm -rf dist
mkdir -p dist
wasm-bindgen --target web --no-typescript --out-dir dist --out-name bevy-civ \
  "target/$TARGET/$PROFILE/bevy-civ.wasm"

# Optional extra size pass when binaryen is installed.
if command -v wasm-opt >/dev/null; then
  wasm-opt -Os --all-features -o dist/bevy-civ_bg.wasm dist/bevy-civ_bg.wasm
fi

# The page, its favicons and anything else in web/.
cp web/* dist/
cp -r assets dist/assets
ls -lh dist
