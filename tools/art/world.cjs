// Cities (four sizes), waving flags and combat effects.
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

function cityFrame(col) {
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
  cities: () => sheet({ cols: 4, rows: 1, fw: 192, fh: 192, draw: cityFrame }),
  flag: () => sheet({ cols: 4, rows: 1, fw: 64, fh: 64, draw: flagFrame }),
  burst: () => sheet({ cols: 6, rows: 1, fw: 128, fh: 128, draw: burstFrame }),
};
