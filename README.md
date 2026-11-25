# Bevy Civ

Cross-platform Civilization-inspired 4X (hex grid) in Rust + Bevy. Free and open source under the
MIT License (see [License](#license)).

> An independent fan project, inspired by the Civilization series. Not affiliated with or endorsed
> by Firaxis Games, 2K or Take-Two Interactive; "Civilization" is their trademark. All code, art,
> music and text here are original.

Turn-based, Civilization-style empire building on a hex map, playable in the browser.

## Current Features
- Hex map with terrain, rivers, resources, roads, railroads, farms, mines and pollution
- Terrain slows movement as in Civ: hills and forest cost 2 moves, mountains 3, and a unit that lacks
  the moves still enters but is worn out for the turns it owes (a one-move unit takes 2 turns a hill,
  3 a mountain); roads triple movement and railroads make it free. Wheeled units (chariots,
  catapults, cannon, artillery) keep out of forest and mountains unless a road leads through
- Cities that grow (up to 6, 12 with an Aqueduct or a river, beyond with a Sewer System), can
  starve, and need happy citizens: disorder stops growth, production, science and taxes
- An economy after Civilization III: trade from the land, corruption and waste by distance from a
  palace and by empire size, science, luxury and tax rates capped by the government (computer
  players set theirs too), buildings that multiply science or taxes, and gold upkeep for buildings
- Specialists (entertainers, tax collectors, scientists)
- Strategic and luxury resources joined to cities by roads and harbours, and traded with leaders
- 53 technologies from Pottery to Genetic Engineering in four ages (Ancient Times, Middle Ages,
  Industrial Age, Modern Age), shown as a tree; click any to research the way to it
- Six governments chosen in a revolution (with anarchy): Despotism to Democracy, Communism and
  Fundamentalism; martial law where it applies
- Combat by Civilization III's rules: rounds over hit points, veterans and elites, retreat, city
  size and walls, fortification, river and terrain defence, guns that bombard from range, great
  generals, zone of control and unit upgrades
- A defender for every age: Warrior, Spearman, Phalanx, Pikeman, Musketman, Rifleman, Infantry and
  Mech. Infantry, and Modern Armor after the Tank
- Ships and aircraft: transports carry troops across the sea, carriers carry aircraft
- World wonders (one per world) with real effects; golden ages begun by a wonder in a people's spirit
  or its unique unit's first victory
- Great people: generals, merchants, scientists, artists, engineers
- Governors: with Governor builds on, a city picks its projects, calms disorder with entertainers (and releases them when they are no longer needed) and sends the Workers it trains to work on automatic; a city in disorder says so plainly in its panel
- Auto-explore (C): scouts, soldiers and warships roam to the edge of the known world by themselves, turn after turn
- Ending the turn with a city that has no soldier in it and an enemy in sight nearby asks for a second End Turn
- Armies: a great general in one of your cities forms up to three land units (four with Sun Tzu) into one army, 10% stronger per member; more troops can join later
- Battles played out on the map, one at a time: the camera goes to every battle you fight (also
  attacks on your units in the other peoples' turns), the attacker lunges, blows land, the fallen
  topple and the result floats over the tile; the next unit waits until the battle is over
- A health gauge beside every military unit on the map (green, yellow, red; gold-framed for armies)
- Culture that widens borders and can win over restless cities
- Caravans (trade routes, help with wonders) and diplomats (embassies and espionage)
- Diplomacy: treaties, attitude and reputation, tribute, tech exchange, resource deals
- Computer players that research, expand, build, trade, spy, race for wonders and the stars
- Victory by conquest, domination, diplomacy (United Nations), culture (after Civ III: 60,000 points with 1.6 times any rival's, or 15,000 in a single city of a people of eight or more cities, scaled to the game's length), space race, or score at the last turn
- Multi-turn orders, route preview, worker automation, stack moves, raze or keep captured cities
- Advisors (science, foreign, trade, empire: government, cities, military, score) and an encyclopedia
- Saved games: automatic every turn, and as files that anyone can load
- Sound effects (can be switched off)
- Headless games between computer players (`--autoplay` for one, `--balance` for a batch of win rates)

## Maps and scenarios
The start menu offers four games; in each you choose which people to lead.

- **New World** — a random continent map shared by five civilizations
- **The Whole Earth** — eleven civilizations in their historical homelands, 4000 BC, on a map of the real world
- **The Rise of Rome** — the Mediterranean in 280 BC: Rome, Carthage, Macedon, Ptolemaic Egypt, the Seleucids and the Gauls
- **The Age of Discovery** — the world in 1492, from Spain and the Ottomans to the Mongols, Ming China, the Aztecs and the Inca; the Atlantic powers start with a galleon and settlers for the New World

**Scenarios of history** play as Civilization III's Conquests do. The Rise of Rome runs to AD 14
and the Age of Discovery to 1700, two years a turn; the American Revolution (April 1775 to September
1783), the Napoleonic Wars (September 1805 to June 1815), the Great War (August 1914 to November 1918)
and the Second World War (September 1939 to August 1945) go month by month. Research stops at the
edge of each scenario's age - no starships in Caesar's day - and history arrives on its date: Italy
and the Ottomans enter the Great War and Russia leaves it at Brest-Litovsk; the Soviet Union, the
United States, Mexico and Brazil join the Allies; Spain joins France against Britain in 1779; the
coalitions against Napoleon rise and fall. Only conquest wins early. At the end, the side - a people
and its allies - that has gained most among the scenario's objective cities wins; if nobody has
gained, the stronger side has held (as the Entente did). The Modern World stays open-ended.

Scenario files live in `assets/scenarios/` and are generated by `node tools/maps/build.cjs` from
Natural Earth coastlines (public domain) and the definitions in `tools/maps/scenarios.cjs`; add a
scenario there, rebuild, and list its JSON in `src/scenario.rs`.

**Roads** speed troops on foot: a step from one road tile to another costs a third of a move, so a
unit covers three times as much ground along them (cities count as road). Once **Steam Power** is
known, a worker on a road lays a **railroad** (the same **R** order): moving from rail to rail costs
nothing at all, and railroads run on through cities. Route planning prefers roads and railroads,
for computer players too, whose workers also lay railroads over their roads once they have Steam
Power (after any roads their network still needs). Ships and aircraft ignore both.

Land units cross water only aboard ships: a Galley or Frigate holds 2, a Galleon 4, a Transport 8.
As Civilization III's galley, a Galley keeps in sight of land - it sails only on water beside the
shore - so the open ocean waits for the Galleon and Frigate.
Troops board a ship with room (from the shore or in port), sail with it, and go ashore onto land next
to it; boarding and landing end the move.

## Resources and trade
Twenty-two resources, Civilization III's, are scattered by terrain:

- **Strategic** (Horses, Iron, Saltpeter, Coal, Oil, Rubber, Aluminum, Uranium) are required to build
  things: Horsemen and War Chariots need Horses, Legions and Immortals need Iron, Knights need both,
  Riflemen need Saltpeter, Cannons Iron and Saltpeter, Cavalry Horses and Saltpeter, Tanks Oil and
  Rubber, Modern Armor Oil and Aluminum, Radar Artillery and the starship's structures and components
  Aluminum, Nuclear Missiles Uranium, and a Factory needs Coal and Iron. Each stays invisible until
  the technology that uses it is known (Animal Husbandry, Iron Working, Gunpowder, Steam Power,
  Refining, Replaceable Parts, Rocketry, Nuclear Fission).
- **Luxuries** (Wine, Spices, Silk, Furs, Dyes, Incense, Ivory, Gems): every different one a city has
  access to keeps one more citizen content; a Marketplace makes further luxuries count for more.
- **Bonus** resources (Wheat, Cattle, Fish, Whales, Game, Gold) add food, shields or gold to the city whose land
  they lie in, as do Farms and Mines.

A source only counts when it is inside your borders **and connected**: its tile needs a road, and
roads must link it to the city that wants it. Cities with a Harbor are linked to each other across
the sea. Roads count inside your borders and on unclaimed land, so a rival's border across the
route cuts it. Cities check access when you choose production and stop work if they lose it.

Computer players follow the same rules: their cities train workers, and the workers road each
source to its city, link cities to the capital, then farm and mine the land inside the borders.

A second connected source of a resource is spare and can be sold; other leaders sell their spares.
Deals are made in the foreign advisor, pay gold every turn, run for 20 turns and end at once if war
breaks out or either side loses the source. The trade advisor (**G**) lists what you own, what
still needs a road, what you import and export, and which units you cannot yet train.

## Diplomacy
Open the foreign advisor (**E**) to deal with the leaders you have met. Each has an attitude towards
you that drifts with treaties, trade, shared enemies, crowded borders, your reputation and your
relative strength. You can propose peace or an alliance, sign a trade agreement (+2 gold a turn),
exchange or buy technology, send a gift, demand tribute or declare war - and they can refuse.
Leaders also come to you: they demand tribute when they are stronger, offer peace when a war drags
on, propose technology swaps, ask you to join their wars, and declare war when relations sour.
Breaking a treaty costs reputation with everyone, allies join each other's wars, and the other
powers fight and make peace among themselves.

**Grievances (just causes for war).** Some wrongs give a people a just cause for war against another
for 30 turns: refusing a demand to withdraw troops or to stop drug smugglers, a diplomat caught
spying, a city taken, an attack on an ally, or a border incident (real or staged). A war declared
with a just cause costs 3 reputation instead of 15, everyone else minds it far less, and each of
the victim's allies stays out with a 60% chance; the cause is spent by the war. The foreign advisor
shows the causes you hold and those held against you. Computer players with a grievance are readier
to go to war - against you or each other - and say why.

## Wonders and great people
There are 29 wonders. Each **wonder** can be built only once in the world: when one civilization finishes it, everyone
else's work on it stops (their shields go to the next project) and the news is announced.

| Wonder | Needs | Effect |
|---|---|---|
| Pyramids | Masonry | a Granary in every city of its owner (+2 food where there is none) |
| Great Library | Writing | its owner learns every technology known to two or more of the peoples it has met (obsolete with Theory of Gravity) |
| Hanging Gardens | Monarchy | +2 food in its city, +1 content in every city |
| Colossus | Bronze Working | +1 gold on every ocean tile its city works (needs Iron; obsolete with Flight) |
| Oracle | Mysticism | +2 content in every city (obsolete with Theology) |
| Great Lighthouse | Masonry | +1 move for ships and troops at sea (obsolete with Navigation) |
| Statue of Zeus | Bronze Working | one more unit per city supported free |
| Mausoleum | Masonry | temples and walls each calm one more citizen |
| Great Wall | Construction | every city counts as walled (obsolete with Gunpowder) |
| Copernicus' Observatory | Astronomy | +100% science in its city |
| Sun Tzu's War Academy | Iron Working | new troops trained as veterans everywhere |
| Marco Polo's Embassy | Currency | an embassy with every civilization |
| King Richard's Crusade | Monarchy | +4 shields in its city |
| Michelangelo's Chapel | Theology | +3 content in every city |
| Leonardo's Workshop | Engineering | one obsolete unit upgraded free every turn (obsolete with Motorized Transport) |
| J.S. Bach's Cathedral | Printing Press | +2 content in every city |
| Magellan's Expedition | Navigation | +2 moves for ships |
| Isaac Newton's College | Theory of Gravity | +100% science in its city |
| Adam Smith's Trading Company | Economics | +1 gold for every Marketplace, Bank and Harbor |
| Shakespeare's Theatre | Medicine | no unhappy citizens in its city |
| Statue of Liberty | Democracy | revolutions without anarchy |
| Darwin's Voyage | Electricity | two technologies at once |
| Eiffel Tower | Refining | +2 culture a turn in every city |
| Hoover Dam | Electronics | +25% shields in every city |
| The Internet | Computers | +3 science in every city with a Library |
| Apollo Program | Space Flight | the whole world revealed to its owner; no people can build spaceship parts until it stands |
| Cure for Cancer | Superconductors | +2 content in every city |
| Manhattan Project | Nuclear Fission | every people that knows Rocketry may build Nuclear Missiles |
| United Nations | Computers | every 10 turns the world votes for Secretary-General (see Diplomacy) |

A captured city's wonders change hands with it. The Colossus, the Great Lighthouse and Magellan's
Expedition can only be built in a coastal city. As in Civ III, a few wonders grow obsolete once
their owner learns what supersedes them (the turn after): their effect ends, though their culture
goes on, and a new owner who does not know it yet enjoys the effect again.

Every wonder speaks to one or two of the six traits (Pyramids: Agricultural and Industrious; Great
Library: Scientific; Colossus: Commercial; Oracle: Religious; Great Wall: Militaristic and
Industrious; and so on - the encyclopedia's Wonders page and the city panel list them). A people's
first wonder that matches one of its own traits begins its **golden age**, and so does the first
victory of its unique unit over another people (not barbarians), whichever comes first: for 20 turns
every worked tile that yields a shield yields one more, and every one that yields gold one more
gold. Each people has one golden age a game.

**Great people** are born in cities. Every turn a city earns great-person points from its buildings
(Marketplace, Harbor and Bank for merchants; Library and University for scientists; Aqueduct and
Factory for engineers), its wonders, its specialists and, in the capital, the court (artists). When
a city's points reach the threshold (40, then 30 more for each great person a civilization has had)
one is born there, its kind drawn in proportion to the points; the city panel's Overview shows the
progress. **Great generals** come from battle instead, as Civilization III's great leaders do: one
victory in sixteen won by an elite unit (attacking or defending) brings one forth.
Select a great person and press **U** (or its Use button):

- **Great Scientist**: completes the current research at once.
- **Great Merchant**: in a foreign city you are not at war with, a trade mission worth 100 gold plus
  12 for each tile from your capital (up to 400).
- **Great Engineer**: in one of your cities, finishes what it is building, wonders included.
- **Great Artist**: in one of your cities, a masterpiece: +3 content citizens there for good.
- **Great General**: needs no orders; troops on its tile attack and defend half again as hard.

Computer players earn great people too: they use them at once, and their generals march with their
armies.

## Caravans and trade routes
Trade routes are opened by **caravans**. Move one into another of your cities (at least 6 tiles
from the city that trained it) or next to a foreign city you are not at war with, and press **T**.
The caravan is used up, and the route pays gold and science every turn: more the longer it is and
the bigger the two cities, and half again as much to a foreign city, whose owner earns half the gold
too. Each city can hold three routes. A foreign route closes if the two go to war, and any route
closes when one of its cities changes hands. The trade advisor (**G**) lists your routes.

A caravan in one of your cities that is building a wonder can instead add its 40 shields to it
(**U**). Computer players train caravans to help their own wonders and to trade, with you as well.

## Diplomats and espionage
A **diplomat** standing in or next to a foreign city can carry out one mission (the unit panel lists
them with their odds; the diplomat is spent either way):

| Mission | Effect | Odds |
|---|---|---|
| Embassy | reveals their cities; the foreign advisor shows their research | always |
| Steal Tech | learn a technology they know and you do not | 55% |
| Sabotage | the city loses its production so far | 70% |
| Steal Gold | a quarter of their treasury, up to 300 | 70% |
| Incite Unrest | the city stands idle for 3 turns: no growth, no production | 50% |
| Stage Incident | a false flag: a faked attack by them on your troops gives you a just cause for war | 60% |
| Frame (a people) | a false flag: a strike in their city disguised as another people you know; the city's owner blames them (attitude -20) and gains a just cause for war against them | 50% |

City walls cut the odds of covert missions by 15 points. A diplomat who fails is caught, and the
victim's opinion of you falls sharply, and they gain a just cause for war against you. An exposed
false flag is worse: 10 reputation lost, every other people thinks less of you, and a people you
tried to frame holds it against you too. Computer players train a diplomat once they have a few
cities: it opens an embassy, then steals technology, sabotages (at war) or steals gold, from each
other and from you, and you hear about it when they target you. Now and then an aggressive one
stages an incident to justify a war, and one at war frames its enemy in a neutral city to drag that
city's owner into the fight; you hear the rumours when you are involved.

## Combat
Battles follow the rules of Civilization III:

- **Rounds**: a battle is fought in rounds until one side has no hit points left. Each round the
  attacker lands a blow with chance attack / (attack + defence), both with all their modifiers;
  whoever is struck loses a hit point. Units have 3 hit points, 4 as veterans and 5 as elites:
  experience adds hit points, never strength. Barracks (and Sun Tzu's War Academy) train veterans,
  an Airport veteran aircraft.
- **Defence**: the defender's bonuses are added up and then multiply its defence: open ground and
  the sea +10%, forest +25%, hills +50%, mountains +100%, a river +25%, fortified +25%, a fortress
  +50%. A city defends by its size: a town (up to the size a city reaches without water works,
  8 now) has no defences of its own, a city (up to the size an Aqueduct allows, 16) +50%, a
  metropolis beyond that +100%. City walls (and the Great Wall) give +100% against land attacks
  wherever that is more than the city's own; a Coastal Fortress +100% against ships. Pikemen
  defend twice as hard against mounted units (chariots, horsemen, knights, cavalry). The attacker
  fights at its plain attack. This game adds two multipliers of its own: a Great General on the
  tile (+50%) and an army's extra members (+10% each), for attack and defence alike.
- **Retreat**: a fast unit (2 moves or more) attacking a slow one, brought down to its last hit
  point while the defender has more than one, pulls back instead of dying half the time - 58% as a
  veteran, 66% as an elite. There is no retreat from a fast defender.
- **Promotion**: a victor (attacking or defending) may be promoted: a regular to veteran one time in
  four, a veteran to elite one time in eight; twice as often for a militaristic people, half as
  often after beating barbarians. A promotion adds a hit point at once.
- **Guns**: catapults, cannon, artillery and radar artillery cannot attack or take cities. They
  bombard (B, or click an enemy within range): a gun fires as many rounds as its rate of fire, each
  hitting with chance bombard / (bombard + defence) against the defender's full defence, and every
  hit takes a hit point - but never the last one, and the gun takes no harm. Frigates, ironclads,
  battleships and bombers may bombard too. Shell a garrison, then storm it in the same turn: units
  in their own city heal fully every turn.

| Unit | Attack | Defence | Moves | Bombard (range, rate of fire) | Shields |
|---|---|---|---|---|---|
| Warrior | 1 | 1 | 1 | | 10 |
| Spearman | 1 | 2 | 1 | | 20 |
| Archer | 2 | 1 | 1 | | 20 |
| Horseman | 2 | 1 | 2 | | 20 |
| War Chariot | 2 | 1 | 2 | | 20 |
| Phalanx | 1 | 3 | 1 | | 30 |
| Legion (swordsmen) | 3 | 2 | 1 | | 30 |
| Immortal | 4 | 2 | 1 | | 40 |
| Catapult | 0 | 1 | 1 | 4 (1, 1) | 20 |
| Pikeman | 1 | 2 (4 against mounted) | 1 | | 30 |
| Longbowman | 4 | 1 | 1 | | 30 |
| Knight | 4 | 3 | 2 | | 50 |
| Musketman | 2 | 4 | 1 | | 30 |
| Cannon | 0 | 1 | 1 | 8 (1, 1) | 40 |
| Cavalry | 6 | 3 | 3 | | 70 |
| Rifleman | 4 | 6 | 1 | | 50 |
| Artillery | 0 | 2 | 1 | 12 (2, 2) | 80 |
| Infantry | 6 | 10 | 1 | | 70 |
| Tank | 16 | 8 | 2 | | 100 |
| Radar Artillery | 0 | 2 | 2 | 16 (2, 3) | 100 |
| Mech. Infantry | 12 | 18 | 2 | | 110 |
| Modern Armor | 24 | 16 | 3 | | 150 |

## Ships and aircraft
- **Warships** (Galley, Frigate, Ironclad, Destroyer, Battleship) are built in coastal cities. They
  sail on water and into your own cities, can attack units at sea or on the shore, and cannot take
  cities. Troops aboard a ship are defenceless at sea, cannot attack from it, and go down with it.
- **Aircraft** (Fighter, Bomber) fly over anything but must stay within eight tiles of one of your
  cities. They attack like other units and cannot take cities.
- Only land armies hold and capture cities.

## Computer players
Computer players follow the same rules as you and are run by three pieces of code:

- **Armies** (`src/ai.rs`): the best defender in each city is its garrison and stays put (the best
  two while enemy troops are within 2 tiles, and one more in every city of a people down to three
  cities or fewer); an empty city, or one short of defenders, is reinforced by the nearest unit; field units strike beatable enemies nearby and march on
  enemy cities, but gather outside a defended city until enough of the army has arrived. Workers
  road resources and cities, then farm and mine. Settlers walk clear of existing cities to found
  new ones: each settler heads for the best free site in reach (good land, resources, not too far),
  sailing there once the civilization knows Seafaring. Research follows temperament: warlike
  leaders study arms first, builders the economy; a civilization hemmed in on a small island
  studies Seafaring first; a technology that gives a better form of government is studied early.
  Like the player, they trade knowledge: neighbours not at war swap a technology each (5% a turn
  for each pair, when on good terms), and one with nothing to teach buys a lesson for gold (three
  gold a beaker of its base cost). Trading builds goodwill between them (+3 a swap, +1 a sale),
  which counts when they vote at the United Nations.
  Troop ships wait in port for a load of four (or eight turns) rather than carry a lone soldier
  to its death; settlers sail at once. A computer player keeps at most three great generals.
- **Cities** (`src/governor.rs`): each city picks its next project in order of need - a defender if
  it has none, workers while roads are missing, a Harbor if it is cut off from the capital, an army
  sized to peace or war, settlers, then buildings and wonders. A city in danger pays gold to hurry
  its defender. A city with nothing left to build adds at most a couple of spare workers (and raises no soldiers beyond the army limit). Settlers keep coming (one per six cities) while there is free land within reach.
- **Economy and diplomacy** (`src/main.rs`, `src/foreign.rs`, `src/economy.rs`): science and gold
  come from their own cities' trade under the same rules as yours. Each turn they set their rates:
  luxury while cities riot, then the most science their bills allow (keeping a small surplus while
  the treasury is low, spending a large one down). Their governors build where buildings pay their
  upkeep (a Courthouse where corruption bites, libraries and markets where there is trade, an
  Aqueduct when a city nears size 6) and hold back while upkeep leaves little for science. Spare
  gold hurries production and is invested in research. Their merchants
  propose resource deals: to buy what you can spare and sell what they can. They also buy and
  sell resources among themselves.
- **Reaching cut-off cities**: a city with no land route to the capital is roaded to the nearest
  coastal city of its owner, and coastal cities on both sides build Harbors to join the networks.
- **Modern armies**: troops and armies in their cities are upgraded while the treasury allows
  (keeping 50 gold in reserve, up to four a turn, soldiers and the most outdated first); at the army size limit the weakest unit that can no longer
  be upgraded is stood down when something twice as strong can be built.
- **Wars of conquest**: an aggressive power with 1.4 times a neighbour's military strength is far
  likelier to attack it and slow to make peace while it wins. Armies grow with the empire (two units
  a city, up to 6 plus three for every two cities, so a great empire keeps a field army beyond its garrisons). A defended city is stormed only once the army
  gathered before it can wear the garrison down: the planner weighs each attacker against the
  garrison's best defence against it, with every bonus of the city (its land, size and walls, dug
  in), takes off what the guns in range will knock off by bombarding first, and every hit point
  left costs about (1-p)/p of an attacker's, p being the odds of one blow. Warlike leaders commit
  with half the margin. Then every unit there joins the assault, guns firing first. Elsewhere a
  cautious leader's troops attack only at even odds or better, a warlike leader's whatever stands
  in their way. One unit in four that an army raises for the field is a gun, once guns are known;
  guns keep their distance and bombard.
  The last cities of a people with three or fewer, and the capital of a people whose spaceship is in
  flight, draw armies from much further away.
- **Settling overseas**: a seafaring people looks for free land up to 20 tiles from its cities and
  ships settlers to sites up to 22 tiles away when its own land is full.
- **Navy and air force**: at war, coastal cities add warships and industrial powers add aircraft;
  both hunt enemies within reach and otherwise stay at their base.

## Controls
- **Click / tap** a unit or city to select it; a bouncing arrow marks the selection
- Where units share a tile they fan out and a **×N** badge shows how many there are; **click the
  tile again** or press **Tab** for the next one, or pick it from the list in the unit panel
- **Click a tile** (or **right-click**) to move the selected unit; moving onto an enemy attacks
- With a unit selected, **hovering** a tile shows the route there: bright dots are reached this
  turn, faded ones later, and a number marks where each turn's march ends
- A destination too far for one turn becomes a standing order: the unit marches on at the start of
  each turn, halts next to anything foreign in its way, and forgets the order on arrival or when
  given a new one (fortifying or building also cancels it)
- **Drag** to pan, **mouse wheel / pinch** to zoom, **WASD / arrows** to pan, click the minimap to jump
- The game selects the next unit that needs orders and moves the camera to it
- **Space** — skip the selected unit and go to the next; when none is left, end the turn
- **N** or the **Next Unit** button — go to the next unit without skipping this one
- **H** — fortify (or put a non-military unit to sleep) so it is no longer asked for orders
- **Enter** or the **End Turn** button — end the turn now
- **B** — settler founds a city; a gun, warship or bomber bombards
- **F / M / R / X** — worker builds Farm / Mine / Road (Railroad on a road, with Steam Power) /
  clears terrain or cleans pollution; **K** — automate the worker
- **Shift-click** — move the whole stack (or switch on *Move together* in the unit panel)
- **Q** — science advisor (research and the tech tree), **E** — foreign advisor (treaties, resource
  deals), **G** — trade advisor (resources and trade routes), **Y** — empire advisor (government,
  cities, military, score), **F1** — encyclopedia
- **U** — use the selected great person, or add a caravan's shields to a wonder
- **T** — the selected caravan opens a trade route
- **F2** — save the game as a file (the **Game** button also has sound and quitting)
- **V** — switch sound effects on or off (remembered between games)
- **Esc** — close a window or deselect

A city's project (anything but a wonder) can be finished at once from the city panel: **Buy now**
costs two gold a missing shield for a building, plus a premium that grows with what is missing for
a unit, doubled if nothing has been built of it yet (computer players pay the same); under
Despotism and Communism **Rush** spends one citizen for every 20 shields instead.

Units also have buttons for upgrading (in a city, for gold), bombarding (artillery, warships,
bombers) and diplomats' missions. A city keeps training the same unit until you change its order;
when it finishes a building or wonder it announces it and waits for its next project, and such
cities are brought up before the turn ends.

## Starting a game
Choose a world and your people, then the difficulty and the length of the game. Difficulty sets how
fast the computer players build and learn (from 0.6x on Chieftain to 1.4x on Emperor); on Chieftain
and Warlord your citizens are also easier to please. Games last 200, 300 or 400 turns; if nobody
has won by the last turn, the highest score wins. The Whole Earth and New Worlds keep Civilization
III's calendar, from 4000 BC to 2050 AD on the last turn: turns of about 70 years at first in a
400-turn game, shortening as history quickens to a year or two at the end (scenarios keep their own
years a turn). When the game ends, a summary shows the final
standing.

Tips for new players appear the first time something happens (your first turn, first city, first
contact, first disorder...). "No more tips" turns them off; the Game menu turns them back on.

## The end of the game
When a game ends - won, lost, or by **Surrender** (Game menu, with a confirmation) - the game-over
window gives the outcome and the standing, and **Final scores and replay** opens the story of the
game: every people's final score, cities, culture and technologies; a graph of every people's score
over the whole game; and the map replayed turn by turn, each people's lands and cities in its
colour, with play/pause, step and jump controls. The replay is recorded as the game goes (only the
tiles and cities that change hands each turn are kept, in the saved game), so a loaded game replays
from its start.

## Saving and loading
The game saves itself at the end of every turn; **Continue your last game** on the start menu picks
it up. **F2** (or *Save to a file* in the Game menu) downloads the whole game as a `.json` file.
Anyone can open it with **Load a saved game file...** on the start menu - send it to a friend, or
keep it as a backup. On the desktop the files are `autosave.json` and `savegame.json` in the
working directory.

## Rules in brief
- **Trade and rates** (after Civilization III): the land a city works brings in trade - the sea 2,
  a river 1, a road on grassland, plains or desert 1, some resources more. Corruption takes a share
  (below), and the rest is split by the rates in the Empire advisor's Government tab into science,
  luxury and taxes. A government limits every share, taxes included: Despotism 60%, Monarchy 70%,
  Communism, Republic and Fundamentalism 80%, Democracy 90%; in anarchy nothing is taxed or
  learned and all trade goes to luxury. Buildings then multiply each part: Library, University
  (needs a Library) and Research Lab +50% science each, Observatory +25%; Marketplace +50% taxes
  and luxury, Bank and Stock Exchange +30% each. Scientists add 3 science and tax collectors 2 gold
  before the multipliers; entertainers calm two citizens. Every 2 luxury make a citizen happy.
- **Governments**: Civilization III's, with the three of its Conquests expansion. Despotism;
  **Tribalism** (Bronze Working: the despot's poor tiles, but five units a city free and less
  corruption, no forced labour); Monarchy; **Feudalism** (Chivalry: five units a city free, rates up to
  70%, war weariness of at most one citizen); Republic; Democracy; Communism; **Fascism** (Steel: five
  units a city free, low corruption, rates up to 80%, strong martial law, forced labour and no war
  weariness); Fundamentalism. Warlike computer leaders favour Tribalism, Feudalism and Fascism.
- **Corruption and waste**: a city's corruption has two halves - distance to the nearest palace
  (the capital or a Forbidden Palace; full at 24 tiles) and rank, the number of its people's cities
  nearer a palace, which counts against the optimal number of cities (the world's land shared
  among its peoples, 10 to 20) and climbs twice as fast past it. The government sets the scale:
  Anarchy 80%, Despotism 60%, Monarchy 45%, Communism 40% (distance does not matter: every city
  counts as halfway), Republic and Fundamentalism 35%, Democracy 20%, at most 90% of trade. A
  Courthouse and a Police Station each halve the distance half and let the city count a quarter
  more cities as optimal; a road or harbour link to the capital takes 15% off the distance half.
  Waste takes 60% of that share of a city's shields (always leaving one). Palace cities have none.
- **Upkeep**: buildings cost gold every turn - most 1, a Factory or a Stock Exchange 2; walls,
  coastal forts, the Forbidden Palace and wonders nothing (no upkeep is paid in anarchy). A city can sell one building a turn for its cost in gold (Overview
  tab); a treasury that cannot pay its bills sells the dearest to keep, empire-wide.
- **Finance**: Currency brings the Marketplace; Banking (Currency and The Republic) the Bank (needs
  a Marketplace); Economics (Banking and Printing Press) the Stock Exchange (needs a Bank) and Adam
  Smith's Trading Company. Savings earn 5% interest a turn, up to 2 gold per Bank and 3 per Stock
  Exchange. A Great Merchant makes a trade mission abroad (gold now) or settles in one of your
  cities as a trading house (+3 gold there every turn).
- **Science and ages**: technologies belong to four ages - Ancient Times, the Middle Ages, the
  Industrial Age and the Modern Age. A people enters an age with its first discovery there, and can
  study an age's technologies only once it knows every required technology of the ages before
  (optional: Horseback Riding, Seafaring and The Republic; Chivalry, Navigation and Democracy;
  Communism and Fundamentalism). Each discovery makes the next ones dearer, and a technology is
  cheaper the more of the other peoples know it - of those you have met (computer players count
  every people) - down to half price when all of them do. A discovery takes at least 4 turns (beakers beyond its cost are lost) and at most
  40, however little science goes into it. The first people to learn Philosophy founds the Way of
  Reason and makes a free discovery; a Scientific people makes one on entering each new age.
- **Traits and unique units**: every civilization has two traits - Militaristic (troops promoted
  twice as often, and one more unit a city supported free), Commercial (+1 gold a city), Scientific (+1 science a city and a
  free discovery in each new age),
  Religious (+1 content a city, shorter revolutions), Industrious (+1 shield a city), Agricultural
  (+1 food a city) - and a unique unit (see below).
- **Unique units**: each people's cities turn out its unique unit in place of one ordinary unit
  (same cost, technology and upgrades): stronger, and with an ability - **Charge** (+50% attack
  against units on open ground: grassland, plains, desert), **Storm** (+50% attack against cities),
  **Shield wall** (double defence against mounted troops), **Bastion** (+50% defence in a city),
  **Outrider** (ignores zones of control), **Plunder** (15 gold from each victory) or
  **Self-sufficient** (needs no Horses or Iron). Its first victory over another people also begins a
  golden age. The city panel shows it in place of the ordinary unit; a crest marks it on the map.

| People | Unique unit | In place of | A/D/M | Ability |
|---|---|---|---|---|
| Rome | Praetorian | Legion | 4/2/1 | Storm |
| Greece | Hoplite | Phalanx | 1/5/1 | Shield wall |
| Macedon | Companion Cavalry | Horseman | 3/1/2 | Charge |
| Carthage | Numidian Rider | Horseman | 2/2/3 | Plunder |
| Egypt | Royal Chariot | War Chariot | 2/2/2 | Charge |
| Persia | Immortal Guard | Immortal | 4/3/1 | Storm |
| Seleucids | Cataphract | Knight | 5/3/2 | Charge |
| Gauls | Gallic Swordsman | Legion | 3/2/2 | Plunder |
| India | War Elephant | Knight | 4/4/1 | Self-sufficient |
| China | Chu-ko-nu | Longbowman | 4/2/1 | Bastion |
| Japan | Samurai | Knight | 4/4/2 | Storm |
| Mongols | Keshik | Knight | 4/3/3 | Outrider |
| Aztecs | Jaguar Warrior | Warrior | 2/1/2 (15 shields, Bronze Working) | Plunder |
| Inca | Chasqui Runner | Spearman | 1/2/2 | Outrider |
| Mali | Mandekalu Cavalry | Knight | 4/4/2 | Charge |
| Songhai | Askia Rider | Horseman | 3/1/2 | Charge |
| Spain | Conquistador | Knight | 4/3/2 | Plunder |
| England | Man-O-War | Frigate | 4/3/4 | Plunder |
| France | Musketeer | Musketman | 2/5/1 | Bastion |
| Ottomans | Sipahi | Cavalry | 8/3/3 | Charge |
| Muscovy | Cossack | Cavalry | 6/3/4 | Outrider |
| United States† | Marine | Infantry | 7/10/1 | Storm |
| Prussia† | Black Hussar | Cavalry | 8/2/3 | Charge |
| Austria† | Grenzer | Musketman | 3/5/1 | Outrider |
| Italy† | Bersaglieri | Infantry | 6/10/2 | Outrider |
| Germany† | Panzer | Tank | 17/8/2 | Charge |
| Russia† | Guards Rifleman | Rifleman | 4/7/1 | Bastion |
| Brazil* | Bandeirante | Musketman | 3/4/2 | Outrider |
| Australia* | Light Horse | Cavalry | 7/3/3 | Charge |
| Canada* | Mountie | Cavalry | 7/4/3 | Outrider |
| Mexico* | Charro | Cavalry | 7/3/3 | Plunder |
| United Kingdom* | Commando | Infantry | 8/9/2 | Outrider |

\* With the Modern Nations pack; † with the Age of Revolution or World Wars pack (see Expansion packs under Themes).

- **Religion**: the first to learn Mysticism, Philosophy, Theology and Printing Press founds the Old
  Gods, the Way of Reason, the One Faith and the Reformed Faith in one of its cities (the holy city).
  Faiths spread by themselves to nearby cities with none, and Missionaries (Mysticism) carry your
  state religion - your capital's faith - to any city at peace with you (U). Cities of your state
  religion get +1 content (+2 with a Temple); the holy city's owner earns a gold for every two
  cities of its faith; peoples of one faith grow friendlier, of different faiths cooler.
- **Unit support**: as in Civilization III, each city supports a few military units free - 4 under
  Despotism or Fundamentalism, 3 under Monarchy or Communism, 2 in anarchy, none under a Republic or
  Democracy - and every unit beyond costs a gold a turn (2 under a Republic or Democracy, nothing
  under Fundamentalism); the Statue of Zeus supports one more per city. Computer players are held
  to an army size instead (see Computer players).
- **War weariness**: under a Republic every 12 turns of war make one more citizen unhappy in every
  city (up to 2), under Democracy every 8 (up to 3); peace brings it back down; a Police Station
  halves it.
- **Resistance**: a captured city produces nothing for 1 turn plus 1 per 3 citizens (up to 6); a
  garrison of two settles it twice as fast.
- **Working the land**: every tile yields food, shields and trade - grassland 2/0/0, plains 1/1/0,
  desert 0/1/0, hills 1/0/0, mountains 0/1/0, forest 1/2/0, ocean 1/0/2, and as in Civilization III
  tundra (near the poles), jungle (the wet tropics) and marsh (the great wetlands) 1/0/0 - plus farms (+1 food),
  mines (+2 shields on hills and mountains, +1 elsewhere), roads (+1 trade on flat land), railroads
  (+1 shield), resources and rivers (+1 trade); a Harbor adds a food to worked ocean tiles. The
  government changes it: under Despotism and Anarchy a tile giving 3 or more of anything gives one
  less; a Republic or Democracy adds a trade to tiles that already give some. A city of three or
  more with nobody unhappy and at least half its people happy celebrates its ruler: under
  Despotism its tiles escape the penalty, under Monarchy or Communism they earn a republic's extra
  trade (luxury is the way to it). As in Civilization III a farm needs fresh water - a river on or
  beside the tile, or the sea, a lake, a city or another farm beside it - until Electricity brings
  water anywhere; tundra, jungle and marsh cannot be farmed, but a worker clears jungle and marsh
  into grassland (T), as it clears forest, desert and hills into plains. Jungle and marsh cost two
  moves and give cover (+25% and +20% defence). Workers farm and
  mine, then road the farmed flat land. Each working citizen works one tile of its city's area and
  every citizen eats 2 food; the city's own tile is worked for free with a bonus. A city grows when
  its food box fills: 20 food as a town (up to size 6), 40 as a city (to 12), 60 as a metropolis;
  a Granary keeps half the food when it grows. A governor picks tiles (food first until the city can grow,
  then a balance); open a city and its area shows each tile's yield, the worked tiles ringed:
  click a tile to move a citizen there or off it (to entertain), or let the governor choose again.
- **Governor builds**: each of your cities' governors also chooses its next project when one is
  done (after anything you queued), with the same judgement the computer players use. It is on in
  new games; switch it off in a city's panel, or for every city under Empire, Cities, to choose by
  hand - then units repeat until you change the order and buildings wait for one.
- **Barbarians**: from turn 10, every 8 turns a band rises on wild land outside every border and
  away from any unit; it grows (one to three units) and re-arms with the age of the world, from
  warriors to infantry and cavalry.
- **United Nations**: the wonder (Computers) holds a vote for Secretary-General every 10 turns
  between its owner and the highest-scoring other people; every people votes for itself if a
  candidate, else for the candidate it likes better (or abstains); a majority of the members - and
  of all the peoples the world began with, so conquest does not make the vote cheap - wins a
  diplomatic victory.
- **More orders**: Sentry (Z) sleeps a unit until an enemy comes within two tiles; Join City (J)
  settles a worker (+1) or settler (+2) into the city it stands in. Missionaries preach in or next
  to a city, and computer players now train them for their state religion.
- **More of the classic rules**: drafting (a city turns a citizen into its best defender under
  Monarchy, Republic, Communism or Fundamentalism once Gunpowder is known); fighters intercept
  bombers within 3 tiles; global warming turns land drier once more than 15 tiles of the world are
  fouled; computer players offer right of passage and fortify hills when enemies are near.
- **Conveniences**: undo the last plain move (Ctrl+Z or Backspace); the Messages window (L) keeps
  the game's last 2,000 messages, and its "great events" view is the chronicle offered at the end;
  a New World can be Small, Normal or Large with 3, 5 or 8 peoples; generated background music
  plays during the game (Music on/off in the Game menu).
- **Big empires**: past 12 cities, every 6 more make one more citizen unhappy in every city, and
  corruption climbs steeply past the optimal number of cities; a Forbidden Palace (Monarchy, one
  per people) is a second palace, corruption counting from the nearer of it and the capital. Great people grow rarer: 60 points for the first, then
  180, 360, 600 and so on. Computer players spend their gold more freely.
- **Capitals and the fallen**: a people that loses its capital moves the palace to its largest
  city (corruption and the state religion follow it). A people with no city and no settler left
  is out of the game: its troops disband and it leaves diplomacy. Scores count citizens (2),
  technologies (10) and wonders (50) the same way for everyone. Holding two in five of the world's
  cities (at least five; more in a world that began with few peoples: half among six to eight, three in
  five among four or five, two thirds among three) for 10
  turns in a row, from turn 150, wins by domination; everyone is
  warned when a people first reaches it. A people down to three cities or fewer makes a **last
  stand**: its cities' defenders get +50%, so nobody is wiped out in a stroke. A scenario's peoples
  keep the government it starts them under for 50 turns. A city whose project loses a resource, or
  that has just changed hands, falls back to the stoutest defender its people can always train (no
  Warriors in 1939).
- **Build queue**: in the city panel, switch "Click to" from *Build now* to *Add to queue* and pick
  up to six projects; each starts when the one before is done (a unit in production gives way to
  the queue, otherwise units keep repeating). Click a queued project to take it off.
- **Pillage and fortresses**: troops on improved enemy land (at war) press P to wreck the railroad,
  fortress, mine, farm or road there, for 5 gold; computer troops raid your land too. Workers build
  a Fortress (O, needs Construction) in the field: troops in it defend 50% harder.
- **Right of passage**: ask a people at peace for one (Diplomacy); your units and theirs may then
  share tiles and pass through each other. Walking into the units or an empty city of a people at
  peace is refused (declare war first). An alliance includes passage; war ends it.
- **Tribal villages**: scattered on the land at the start (about one per 30 land tiles). The first
  unit in finds gold, a technology, warriors who join, maps of the area, or now and then hostile
  tribesmen (scouts are careful and rarely meet them).
- **Late buildings**: Observatory (Astronomy, +25% science), Research Lab (Computers, +50% science,
  needs a University), Factory (Industrialization, +25% shields), Power Plant (Electronics, +25%
  shields, needs a Factory), Police Station (Sanitation, +2 content, cuts corruption), Supermarket
  (Sanitation, +3 food), Mass Transit (Mass Production, no
  pollution), Recycling Center (Ecology, halves pollution), Coastal Fortress (Steel, +100% defence
  against ships) and Airport (Flight, veteran aircraft).
- **Civilization III's national wonders and later works**: one of each per people - **Iron Works**
  (Industrialization, needs Coal and Iron: +50% shields there), **Wall Street** (Economics, needs a
  Bank: +10 to the interest cap), **Military Academy** (Gunpowder: troops trained there one level
  higher), **Battlefield Medicine** (Sanitation: troops heal two hit points a turn in the field) - beside
  the Forbidden Palace; the world wonder **Women's Suffrage** (Industrialization: war weariness halved
  everywhere); and the **Offshore Platform** (Computers: +1 shield on worked ocean tiles),
  **Manufacturing Plant** (Computers, needs a Factory: +25% shields) and **Civil Defense**
  (Electronics: a nuclear strike costs a quarter of the city, not half).
- **Civilization III's later units**: the **Crusader** (Theology, 5/3/1, Horses), **Medieval Infantry**
  (Chivalry, 4/2/1, Iron: Legions and Immortals upgrade to it), the **Trebuchet** (Engineering, bombards
  6 - between the Catapult and the Cannon), the **Caravel** (Astronomy: crosses the ocean with 3
  aboard, between the Galley and the Galleon), the **Jet Fighter** (Rocketry), **AEGIS Cruiser**
  (Rocketry), **Nuclear Submarine** (Nuclear Fission, Uranium) and **Stealth Bomber** (Superconductors,
  hard to intercept). Jet fighters intercept as fighters do.
- **More units**: Scout (explorer, 2 moves), Longbowman (Monarchy), Knight (Chivalry), Galleon
  (Navigation), Submarine (Electronics), Engineers (Steam Power: workers that move 2 and finish jobs
  in 2 turns), Radar Artillery (Rocketry). Battleship needs Steel, Fighter Flight, Bomber Advanced
  Flight; Copernicus needs Astronomy, Magellan Navigation, Newton Theory of Gravity, Shakespeare
  Medicine and the Hoover Dam Electronics.
- **Defenders**: Pikeman (Engineering, Iron), Musketman (Gunpowder, Saltpeter), Rifleman
  (Industrialization, Saltpeter), Infantry (Electricity), Mech. Infantry (Computers, Oil); Modern
  Armor (Plastics, Oil) follows the Tank. Older units upgrade along this line in a city for gold, skipping
  straight to the best the city can equip (a city without Saltpeter can still turn Longbowmen into
  Infantry). Upgrade all here re-equips a whole garrison at once, cheapest first.
- **Happiness**: five citizens are content; the rest are unhappy until calmed by temples (+2),
  colosseums and cathedrals (+3), wonders, masterpieces, entertainers or martial law (troops in the
  city under Despotism, Monarchy and Communism). Each luxury the city can reach makes one citizen
  happy. Disorder (more unhappy than happy) stops the city.
- **Culture**: as in Civ III, each turn a Temple or Colosseum earns 2 culture, a Library or
  Cathedral 3, a University 4, a Research Lab or Forbidden Palace 2, every wonder 4 and the palace 1,
  and a building or wonder that has stood a quarter of the game (100 turns of 400) earns double.
  Borders reach 2 tiles, 3 from 100 culture, 4 from 1,000 and 5 from 10,000. Where borders
  meet, each tile goes to the city pressing hardest on it: (culture + 50) / distance², so a great
  culture pushes its neighbours back (every city keeps its own ring). Land lost this way takes its
  resources and river gold with it. A city in disorder within 5 tiles of a civilization with twice
  its owner's culture may defect: 20% a turn, more the further it lies from its own capital
  (+10% of that for every tile) and the more of its land the tempter's borders hold (+1/6 for each
  tile), divided by one plus the troops in it (at most 60%).
- **Cultural victory**: 60,000 culture points with 1.6 times as much as any other people, or 15,000
  in a single city under your rule if you hold eight cities or more and half the 60,000 (for a 400-turn game; scaled to shorter games). Civ III asks
  100,000 and 20,000 over a longer game; these are tuned so culture, space, diplomacy and
  domination all win a fair share of games.
  As in Civ III a people keeps the culture it has earned even if its cities fall, and a conqueror
  counts only what a captured city earns from then on (the city keeps its own culture for its
  borders).
- **Golden age**: once a game, begun by a people's first wonder that matches one of its traits or by
  its unique unit's first victory over another people: 20 turns of one more shield on every worked
  tile that yields shields, and one more trade on every one that yields trade.
- **Rivers**: +1 gold per river tile in a city's land, +25% defence, and fresh water.
- **Pollution**: big industrial cities spoil tiles, which yield nothing until cleaned.
- **Combat**: Civilization III's rules (see [Combat](#combat)): 3 hit points (4 veteran, 5 elite);
  winners may be promoted, Barracks train veterans, wounds heal while resting (fully in a city).
  Fast attackers may pull back from slow defenders; guns bombard but cannot attack.
- **Armies**: select a Great General in one of your cities with two or three land units and press
  U (Form Army). The strongest unit leads; the army fights with its strength and the hit points of
  every member, moves at its slowest member's pace and is re-equipped member by member. The general is used up.
  Computer players form armies too.
- **Space race to the nearest star**: research Space Flight, Plastics and Superconductors; once
  someone has built the Apollo Program, cities with a Factory build 4 structures, 2 to 4 components
  (engines) and 2 modules. Launch from the
  Empire advisor's Space page, which shows every known ship's voyage from Sol to Alpha Centauri
  (4.37 light-years): 22 turns, 2 fewer per extra engine. Computer players launch when ready. The
  first ship to arrive wins; a ship in flight is lost with its capital - so a launch turns the world
  against the launcher: rivals declare war and march on its capital.
- **Nuclear weapons**: the Manhattan Project (Nuclear Fission) lets everyone who knows Rocketry build
  Nuclear Missiles. Select one, click a target within 12 tiles and click again to launch: every unit
  within a tile dies, a city there loses half its people and two buildings, fallout spoils the land,
  the victim declares war and the world trusts the attacker less. SDI Defense (Superconductors)
  shoots down missiles aimed within 3 tiles. Computer players at war build them and keep to
  deterrence: they answer a people that has struck them, strike first only in desperation (down to
  three cities or fewer), never first at a people with missiles of its own, and never at raiders.

Every order also has a button, so the game is playable with a mouse or touch screen alone.
City production is chosen from the city panel, whose tabs split **Units**, **Buildings** and
**Wonders**; **Overview** lists what the city has built, its resources and its great-person progress.

## Code layout (and reuse)
The project is organised so that the rules, the engine-level client pieces and this game's content
can be told apart - a step towards reskinning it as other games:

- `src/lib.rs` and its modules (`unit`, `city`, `tech`, `combat`, `ai`, `cargo`, `troubles`, ...):
  the rules, with no drawing code. Headless autoplay and the simulator run on them.
- `src/client/layout.rs`: **game-agnostic** screen layout. A `Screen` class (size, portrait or
  landscape, compact phone or not) recomputed on every resize or rotation; panels that declare a
  `Dock` (top, bottom corners, right drawer, centre) and are placed so they never collide (on a phone
  the bottom corners share a strip and the right drawer becomes a full-width sheet); `Scrollable`
  panels that scroll with the wheel or a finger; button `Activated` events (a mouse acts on press, a
  finger on release unless it scrolled); and browser canvas sizing that gets the pixel ratio right on
  high-density screens.
- `src/client/paint.rs`: painting lines into images for diagrams (the relations web, the tech tree).
- `src/client/input.rs`: camera pan and zoom, and taps and drags on the hex map.
- `src/client/ui.rs` and the other client modules: this game's panels, advisors and views. They
  describe content and dock it; the layout module fits it to the device.

## Themes (reskinning)
The rules never name what things look like or are called: a **theme** does (`src/theme.rs`).
A theme is a JSON file in `assets/themes/`, built into the game, that can rename every unit,
building and wonder, technology, terrain, resource, government and faith (by its Rust variant
name), rewrite what buildings do, count years its own way (`"{year} BC"`), set a few proper nouns
(`game_title`, `destination` for the space race, `barbarian`), and point at its own folder of sprite
sheets. Anything left out keeps its Earth name. Scenarios carry a `"theme"` and the menu lists those
of the theme in force (a theme can borrow another's with `"base"`).

Choose one with `?theme=<id>` in the browser or `--theme <id>` natively (also for `--autoplay`).
`assets/themes/earth.json` documents the format; `example.json` is a small demonstration
(`?theme=example`: Earth's maps with Colony Pods, Plasma Troopers, Methane Seas and years counted
from the Landing).

**Cities show their age**: a city is drawn in the style of its people's age - mud brick and a
stepped temple in ancient times, castles and red roofs in the middle ages, brick terraces and
smoking chimneys in the industrial age, towers of glass in the modern age - at four sizes each
(`cities.png`, 4 sizes x 4 ages, drawn by `tools/art/world.cjs`).

**Expansion packs** add nations and scenarios of later periods while the base game - the Whole
Earth, New World, Rome and Discovery with their peoples - stays as it is. Each pack is switched on
from the start menu (off by default, remembered):

| Pack | Nations (lead them from 4000 BC) | Scenarios |
|------|-----------------------------------|-----------|
| Age of Revolution | the United States, France, Spain, Prussia, Austria | *The American Revolution* (1775): the colonies and France against Britain, Spain watching. *The Napoleonic Wars* (1805): France and Spain against the Third Coalition (Britain, Austria, Russia); Prussia and the Ottomans neutral |
| World Wars | Germany, Italy, Russia, the Ottomans | *The Great War* (1914): the Central Powers against the Entente, Italy neutral, the Ottomans allied to Germany. *The Second World War* (1939): Germany against Britain and France, Japan against China, the US and USSR at peace |
| Modern Nations | the United Kingdom, Canada, Mexico, Brazil, Australia | *The Modern World* (2025): thirty-seven of today's nations on a world already built |

A pack's nations can be led in a New World or on the Whole Earth from their real homelands; they
join only a game whose player leads them, so the classic line-up never changes. One that shares an
ancient people's homeland takes its place (the United Kingdom replaces England, Italy Rome, Mexico
the Aztecs). Period scenarios start with their alliances (`"alliances"`), wars, governments
(`"government"` per people) and the knowledge of their time; faiths that already exist are not
announced as new. Scenarios name their pack with `"expansion"`. Packs live in `assets/expansions/`
and are built with their scenarios by `tools/maps/build.cjs` from `tools/maps/expansions.cjs`
(`ONLY=revolution,napoleon,greatwar,ww2` rebuilds just those). **Objectives**: each period scenario names objective cities - the great powers' capitals and the
war's key places - worth 200 points to whoever holds them (shown at the start and in the score).
A score at the turn limit then rewards winning the period's war, not sitting it out on empty land.

**The Modern World** is *developed*: 37 nations and some 260 cities on an equal-area world map
(so India, China, Africa and Europe have as much room per city as the Americas), every city linked
by roads and railroads to its neighbours and capital, the land around it farmed and mined, and
its buildings those of its size (granary, temple, library, market, harbour and supermarket; then
courthouse, aqueduct, university, bank; in great cities factories, sewers and exchanges). The wonders
stand as they do today: the Pyramids in Cairo and the Great Wall in Beijing; the long-destroyed
(the Colossus, the Great Library, the Hanging Gardens...) lost to history; the Statue of Liberty,
the United Nations, Apollo, the Hoover Dam and the Manhattan Project in the United States, the
Eiffel Tower in Paris; the Internet shared by the whole world (nobody has its bonus); the Cure for
Cancer still to be found. Forces are those of 2025, scaled: the American carrier fleet, Russian,
Chinese and American armour and bombers, the nuclear arsenals of the seven nuclear powers here
(the US, Russia, China, France, the UK, India, Pakistan), submarine fleets and air forces down to a garrison in every city,
and nobody needs settlers or workers - in a developed world no one founds new cities. Scenario
options for this: `"developed"`, `"wonders"` (wonder and city, or null) and `"objectives"`.

Leaders are historical up to 1914; the
1939 and 2025 scenarios use generic titles and portraits, never real people. Not yet adapted for
the later starts: ancient wonders can still be built.

Still Earth-only (later stages): the wording of tips, help and many messages, the civilizations'
traits (keyed by name in `src/traits.rs`), and the rules themselves, which are written against
specific units and buildings.

## Art
All sprites are original and generated from code: `tools/art/*.cjs` describe each sprite as SVG
and `node tools/art/generate.cjs` rasterises them into the atlases in `assets/sprites/`
(needs Playwright with Chromium). Fonts are Cinzel and Alegreya Sans (SIL Open Font License,
see `assets/fonts/`).

## Sound
Sound effects play when you select a unit or city, order troops, ships or aircraft to move, a fight
breaks out in sight, and on interface buttons. They are synthesised from code:
`node tools/audio/generate.cjs` writes the small WAV files in `assets/sounds/` (no dependencies).

Players switch sound off with **V** or the Sound button; the choice is saved in `sound.json` (or the
browser's localStorage). To build without audio support at all (no ALSA needed on Linux), turn off
the default `audio` feature: `cargo run --no-default-features --features full`.

## Autoplay
`cargo run --release -- --autoplay [rome|earth|discovery] [turns]` plays a whole game between
computer players under the real rules, without a window, and prints a snapshot every 25 turns plus
the milestones (ages, wonders, governments, golden ages, spaceships) and how it ends, with the
technologies the computer players traded and the largest peoples' shares of the world's cities,
land and citizens. With `BEVY_CIV_CULTURE_TRACE=1` it also prints the leading peoples' culture
(and the best city's) every five turns, for tuning the cultural victory. The Game Simulator
workflow runs all three scenarios to 400 turns on demand. Across recent batches of 18
games (six per scenario), the space race, culture and diplomacy each win a share, mostly between
turns 250 and 380; a few games go to the turn limit and are decided on score.

## Balance
`cargo run --release --no-default-features --features full --bin bevy-civ -- --balance [games] [turns] [directory]`
plays many New World games between computer players under the real rules, and writes each people's
win share, the kind of victory and the turn the game ended. See [docs/BALANCE.md](docs/BALANCE.md).
Seats are shuffled (every people the same chance of a seat and of moving first; `--fixed-seats`
turns that off), `--seed N` rebuilds the same worlds, `--expansions` adds the packs' peoples and
`--scenario ID` measures a fixed scenario. A two-game smoke is `scripts/balance-smoke.sh`: it
reports win share against a 45-55% fair band and does not treat an imbalance as a failure.

## Simulator
A standalone dice model with no Bevy dependency and none of the game's rules. It is not a balance
result; use [Balance](#balance) for that.

```bash
cd simulate && cargo run -- --games 100 --max-turns 120
```

### CI
The Game Simulator workflow (`.github/workflows/simulate.yml`) runs on demand: Actions, Game
Simulator, Run workflow (choose how many games and turns). The Balance harness workflow
(`.github/workflows/balance.yml`) runs a short real-rules smoke the same way. The fair-band report
does not fail that job.

## Running (native)
```bash
cargo run
```

## Play in the browser (Cloudflare)
The game compiles to WebAssembly and is served by a static-assets Cloudflare Worker (`wrangler.jsonc`).

One-time tooling: `rustup target add wasm32-unknown-unknown`, a `wasm-bindgen` CLI matching the
version in `Cargo.lock`, and `npm install`.

```bash
npm run build    # builds ./dist (scripts/build-web.sh)
npm run dev      # serve locally at http://localhost:8787
npm run deploy   # build + wrangler deploy (needs `wrangler login` or an API token)
```

`.github/workflows/deploy.yml` builds and deploys on every push to `main` (or on demand). It needs two
repository secrets: `CLOUDFLARE_API_TOKEN` (the "Edit Cloudflare Workers" token template) and
`CLOUDFLARE_ACCOUNT_ID`. The game is then live at `https://bevy-civ.<your-subdomain>.workers.dev`.

## Players, sign-ups and statistics
The page opens on a splash screen (`web/splash.jpg`, drawn by `node tools/art/splash.cjs`) that
stays while the game loads; **Play** takes the player in. Nobody needs an account. Two things are
offered, both optional and folded out of the way:
- **News of our next games**: an email, interests to tick (strategy, history, sci-fi, fantasy,
  mobile, multiplayer) and a consent box whose exact wording is stored with the sign-up.
- **Anonymous play statistics**: a box to tick. Only then does the page send a random player id
  (kept in the browser), the campaign tags (`utm_*`) and referrer it arrived with, and the
  milestones the game reports through `window.bevyCivEvent(kind, scenario, turn, detailJson)`
  (`src/telemetry.rs`): a game starting or resuming, every 25th turn, and how it ended - with the
  civilization, difficulty, city count, score and government.

The Worker in `worker/index.js` serves the game and an API under `/api/` (`hello`, `subscribe`,
`stats`, `forget`), storing into the D1 database `bevy-civ-players` (schema in `migrations/`).
`web/privacy.html` explains it and lets a player delete their statistics.

Export the list, e.g. as JSON to convert for a mailing tool:
```
npx wrangler d1 execute bevy-civ-players --remote --json \
  --command "SELECT email, interests, utm_source, utm_campaign, country, created_at FROM signups WHERE unsubscribed_at IS NULL"
```
Local testing: `npx wrangler d1 migrations apply bevy-civ-players --local`, then
`npx wrangler dev --local` after `npm run build`.

## License
Released under the [MIT License](LICENSE). That covers the code and the art, sounds, music and
scenario files made for this game (the generators are in `tools/`).

Not covered by that choice, and keeping their own licenses:
- the fonts in `assets/fonts/` (Cinzel and Alegreya Sans), under the SIL Open Font License (texts
  alongside them);
- the Natural Earth coastlines the maps are built from (public domain);
- the Rust and npm dependencies (Bevy, hexx and the rest), each under its own license.

Bevy Civ is an independent project inspired by the Civilization series. It is not affiliated with or
endorsed by Firaxis Games, 2K or Take-Two Interactive, and "Civilization" is their trademark.
