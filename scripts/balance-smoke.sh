#!/usr/bin/env bash
# A short batch of the balance harness (real rules, not the dice-model simulator).
# Exits 0 when the harness finishes and writes its report.
# A win rate outside the 45-55% fair band is printed and does not fail this script.
set -euo pipefail
cd "$(dirname "$0")/.."

OUT="${1:-balance-smoke}"
GAMES="${BALANCE_GAMES:-2}"
TURNS="${BALANCE_TURNS:-40}"

cargo run --locked --release --no-default-features --features full --bin bevy-civ -- \
  --balance "$GAMES" "$TURNS" "$OUT" --shuffle --peoples 3 --size small

test -s "$OUT/summary.json"
test -s "$OUT/games.csv"
test -s "$OUT/civs.csv"

echo "Smoke finished. A win rate outside the fair band does not fail this script."
