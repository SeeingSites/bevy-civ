// Cities (four sizes in each of the four ages), waving flags and combat effects.
const { rng, between, n, nextId, sheet } = require('./lib.cjs');

const OUT = '#231a12';
const wrap = (inner) => `<g stroke="${OUT}" stroke-width="2" stroke-linejoin="round" stroke-linecap="round">${inner}</g>`;

function house(x, y, w, h, wall, roof) {
  const rh = w * 0.45;
  return (
    `<path d="M${n(x)},${n(y)} v${n(-h)} h${n(w)} v${n(h)} Z" fill="${wall}"/>` +
    `<path d="M${n(x + w * 0.62)},${n(y)} v${n(-h)} h${n(w * 0.38)} v${n(h)} Z" fill="rgba(0,0,0,0.16)" stroke="none"/>` +
    `<path d="M${n(x - 3)},${n(y - h)} L${n(x + w / 2)},${n(y - h - rh)} L${n(x + w + 3)},${n(y - h)} Z" fill="${roof}"/>` +
    `<path d="M${n(x + w * 0.4)},${n(y)} v${n(-h * 0.5)} h${n(w * 0.22)} v${n(h * 0.5)}" fill="#4a3220" stroke-width="1.3"/>`
  );
}

function tower(x, y, w, h, wall, roof) {
  let s = `<path d="M${n(x)},${n(y)} v${n(-h)} h${n(w)} v${n(h)} Z" fill="${wall}"/><path d="M${n(x + w * 0.6)},${n(y)} v${n(-h)} h${n(w * 0.4)} v${n(h)} Z" fill="rgba(0,0,0,0.18)" stroke="none"/>`;
  if (roof) s += `<path d="M${n(x - 3)},${n(y - h)} L${n(x + w / 2)},${n(y - h - w * 0.9)} L${n(x + w + 3)},${n(y - h)} Z" fill="${roof}"/>`;
  else s += `<path d="M${n(x - 2)},${n(y - h)} v-6 h${n(w / 4)} v4 h${n(w / 4)} v-4 h${n(w / 4 + 1)} v4 h${n(w / 4)} v-4 h3 v6 Z" fill="${wall}"/>`;
  return s + `<path d="M${n(x + w / 2 - 2)},${n(y - h * 0.62)} v6" stroke-width="3"/>`;
}

function rampart(x1, x2, y, h, wall) {
  let top = `M${x1},${y - h} `;
  for (let x = x1; x < x2; x += 12) top += `v-5 h6 v5 h6 `;
  return (
    `<path d="M${x1},${y} v${-h} H${x2} v${h} Z" fill="${wall}"/><path d="${top}" fill="none"/>` +
    `<path d="M${(x1 + x2) / 2 - 9},${y} v-12 a9,9 0 0 1 18,0 v12 Z" fill="#3a2717"/>`
  );
}

function medievalCity(col) {
  const r = rng(900 + col * 31);
  const gy = 158;
  const walls = ['#e6d6b4', '#d9c7a0', '#efe2c4'];
  const roofs = ['#b5482f', '#a23f2a', '#c25a36', '#8c5a34'];
  const wall = () => walls[Math.floor(r() * walls.length)];
  const roof = () => roofs[Math.floor(r() * roofs.length)];
  let s = `<ellipse cx="96" cy="${gy + 2}" rx="${52 + col * 9}" ry="${16 + col * 2}" fill="rgba(0,0,0,0.3)"/>`;
  s += `<ellipse cx="96" cy="${gy - 2}" rx="${50 + col * 9}" ry="${15 + col * 2}" fill="#b89d6c" stroke="#6f5a3a" stroke-width="2"/>`;
  const parts = [];
  const add = (y, svg) => parts.push([y, svg]);
  if (col === 0) {
    // village: huts and a palisade
    add(gy - 20, house(58, gy - 20, 26, 16, wall(), '#9a7a44'));
    add(gy - 12, house(98, gy - 12, 30, 18, wall(), '#a88a4c'));
    add(gy - 4, house(70, gy - 4, 24, 14, wall(), '#9a7a44'));
    add(gy - 2, `<path d="M40,${gy - 2} v-12 M48,${gy + 2} v-12 M140,${gy + 2} v-12 M148,${gy - 2} v-12" stroke="#6b4a2a" stroke-width="4"/>`);
  } else if (col === 1) {
    add(gy - 34, tower(88, gy - 34, 16, 34, '#cfc3ad', '#7a3a2a'));
    add(gy - 26, house(50, gy - 26, 28, 18, wall(), roof()));
    add(gy - 24, house(112, gy - 24, 30, 20, wall(), roof()));
    add(gy - 10, house(64, gy - 10, 26, 16, wall(), roof()));
    add(gy - 8, house(100, gy - 8, 28, 18, wall(), roof()));
    add(gy + 2, house(82, gy + 2, 24, 14, wall(), roof()));
  } else if (col === 2) {
    add(gy - 50, tower(84, gy - 44, 22, 50, '#d4c9b5', '#6f3526'));
    add(gy - 36, house(46, gy - 34, 28, 22, wall(), roof()));
    add(gy - 34, house(116, gy - 32, 30, 24, wall(), roof()));
    add(gy - 22, house(62, gy - 20, 26, 18, wall(), roof()));
    add(gy - 20, house(98, gy - 18, 30, 20, wall(), roof()));
    add(gy - 6, rampart(30, 162, gy + 2, 16, '#bdb29d'));
    add(gy - 5, tower(24, gy + 4, 14, 30, '#c9bea9', null));
    add(gy - 5, tower(154, gy + 4, 14, 30, '#c9bea9', null));
  } else {
    // metropolis: keep, temple dome and tall walls
    add(gy - 70, tower(82, gy - 52, 28, 64, '#ddd3c0', '#5f2f22'));
    add(gy - 60, `<path d="M118,${gy - 44} v-22 a16,16 0 0 1 32,0 v22 Z" fill="#e9e0cc"/><path d="M118,${gy - 66} a16,16 0 0 1 32,0 Z" fill="#d8b24a"/><path d="M134,${gy - 82} v-9" stroke-width="2.5"/>`);
    add(gy - 56, tower(38, gy - 40, 18, 46, '#d4c9b5', '#6f3526'));
    add(gy - 40, house(54, gy - 34, 30, 24, wall(), roof()));
    add(gy - 38, house(110, gy - 30, 34, 24, wall(), roof()));
    add(gy - 24, house(40, gy - 18, 28, 20, wall(), roof()));
    add(gy - 22, house(76, gy - 16, 30, 22, wall(), roof()));
    add(gy - 20, house(118, gy - 14, 30, 20, wall(), roof()));
    add(gy - 4, rampart(22, 170, gy + 6, 20, '#bdb29d'));
    add(gy - 3, tower(14, gy + 8, 16, 38, '#c9bea9', null));
    add(gy - 3, tower(162, gy + 8, 16, 38, '#c9bea9', null));
  }
  parts.sort((a, b) => a[0] - b[0]);
  return s + wrap(parts.map((p) => p[1]).join(''));
}

// ---- cities of the other ages -------------------------------------------------------------------
// Each age draws its own buildings on the same ground: four sizes, from a village to a metropolis.

function ground(col, gy) {
  return `<ellipse cx="96" cy="${gy + 2}" rx="${52 + col * 9}" ry="${16 + col * 2}" fill="rgba(0,0,0,0.3)"/>` +
    `<ellipse cx="96" cy="${gy - 2}" rx="${50 + col * 9}" ry="${15 + col * 2}" fill="#b89d6c" stroke="#6f5a3a" stroke-width="2"/>`;
}

/** A mud-brick house with a flat roof and a dark doorway. */
function adobe(x, y, w, h, wall) {
  return `<path d="M${n(x)},${n(y)} v${n(-h)} h${n(w)} v${n(h)} Z" fill="${wall}"/>` +
    `<path d="M${n(x - 2)},${n(y - h)} h${n(w + 4)} v-4 h${n(-w - 4)} Z" fill="#a8835a"/>` +
    `<path d="M${n(x + w * 0.62)},${n(y)} v${n(-h)} h${n(w * 0.38)} v${n(h)} Z" fill="rgba(0,0,0,0.16)" stroke="none"/>` +
    `<path d="M${n(x + w * 0.3)},${n(y)} v${n(-h * 0.5)} h${n(w * 0.22)} v${n(h * 0.5)}" fill="#4a3220" stroke-width="1.3"/>`;
}

/** A stepped temple (ziggurat) with a shrine on top. */
function ziggurat(x, y, w, steps, colour) {
  let s = '';
  for (let i = 0; i < steps; i++) {
    const sw = w - i * (w / (steps + 1));
    s += `<path d="M${n(x - sw / 2)},${n(y - i * 12)} v-12 h${n(sw)} v12 Z" fill="${colour}"/>`;
  }
  const top = y - steps * 12;
  return s + `<path d="M${n(x - 8)},${n(top)} v-12 h16 v12 Z" fill="#e8d9b8"/><path d="M${n(x - 10)},${n(top - 12)} h20 l-10,-8 Z" fill="#c9a24a"/>` +
    `<path d="M${n(x - 3)},${n(y)} v${-steps * 12}" stroke="rgba(0,0,0,0.25)" stroke-width="3"/>`;
}

/** A columned temple front. */
function colonnade(x, y, w, h) {
  let s = `<path d="M${n(x)},${n(y)} h${n(w)} v-5 h${n(-w)} Z" fill="#e9e0cc"/>`;
  for (let cx = x + 4; cx < x + w - 2; cx += 8) s += `<path d="M${n(cx)},${n(y - 5)} v${n(-h)} h4 v${n(h)} Z" fill="#f1ead8"/>`;
  return s + `<path d="M${n(x - 3)},${n(y - h - 5)} h${n(w + 6)} l${n(-(w + 6) / 2)},-12 Z" fill="#e9e0cc"/>`;
}

function ancientCity(col) {
  const r = rng(700 + col * 37);
  const gy = 158;
  const walls = ['#d8b98a', '#cfae7c', '#e0c497'];
  const wall = () => walls[Math.floor(r() * walls.length)];
  const parts = [];
  const add = (y, svg) => parts.push([y, svg]);
  const houses = [[[56, -18], [96, -10], [72, -2]], [[48, -24], [110, -22], [64, -8], [100, -6], [82, 2]],
    [[44, -32], [118, -30], [60, -18], [100, -16], [80, -2], [130, -6]], [[36, -34], [126, -34], [52, -20], [110, -20], [70, -6], [96, -4], [138, -8], [44, -4]]][col];
  for (const [x, dy] of houses) add(gy + dy, adobe(x, gy + dy, 22 + r() * 8, 14 + r() * 6, wall()));
  if (col >= 2) add(gy - 40, ziggurat(92, gy - 30, 54 + col * 6, col + 1, '#c9a57a'));
  if (col === 3) add(gy - 36, colonnade(126, gy - 34, 34, 18));
  if (col === 1) add(gy - 26, colonnade(80, gy - 24, 26, 14));
  // A palisade of stakes, then (larger towns) a mud-brick wall.
  if (col <= 1) add(gy + 4, [40, 48, 56, 136, 144, 152].map((x) => `<path d="M${x},${gy + 6} v-12" stroke="#6b4a2a" stroke-width="4"/>`).join(''));
  else add(gy + 2, `<path d="M${30 - col * 4},${gy + 8} v-14 H${162 + col * 4} v14 Z" fill="#c4a173"/>` + `<path d="M${30 - col * 4},${gy - 6} ${Array.from({ length: 12 }, () => 'h6 v-4 h6 v4').join(' ')}" fill="none"/>`);
  parts.sort((a, b) => a[0] - b[0]);
  return ground(col, gy) + wrap(parts.map((p) => p[1]).join(''));
}

/** A brick building with a slate roof and rows of windows. */
function brick(x, y, w, h, wall, floors) {
  let s = `<path d="M${n(x)},${n(y)} v${n(-h)} h${n(w)} v${n(h)} Z" fill="${wall}"/>` +
    `<path d="M${n(x - 2)},${n(y - h)} L${n(x + 4)},${n(y - h - 8)} h${n(w - 8)} L${n(x + w + 2)},${n(y - h)} Z" fill="#4a4f58"/>`;
  for (let f = 0; f < floors; f++) {
    for (let wx = x + 4; wx < x + w - 4; wx += 8) s += `<rect x="${n(wx)}" y="${n(y - h + 5 + f * 10)}" width="4" height="5" fill="#f2d98a" stroke-width="1"/>`;
  }
  return s + `<path d="M${n(x + w * 0.65)},${n(y)} v${n(-h)} h${n(w * 0.35)} v${n(h)} Z" fill="rgba(0,0,0,0.14)" stroke="none"/>`;
}

/** A factory: a saw-tooth roofed shed and a tall chimney trailing smoke. */
function factory(x, y, w, chimney) {
  let s = `<path d="M${n(x)},${n(y)} v-18 l10,-8 v8 l10,-8 v8 l10,-8 v8 h${n(w - 30)} v18 Z" fill="#8a4a36"/>` +
    `<path d="M${n(x + w - 10)},${n(y - 18)} v${-chimney} h7 v${chimney} Z" fill="#6b3a2a"/>`;
  const cx = x + w - 6;
  const top = y - 18 - chimney;
  s += `<g stroke="none"><circle cx="${n(cx + 3)}" cy="${n(top - 7)}" r="6" fill="rgba(120,115,110,0.75)"/><circle cx="${n(cx + 11)}" cy="${n(top - 15)}" r="8" fill="rgba(140,135,130,0.6)"/><circle cx="${n(cx + 22)}" cy="${n(top - 21)}" r="9" fill="rgba(160,155,150,0.45)"/></g>`;
  return s;
}

/** A clock tower. */
function clockTower(x, y, h) {
  return `<path d="M${n(x)},${n(y)} v${-h} h14 v${h} Z" fill="#b0886a"/><path d="M${n(x - 2)},${n(y - h)} l9,-14 l9,14 Z" fill="#4a4f58"/>` +
    `<circle cx="${n(x + 7)}" cy="${n(y - h + 9)}" r="5" fill="#f1ead8" stroke-width="1.5"/><path d="M${n(x + 7)},${n(y - h + 9)} v-3 M${n(x + 7)},${n(y - h + 9)} h2" stroke-width="1.2"/>`;
}

function industrialCity(col) {
  const r = rng(500 + col * 41);
  const gy = 158;
  const walls = ['#a85a44', '#9a4f3c', '#b86a4e', '#c9b9a0'];
  const wall = () => walls[Math.floor(r() * walls.length)];
  const parts = [];
  const add = (y, svg) => parts.push([y, svg]);
  const blocks = [[[60, -16, 1], [96, -8, 1]], [[46, -24, 2], [108, -22, 2], [70, -6, 1]],
    [[40, -32, 2], [116, -30, 3], [62, -16, 2], [96, -14, 2], [80, 0, 1]], [[34, -36, 3], [124, -34, 3], [56, -20, 2], [104, -20, 3], [74, -4, 2], [132, -6, 1]]][col];
  for (const [x, dy, floors] of blocks) add(gy + dy, brick(x, gy + dy, 26 + r() * 8, 10 + floors * 10, wall(), floors));
  if (col >= 1) add(gy - 30 - col, factory(col === 1 ? 124 : 128, gy - 26, 40, 22 + col * 8));
  if (col >= 2) add(gy - 44, clockTower(84, gy - 38, 34 + col * 6));
  if (col === 3) add(gy - 40, factory(18, gy - 30, 38, 40));
  parts.sort((a, b) => a[0] - b[0]);
  return ground(col, gy) + wrap(parts.map((p) => p[1]).join(''));
}

/** A tower of glass and concrete with a grid of lit windows. */
function skyscraper(x, y, w, h, glass) {
  let s = `<path d="M${n(x)},${n(y)} v${n(-h)} h${n(w)} v${n(h)} Z" fill="${glass}"/>`;
  for (let wy = y - h + 5; wy < y - 4; wy += 7) {
    for (let wx = x + 3; wx < x + w - 3; wx += 6) s += `<rect x="${n(wx)}" y="${n(wy)}" width="3" height="4" fill="rgba(255,236,170,0.85)" stroke="none"/>`;
  }
  return s + `<path d="M${n(x + w * 0.6)},${n(y)} v${n(-h)} h${n(w * 0.4)} v${n(h)} Z" fill="rgba(0,0,0,0.18)" stroke="none"/>` +
    `<path d="M${n(x + w / 2)},${n(y - h)} v-8" stroke-width="1.5"/>`;
}

function modernCity(col) {
  const r = rng(300 + col * 43);
  const gy = 158;
  const glass = ['#5f86a8', '#7d9bb5', '#4f6f8f', '#9fb3c4', '#c9ccd1'];
  const tone = () => glass[Math.floor(r() * glass.length)];
  const parts = [];
  const add = (y, svg) => parts.push([y, svg]);
  const towers = [[[66, -12, 30], [100, -6, 22]], [[50, -22, 44], [104, -20, 56], [76, -4, 30], [124, -4, 24]],
    [[40, -30, 60], [118, -30, 74], [64, -14, 44], [96, -16, 90], [140, -8, 34], [80, 0, 28]],
    [[30, -34, 70], [138, -34, 82], [58, -22, 96], [92, -24, 118], [118, -14, 64], [44, -4, 40], [74, 0, 50], [150, -2, 36]]][col];
  for (const [x, dy, h] of towers) add(gy + dy, skyscraper(x, gy + dy, 18 + r() * 8, h, tone()));
  // A road ring and a park.
  add(gy + 6, `<path d="M${40 - col * 4},${gy + 8} Q96,${gy + 22} ${152 + col * 4},${gy + 8}" fill="none" stroke="#5a5a5a" stroke-width="4"/>` +
    `<ellipse cx="${150 - col * 2}" cy="${gy + 4}" rx="9" ry="4" fill="#4f8a3a"/>`);
  parts.sort((a, b) => a[0] - b[0]);
  return ground(col, gy) + wrap(parts.map((p) => p[1]).join(''));
}

/** A city of size `col` in age `row` (ancient, middle, industrial, modern). */
function cityFrame(col, row) {
  return [ancientCity, medievalCity, industrialCity, modernCity][row](col);
}

function flagFrame(col) {
  const p = (col / 4) * Math.PI * 2;
  const w = (x) => 4 * Math.sin(p + x / 9);
  const top = `M12,8 C22,${8 + w(10)} 34,${8 + w(24)} 50,${10 + w(40)}`;
  const bottom = `L50,${30 + w(40)} C34,${30 + w(24)} 22,${30 + w(10)} 12,30 Z`;
  return (
    `<path d="M10,4 V60" stroke="#1b1510" stroke-width="5" stroke-linecap="round"/><path d="M10,4 V60" stroke="#8d8578" stroke-width="2.4" stroke-linecap="round"/>` +
    `<path d="${top} ${bottom}" fill="#fff" stroke="rgba(0,0,0,0.75)" stroke-width="2" stroke-linejoin="round"/>` +
    `<path d="M24,${10 + w(14)} V${29 + w(14)} M38,${11 + w(30)} V${30 + w(30)}" stroke="rgba(0,0,0,0.16)" stroke-width="5"/>` +
    `<circle cx="10" cy="4" r="3" fill="#e8c14a" stroke="#1b1510" stroke-width="1.5"/>`
  );
}

function burstFrame(col) {
  const t = col / 5;
  const r = rng(321);
  const id = nextId('f');
  let s = `<filter id="${id}" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="${n(1.5 + t * 3)}"/></filter><g filter="url(#${id})">`;
  for (let i = 0; i < 9; i++) {
    const a = (i / 9) * Math.PI * 2 + between(r, -0.3, 0.3);
    const d = (10 + 34 * t) * between(r, 0.6, 1.1);
    const rad = (16 - 6 * t) * between(r, 0.7, 1.2);
    const smoke = Math.round(90 + 110 * t);
    s += `<circle cx="${n(64 + d * Math.cos(a))}" cy="${n(64 + d * Math.sin(a) * 0.8 - t * 14)}" r="${n(rad)}" fill="rgba(${smoke},${smoke - 10},${smoke - 20},${n(0.75 * (1 - t * 0.8))})"/>`;
  }
  if (t < 0.7) {
    const fr = 30 * (1 - t) + 6;
    s += `<circle cx="64" cy="64" r="${n(fr)}" fill="rgba(255,150,40,${n(0.95 - t)})"/><circle cx="64" cy="64" r="${n(fr * 0.6)}" fill="rgba(255,236,150,${n(1 - t)})"/>`;
  }
  s += '</g>';
  if (t < 0.5) for (let i = 0; i < 8; i++) {
    const a = (i / 8) * Math.PI * 2 + 0.2;
    const d1 = 14 + 50 * t;
    s += `<path d="M${n(64 + d1 * Math.cos(a))},${n(64 + d1 * Math.sin(a))} l${n(9 * Math.cos(a))},${n(9 * Math.sin(a))}" stroke="rgba(255,240,190,${n(1 - t * 2)})" stroke-width="3" stroke-linecap="round"/>`;
  }
  return s;
}

/** A tribal village: two thatched huts, a fence and a campfire. */
function villageFrame() {
  const hut = (x, y, s) =>
    `<ellipse cx="${x}" cy="${y + 2}" rx="${22 * s}" ry="${6 * s}" fill="rgba(0,0,0,0.28)" stroke="none"/>` +
    `<path d="M${x - 18 * s},${y} v${-16 * s} h${36 * s} v${16 * s} Z" fill="#b58a55"/>` +
    `<path d="M${x - 24 * s},${y - 14 * s} L${x},${y - 40 * s} L${x + 24 * s},${y - 14 * s} Z" fill="#d9b25f"/>` +
    `<path d="M${x - 16 * s},${y - 20 * s} L${x},${y - 34 * s} M${x - 8 * s},${y - 16 * s} L${x + 6 * s},${y - 31 * s} M${x + 4 * s},${y - 15 * s} L${x + 14 * s},${y - 24 * s}" stroke="#a9853c" stroke-width="1.5" fill="none"/>` +
    `<path d="M${x - 5 * s},${y} v${-10 * s} a${5 * s},${5 * s} 0 0 1 ${10 * s},0 v${10 * s} Z" fill="#4a3220"/>`;
  return wrap(
    hut(46, 92, 1.1) + hut(86, 84, 0.85) +
      `<path d="M14,104 h100" stroke-width="2" fill="none"/>` +
      [18, 34, 50, 66, 82, 98, 112].map((x) => `<path d="M${x},106 v-12" stroke="#7a5a34" stroke-width="3"/>`).join('') +
      `<path d="M90,108 l6,-4 l6,4 Z" fill="#6b4424"/><path d="M96,104 q-5,-8 0,-14 q5,6 0,14 Z" fill="#f28c28" stroke="#c0561a" stroke-width="1"/>`,
  );
}

module.exports = {
  village: () => sheet({ cols: 1, rows: 1, fw: 128, fh: 128, draw: villageFrame }),
  cities: () => sheet({ cols: 4, rows: 4, fw: 192, fh: 192, draw: cityFrame }),
  flag: () => sheet({ cols: 4, rows: 1, fw: 64, fh: 64, draw: flagFrame }),
  burst: () => sheet({ cols: 6, rows: 1, fw: 128, fh: 128, draw: burstFrame }),
};
