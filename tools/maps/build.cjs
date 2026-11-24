#!/usr/bin/env node
// Builds the scenario files in assets/scenarios/ from real coastlines.
//
// Land outlines come from Natural Earth (public domain). The data file is not kept in the
// repository; pass its path or let the script download it:
//   node tools/maps/build.cjs [path/to/ne_50m_land.geojson]
const fs = require('fs');
const path = require('path');
const os = require('os');
const { execSync } = require('child_process');
const expansions = require('./expansions.cjs');
const scenarios = [...require('./scenarios.cjs'), ...expansions.scenarios];

const SOURCE = 'https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_50m_land.geojson';

function loadLand() {
  let file = process.argv[2];
  if (!file) {
    file = path.join(os.tmpdir(), 'ne_50m_land.geojson');
    if (!fs.existsSync(file)) execSync(`curl -sSL -o "${file}" "${SOURCE}"`);
  }
  const polygons = [];
  for (const feature of JSON.parse(fs.readFileSync(file, 'utf8')).features) {
    const g = feature.geometry;
    for (const poly of g.type === 'Polygon' ? [g.coordinates] : g.coordinates) {
      const ring = poly[0];
      let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
      for (const [x, y] of ring) {
        x0 = Math.min(x0, x); x1 = Math.max(x1, x); y0 = Math.min(y0, y); y1 = Math.max(y1, y);
      }
      polygons.push({ ring, x0, y0, x1, y1 });
    }
  }
  return polygons;
}

function isLand(polygons, lon, lat) {
  if (lon > 180) lon -= 360;
  for (const p of polygons) {
    if (lon < p.x0 || lon > p.x1 || lat < p.y0 || lat > p.y1) continue;
    let inside = false;
    const r = p.ring;
    for (let i = 0, j = r.length - 1; i < r.length; j = i++) {
      if (r[i][1] > lat !== r[j][1] > lat && lon < ((r[j][0] - r[i][0]) * (lat - r[i][1])) / (r[j][1] - r[i][1]) + r[i][0]) inside = !inside;
    }
    if (inside) return true;
  }
  return false;
}

const mercator = (lat) => Math.log(Math.tan(Math.PI / 4 + (lat * Math.PI) / 360));
const unmercator = (y) => (Math.atan(Math.exp(y)) * 360) / Math.PI - 90;
const hash = (a, b) => {
  let h = (a * 374761393 + b * 668265263) | 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
};

// An equal-area projection (Behrmann: true scale at 30 degrees): every region gets land in
// proportion to its real area, where Mercator swells the north and shrinks the tropics.
const STANDARD = Math.cos((30 * Math.PI) / 180) ** 2;
const equal = (lat) => Math.sin((lat * Math.PI) / 180) / STANDARD;
const unequal = (y) => (Math.asin(Math.max(-1, Math.min(1, y * STANDARD))) * 180) / Math.PI;

/** Map projection onto an odd-q offset hex grid (odd columns sit half a row lower): Mercator, or
 * equal-area with `equalArea: true`. */
function projection({ lon0, lon1, latTop, latBottom, cols, equalArea }) {
  const merc = equalArea ? equal : mercator;
  const unmerc = equalArea ? unequal : unmercator;
  const top = merc(latTop);
  const bottom = merc(latBottom);
  const lonSpan = lon1 - lon0;
  // One column is 1.5 hex radii wide, one row sqrt(3) radii tall; keep the map conformal.
  const radiansPerRadius = ((lonSpan * Math.PI) / 180) / (1.5 * cols);
  const rows = Math.round((top - bottom) / (Math.sqrt(3) * radiansPerRadius));
  const toLonLat = (col, row, dx = 0, dy = 0) => {
    const x = (col + 0.5 + dx) / cols;
    const y = (row + 0.5 + (col % 2 ? 0.5 : 0) + dy) / rows;
    return [lon0 + x * lonSpan, unmerc(top - y * (top - bottom))];
  };
  const toCell = (lon, lat) => {
    if (lon < lon0) lon += 360;
    const col = Math.min(cols - 1, Math.max(0, Math.floor(((lon - lon0) / lonSpan) * cols)));
    const y = ((top - merc(lat)) / (top - bottom)) * rows - (col % 2 ? 0.5 : 0);
    return [col, Math.min(rows - 1, Math.max(0, Math.floor(y)))];
  };
  return { cols, rows, toLonLat, toCell };
}

// ---- climate -----------------------------------------------------------------------------------
const inBox = (lon, lat, [x0, x1, y0, y1]) => lon >= x0 && lon <= x1 && lat >= y0 && lat <= y1;
const DESERTS = [
  [-16, 34, 17, 30.5], [34, 59, 13, 31], [50, 66, 31, 44], [67, 75, 24, 30], [76, 111, 37, 46],
  [116, 142, -30, -19], [12, 25, -29, -18], [-72, -68, -28, -16], [-117, -103, 26, 36], [-70, -64, -48, -38],
];
const JUNGLES = [[-76, -48, -13, 5], [8, 29, -5, 5], [-12, 9, 4, 8.5], [93, 152, -10, 19], [-92, -77, 7, 18], [72, 78, 8, 20]];
// [points along the crest, reach in degrees, terrain at the crest]
const RANGES = [
  [[[73, 36], [80, 31.5], [88, 28], [97, 28]], 2.6, 'm'], // Himalaya
  [[[78, 34], [90, 33], [100, 32]], 5, 'h'], // Tibet
  [[[-77, 8], [-78, -2], [-76, -10], [-70, -17], [-69, -28], [-71, -40], [-73, -52]], 2.2, 'm'], // Andes
  [[[-150, 62], [-135, 60], [-125, 54], [-116, 48], [-110, 42], [-106, 36]], 3, 'm'], // Rockies
  [[[-122, 47], [-120, 38], [-116, 33]], 1.6, 'h'],
  [[[-106, 28], [-101, 20], [-97, 17]], 1.8, 'h'], // Sierra Madre
  [[[-84, 34], [-79, 39], [-72, 44]], 1.4, 'h'], // Appalachians
  [[[5.5, 44.5], [9, 46.3], [14, 47]], 1.3, 'm'], // Alps
  [[[-1.5, 42.8], [2, 42.5]], 0.8, 'm'], // Pyrenees
  [[[18, 49], [24, 47.5], [26, 45.5], [22, 45]], 1.1, 'h'], // Carpathians
  [[[19, 43.5], [21, 41], [22, 39]], 1.0, 'h'], // Dinarides / Pindus
  [[[10, 44], [13.5, 42.5], [16, 40]], 0.7, 'h'], // Apennines
  [[[38, 43.6], [44, 42.6], [48, 41]], 1.2, 'm'], // Caucasus
  [[[30, 37], [36, 37.5], [42, 38.5]], 1.2, 'h'], // Taurus
  [[[45, 37], [50, 32.5], [56, 28.5]], 1.8, 'm'], // Zagros
  [[[52, 36.5], [58, 36.5]], 1.0, 'h'], // Alborz
  [[[66, 35], [72, 36.5]], 2, 'm'], // Hindu Kush
  [[[59, 51], [59.5, 60], [62, 67]], 1.5, 'h'], // Urals
  [[[84, 50], [92, 48], [98, 46]], 2.2, 'm'], // Altai
  [[[72, 42], [80, 42]], 1.8, 'm'], // Tian Shan
  [[[-8, 31], [-2, 33.5], [6, 35.5]], 1.2, 'h'], // Atlas
  [[[36, 13], [39, 9], [38, 5]], 2.4, 'h'], // Ethiopian highlands
  [[[29, -2], [31, -9], [35, -13]], 1.6, 'h'], // Rift
  [[[27, -31], [30, -28]], 1.5, 'h'], // Drakensberg
  [[[6, 60], [13, 64], [19, 68]], 1.6, 'h'], // Scandinavian mountains
  [[[146, -37], [150, -32], [152, -26]], 1.3, 'h'], // Great Dividing Range
  [[[136, 35], [139.5, 36.5], [141, 40]], 0.9, 'h'], // Japan
  [[[158, 53], [161, 57]], 1.5, 'm'], // Kamchatka
  [[[169, -44.5], [172, -42]], 1, 'm'], // New Zealand
  [[[101, 30], [104, 26]], 2.2, 'h'], // Hengduan
  [[[-45, 62], [-42, 70], [-38, 78]], 9, 'm'], // Greenland ice
];

function distToSegment(px, py, [ax, ay], [bx, by]) {
  const dx = bx - ax;
  const dy = by - ay;
  const t = Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy || 1)));
  return Math.hypot(px - ax - t * dx, py - ay - t * dy);
}

function terrainAt(lon, lat, col, row) {
  if (lon > 180) lon -= 360;
  const noise = hash(col, row);
  for (const [points, reach, kind] of RANGES) {
    let d = Infinity;
    for (let i = 0; i + 1 < points.length; i++) d = Math.min(d, distToSegment(lon, lat, points[i], points[i + 1]));
    if (d < reach * 0.55) return kind === 'm' && noise < 0.8 ? 'm' : 'h';
    if (d < reach && noise < 0.7) return 'h';
  }
  if (DESERTS.some((b) => inBox(lon, lat, b))) return noise < 0.88 ? 'd' : 'p';
  if (JUNGLES.some((b) => inBox(lon, lat, b))) return noise < 0.8 ? 'f' : 'g';
  const a = Math.abs(lat);
  if (a > 67) return noise < 0.5 ? 'h' : 'p'; // tundra
  if (a > 52) return noise < 0.68 ? 'f' : noise < 0.85 ? 'p' : 'g'; // taiga
  if (a > 32) return noise < 0.45 ? 'g' : noise < 0.72 ? 'p' : noise < 0.94 ? 'f' : 'h'; // temperate
  if (a > 12) return noise < 0.55 ? 'p' : noise < 0.85 ? 'g' : noise < 0.95 ? 'f' : 'h'; // savanna
  return noise < 0.5 ? 'g' : noise < 0.85 ? 'f' : 'p'; // tropics
}

const maps = new Map();
function buildMap(polygons, spec) {
  const key = JSON.stringify(spec);
  if (!maps.has(key)) maps.set(key, buildMapUncached(polygons, spec));
  return maps.get(key);
}

function buildMapUncached(polygons, spec) {
  const proj = projection(spec);
  const samples = [[0, 0], [-0.28, -0.28], [0.28, -0.28], [-0.28, 0.28], [0.28, 0.28]];
  const grid = [];
  for (let row = 0; row < proj.rows; row++) {
    let line = '';
    for (let col = 0; col < proj.cols; col++) {
      const hits = samples.filter(([dx, dy]) => isLand(polygons, ...proj.toLonLat(col, row, dx, dy))).length;
      const [lon, lat] = proj.toLonLat(col, row);
      line += hits >= (spec.landThreshold || 3) ? terrainAt(lon, lat, col, row) : '~';
    }
    grid.push(line);
  }
  return { proj, grid };
}

// ---- scenario assembly -------------------------------------------------------------------------
const NAVAL = new Set(['Galley', 'Frigate', 'Galleon', 'Ironclad', 'Destroyer', 'Battleship', 'Submarine', 'Transport', 'Carrier', 'Man-O-War']);
function neighbours(col, row) {
  const odd = col % 2;
  return [[col, row - 1], [col, row + 1], [col - 1, row - 1 + odd], [col - 1, row + odd], [col + 1, row - 1 + odd], [col + 1, row + odd]];
}
function hexDistance(a, b) {
  const cube = ([col, row]) => { const q = col; const r = row - (col - (col & 1)) / 2; return [q, r, -q - r]; };
  const [x, y, z] = cube(a); const [u, v, w] = cube(b);
  return Math.max(Math.abs(x - u), Math.abs(y - v), Math.abs(z - w));
}

function assemble(def, polygons) {
  const { proj, grid } = buildMap(polygons, def.map);
  const rows = grid.map((line) => line.split(''));
  const land = (c, r) => r >= 0 && r < proj.rows && c >= 0 && c < proj.cols && rows[r][c] !== '~';
  const cityCells = [];
  // Snap a site to the nearest land tile that is not crowding an earlier city.
  const place = (lon, lat, spacing) => {
    const start = proj.toCell(lon, lat);
    const seen = new Set([start.join()]);
    const queue = [start];
    while (queue.length) {
      const cell = queue.shift();
      if (land(...cell) && cityCells.every((c) => hexDistance(c, cell) >= spacing)) return cell;
      for (const n of neighbours(...cell)) {
        if (n[0] < 0 || n[1] < 0 || n[0] >= proj.cols || n[1] >= proj.rows || seen.has(n.join())) continue;
        seen.add(n.join());
        queue.push(n);
      }
    }
    throw new Error(`no land near ${lon},${lat}`);
  };
  const civs = def.civs.map((civ) => {
    const cities = civ.cities.map(([name, lon, lat, pop], i) => {
      const cell = place(lon, lat, 2);
      cityCells.push(cell);
      // City sites are always workable land.
      if ('mhd'.includes(rows[cell[1]][cell[0]])) rows[cell[1]][cell[0]] = 'p';
      return { name, col: cell[0], row: cell[1], pop: pop || 1, capital: i === 0 };
    });
    const units = [];
    // A unit's city is an index, or 'coast' for the first city by the sea (a navy's home port).
    const coastal = cities.findIndex((c) => neighbours(c.col, c.row).some(([nc, nr]) => nr >= 0 && nr < proj.rows && nc >= 0 && nc < proj.cols && rows[nr][nc] === '~'));
    (civ.units || []).forEach(([type, count, cityIndex]) => {
      if (cityIndex === 'coast' && coastal < 0) return;
      const home = cities[cityIndex === 'coast' ? coastal : cityIndex || 0];
      const spots = [[home.col, home.row], ...neighbours(home.col, home.row).filter((n) => land(...n))];
      // Ships start in port (the city itself); troops spread over the land around it.
      const naval = NAVAL.has(type);
      for (let i = 0; i < count; i++) {
        const [col, row] = naval ? [home.col, home.row] : spots[(units.length + 1) % spots.length];
        units.push({ type, col, row });
      }
    });
    return {
      name: civ.name, adjective: civ.adjective, leader: civ.leader, portrait: civ.portrait,
      aggression: civ.aggression, gold: civ.gold || 50, techs: civ.techs || [], city_names: civ.cityNames || [],
      cities, units, ...(civ.government ? { government: civ.government } : {}),
    };
  });
  return {
    id: def.id, name: def.name, blurb: def.blurb, start_year: def.startYear, years_per_turn: def.yearsPerTurn,
    width: proj.cols, height: proj.rows, rows: rows.map((r) => r.join('')), civs, wars: def.wars || [], known: !!def.known,
    ...(def.alliances ? { alliances: def.alliances } : {}), ...(def.expansion ? { expansion: def.expansion } : {}),
    ...(def.objectives ? { objectives: def.objectives } : {}), ...(def.developed ? { developed: true } : {}), ...(def.wonders ? { wonders: def.wonders } : {}), ...(def.researchCost ? { research_cost: def.researchCost } : {}),
  };
}

const polygons = loadLand();
const outDir = path.join(__dirname, '..', '..', 'assets', 'scenarios');
fs.mkdirSync(outDir, { recursive: true });
// ONLY=a,b builds just those scenarios (and the expansion packs); hand-tuned files stay as they are.
const only = process.env.ONLY ? process.env.ONLY.split(',') : null;
for (const def of scenarios) {
  if (only && !only.includes(def.id)) continue;
  const scenario = assemble(def, polygons);
  fs.writeFileSync(path.join(outDir, `${def.id}.json`), JSON.stringify(scenario));
  console.log(`${def.id}: ${scenario.width}x${scenario.height}, ${scenario.civs.length} civilizations`);
  if (process.env.SHOW) console.log(scenario.rows.join('\n'));
}

// Expansion packs: each people's home on the Whole Earth map, placed clear of the Earth peoples
// (all but the one it replaces) so it can join that game.
const earth = scenarios.find((s) => s.id === 'earth');
const packDir = path.join(__dirname, '..', '..', 'assets', 'expansions');
fs.mkdirSync(packDir, { recursive: true });
for (const pack of expansions.packs) {
  const peoples = pack.peoples.map((p) => {
    const civ = { ...p, cities: [p.home], units: [['Settler', 1, 0], ['Worker', 1, 0], ['Warrior', 1, 0]] };
    const placed = assemble({ ...earth, civs: [...earth.civs.filter((c) => c.name !== p.replaces), civ] }, polygons).civs.at(-1);
    return { ...placed, ...(p.replaces ? { replaces: p.replaces } : {}) };
  });
  const out = { id: pack.id, name: pack.name, blurb: pack.blurb, peoples };
  fs.writeFileSync(path.join(packDir, `${pack.id}.json`), JSON.stringify(out, null, 1) + '\n');
  console.log(`pack ${pack.id}: ${peoples.map((p) => `${p.name} (${p.cities[0].col},${p.cities[0].row})`).join(', ')}`);
}
