#!/usr/bin/env node
// The title illustration on the loading screen: a walled city on a hill at dusk above a river
// valley, riders on the road, a galleon in the bay and mountains beyond. Drawn as SVG in the same
// flat, outlined style as the game's sprites and saved as web/splash.jpg.
// Usage: node tools/art/splash.cjs   (needs Playwright + Chromium)
const path = require('path');
const { chromium } = require('playwright');

const W = 1920;
const H = 1080;
const O = '#1d1611';

// A small seeded random generator, so the picture is the same every time.
function rng(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const r = rng(1492);
const n = (v) => Math.round(v * 10) / 10;

// A ridge line across the picture: base height y, wobble amplitude a, roughness steps.
function ridge(y, a, steps, seed, fill, extra = '') {
  const rr = rng(seed);
  let d = `M0,${H} L0,${y}`;
  let h = y;
  for (let i = 1; i <= steps; i++) {
    const x = (W * i) / steps;
    h = y + (rr() - 0.5) * a * 2;
    d += ` L${n(x)},${n(h)}`;
  }
  d += ` L${W},${H} Z`;
  return `<path d="${d}" fill="${fill}" ${extra}/>`;
}

// Smooth rolling hills.
function hills(y, a, waves, seed, fill, stroke = 'none') {
  const rr = rng(seed);
  let d = `M0,${H} L0,${y}`;
  const step = W / waves;
  for (let i = 0; i < waves; i++) {
    const x0 = i * step;
    const peak = y - a * (0.4 + rr() * 0.6);
    d += ` Q${n(x0 + step / 2)},${n(peak)} ${n(x0 + step)},${n(y + (rr() - 0.5) * a * 0.3)}`;
  }
  d += ` L${W},${H} Z`;
  return `<path d="${d}" fill="${fill}" stroke="${stroke}" stroke-width="3"/>`;
}

function tree(x, y, s, dark) {
  return `<g stroke="${O}" stroke-width="2"><rect x="${n(x - 2 * s)}" y="${n(y - 6 * s)}" width="${n(4 * s)}" height="${n(8 * s)}" fill="#5a3d24"/>` +
    `<path d="M${n(x)},${n(y - 34 * s)} L${n(x + 13 * s)},${n(y - 4 * s)} L${n(x - 13 * s)},${n(y - 4 * s)} Z" fill="${dark ? '#2f5a34' : '#3f7a3a'}"/>` +
    `<path d="M${n(x)},${n(y - 44 * s)} L${n(x + 10 * s)},${n(y - 20 * s)} L${n(x - 10 * s)},${n(y - 20 * s)} Z" fill="${dark ? '#3a6a3c' : '#4f8f45'}"/></g>`;
}

// The city: walls along the hilltop, towers, houses with red roofs, a keep with a banner.
function city(cx, cy) {
  let s = `<g stroke="${O}" stroke-width="3" stroke-linejoin="round">`;
  // houses behind the wall
  const houses = [[-230, -40], [-170, -70], [-110, -55], [-40, -95], [40, -80], [110, -60], [175, -75], [235, -45], [-75, -120], [80, -125]];
  for (const [dx, dy] of houses) {
    const x = cx + dx, y = cy + dy, w = 46 + r() * 20, h = 34 + r() * 18;
    s += `<rect x="${n(x - w / 2)}" y="${n(y - h)}" width="${n(w)}" height="${n(h)}" fill="#e6d3ae"/>`;
    s += `<path d="M${n(x - w / 2 - 6)},${n(y - h)} L${n(x)},${n(y - h - 26)} L${n(x + w / 2 + 6)},${n(y - h)} Z" fill="#b8452f"/>`;
    s += `<rect x="${n(x - 6)}" y="${n(y - h + 10)}" width="10" height="12" fill="#f3c25a"/>`;
  }
  // keep
  s += `<rect x="${cx - 45}" y="${cy - 250}" width="90" height="170" fill="#d9c49a"/>`;
  s += `<path d="M${cx - 52},${cy - 250} h104 v-18 h-14 v10 h-14 v-10 h-20 v10 h-14 v-10 h-14 v10 h-14 v-10 h-14 Z" fill="#cbb489"/>`;
  s += `<rect x="${cx - 10}" y="${cy - 220}" width="20" height="30" rx="10" fill="#f3c25a"/>`;
  s += `<path d="M${cx},${cy - 268} v-70" stroke-width="4" fill="none"/><path d="M${cx},${cy - 338} q34,8 60,0 q-12,14 0,26 q-26,8 -60,0 Z" fill="#c8281c"/>`;
  // wall with crenellations and two towers
  s += `<path d="M${cx - 300},${cy} L${cx - 300},${cy - 55} L${cx + 300},${cy - 55} L${cx + 300},${cy} Z" fill="#cdb78c"/>`;
  for (let x = cx - 300; x < cx + 300; x += 30) s += `<rect x="${x}" y="${cy - 70}" width="18" height="16" fill="#cdb78c"/>`;
  for (const tx of [cx - 300, cx + 300, cx - 20]) {
    const tw = tx === cx - 20 ? 0 : 1;
    if (!tw) continue;
    s += `<rect x="${tx - 34}" y="${cy - 130}" width="68" height="130" fill="#d6c08f"/>`;
    s += `<path d="M${tx - 42},${cy - 130} L${tx},${cy - 190} L${tx + 42},${cy - 130} Z" fill="#8f3a2a"/>`;
    s += `<rect x="${tx - 8}" y="${cy - 105}" width="16" height="24" rx="8" fill="#f3c25a"/>`;
  }
  // gate
  s += `<path d="M${cx - 34},${cy} v-40 q34,-34 68,0 v40 Z" fill="#4a2f1c"/>`;
  return s + '</g>';
}

function rider(x, y, s, flip, colour) {
  // Facing left when flipped. Drawn around the horse's hooves at (x, y).
  const f = flip ? -1 : 1;
  const t = (dx) => n(x + dx * s * f);
  const v = (dy) => n(y + dy * s);
  const legs = [[-22, 4], [-12, -3], [16, 3], [26, -4]]
    .map(([dx, k]) => `<path d="M${t(dx)},${v(-34)} l${n(k * s * f)},${n(18 * s)} l${n(-k * 0.5 * s * f)},${n(16 * s)}" stroke-width="${n(4.5 * s)}" fill="none"/>`)
    .join('');
  return `<g stroke="${O}" stroke-width="2.5" stroke-linejoin="round" stroke-linecap="round">` +
    legs +
    `<ellipse cx="${t(2)}" cy="${v(-40)}" rx="${n(32 * s)}" ry="${n(13 * s)}" fill="#6b4428"/>` +
    `<path d="M${t(26)},${v(-46)} L${t(40)},${v(-72)} L${t(56)},${v(-64)} L${t(52)},${v(-56)} L${t(42)},${v(-58)} L${t(34)},${v(-38)} Z" fill="#6b4428"/>` +
    `<path d="M${t(-29)},${v(-44)} q${n(-12 * s * f)},${n(6 * s)} ${n(-10 * s * f)},${n(26 * s)}" stroke-width="${n(4 * s)}" fill="none"/>` +
    `<path d="M${t(-6)},${v(-50)} l${n(2 * s * f)},${n(-30 * s)} h${n(16 * s * f)} l${n(2 * s * f)},${n(30 * s)} Z" fill="${colour}"/>` +
    `<circle cx="${t(4)}" cy="${v(-90)}" r="${n(8 * s)}" fill="#e0b088"/>` +
    `<path d="M${t(-5)},${v(-95)} q${n(9 * s * f)},${n(-14 * s)} ${n(18 * s * f)},0 Z" fill="#c4ccd6"/>` +
    `<path d="M${t(12)},${v(-70)} L${t(40)},${v(-140)}" stroke-width="3" fill="none"/>` +
    `<path d="M${t(40)},${v(-140)} l${n(-18 * s * f)},${n(5 * s)} l${n(16 * s * f)},${n(9 * s)} Z" fill="${colour}"/>` +
    '</g>';
}

function galleon(x, y, s) {
  const t = (dx) => n(x + dx * s);
  const v = (dy) => n(y + dy * s);
  return `<g stroke="${O}" stroke-width="2.5" stroke-linejoin="round">` +
    `<path d="M${t(-70)},${v(-10)} L${t(80)},${v(-10)} L${t(62)},${v(14)} L${t(-56)},${v(14)} Z" fill="#7a4a24"/>` +
    `<path d="M${t(-70)},${v(-10)} l-8,-18 h30 l6,18 Z" fill="#8b5a2b"/>` +
    `<path d="M${t(-20)},${v(-10)} V${v(-110)} M${t(28)},${v(-10)} V${v(-96)}" stroke-width="4" fill="none"/>` +
    `<path d="M${t(-50)},${v(-98)} q30,20 60,0 v50 q-30,14 -60,0 Z" fill="#f1ecdf"/>` +
    `<path d="M${t(4)},${v(-86)} q24,16 48,0 v44 q-24,12 -48,0 Z" fill="#f1ecdf"/>` +
    `<path d="M${t(-20)},${v(-110)} l22,6 l-22,6 Z" fill="#c8281c"/>` +
    '</g>';
}

function scene() {
  let s = '';
  // Sky: dusk gradient, a low sun and glow.
  s += `<defs>
    <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#1e2a4a"/><stop offset="0.45" stop-color="#6a4a6a"/>
      <stop offset="0.72" stop-color="#e08a4a"/><stop offset="1" stop-color="#f6c96a"/>
    </linearGradient>
    <radialGradient id="sun" cx="0.5" cy="0.5" r="0.5"><stop offset="0" stop-color="#fff6d6"/><stop offset="0.35" stop-color="#ffd27a" stop-opacity="0.9"/><stop offset="1" stop-color="#ffb04a" stop-opacity="0"/></radialGradient>
    <linearGradient id="sea" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#e3a25e"/><stop offset="0.25" stop-color="#5a7a9a"/><stop offset="1" stop-color="#2a4a6a"/></linearGradient>
    <linearGradient id="vignette" x1="0" y1="0" x2="0" y2="1"><stop offset="0.55" stop-color="#000" stop-opacity="0"/><stop offset="1" stop-color="#0c0a08" stop-opacity="0.75"/></linearGradient>
  </defs>`;
  s += `<rect width="${W}" height="${H}" fill="url(#sky)"/>`;
  // stars in the upper sky
  for (let i = 0; i < 70; i++) {
    const x = r() * W, y = r() * H * 0.3;
    s += `<circle cx="${n(x)}" cy="${n(y)}" r="${n(0.8 + r() * 1.6)}" fill="#fff8e0" opacity="${n(0.3 + r() * 0.6)}"/>`;
  }
  s += `<circle cx="1360" cy="560" r="300" fill="url(#sun)"/><circle cx="1360" cy="560" r="62" fill="#fff1c4"/>`;
  // clouds
  for (const [x, y, w] of [[260, 260, 300], [760, 190, 220], [1580, 300, 260], [1100, 340, 180]]) {
    s += `<path d="M${x},${y} q${w * 0.2},-40 ${w * 0.45},-10 q${w * 0.25},-34 ${w * 0.55},10 q-${w * 0.4},20 -${w},0 Z" fill="#f0b48a" opacity="0.55"/>`;
  }
  // Mountains, far to near.
  s += ridge(560, 90, 22, 3, '#6b5a7a');
  s += ridge(610, 70, 30, 5, '#5a4a66');
  // Snow caps on the far ridge would need its points; a pale haze band instead.
  s += `<rect x="0" y="600" width="${W}" height="60" fill="#e8a070" opacity="0.18"/>`;
  // Rolling land.
  s += hills(700, 90, 5, 9, '#5f7a3a');
  s += hills(790, 60, 4, 11, '#4f6a32');
  // A bay opening on the right, with the sun's path on the water and a galleon at anchor.
  s += `<path d="M1040,${H} C1060,860 1120,760 1260,730 Q1500,700 ${W},708 L${W},${H} Z" fill="url(#sea)" stroke="${O}" stroke-width="3"/>`;
  for (let i = 0; i < 24; i++) {
    const y = 728 + i * 10 + r() * 4, w = 40 + r() * 120;
    s += `<rect x="${n(1380 + (r() - 0.5) * 140 - w / 2)}" y="${n(y)}" width="${n(w)}" height="3" fill="#ffe2a0" opacity="${n(0.6 - i * 0.022)}"/>`;
  }
  s += galleon(1600, 840, 1.2);
  // A river from the hills down to the bay.
  s += `<path d="M700,760 C760,800 700,840 820,870 S1000,900 1080,940 L1110,980 C1010,950 900,930 790,900 S660,820 670,770 Z" fill="#6a9ac0" stroke="${O}" stroke-width="2"/>`;
  // The city on its hill.
  s += `<path d="M120,${H} L120,860 Q300,640 560,640 Q820,640 980,860 L980,${H} Z" fill="#6a8a3e" stroke="${O}" stroke-width="3"/>`;
  s += city(560, 700);
  // Fields and trees in the foreground.
  s += hills(900, 60, 3, 13, '#3f5a2a', O);
  for (let i = 0; i < 22; i++) {
    const x = r() * W, y = 900 + r() * 140, sc = 0.9 + (y - 900) / 140;
    if (x > 1040) continue;
    s += tree(x, y, sc, r() < 0.5);
  }
  // A road and riders heading for the city.
  s += `<path d="M1040,${H} Q900,960 760,900 T600,760" fill="none" stroke="#b89a64" stroke-width="22" stroke-linecap="round"/>`;
  s += `<path d="M1040,${H} Q900,960 760,900 T600,760" fill="none" stroke="${O}" stroke-width="2" stroke-dasharray="2 18" opacity="0.4"/>`;
  s += rider(880, 985, 0.95, true, '#2a5fa8');
  s += rider(990, 1035, 1.05, true, '#c8281c');
  // Birds over the valley.
  for (const [x, y, k] of [[880, 420, 1], [930, 400, 0.8], [990, 430, 0.9], [1520, 380, 0.7]]) {
    s += `<path d="M${x - 14 * k},${y} q${7 * k},-8 ${14 * k},0 q${7 * k},-8 ${14 * k},0" fill="none" stroke="${O}" stroke-width="2.5"/>`;
  }
  s += `<rect width="${W}" height="${H}" fill="url(#vignette)"/>`;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}">${s}</svg>`;
}

(async () => {
  const out = path.join(__dirname, '..', '..', 'web', 'splash.jpg');
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: W, height: H }, deviceScaleFactor: 1 });
  await page.setContent(`<!doctype html><body style="margin:0">${scene()}</body>`);
  await page.screenshot({ path: out, type: 'jpeg', quality: 82, clip: { x: 0, y: 0, width: W, height: H } });
  await browser.close();
  console.log(`splash.jpg ${W}x${H}`);
})();
