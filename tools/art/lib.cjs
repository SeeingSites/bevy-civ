// Shared helpers for the procedural art generator.
// Every sprite is authored as SVG and rasterised to a PNG atlas by headless Chromium.

function rng(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const between = (r, a, b) => a + (b - a) * r();
const pick = (r, list) => list[Math.floor(r() * list.length)];
const n = (v) => Math.round(v * 100) / 100;

let uid = 0;
const nextId = (prefix) => `${prefix}${uid++}`;

/** Flat-top hexagon points around (cx, cy). */
function hexPoints(cx, cy, radius) {
  const pts = [];
  for (let i = 0; i < 6; i++) {
    const a = (Math.PI / 3) * i;
    pts.push(`${n(cx + radius * Math.cos(a))},${n(cy + radius * Math.sin(a))}`);
  }
  return pts.join(' ');
}

/** A rect filled with tinted fractal noise; alpha = gain * noise + bias. */
function noise({ w, h, freq, octaves = 3, seed, rgb, gain, bias, clip }) {
  const id = nextId('n');
  const [r, g, b] = rgb.map((v) => n(v / 255));
  return (
    `<filter id="${id}" x="0" y="0" width="100%" height="100%" color-interpolation-filters="sRGB">` +
    `<feTurbulence type="fractalNoise" baseFrequency="${freq}" numOctaves="${octaves}" seed="${seed}"/>` +
    `<feColorMatrix type="matrix" values="0 0 0 0 ${r} 0 0 0 0 ${g} 0 0 0 0 ${b} ${gain} 0 0 0 ${bias}"/></filter>` +
    `<rect width="${w}" height="${h}" filter="url(#${id})"${clip ? ` clip-path="url(#${clip})"` : ''}/>`
  );
}

/** Lay frames out on a grid. `draw(col, row)` returns SVG markup in frame-local coordinates. */
function sheet({ cols, rows, fw, fh, draw }) {
  let body = '';
  for (let row = 0; row < rows; row++) {
    for (let col = 0; col < cols; col++) {
      const inner = draw(col, row);
      if (!inner) continue;
      body += `<svg x="${col * fw}" y="${row * fh}" width="${fw}" height="${fh}" viewBox="0 0 ${fw} ${fh}" overflow="hidden">${inner}</svg>`;
    }
  }
  const width = cols * fw;
  const height = rows * fh;
  return {
    width,
    height,
    svg: `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">${body}</svg>`,
  };
}

module.exports = { rng, between, pick, n, nextId, hexPoints, noise, sheet };
