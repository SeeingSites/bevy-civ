# Balance harness

Win rates for a people should come from the data that describes it (its traits, its units, the
aggression it is given), not from a seat the engine always moves first. `--balance` plays many
New World games between computer players under the real rules — the same turn processing as
`--autoplay` — and writes how often each people wins, how the game was won, and the turn it ended.

The `simulate` binaries are dice models. They do not play these rules, so they are not a balance
result.

## Running

```bash
cargo run --release --no-default-features --features full --bin bevy-civ -- --balance [games] [turns] [directory]
```

| Argument | Default | Meaning |
|---|---|---|
| games | 1 | How many games to play |
| turns | 400 | Turn limit. A game ends sooner when someone wins |
| directory | `balance-out` | Where the report is written |

Flags, anywhere on the command line:

- Seats are shuffled by default: every people has the same chance of a seat, and the seated peoples
  are shuffled, so who moves first is not always the theme's first people. `--fixed-seats` turns
  that off: the theme's first people (Rome, on Earth) is then in every game and moves first, as when
  a player leads them. (`--shuffle` is still accepted.)
- `--seed N` — game `i` draws its world from seed `N + i`: the same land, homes, peoples, resources
  and villages each time the command is run, so two versions of the rules can be compared on the
  same worlds. Play still varies from run to run (the turns themselves are not seeded), so compare
  batches, not single games.
- `--expansions` — the expansion packs' peoples (the United States, Germany, Canada and the rest)
  join the New World's candidates.
- `--scenario ID` — play a fixed scenario (`earth`, `rome`, `discovery`, `revolution`, `napoleon`,
  `greatwar`, `ww2`, `modern`) instead of New Worlds: its own map and peoples, with the move order
  shuffled.
- `--peoples N` — seat this many peoples (at least 2, and at most the peoples the theme has: 8 on
  Earth). Without it, games rotate through the start menu's choices, 3, then 5, then 8.
- `--size small|normal|large` — fix the map. Without it, the three sizes take turns, and the people
  count steps once each size has been used: small/3, normal/3, large/3, small/5, and so on.
- `--from N` — number the first game `N`, and continue the rotation from there, so a batch can be
  split across processes.

Difficulty is Regent, so every computer player produces shields and science at the ordinary rate.
The human seat is an empty observer and does not play. `--theme` chooses the peoples, as it does
for a real game (Earth when it is left out).

Each game draws a new map, unless `--seed` is given, which rebuilds the same worlds. A full game is a few seconds in a release build. A sample
large enough to judge a people is hundreds of games at the 400-turn limit. A short limit ends on
score and only checks that the harness runs.

## Output

The directory holds three files, rewritten as each game finishes:

- `games.csv` — one row per game: `game`, `map`, `peoples`, `shuffled`, `end_turn`, `victory`, `winner`.
  Victory is Conquest, Domination, Cultural, Space, Diplomatic, Score, or None.
- `civs.csv` — one row per people per game: seat (1 moves first), name, traits, unique unit, the
  aggression it was given, cities, population, technologies, whether it was eliminated, whether it won.
- `summary.json` — the games, the mean end turn, victory counts, and one row per people: games
  played, wins, win rate, the equal share it would have (`fair_share`), a 95% Wilson interval, and
  `band`.

The same summary is printed.

Seat 1 is the first people to move. The world is built in seat order, and the AI takes its research
and its moves in that order. The observer occupies the human seat and is not listed.

## Fair band

A people is **inside** the band when its win rate is within 5 percentage points of an equal share
of the decided games it played. Two peoples each expect half the wins, so the band is 45-55%.
Three peoples each expect a third, so the band is about 28-38%, and so on.

- `inside` — the rate is in the band.
- `unclear` — the rate is outside, but the 95% interval still overlaps the band. A short smoke
  usually lands here.
- `outside` — the interval misses the band.
- `none` — that people won or lost no decided game.

The process exits 0 after a finished batch either way. An imbalance is a finding. It exits 1 only
when a game cannot be set up or the report cannot be written.

## Smoke

```bash
scripts/balance-smoke.sh [directory]
```

Two games, 40 turns, a small map, 3 peoples, seats shuffled (the default). It checks that the three files were
written. It does not fail when a win rate sits outside the band. `BALANCE_GAMES` and
`BALANCE_TURNS` change the length.

The Balance harness workflow (`.github/workflows/balance.yml`) runs that smoke on demand (Actions,
Balance harness, Run workflow). It does not run on pull requests.

## Toolchain

The repository does not pin a Rust version. Continuous integration installs the stable channel.
The lockfile (`image` 0.25, `uuid` 1.29) does not build on Rust 1.85. Use a current stable.

## Findings

The first full batches (60 seeded New World games each, 400 turns) found domination winning 43% of
games, and 14 of 20 among three peoples: half the cities is only one and a half times an equal
share. Domination now needs a share that grows as the world gets smaller - two in five beyond eight
peoples, half among six to eight, three in five among four or five, two thirds among three - still
held 10 turns from turn 150. Egypt's Royal Chariot (2/2/2, was 3/1/2) no longer decides early wars,
and Greece's Hoplite holds better (1/5/1). Over the next 120 games: space 34%, culture 23%, score
18%, domination 16%, diplomacy 9%; every people won between 0.67 and 1.33 times its equal share,
within the noise of a sample this size.

The Modern World's nations now know what 2025 knows (computers, jets, rockets, the atom for the
great powers), not the techs of a century earlier. That left only Plastics and Superconductors
between them and a starship, and great scientists, which completed research outright, bridged even
that by turn 13: every game ended in space by turn 50. The scenario's `research_cost` (300) makes
research four times dearer, stretches the 40-turn research limit to match, and where research is
dearer a great scientist pays one ordinary discovery's worth. 16 games: mean end turn 105 (was 95
with the old techs); the United States wins 13 (it did 6 of 12 before, with lucky great
scientists), China 3. In 1775 the United States wins 13-14 of 16 with or without the Iroquois'
Republic.

Scenarios of history used to run open-ended, so 280 BC Rome launched starships around AD 390 and
the 1939 war ended in a space race in 2180. They now end on their date (month by month for the
four wars), study nothing past their age, and are won by objective cities, with history's entries
into the war on their dates. 16 games each: Rise of Rome - Rome 7, the Gauls 5, Carthage 3, Macedon
1; Age of Discovery - China 7, the Inca 4, Muscovy 3, Spain 1, India 1; Napoleonic Wars - the
Coalition 12, France 4; 1775 - the American side 15; 1914 - the Entente 16 (no objective changes
hands: a stalemate, as on the Western Front); 1939 - the Axis 13, the Allies 3 (Germany takes Paris
and often London, and the Allies' AI seldom strikes back across the sea - a known weakness).

After the Civilization III parity work (calendar, tundra and jungle, irrigation, new resources,
governments, buildings and units) the Whole Earth ended in a diplomatic victory in 5 to 8 of every
12 games: slower growth on poorer land left the United Nations voting longer before anyone reached
the stars. A voter now backs a candidate only with goodwill of 20 (was 10). 16 games each: Whole
Earth - culture 6, space 5, score 3, diplomacy 2; New World - culture 5, space 5, domination 3,
score 3; Modern World (12) - space 9, diplomacy 2, culture 1.

## What does not change

Without `--balance` the game is unchanged. The harness does not retune combat, victory or the AI,
and it does not record a log of every battle during ordinary play.
