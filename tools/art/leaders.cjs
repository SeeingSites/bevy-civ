// Leader portraits for the diplomacy screens: one 128px bust per civilization.
const { sheet } = require('./lib.cjs');

const O = '#1f1711';
const GOLD = '#e3b341';

// Headgear and hair are drawn around a head centred on (64, 58), about 40 wide and 50 tall.
const gear = {
  laurel: () => `<path d="M40,50 q24,-22 48,0" fill="none" stroke="#4f8a3a" stroke-width="7"/><path d="M40,50 q24,-22 48,0" fill="none" stroke="#7fbf5a" stroke-width="3" stroke-dasharray="5 4"/>`,
  nemes: () => `<path d="M38,58 q-4,-38 26,-38 q30,0 26,38 l8,40 h-18 l-4,-36 h-24 l-4,36 h-18 Z" fill="#2a5fa8"/><path d="M40,44 h48 M37,56 h10 M81,56 h10 M34,70 h12 M82,70 h12 M32,84 h14 M82,84 h14" stroke="${GOLD}" stroke-width="4"/><path d="M60,22 q4,-8 8,0 v8 h-8 Z" fill="${GOLD}"/>`,
  helmet: (c, plume) => `<path d="M52,24 q12,-20 30,-4 q-10,0 -14,10 Z" fill="${plume}"/><path d="M40,60 v-12 q0,-24 24,-24 q24,0 24,24 v12 h-8 v-12 h-32 v12 Z" fill="${c}"/><path d="M62,36 h4 v16 h-4 Z" fill="${c}"/>`,
  tiara: () => `<path d="M42,46 l2,-26 h40 l2,26 Z" fill="#6a3a8f"/><path d="M42,44 h44 M44,32 h40" stroke="${GOLD}" stroke-width="4"/><circle cx="64" cy="26" r="3.5" fill="#d43c2c"/>`,
  turban: (c, jewel) => `<path d="M38,50 q-2,-30 26,-30 q28,0 26,30 q-26,-12 -52,0 Z" fill="${c}"/><path d="M42,38 q22,-10 44,2 M40,46 q24,-12 48,0" fill="none" stroke="rgba(0,0,0,0.25)" stroke-width="2"/>${jewel ? `<circle cx="64" cy="32" r="5" fill="${jewel}"/><path d="M64,27 q2,-12 8,-14" fill="none" stroke="#fff" stroke-width="3"/>` : ''}`,
  mianguan: () => `<path d="M44,46 v-12 h40 v12 Z" fill="#22201d"/><path d="M30,30 h68 v6 h-68 Z" fill="#22201d"/><path d="M36,36 v14 M44,36 v10 M84,36 v10 M92,36 v14" stroke="${GOLD}" stroke-width="2.5" stroke-dasharray="3 2"/>`,
  kabuto: () => `<path d="M46,26 q-14,-14 -8,-22 q6,12 20,14 Z M82,26 q14,-14 8,-22 q-6,12 -20,14 Z" fill="${GOLD}"/><path d="M38,54 q-2,-32 26,-32 q28,0 26,32 l8,12 h-16 l-2,-14 h-32 l-2,14 h-16 Z" fill="#33363f"/><path d="M40,46 h48" stroke="#c8281c" stroke-width="4"/>`,
  crown: (hair) => `<path d="M36,62 q-6,-40 28,-40 q34,0 28,40 q-4,16 2,30 h-14 l-2,-40 h-28 l-2,40 h-14 q6,-14 2,-30 Z" fill="${hair}"/><path d="M44,34 l4,-16 l8,10 l8,-14 l8,14 l8,-10 l4,16 Z" fill="${GOLD}"/><circle cx="64" cy="26" r="3" fill="#d43c2c"/>`,
  cap: (c) => `<path d="M40,46 q0,-26 24,-26 q24,0 24,26 Z" fill="${c}"/><path d="M40,46 h48" stroke="${GOLD}" stroke-width="5"/>`,
  feathers: () => { let s = ''; for (let i = 0; i < 9; i++) { const a = -70 + i * 17.5; s += `<path d="M64,44 q-6,-24 0,-40 q6,16 0,40 Z" fill="${i % 2 ? '#2f9a5a' : '#39b3a0'}" transform="rotate(${a} 64 50)"/>`; } return s + `<path d="M40,48 q24,-12 48,0 v6 q-24,-10 -48,0 Z" fill="${GOLD}"/><circle cx="64" cy="44" r="4" fill="#2a8ad0"/>`; },
  fringe: () => `<path d="M40,46 q0,-26 24,-26 q24,0 24,26 Z" fill="#22201d"/><path d="M40,42 h48 v8 h-48 Z" fill="#c8281c"/><path d="M52,50 v8 M58,50 v10 M64,50 v10 M70,50 v10 M76,50 v8" stroke="#c8281c" stroke-width="3"/><circle cx="38" cy="66" r="6" fill="${GOLD}"/><circle cx="90" cy="66" r="6" fill="${GOLD}"/>`,
  wings: () => `<path d="M42,34 q-22,-10 -24,-28 q14,10 30,14 Z M86,34 q22,-10 24,-28 q-14,10 -30,14 Z" fill="#f1ecdf"/><path d="M40,50 q0,-28 24,-28 q24,0 24,28 Z" fill="#a9743c"/><path d="M40,48 h48" stroke="#6b4424" stroke-width="5"/>`,
  veil: () => `<path d="M34,64 q-6,-44 30,-44 q36,0 30,44 q-2,18 4,32 h-18 l-2,-44 h-28 l-2,44 h-18 q6,-14 4,-32 Z" fill="#f1ecdf"/><path d="M48,32 l4,-10 l6,6 l6,-10 l6,10 l6,-6 l4,10 Z" fill="${GOLD}"/>`,
  beret: () => `<path d="M36,44 q4,-24 34,-22 q24,2 22,18 q-28,-8 -56,4 Z" fill="#2a2f5a"/><path d="M84,30 q14,-16 22,-12 q-6,10 -18,16 Z" fill="#f1ecdf"/>`,
  bigTurban: () => `<path d="M32,48 q-4,-38 32,-38 q36,0 32,38 q-32,-14 -64,0 Z" fill="#f4f1e6"/><path d="M38,34 q26,-14 52,2 M34,44 q30,-14 60,0" fill="none" stroke="rgba(0,0,0,0.2)" stroke-width="2"/><path d="M60,14 q4,-10 8,0 v10 h-8 Z" fill="#c8281c"/>`,
  furCrown: () => `<path d="M44,40 q0,-22 20,-22 q20,0 20,22 Z" fill="${GOLD}"/><path d="M38,40 h52 v10 h-52 Z" fill="#5a3d24"/><path d="M62,10 h4 v10 h-4 Z M58,13 h12 v3 h-12 Z" fill="${GOLD}"/>`,
};
const beard = {
  none: () => '',
  full: (c) => `<path d="M46,68 q2,26 18,28 q16,-2 18,-28 q-18,10 -36,0 Z" fill="${c}"/>`,
  long: (c) => `<path d="M46,68 q-2,34 18,44 q20,-10 18,-44 q-18,10 -36,0 Z" fill="${c}"/><path d="M54,84 q2,12 0,20 M64,86 v22 M74,84 q-2,12 0,20" stroke="rgba(0,0,0,0.3)" stroke-width="1.5" fill="none"/>`,
  moustache: (c) => `<path d="M50,74 q14,-8 28,0 q-6,6 -14,2 q-8,4 -14,-2 Z" fill="${c}"/>`,
  droop: (c) => `<path d="M48,72 q16,-8 32,0 l4,20 l-8,-12 q-12,6 -24,0 l-8,12 Z" fill="${c}"/>`,
  thin: (c) => `<path d="M58,82 q6,18 12,0 q-6,6 -12,0 Z M52,74 q12,-5 24,0" fill="${c}" stroke="${c}" stroke-width="2"/>`,
};

// [background, skin, robe, robe trim, headgear, beard, female]
const LEADERS = [
  ['#7a2a22', '#e2b08a', '#f1ecdf', '#b3261e', gear.laurel, beard.none, false],
  ['#1f4f6a', '#c98f62', '#f1ecdf', GOLD, gear.nemes, beard.none, true],
  ['#2a4f7a', '#dba983', '#f1ecdf', '#2a6fb0', () => gear.helmet('#c9973f', '#c8281c'), beard.full('#4a2c14'), false],
  ['#4a2a6a', '#c79468', '#6a3a8f', GOLD, gear.tiara, beard.long('#22160e'), false],
  ['#8a4a14', '#b07a4e', '#e08a2a', GOLD, () => gear.turban('#c8281c', '#39b3a0'), beard.moustache('#22160e'), false],
  ['#7a5a12', '#e3bd8c', '#e2b030', '#b3261e', gear.mianguan, beard.thin('#22160e'), false],
  ['#3a2228', '#e0b68a', '#33363f', '#c8281c', gear.kabuto, beard.moustache('#22160e'), false],
  ['#5a1f2a', '#f0cfb2', '#8f1f2f', GOLD, () => gear.crown('#b5562a'), beard.none, true],
  ['#6a4a10', '#6b4428', '#e2b030', '#f1ecdf', () => gear.cap(GOLD), beard.full('#1a120c'), false],
  ['#1f5a4a', '#b9794c', '#2f9a5a', GOLD, gear.feathers, beard.none, false],
  ['#6a2a1a', '#b9794c', '#c8281c', GOLD, gear.fringe, beard.none, false],
  ['#4a2a5a', '#c08a5c', '#6a3a8f', '#c9973f', () => gear.helmet('#8a8f98', '#6a3a8f'), beard.full('#22160e'), false],
  ['#2f4a2a', '#e6bd98', '#4f7a3a', '#a9743c', gear.wings, beard.droop('#d9a441'), false],
  ['#5a2a2a', '#ecc8a6', '#22201d', GOLD, gear.veil, beard.none, true],
  ['#2a2f5a', '#e6bd98', '#2a2f5a', GOLD, gear.beret, beard.full('#5a3418'), false],
  ['#6a1f1f', '#d1a074', '#b3261e', GOLD, gear.bigTurban, beard.full('#22160e'), false],
  ['#3a2f4a', '#e6bd98', '#7a2a22', GOLD, gear.furCrown, beard.long('#6b4a2a'), false],
];

function portrait(index) {
  const [bg, skin, robe, trim, head, chin, female] = LEADERS[index];
  return (
    `<defs><clipPath id="p${index}"><rect x="4" y="4" width="120" height="120" rx="12"/></clipPath><radialGradient id="pg${index}" cx="0.5" cy="0.35" r="0.8"><stop offset="0" stop-color="${bg}"/><stop offset="1" stop-color="#14100c"/></radialGradient></defs>` +
    `<rect x="4" y="4" width="120" height="120" rx="12" fill="url(#pg${index})"/>` +
    `<g clip-path="url(#p${index})" stroke="${O}" stroke-width="2.2" stroke-linejoin="round" stroke-linecap="round">` +
    `<path d="M14,128 q4,-34 34,-38 h32 q30,4 34,38 Z" fill="${robe}"/><path d="M48,90 l16,20 l16,-20" fill="none" stroke="${trim}" stroke-width="6"/>` +
    `<path d="M54,78 h20 v16 q-10,8 -20,0 Z" fill="${skin}"/>` +
    `<ellipse cx="64" cy="58" rx="${female ? 19 : 21}" ry="25" fill="${skin}"/>` +
    `<path d="M50,${female ? 55 : 54} h8 M70,${female ? 55 : 54} h8" stroke-width="${female ? 2 : 3}"/>` +
    `<circle cx="54" cy="59" r="2" fill="${O}" stroke="none"/><circle cx="74" cy="59" r="2" fill="${O}" stroke="none"/>` +
    `<path d="M64,60 l-3,9 h5" fill="none" stroke-width="1.8"/><path d="M57,76 q7,${female ? 5 : 3} 14,0" fill="${female ? '#b3423a' : 'none'}" stroke-width="2"/>` +
    (typeof chin === 'function' ? chin() : chin) + head() +
    `</g><rect x="4" y="4" width="120" height="120" rx="12" fill="none" stroke="#c79a45" stroke-width="4"/><rect x="2" y="2" width="124" height="124" rx="14" fill="none" stroke="#14100c" stroke-width="2"/>`
  );
}

module.exports = {
  leaders: () => sheet({ cols: LEADERS.length, rows: 1, fw: 128, fh: 128, draw: (c) => portrait(c) }),
};
