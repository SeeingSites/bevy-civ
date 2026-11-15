// Interface art: nine-slice panel and button frames plus the icon strip.
const { nextId, noise, sheet } = require('./lib.cjs');

function panel() {
  const g = nextId('g');
  return (
    `<linearGradient id="${g}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#2b3340"/><stop offset="1" stop-color="#1a2029"/></linearGradient>` +
    `<rect x="1.5" y="1.5" width="93" height="93" rx="9" fill="#0d0f13"/>` +
    `<rect x="3" y="3" width="90" height="90" rx="8" fill="#c79a45"/>` +
    `<rect x="4" y="4" width="88" height="44" rx="7" fill="#f0d48a" opacity="0.55"/>` +
    `<rect x="6.5" y="6.5" width="83" height="83" rx="5.5" fill="#6b4a1c"/>` +
    `<rect x="9" y="9" width="78" height="78" rx="4" fill="url(#${g})"/>` +
    noise({ w: 96, h: 96, freq: 0.6, octaves: 2, seed: 4, rgb: [255, 255, 255], gain: 0.16, bias: -0.05 }).replace('<rect ', '<rect x="9" y="9" ').replace('width="96" height="96" filter', 'width="78" height="78" filter') +
    `<rect x="9.5" y="9.5" width="77" height="77" rx="4" fill="none" stroke="rgba(255,255,255,0.08)"/>` +
    [[13, 13], [83, 13], [13, 83], [83, 83]].map(([x, y]) => `<circle cx="${x}" cy="${y}" r="0" fill="#f0d48a"/>`).join('')
  );
}

function parchment() {
  return (
    `<rect x="1.5" y="1.5" width="93" height="93" rx="8" fill="#2a1d10"/>` +
    `<rect x="3.5" y="3.5" width="89" height="89" rx="6.5" fill="#e9dcb8"/>` +
    noise({ w: 96, h: 96, freq: 0.05, octaves: 3, seed: 8, rgb: [150, 110, 60], gain: 1.4, bias: -0.55 }).replace('<rect ', '<rect x="4" y="4" rx="6" ').replace('width="96" height="96" filter', 'width="88" height="88" filter') +
    `<rect x="6.5" y="6.5" width="83" height="83" rx="4.5" fill="none" stroke="rgba(110,80,40,0.45)" stroke-width="1.5"/>`
  );
}

function button() {
  const g = nextId('g');
  return (
    `<linearGradient id="${g}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffffff"/><stop offset="0.5" stop-color="#d9d9d9"/><stop offset="1" stop-color="#a8a8a8"/></linearGradient>` +
    `<rect x="1" y="1" width="62" height="62" rx="9" fill="#111"/>` +
    `<rect x="3" y="3" width="58" height="58" rx="7.5" fill="url(#${g})"/>` +
    `<rect x="5" y="5" width="54" height="54" rx="6" fill="none" stroke="rgba(255,255,255,0.75)" stroke-width="1.5"/>` +
    `<rect x="3" y="44" width="58" height="17" rx="7.5" fill="rgba(0,0,0,0.12)"/>`
  );
}

const O = '#221a10';
const icon = (inner) => `<g stroke="${O}" stroke-width="2.2" stroke-linejoin="round" stroke-linecap="round">${inner}</g>`;
const ICONS = [
  // gold
  () => icon(`<ellipse cx="24" cy="34" rx="16" ry="7" fill="#b9861f"/><path d="M8,26 v8 M40,26 v8" /><ellipse cx="24" cy="26" rx="16" ry="7" fill="#f2c94c"/><path d="M8,18 v8 M40,18 v8"/><ellipse cx="24" cy="18" rx="16" ry="7" fill="#ffe08a"/><path d="M17,18 h14" stroke="#b9861f"/>`),
  // science
  () => icon(`<path d="M19,6 h10 M21,6 v12 L10,38 a4,4 0 0 0 4,5 h20 a4,4 0 0 0 4,-5 L27,18 V6" fill="#dff3ff"/><path d="M15,30 h18 l5,9 a3,3 0 0 1 -3,3 h-22 a3,3 0 0 1 -3,-3 Z" fill="#3fa9f5"/><circle cx="21" cy="36" r="1.6" fill="#fff" stroke="none"/><circle cx="28" cy="33" r="1.2" fill="#fff" stroke="none"/>`),
  // food
  () => icon(`<path d="M24,44 V14" stroke="#b9861f" stroke-width="3"/><g fill="#f2c94c"><path d="M24,14 q-4,-6 0,-10 q4,4 0,10 Z"/><path d="M24,20 q-9,-1 -10,-9 q8,1 10,9 Z"/><path d="M24,20 q9,-1 10,-9 q-8,1 -10,9 Z"/><path d="M24,28 q-9,-1 -10,-9 q8,1 10,9 Z"/><path d="M24,28 q9,-1 10,-9 q-8,1 -10,9 Z"/><path d="M24,36 q-9,-1 -10,-9 q8,1 10,9 Z"/><path d="M24,36 q9,-1 10,-9 q-8,1 -10,9 Z"/></g>`),
  // production
  () => icon(`<path d="M12,40 L28,22" stroke="#6b4424" stroke-width="7"/><path d="M12,40 L28,22" stroke="#a9743c" stroke-width="3"/><path d="M22,10 l14,-4 l8,10 l-6,10 l-8,-4 l-6,2 l-6,-8 Z" fill="#b8c2cf"/><path d="M26,12 l9,-3" stroke="#fff" stroke-width="1.6"/>`),
  // happy
  () => icon(`<circle cx="24" cy="24" r="17" fill="#ffd95a"/><circle cx="18" cy="20" r="2" fill="${O}"/><circle cx="30" cy="20" r="2" fill="${O}"/><path d="M15,28 q9,9 18,0" fill="none" stroke-width="2.6"/>`),
  // population
  () => icon(`<circle cx="16" cy="18" r="6" fill="#d9b48a"/><path d="M5,42 q0,-16 11,-16 q11,0 11,16 Z" fill="#7a4fa0"/><circle cx="31" cy="16" r="7" fill="#e6c49c"/><path d="M18,43 q0,-18 13,-18 q13,0 13,18 Z" fill="#2f7fc4"/>`),
  // turn (hourglass)
  () => icon(`<path d="M12,6 h24 M12,42 h24" stroke-width="4"/><path d="M14,7 h20 q0,12 -8,17 q8,5 8,17 h-20 q0,-12 8,-17 q-8,-5 -8,-17 Z" fill="#dff3ff"/><path d="M18,12 h12 q-2,7 -6,10 q-4,-3 -6,-10 Z M17,40 q2,-8 7,-9 q5,1 7,9 Z" fill="#e2b04a" stroke="none"/>`),
  // attack
  () => icon(`<path d="M10,38 L36,8 l4,0 l0,4 L14,42 Z" fill="#d5dde6"/><path d="M8,32 l10,10" stroke-width="4"/><path d="M8,32 l10,10" stroke="#b9861f" stroke-width="2"/><path d="M6,44 l6,-6" stroke-width="5"/><path d="M6,44 l6,-6" stroke="#7a4a24" stroke-width="2.4"/>`),
  // defense
  () => icon(`<path d="M24,5 l16,5 v13 q0,13 -16,20 q-16,-7 -16,-20 V10 Z" fill="#4a7fc0"/><path d="M24,5 v38 M8,21 h32" stroke="#f0d48a" stroke-width="3"/>`),
  // moves (boot)
  () => icon(`<path d="M14,6 h14 v20 l12,7 q3,2 3,8 H12 q-2,-8 2,-14 Z" fill="#9a6534"/><path d="M12,41 h31" stroke-width="4"/><path d="M14,12 h14" stroke="#e8c14a"/>`),
  // culture
  () => icon(`<path d="M24,4 l5.5,12.5 l13.5,1.4 l-10.2,9 l3,13.3 L24,33 l-11.8,7.2 l3,-13.3 L5,17.9 l13.5,-1.4 Z" fill="#c66fd6"/><path d="M24,11 l3,7 l7,1" fill="none" stroke="#fff" stroke-width="1.6"/>`),
  // trade
  () => icon(`<path d="M6,18 h28 l-6,-7 M42,30 H14 l6,7" fill="none" stroke-width="6"/><path d="M6,18 h28 l-6,-7" fill="none" stroke="#6fcf6a" stroke-width="2.8"/><path d="M42,30 H14 l6,7" fill="none" stroke="#f2c94c" stroke-width="2.8"/>`),
];

module.exports = {
  panel: () => sheet({ cols: 1, rows: 1, fw: 96, fh: 96, draw: panel }),
  parchment: () => sheet({ cols: 1, rows: 1, fw: 96, fh: 96, draw: parchment }),
  button: () => sheet({ cols: 1, rows: 1, fw: 64, fh: 64, draw: button }),
  icons: () => sheet({ cols: ICONS.length, rows: 1, fw: 48, fh: 48, draw: (c) => ICONS[c]() }),
};
