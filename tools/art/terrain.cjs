// Terrain tiles, map overlays and resource icons.
const { rng, between, pick, n, nextId, hexPoints, noise, sheet } = require('./lib.cjs');

// Tile frame: the hexagon sits at the bottom, leaving headroom for trees and peaks.
const TW = 192;
const TH = 224;
const CX = 96;
const CY = 140;
const R = 96;

function hexClip(cx, cy) {
  const id = nextId('c');
  return { id, def: `<clipPath id="${id}"><polygon points="${hexPoints(cx, cy, R + 1.5)}"/></clipPath>` };
}

function ground({ base, dark, light, seed, w = TW, h = TH, cx = CX, cy = CY }) {
  const clip = hexClip(cx, cy);
  return {
    clip: clip.id,
    svg:
      clip.def +
      `<polygon points="${hexPoints(cx, cy, R + 1.5)}" fill="${base}"/>` +
      noise({ w, h, freq: 0.022, octaves: 4, seed, rgb: dark, gain: 2.6, bias: -1.05, clip: clip.id }) +
      noise({ w, h, freq: 0.05, octaves: 3, seed: seed + 50, rgb: light, gain: 2.4, bias: -1.1, clip: clip.id }) +
      noise({ w, h, freq: 0.9, octaves: 1, seed: seed + 99, rgb: [0, 0, 0], gain: 0.5, bias: -0.2, clip: clip.id }),
  };
}

const rim = (cx = CX, cy = CY) =>
  `<polygon points="${hexPoints(cx, cy, R - 1)}" fill="none" stroke="rgba(18,26,10,0.2)" stroke-width="2.5"/>`;

/** Random point inside the hexagon, kept `margin` away from the rim. */
function inHex(r, margin = 14) {
  for (;;) {
    const x = between(r, CX - R, CX + R);
    const y = between(r, CY - 83, CY + 83);
    const dx = Math.abs(x - CX);
    const dy = Math.abs(y - CY);
    const lim = R - margin;
    if (dy <= lim * 0.866 && dx <= lim - dy / 1.732) return [x, y];
  }
}

function tufts(r, count, colors, clip) {
  let s = `<g clip-path="url(#${clip})" fill="none" stroke-linecap="round" stroke-width="2">`;
  for (let i = 0; i < count; i++) {
    const [x, y] = inHex(r, 10);
    const c = pick(r, colors);
    const h = between(r, 5, 9);
    s += `<path d="M${n(x - 3)},${n(y)} q1,${n(-h)} -1,${n(-h - 1)} M${n(x)},${n(y)} v${n(-h - 2)} M${n(x + 3)},${n(y)} q-1,${n(-h)} 2,${n(-h)}" stroke="${c}"/>`;
  }
  return s + '</g>';
}

function tree(x, y, s, kind, r) {
  const shade = `<ellipse cx="${n(x + 5 * s)}" cy="${n(y + 1)}" rx="${n(13 * s)}" ry="${n(5 * s)}" fill="rgba(10,25,10,0.35)"/>`;
  if (kind === 'conifer') {
    const g1 = pick(r, ['#1f5a34', '#235f3a', '#1b5230']);
    let out = shade + `<rect x="${n(x - 2 * s)}" y="${n(y - 8 * s)}" width="${n(4 * s)}" height="${n(9 * s)}" fill="#5a3d24"/>`;
    for (let i = 0; i < 3; i++) {
      const w = (15 - i * 3.5) * s;
      const by = y - (6 + i * 11) * s;
      const ty = by - 17 * s;
      out += `<path d="M${n(x - w)},${n(by)} L${n(x)},${n(ty)} L${n(x + w)},${n(by)} Z" fill="${g1}" stroke="#12331f" stroke-width="1.2" stroke-linejoin="round"/>`;
      out += `<path d="M${n(x - w)},${n(by)} L${n(x)},${n(ty)} L${n(x - w * 0.15)},${n(by)} Z" fill="rgba(140,200,120,0.28)"/>`;
    }
    return out;
  }
  const g = pick(r, ['#2f7a35', '#35853a', '#2a6f33']);
  const cy = y - 22 * s;
  return (
    shade +
    `<path d="M${n(x - 2.5 * s)},${n(y)} L${n(x - 1.5 * s)},${n(y - 14 * s)} h${n(3 * s)} L${n(x + 2.5 * s)},${n(y)} Z" fill="#5f4026"/>` +
    `<g stroke="#17401f" stroke-width="1.2"><circle cx="${n(x - 8 * s)}" cy="${n(cy + 4 * s)}" r="${n(10 * s)}" fill="${g}"/>` +
    `<circle cx="${n(x + 8 * s)}" cy="${n(cy + 3 * s)}" r="${n(10 * s)}" fill="${g}"/>` +
    `<circle cx="${n(x)}" cy="${n(cy - 5 * s)}" r="${n(12 * s)}" fill="${g}"/></g>` +
    `<circle cx="${n(x - 4 * s)}" cy="${n(cy - 8 * s)}" r="${n(6 * s)}" fill="rgba(170,225,130,0.3)"/>`
  );
}

function mound(x, y, w, h, hue) {
  const id = nextId('g');
  const [lightC, midC, darkC] = hue;
  return (
    `<linearGradient id="${id}" x1="0" y1="0" x2="1" y2="0.6"><stop offset="0" stop-color="${lightC}"/><stop offset="0.55" stop-color="${midC}"/><stop offset="1" stop-color="${darkC}"/></linearGradient>` +
    `<ellipse cx="${n(x + 6)}" cy="${n(y)}" rx="${n(w * 0.56)}" ry="${n(h * 0.22)}" fill="rgba(20,30,10,0.28)"/>` +
    `<path d="M${n(x - w / 2)},${n(y)} C${n(x - w / 2)},${n(y - h * 1.25)} ${n(x + w / 2)},${n(y - h * 1.25)} ${n(x + w / 2)},${n(y)} Z" fill="url(#${id})" stroke="rgba(30,40,15,0.55)" stroke-width="1.4"/>` +
    `<path d="M${n(x - w * 0.28)},${n(y - h * 0.55)} q${n(w * 0.12)},${n(-h * 0.3)} ${n(w * 0.3)},${n(-h * 0.3)}" fill="none" stroke="rgba(255,255,220,0.35)" stroke-width="2" stroke-linecap="round"/>`
  );
}

function peak(x, y, w, h, r) {
  const tipX = x + between(r, -0.08, 0.08) * w;
  const tipY = y - h;
  const ridgeX = tipX + w * 0.06;
  const snowY = tipY + h * 0.38;
  const lerp = (a, b, t) => a + (b - a) * t;
  const t = 0.38;
  const lx = lerp(tipX, x - w / 2, t);
  const rx = lerp(tipX, x + w / 2, t);
  const mx = lerp(tipX, ridgeX, t);
  return (
    `<ellipse cx="${n(x + 8)}" cy="${n(y)}" rx="${n(w * 0.55)}" ry="${n(h * 0.1)}" fill="rgba(15,20,25,0.3)"/>` +
    `<path d="M${n(x - w / 2)},${n(y)} L${n(tipX)},${n(tipY)} L${n(ridgeX)},${n(y)} Z" fill="#a89f92"/>` +
    `<path d="M${n(ridgeX)},${n(y)} L${n(tipX)},${n(tipY)} L${n(x + w / 2)},${n(y)} Z" fill="#6b645c"/>` +
    `<path d="M${n(x - w * 0.3)},${n(y)} L${n(x - w * 0.12)},${n(y - h * 0.42)} L${n(x - w * 0.02)},${n(y)} Z" fill="rgba(60,55,50,0.25)"/>` +
    `<path d="M${n(lx)},${n(snowY)} L${n(tipX)},${n(tipY)} L${n(mx)},${n(snowY + 4)} L${n(lerp(lx, mx, 0.66))},${n(snowY - 5)} L${n(lerp(lx, mx, 0.33))},${n(snowY + 5)} Z" fill="#f4f8fb"/>` +
    `<path d="M${n(mx)},${n(snowY + 4)} L${n(tipX)},${n(tipY)} L${n(rx)},${n(snowY)} L${n(lerp(mx, rx, 0.5))},${n(snowY + 7)} Z" fill="#c3d0dc"/>` +
    `<path d="M${n(x - w / 2)},${n(y)} L${n(tipX)},${n(tipY)} L${n(x + w / 2)},${n(y)}" fill="none" stroke="#3d3833" stroke-width="1.6" stroke-linejoin="round"/>`
  );
}

const TERRAIN_ROWS = ['grassland', 'plains', 'desert', 'forest', 'hills', 'mountain', 'water'];

function terrainFrame(col, row) {
  const kind = TERRAIN_ROWS[row];
  if (kind !== 'water' && col >= 4) return '';
  const seed = row * 100 + col * 7 + 3;
  const r = rng(seed * 977 + 13);

  if (kind === 'water') {
    const g = ground({ base: '#2f6fb3', dark: [18, 52, 110], light: [110, 180, 235], seed: 700 });
    const phase = (col / 8) * Math.PI * 2;
    const wr = rng(4242);
    let waves = `<g clip-path="url(#${g.clip})" fill="none" stroke-linecap="round">`;
    for (let i = 0; i < 12; i++) {
      const [x, y] = inHex(wr, 22);
      const off = wr() * Math.PI * 2;
      const dx = 7 * Math.sin(phase + off);
      const op = 0.18 + 0.5 * (0.5 + 0.5 * Math.sin(phase + off + 1));
      const len = between(wr, 8, 13);
      waves += `<path d="M${n(x - len + dx)},${n(y)} q${n(len / 2)},-5 ${n(len)},0 t${n(len)},0" stroke="rgba(205,235,255,${n(op)})" stroke-width="2.4"/>`;
      waves += `<path d="M${n(x - len + dx)},${n(y + 4)} q${n(len / 2)},-5 ${n(len)},0 t${n(len)},0" stroke="rgba(10,40,90,${n(op * 0.5)})" stroke-width="2"/>`;
    }
    return g.svg + waves + '</g>' + `<polygon points="${hexPoints(CX, CY, R - 1)}" fill="none" stroke="rgba(8,30,70,0.2)" stroke-width="2.5"/>`;
  }

  if (kind === 'grassland') {
    const g = ground({ base: '#5c9a3e', dark: [38, 96, 34], light: [150, 200, 90], seed });
    let flowers = '';
    for (let i = 0; i < 5; i++) {
      const [x, y] = inHex(r, 20);
      flowers += `<circle cx="${n(x)}" cy="${n(y)}" r="2" fill="${pick(r, ['#f6e27a', '#ffffff', '#f2a6c0'])}"/>`;
    }
    return g.svg + tufts(r, 26, ['#3c7a2c', '#7fbd55', '#2f6a26'], g.clip) + flowers + rim();
  }

  if (kind === 'plains') {
    const g = ground({ base: '#b3a55a', dark: [128, 122, 52], light: [226, 210, 130], seed });
    let bushes = '';
    for (let i = 0; i < 3; i++) {
      const [x, y] = inHex(r, 26);
      bushes += `<ellipse cx="${n(x + 2)}" cy="${n(y + 3)}" rx="8" ry="3" fill="rgba(40,40,10,0.25)"/><circle cx="${n(x)}" cy="${n(y)}" r="5.5" fill="#7c8f3c" stroke="#4d5c22" stroke-width="1"/><circle cx="${n(x + 5)}" cy="${n(y + 1)}" r="4" fill="#8a9c45" stroke="#4d5c22" stroke-width="1"/>`;
    }
    return g.svg + tufts(r, 30, ['#8f8436', '#d9c877', '#a09040'], g.clip) + bushes + rim();
  }

  if (kind === 'desert') {
    const g = ground({ base: '#e3c886', dark: [196, 150, 84], light: [250, 236, 180], seed });
    let dunes = `<g clip-path="url(#${g.clip})" fill="none" stroke-linecap="round">`;
    for (let i = 0; i < 6; i++) {
      const [x, y] = inHex(r, 26);
      const w = between(r, 26, 44);
      dunes += `<path d="M${n(x - w)},${n(y)} q${n(w)},${n(-w * 0.45)} ${n(w * 2)},0" stroke="rgba(150,105,50,0.5)" stroke-width="3"/>`;
      dunes += `<path d="M${n(x - w)},${n(y - 3)} q${n(w)},${n(-w * 0.45)} ${n(w * 2)},0" stroke="rgba(255,246,205,0.6)" stroke-width="2"/>`;
    }
    dunes += '</g>';
    let extra = '';
    if (col % 2 === 1) {
      const [x, y] = inHex(r, 40);
      extra += `<ellipse cx="${n(x + 4)}" cy="${n(y + 1)}" rx="9" ry="3" fill="rgba(90,60,20,0.3)"/><g fill="#4f8a45" stroke="#2c5a2a" stroke-width="1.3" stroke-linejoin="round"><path d="M${n(x - 3)},${n(y)} v-22 a3,3 0 0 1 6,0 v22 Z"/><path d="M${n(x + 3)},${n(y - 9)} h5 v-7 a2.5,2.5 0 0 1 5,0 v12 h-10 Z"/><path d="M${n(x - 3)},${n(y - 6)} h-5 v-6 a2.5,2.5 0 0 0 -5,0 v11 h10 Z"/></g>`;
    }
    for (let i = 0; i < 3; i++) {
      const [x, y] = inHex(r, 22);
      extra += `<path d="M${n(x - 5)},${n(y)} l2,-5 l5,-1 l3,6 Z" fill="#a98d62" stroke="#6f5a3a" stroke-width="1"/>`;
    }
    return g.svg + dunes + extra + rim();
  }

  if (kind === 'forest') {
    const g = ground({ base: '#4a8038', dark: [26, 70, 30], light: [120, 170, 80], seed });
    const trees = [];
    for (let i = 0; i < 17; i++) {
      const [x, y] = inHex(r, 20);
      trees.push([x, y + 6, between(r, 0.8, 1.1), r() < 0.55 ? 'conifer' : 'leafy']);
    }
    trees.sort((a, b) => a[1] - b[1]);
    return g.svg + tufts(r, 10, ['#2f6a26', '#3c7a2c'], g.clip) + rim() + trees.map((t) => tree(t[0], t[1], t[2], t[3], r)).join('');
  }

  if (kind === 'hills') {
    const g = ground({ base: '#7f9a48', dark: [70, 96, 40], light: [190, 200, 110], seed });
    const hue = ['#c2cf7a', '#8ea54e', '#56702f'];
    const mounds = [];
    const spots = [
      [CX - 34, CY - 18, 74, 36],
      [CX + 36, CY - 6, 80, 40],
      [CX - 8, CY + 34, 92, 42],
      [CX + 46, CY + 44, 56, 26],
      [CX - 54, CY + 30, 50, 24],
    ];
    for (const [x, y, w, h] of spots) mounds.push([x + between(r, -8, 8), y + between(r, -6, 6), w * between(r, 0.9, 1.1), h * between(r, 0.9, 1.15)]);
    mounds.sort((a, b) => a[1] - b[1]);
    return g.svg + rim() + mounds.map((m) => mound(m[0], m[1], m[2], m[3], hue)).join('') + tufts(r, 8, ['#4d6a2a', '#a9bf63'], g.clip);
  }

  // mountain
  const g = ground({ base: '#8d8a78', dark: [80, 78, 66], light: [190, 186, 160], seed });
  const peaks = [
    [CX - 40, CY + 6, 74, 86],
    [CX + 34, CY + 16, 86, 104],
    [CX - 6, CY + 52, 104, 120],
    [CX + 56, CY + 52, 50, 50],
    [CX - 58, CY + 46, 48, 46],
  ].map(([x, y, w, h]) => [x + between(r, -7, 7), y + between(r, -5, 5), w * between(r, 0.92, 1.08), h * between(r, 0.9, 1.08)]);
  peaks.sort((a, b) => a[1] - b[1]);
  return g.svg + rim() + peaks.map((p) => peak(p[0], p[1], p[2], p[3], r)).join('');
}

// ---------------------------------------------------------------------------------------------
// Overlays share one hex-sized frame (192 x 168) centred on the hexagon.
const OW = 192;
const OH = 168;
const OX = 96;
const OY = 84;
const OVERLAYS = ['shore', 'border', 'farm', 'road', 'roadhub', 'hover', 'select', 'unknown', 'mine', 'dot', 'base', 'reach'];

function overlayFrame(col) {
  const kind = OVERLAYS[col];
  const clip = hexClip(OX, OY);
  const top = OY - 83.1;
  switch (kind) {
    case 'shore': {
      const id = nextId('g');
      let foam = '';
      for (let x = 34; x < 160; x += 14) foam += `q7,${x % 28 === 6 ? 5 : -5} 14,0 `;
      return (
        clip.def +
        `<linearGradient id="${id}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ead9a2"/><stop offset="0.45" stop-color="#e2cf93" stop-opacity="0.95"/><stop offset="0.8" stop-color="#bfe6f5" stop-opacity="0.55"/><stop offset="1" stop-color="#bfe6f5" stop-opacity="0"/></linearGradient>` +
        `<g clip-path="url(#${clip.id})"><path d="M20,${top - 4} H172 V${top + 13} q-10,9 -20,6 t-22,2 t-22,-3 t-22,3 t-22,-2 t-22,-4 Z" fill="url(#${id})" transform="translate(0,2)"/>` +
        `<path d="M34,${top + 24} ${foam}" fill="none" stroke="rgba(255,255,255,0.75)" stroke-width="2.2" stroke-linecap="round"/></g>`
      );
    }
    case 'border': {
      const id = nextId('g');
      return (
        clip.def +
        `<linearGradient id="${id}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff" stop-opacity="0.55"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></linearGradient>` +
        `<g clip-path="url(#${clip.id})"><rect x="30" y="${top}" width="132" height="24" fill="url(#${id})"/>` +
        `<path d="M44,${top + 4.5} H148" stroke="#fff" stroke-width="6" stroke-dasharray="18 8" stroke-linecap="butt"/></g>`
      );
    }
    case 'farm': {
      const r = rng(77);
      let s = clip.def + `<g clip-path="url(#${clip.id})">`;
      const fields = [
        [40, 44, 52, 34, '#c9b04a', '#a38a2c'],
        [98, 40, 56, 30, '#8fb546', '#5f8a2a'],
        [30, 86, 50, 36, '#9cbd4e', '#688f2e'],
        [86, 78, 44, 36, '#d6bd58', '#ab8f30'],
        [136, 76, 34, 40, '#b99a3e', '#8c7426'],
        [60, 124, 74, 22, '#84ab40', '#587f26'],
      ];
      for (const [x, y, w, h, c, d] of fields) {
        s += `<g transform="skewX(-8)"><rect x="${x + 12}" y="${y}" width="${w}" height="${h}" rx="3" fill="${c}" stroke="${d}" stroke-width="1.5"/>`;
        for (let ly = y + 5; ly < y + h - 2; ly += 5) s += `<path d="M${x + 15},${ly} h${w - 6}" stroke="${d}" stroke-width="1.2" opacity="0.7"/>`;
        s += '</g>';
      }
      // farmhouse
      s += `<ellipse cx="144" cy="62" rx="16" ry="4" fill="rgba(0,0,0,0.25)"/><path d="M130,60 v-12 h22 v12 Z" fill="#d9c7a0" stroke="#5b4630" stroke-width="1.3"/><path d="M127,49 l14,-10 l14,10 Z" fill="#a4482f" stroke="#5a2418" stroke-width="1.3" stroke-linejoin="round"/><rect x="138" y="52" width="5" height="8" fill="#5b4630"/>`;
      void r;
      return s + '</g>';
    }
    case 'road':
      return (
        `<path d="M${OX},${OY + 2} C${OX + 7},${OY - 28} ${OX - 7},${OY - 54} ${OX},${top - 1}" fill="none" stroke="#5e4a30" stroke-width="19" stroke-linecap="round"/>` +
        `<path d="M${OX},${OY + 2} C${OX + 7},${OY - 28} ${OX - 7},${OY - 54} ${OX},${top - 1}" fill="none" stroke="#b79a68" stroke-width="14" stroke-linecap="round"/>` +
        `<path d="M${OX - 3},${OY + 2} C${OX + 4},${OY - 28} ${OX - 10},${OY - 54} ${OX - 3},${top - 1} M${OX + 3},${OY + 2} C${OX + 10},${OY - 28} ${OX - 4},${OY - 54} ${OX + 3},${top - 1}" fill="none" stroke="rgba(94,74,48,0.55)" stroke-width="1.5" stroke-dasharray="7 5"/>`
      );
    case 'roadhub':
      return `<circle cx="${OX}" cy="${OY}" r="11.5" fill="#b79a68" stroke="#5e4a30" stroke-width="2.5"/>`;
    case 'hover':
      return `<polygon points="${hexPoints(OX, OY, R - 5)}" fill="rgba(255,255,255,0.1)" stroke="#fff" stroke-width="4" stroke-linejoin="round"/>`;
    case 'select': {
      const id = nextId('f');
      return (
        `<filter id="${id}" x="-20%" y="-40%" width="140%" height="180%"><feGaussianBlur stdDeviation="3"/></filter>` +
        `<ellipse cx="${OX}" cy="${OY + 20}" rx="50" ry="28" fill="none" stroke="#fff" stroke-width="9" opacity="0.55" filter="url(#${id})"/>` +
        `<ellipse cx="${OX}" cy="${OY + 20}" rx="50" ry="28" fill="rgba(255,255,255,0.12)" stroke="#fff" stroke-width="4.5" stroke-dasharray="20 9"/>`
      );
    }
    case 'unknown': {
      let hatch = `<g clip-path="url(#${clip.id})" stroke="rgba(214,190,140,0.07)" stroke-width="1.5">`;
      for (let x = -160; x < 200; x += 13) hatch += `<path d="M${x},0 l168,168"/>`;
      return (
        clip.def +
        `<polygon points="${hexPoints(OX, OY, R + 1.5)}" fill="#2a251d"/>` +
        noise({ w: OW, h: OH, freq: 0.03, octaves: 4, seed: 5, rgb: [92, 76, 50], gain: 0.9, bias: -0.3, clip: clip.id }) +
        hatch +
        '</g>'
      );
    }
    case 'mine':
      return (
        `<ellipse cx="100" cy="112" rx="46" ry="10" fill="rgba(0,0,0,0.3)"/>` +
        `<path d="M54,112 C56,70 76,52 98,52 C122,52 140,72 144,112 Z" fill="#8a8172" stroke="#3f3a32" stroke-width="2"/>` +
        `<path d="M64,104 C70,74 84,62 98,60" fill="none" stroke="rgba(255,255,255,0.3)" stroke-width="3" stroke-linecap="round"/>` +
        `<path d="M80,112 V88 a18,16 0 0 1 36,0 V112 Z" fill="#17130f"/>` +
        `<g stroke="#3a2512" stroke-width="1.3" fill="#8b5a2b"><rect x="75" y="80" width="7" height="33"/><rect x="114" y="80" width="7" height="33"/><rect x="72" y="74" width="52" height="8"/></g>` +
        `<path d="M88,124 h26 M92,118 v10 M110,118 v10 M101,116 v12" stroke="#4a3520" stroke-width="2.2"/>` +
        `<path d="M132,112 l6,-9 l8,3 l4,6 Z M44,114 l5,-7 l8,2 l2,5 Z" fill="#a39988" stroke="#3f3a32" stroke-width="1.3"/>`
      );
    case 'dot':
      return `<circle cx="${OX}" cy="${OY}" r="11" fill="#fff" stroke="rgba(0,0,0,0.55)" stroke-width="3"/>`;
    case 'base': {
      const id = nextId('g');
      return (
        `<radialGradient id="${id}" cx="0.5" cy="0.4" r="0.6"><stop offset="0" stop-color="#fff"/><stop offset="1" stop-color="#bdbdbd"/></radialGradient>` +
        `<ellipse cx="${OX}" cy="${OY + 24}" rx="44" ry="23" fill="rgba(0,0,0,0.35)"/>` +
        `<ellipse cx="${OX}" cy="${OY + 20}" rx="42" ry="22" fill="url(#${id})" stroke="rgba(0,0,0,0.6)" stroke-width="2.5"/>` +
        `<ellipse cx="${OX}" cy="${OY + 20}" rx="34" ry="16.5" fill="none" stroke="rgba(255,255,255,0.7)" stroke-width="2"/>`
      );
    }
    case 'reach':
      return `<polygon points="${hexPoints(OX, OY, R - 9)}" fill="rgba(255,255,255,0.16)" stroke="rgba(255,255,255,0.75)" stroke-width="2.5" stroke-dasharray="10 7" stroke-linejoin="round"/>`;
  }
  return '';
}

// ---------------------------------------------------------------------------------------------
const RESOURCES = ['horses', 'iron', 'wine', 'spices', 'silk'];

function resourceFrame(col) {
  const plate =
    `<ellipse cx="48" cy="80" rx="30" ry="8" fill="rgba(0,0,0,0.3)"/>`;
  switch (RESOURCES[col]) {
    case 'horses':
      return (
        plate +
        `<g stroke="#2b1a0e" stroke-width="2.2" stroke-linejoin="round" stroke-linecap="round">` +
        `<path d="M26,78 l2,-22 M36,78 l-1,-20 M58,78 l1,-20 M68,78 l-3,-22" fill="none" stroke-width="5" stroke="#5a3418"/>` +
        `<path d="M22,48 C24,38 40,36 54,38 C62,38 66,32 68,24 L76,14 L80,22 L88,34 L82,38 L76,34 C76,46 72,58 62,60 L30,60 C24,60 21,55 22,48 Z" fill="#8b5a2b"/>` +
        `<path d="M22,48 C14,50 13,62 16,70" fill="none" stroke="#2b1a0e" stroke-width="5"/>` +
        `<path d="M66,26 C60,28 58,36 56,40" fill="none" stroke="#2b1a0e" stroke-width="5"/></g>` +
        `<circle cx="80" cy="27" r="1.8" fill="#111"/>`
      );
    case 'iron':
      return (
        plate +
        `<g stroke="#22262b" stroke-width="2" stroke-linejoin="round">` +
        `<path d="M16,78 l8,-22 l16,-8 l14,10 l4,20 Z" fill="#6c7480"/><path d="M24,56 l16,-8 l-2,16 l-12,6 Z" fill="#a9b3c0"/>` +
        `<path d="M46,80 l6,-26 l18,-6 l12,14 l-2,18 Z" fill="#58606b"/><path d="M52,54 l18,-6 l-4,16 l-12,4 Z" fill="#96a0ae"/>` +
        `<path d="M30,44 l10,-18 l14,2 l6,16 l-12,8 Z" fill="#7a838f"/><path d="M40,26 l14,2 l-8,10 Z" fill="#c9d2dd"/></g>` +
        `<path d="M62,60 l6,-3 M30,66 l5,-2" stroke="#d98a3a" stroke-width="3" stroke-linecap="round"/>`
      );
    case 'wine': {
      let grapes = '';
      const rows = [[4, 36], [5, 46], [4, 56], [3, 66], [2, 75]];
      for (const [count, y] of rows) for (let i = 0; i < count; i++) grapes += `<circle cx="${48 - (count - 1) * 5.5 + i * 11}" cy="${y}" r="6.5" fill="#6d2a86" stroke="#32113f" stroke-width="1.6"/><circle cx="${46 - (count - 1) * 5.5 + i * 11}" cy="${y - 2}" r="2" fill="rgba(255,255,255,0.45)"/>`;
      return (
        plate +
        `<path d="M48,30 C48,22 52,16 58,12" fill="none" stroke="#5a3d1c" stroke-width="3.5" stroke-linecap="round"/>` +
        `<path d="M50,24 C60,12 76,16 80,26 C70,34 58,32 50,24 Z" fill="#4f9a3a" stroke="#234d1c" stroke-width="1.8"/>` +
        grapes
      );
    }
    case 'spices':
      return (
        plate +
        `<path d="M24,80 C18,60 22,44 30,40 L66,40 C74,44 78,60 72,80 Z" fill="#c9a36a" stroke="#5b4222" stroke-width="2.2" stroke-linejoin="round"/>` +
        `<path d="M28,42 C36,34 60,34 68,42 C60,48 36,48 28,42 Z" fill="#d9541e" stroke="#7a2a0c" stroke-width="2"/>` +
        `<path d="M36,40 C42,36 54,36 60,40" fill="none" stroke="#f6b04a" stroke-width="3" stroke-linecap="round"/>` +
        `<path d="M32,58 q6,4 12,0 M50,66 q6,4 12,0" fill="none" stroke="#8d6d3c" stroke-width="2"/>` +
        `<g stroke="#5a1408" stroke-width="1.6"><path d="M70,30 C78,22 86,24 88,30 C82,34 76,34 70,30 Z" fill="#c8281c"/><path d="M12,36 C16,26 24,24 28,28 C24,34 18,38 12,36 Z" fill="#e0a02a"/></g>`
      );
    case 'silk':
      return (
        plate +
        `<g stroke="#12403f" stroke-width="2.2" stroke-linejoin="round">` +
        `<path d="M18,34 C30,28 50,40 62,34 L74,62 C62,70 42,58 28,66 Z" fill="#2aa7a0"/>` +
        `<path d="M28,66 C42,58 62,70 74,62 L78,76 C66,84 44,72 32,80 Z" fill="#1d8580"/>` +
        `<ellipse cx="68" cy="34" rx="8" ry="11" transform="rotate(-22 68 34)" fill="#e8d7b0" stroke="#5b4630"/></g>` +
        `<path d="M26,40 C38,36 52,46 62,42 M30,52 C42,48 56,58 68,52" fill="none" stroke="rgba(200,255,250,0.6)" stroke-width="2.5" stroke-linecap="round"/>`
      );
  }
  return '';
}

module.exports = {
  terrain: () => sheet({ cols: 8, rows: TERRAIN_ROWS.length, fw: TW, fh: TH, draw: terrainFrame }),
  overlays: () => sheet({ cols: OVERLAYS.length, rows: 1, fw: OW, fh: OH, draw: overlayFrame }),
  resources: () => sheet({ cols: RESOURCES.length, rows: 1, fw: 96, fh: 96, draw: resourceFrame }),
};
