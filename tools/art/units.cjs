// Animated unit sprites: one row per unit type, four idle frames each, all facing right.
const { n, sheet } = require('./lib.cjs');

const FW = 128;
const FH = 128;
const GROUND = 114;
const OUT = '#1d1611';
const WOOD = '#8b5a2b';
const STEEL = '#c4ccd6';
const BRONZE = '#c9973f';
const SKIN = '#e0b088';

const wrap = (inner) => `<g stroke="${OUT}" stroke-width="2" stroke-linejoin="round" stroke-linecap="round">${inner}</g>`;
const shadow = (cx, rx, ry = 6) => `<ellipse cx="${cx}" cy="${GROUND + 1}" rx="${rx}" ry="${ry}" fill="rgba(0,0,0,0.32)"/>`;
const limb = (x1, y1, x2, y2, color, w = 4.5) =>
  `<path d="M${n(x1)},${n(y1)} L${n(x2)},${n(y2)}" stroke="${OUT}" stroke-width="${w + 3}" fill="none"/>` +
  `<path d="M${n(x1)},${n(y1)} L${n(x2)},${n(y2)}" stroke="${color}" stroke-width="${w}" fill="none"/>`;
const wheel = (cx, cy, r, rot, rim = '#6b4424', spokes = 8) => {
  let s = `<circle cx="${cx}" cy="${cy}" r="${r}" fill="rgba(60,40,20,0.25)" stroke="${OUT}" stroke-width="${r > 10 ? 5.5 : 4.5}"/><circle cx="${cx}" cy="${cy}" r="${r}" fill="none" stroke="${rim}" stroke-width="${r > 10 ? 3 : 2.2}"/>`;
  for (let i = 0; i < spokes / 2; i++) {
    const a = rot + (i * Math.PI * 2) / spokes;
    s += `<path d="M${n(cx - r * Math.cos(a))},${n(cy - r * Math.sin(a))} L${n(cx + r * Math.cos(a))},${n(cy + r * Math.sin(a))}" stroke="${rim}" stroke-width="1.8"/>`;
  }
  return s + `<circle cx="${cx}" cy="${cy}" r="${Math.max(2, r * 0.2)}" fill="#3a2512"/>`;
};

// ---- headgear, drawn around the head centre (hx, hy) -----------------------------------------
const head = {
  hair: (c) => (hx, hy) => `<path d="M${hx - 8},${hy + 1} a8,8 0 0 1 16,-2 q-7,-5 -12,-1 q-2,2 -4,3 Z" fill="${c}"/>`,
  cap: (c) => (hx, hy) => `<path d="M${hx - 8.5},${hy - 1} a8.5,8.5 0 0 1 17,0 Z" fill="${c}"/>`,
  crest: (c, plume) => (hx, hy) =>
    `<path d="M${hx - 9},${hy - 9} q9,-13 19,-2 l-3,3 q-7,-7 -14,1 Z" fill="${plume}"/>` +
    `<path d="M${hx - 8.5},${hy - 1} a8.5,8.5 0 0 1 17,0 l1,4 h-4 l-1,-4 Z" fill="${c}"/>`,
  corinthian: (c, plume) => (hx, hy) =>
    `<path d="M${hx - 12},${hy - 6} q10,-18 22,-4 q-2,3 -5,2 q-7,-7 -13,3 Z" fill="${plume}"/>` +
    `<path d="M${hx - 8.5},${hy + 6} v-7 a8.5,8.5 0 0 1 17,0 v8 h-4 v-6 h-5 v5 Z" fill="${c}"/>`,
  hood: (c) => (hx, hy) => `<path d="M${hx - 9},${hy + 7} q-3,-17 9,-17 q9,0 9,9 q-6,-3 -10,0 q-3,3 -2,8 Z" fill="${c}"/>`,
  straw: () => (hx, hy) => `<path d="M${hx - 6},${hy - 4} q6,-9 12,0 Z" fill="#e3c86a"/><ellipse cx="${hx}" cy="${hy - 4}" rx="14" ry="3.5" fill="#d9b94f"/>`,
  shako: (c) => (hx, hy) => `<path d="M${hx - 7},${hy - 3} l1,-13 h12 l1,13 Z" fill="${c}"/><path d="M${hx - 8},${hy - 3} h19" stroke-width="3"/><circle cx="${hx}" cy="${hy - 17}" r="3" fill="#d43c2c"/>`,
  helm: (plume) => (hx, hy) =>
    `<path d="M${hx - 2},${hy - 8} q-10,-16 -18,-6 q8,-2 12,8 Z" fill="${plume}"/>` +
    `<path d="M${hx - 9},${hy + 8} v-9 a9,9 0 0 1 18,0 v9 Z" fill="${STEEL}"/><path d="M${hx + 1},${hy - 1} h8" stroke-width="2.5"/>`,
  tiara: (c) => (hx, hy) => `<path d="M${hx - 8},${hy - 2} q0,-14 8,-14 q8,0 8,14 Z" fill="${c}"/><path d="M${hx - 8},${hy - 3} h16" stroke="#e8c14a" stroke-width="3"/>`,
  brim: (c) => (hx, hy) => `<path d="M${hx - 7},${hy - 3} q7,-12 14,0 Z" fill="${c}"/><path d="M${hx - 13},${hy - 3} h26" stroke-width="5"/><path d="M${hx - 13},${hy - 3} h26" stroke="${c}" stroke-width="2.5"/>`,
  turban: (c) => (hx, hy) => `<path d="M${hx - 9},${hy - 1} q-1,-11 9,-11 q10,0 9,10 q-9,-4 -18,1 Z" fill="${c}"/><path d="M${hx - 9},${hy - 1} q-4,8 -2,14" fill="none" stroke="${c}" stroke-width="3"/>`,
};

/**
 * A standing figure with its feet at (x, y). Options:
 *  tunic, legs, skin, boots, robe (long garment), cape, head (headgear fn),
 *  back / front (fn(bob, sway) -> svg drawn behind / in front of the torso), mounted.
 */
function human(x, y, o, f) {
  const bob = [0, -1.3, 0, 1.3][f];
  const sway = [0, 1, 0, -1][f];
  const stride = [0, 1.2, 0, -1.2][f];
  const skin = o.skin || SKIN;
  const s = o.scale || 1.3;
  const sh = -46 + bob; // shoulder line
  let g = `<g transform="translate(${x},${y}) scale(${s})">`;
  if (o.cape) g += `<path d="M-7,${sh + 1} C-19,${sh + 12} -${17 + sway},-14 -13,-5 L-3,-9 Z" fill="${o.cape}"/>`;
  if (o.back) g += o.back(bob, sway);
  if (o.mounted) {
    g += `<path d="M-4,-25 L7,-16 L5,-3 h7 L13,-19 L4,-28 Z" fill="${o.legs}"/><path d="M4,-3 h9 v4 h-9 Z" fill="${o.boots || '#3a2a1c'}"/>`;
  } else if (!o.robe) {
    g += `<path d="M-6,-25 L${-7 - stride},-2 h6 L0,-23 Z" fill="${o.legs}"/><path d="M0,-25 L${2 + stride},-2 h6 L7,-24 Z" fill="${o.legs}"/>`;
    g += `<path d="M${-8 - stride},-3 h8 v4 h-9 Z M${1 + stride},-3 h9 l1,4 h-10 Z" fill="${o.boots || '#3a2a1c'}"/>`;
  }
  // rear arm
  g += limb(-5, sh + 4, -9, sh + 18 + sway, o.sleeve || skin, 4);
  // torso
  if (o.robe) g += `<path d="M-8,${sh} h15 l5,${44 - bob} h-25 Z" fill="${o.tunic}"/><path d="M-9,-3 h21" stroke="${o.trim || '#e8c14a'}" stroke-width="3"/>`;
  else g += `<path d="M-8,${sh} q8,-3 16,0 l2,${23 - bob} h-20 Z" fill="${o.tunic}"/>`;
  if (o.trim && !o.robe) g += `<path d="M-9,-25 h19" stroke="${o.trim}" stroke-width="3"/>`;
  if (o.chest) g += o.chest(bob);
  // head
  const hx = 2;
  const hy = sh - 9;
  g += `<path d="M-1,${sh - 2} h5 v4 h-5 Z" fill="${skin}"/><circle cx="${hx}" cy="${hy}" r="7.5" fill="${skin}"/><circle cx="${hx + 4}" cy="${hy - 0.5}" r="0.9" fill="${OUT}"/>`;
  if (o.beard) g += `<path d="M${hx - 3},${hy + 3} q5,9 9,0 q-4,2 -9,0 Z" fill="${o.beard}"/>`;
  if (o.head) g += o.head(hx, hy);
  if (o.front) g += o.front(bob, sway);
  return g + '</g>';
}

// ---- hand-held kit ---------------------------------------------------------------------------
const spear = (hx, len, tip = STEEL) => (bob, sway) =>
  `<path d="M${hx},2 L${hx + sway},${-len}" stroke-width="4.5"/><path d="M${hx},2 L${hx + sway},${-len}" stroke="${WOOD}" stroke-width="2.2"/>` +
  `<path d="M${hx + sway - 3},${-len + 2} l3,-11 l3,11 Z" fill="${tip}"/>`;
const armTo = (x, y, color) => (bob) => limb(5, -42 + bob, x, y + bob, color || SKIN, 4);
const roundShield = (cx, cy, r, fill, boss = BRONZE) => (bob) =>
  `<circle cx="${cx}" cy="${cy + bob}" r="${r}" fill="${fill}"/><circle cx="${cx}" cy="${cy + bob}" r="${r - 3.5}" fill="none" stroke="rgba(255,255,255,0.35)" stroke-width="1.5"/><circle cx="${cx}" cy="${cy + bob}" r="3" fill="${boss}"/>`;
const both = (...fns) => (bob, sway) => fns.map((fn) => fn(bob, sway)).join('');

// ---- animals ---------------------------------------------------------------------------------
function horse(x, y, o, f) {
  const s = o.scale || 1.15;
  const step = [0, 4, 0, -4][f];
  const lift = [0, -3, 0, -3][f];
  const nod = [0, 1.5, 0, -1][f];
  const coat = o.coat || '#8b5a2b';
  const dark = o.dark || '#4a2c14';
  const leg = (hipX, hoofX, raise) =>
    `<path d="M${hipX - 3},-26 L${hoofX - 2.5},${-3 + raise} h5.5 L${hipX + 3.5},-26 Z" fill="${coat}"/><path d="M${hoofX - 3},${-4 + raise} h6.5 v4 h-6.5 Z" fill="${dark}"/>`;
  let g = `<g transform="translate(${x},${y}) scale(${s})">`;
  g += `<path d="M-23,-40 C-36,-38 -${34 + step * 0.5},-20 -30,-12" fill="none" stroke="${OUT}" stroke-width="7"/><path d="M-23,-40 C-36,-38 -${34 + step * 0.5},-20 -30,-12" fill="none" stroke="${dark}" stroke-width="4"/>`;
  g += leg(-15, -17 - step, f === 1 ? lift : 0) + leg(15, 17 + step, f === 3 ? lift : 0);
  g += `<path d="M-24,-34 C-26,-48 -8,-48 6,-47 C12,-48 16,-54 ${20},${-62 + nod} L${30},${-72 + nod} L${33},${-64 + nod} L${43},${-55 + nod} L${40},${-49 + nod} L${32},${-52 + nod} C30,-42 28,-28 16,-25 L-14,-24 C-22,-24 -24,-28 -24,-34 Z" fill="${coat}"/>`;
  g += `<path d="M${19},${-63 + nod} C12,-58 9,-50 7,-45" fill="none" stroke="${dark}" stroke-width="4.5"/>`;
  g += `<circle cx="${36}" cy="${-59 + nod}" r="1.1" fill="${OUT}"/>`;
  g += leg(-19, -20 + step, f === 3 ? lift : 0) + leg(11, 12 - step, f === 1 ? lift : 0);
  if (o.cloth) g += `<path d="M-14,-46 h22 l3,20 q-14,5 -28,0 Z" fill="${o.cloth}"/><path d="M-16,-27 q14,5 28,0" fill="none" stroke="#e8c14a" stroke-width="2.5"/>`;
  else g += `<path d="M-9,-47 h14 l1,9 h-16 Z" fill="${o.saddle || '#5b2e1a'}"/>`;
  if (o.barding) g += `<path d="M${17},${-60 + nod} L${31},${-70 + nod} L${42},${-56 + nod} L${38},${-50 + nod} L${30},${-55 + nod} L24,-46 Z" fill="${o.barding}"/>`;
  return g + '</g>';
}

function camel(x, y, f) {
  const step = [0, 3, 0, -3][f];
  const nod = [0, 1, 0, -1][f];
  const coat = '#c9a063';
  const dark = '#8a6a3a';
  const leg = (hipX, hoofX) => `<path d="M${hipX - 3},-30 L${hoofX - 2},-3 h5 L${hipX + 3},-30 Z" fill="${coat}"/><path d="M${hoofX - 3},-4 h7 v4 h-7 Z" fill="${dark}"/>`;
  let g = `<g transform="translate(${x},${y}) scale(1.12)">`;
  g += leg(-16, -18 - step) + leg(14, 16 + step);
  g += `<path d="M-26,-36 C-28,-50 -18,-52 -12,-52 C-8,-66 6,-66 10,-52 C16,-52 20,-56 ${22},${-66 + nod} C${22},${-76 + nod} ${28},${-80 + nod} ${36},${-78 + nod} L${42},${-74 + nod} L${40},${-69 + nod} L${32},${-70 + nod} C30,-56 30,-36 16,-30 L-16,-29 C-24,-29 -26,-32 -26,-36 Z" fill="${coat}"/>`;
  g += `<circle cx="${35}" cy="${-75 + nod}" r="1.1" fill="${OUT}"/><path d="M-26,-40 q-6,6 -4,14" fill="none" stroke-width="3"/>`;
  g += leg(-20, -21 + step) + leg(10, 11 - step);
  // cargo
  g += `<path d="M-18,-52 h30 l3,18 h-36 Z" fill="#b5462e"/><path d="M-18,-46 h31 M-19,-40 h34" stroke="#e8c14a" stroke-width="2"/>`;
  g += `<rect x="-14" y="-66" width="12" height="15" rx="3" fill="#7a5a34"/><rect x="0" y="-64" width="12" height="13" rx="3" fill="#d9c7a0"/><path d="M-8,-66 v15 M6,-64 v13" stroke-width="1.5"/>`;
  return g + '</g>';
}

// ---- the roster (order matches UnitType in src/unit.rs) ---------------------------------------
const UNITS = [
  // Settler: traveller with staff and handcart
  (f) => {
    const roll = f * 0.4;
    return (
      shadow(60, 42) +
      wrap(
        `<path d="M14,${GROUND - 24} h34 l4,14 h-42 Z" fill="#9a6a3a"/><path d="M12,${GROUND - 26} q20,-26 40,0 Z" fill="#e9dfc4"/><path d="M22,${GROUND - 27} q3,-12 10,-17 M42,${GROUND - 27} q-3,-12 -10,-17" fill="none" stroke-width="1.4"/>` +
          wheel(30, GROUND - 9, 9, roll) +
          `<path d="M52,${GROUND - 16} L70,${GROUND - 26}" stroke-width="3"/>` +
          human(84, GROUND, { tunic: '#b98a4a', legs: '#6b5137', head: head.brim('#7a5a34'), beard: '#6b4a2a', back: spear(-9, 60, WOOD), cape: '#7f6a4a' }, f),
      )
    );
  },
  // Worker: swinging a pickaxe
  (f) => {
    const ang = [-55, -20, 25, -20][f];
    const tool = (bob) =>
      limb(5, -42 + bob, 14, -34 + bob, SKIN, 4) +
      `<g transform="rotate(${ang} 14 ${-34 + bob})"><path d="M14,${-30 + bob} v-30" stroke-width="5"/><path d="M14,${-30 + bob} v-30" stroke="${WOOD}" stroke-width="2.5"/><path d="M2,${-56 + bob} q12,-9 24,0 l-2,3 q-10,-6 -20,0 Z" fill="${STEEL}"/></g>`;
    return shadow(60, 24) + wrap(human(58, GROUND, { tunic: '#c9b28a', legs: '#7a5a3c', head: head.straw(), front: tool }, f));
  },
  // Warrior: club and hide shield
  (f) => {
    const club = (bob, sway) =>
      limb(5, -42 + bob, 15, -36 + bob, SKIN, 4) + `<path d="M13,${-32 + bob} l${4 + sway},-26 q6,-5 8,2 q1,6 -5,8 l-3,17 Z" fill="${WOOD}"/>`;
    return (
      shadow(62, 24) +
      wrap(human(60, GROUND, { tunic: '#a9743c', legs: SKIN, boots: '#6b4a2a', head: head.hair('#3a2414'), beard: '#3a2414', trim: '#5b3a1c', front: both(club, roundShield(-6, -32, 9, '#8a6a44', '#5b3a1c')) }, f))
    );
  },
  // Spearman
  (f) =>
    shadow(62, 24) +
    wrap(human(60, GROUND, { tunic: '#3f6fb0', legs: '#d9c7a0', head: head.cap('#8a5a2c'), back: spear(-10, 74), front: roundShield(9, -33, 11.5, '#b8452f') }, f)),
  // Archer
  (f) => {
    const pull = [0, 1.5, 3, 1.5][f];
    const bow = (bob) =>
      limb(5, -42 + bob, 17, -40 + bob, '#3d7a3a', 4) +
      `<path d="M18,${-62 + bob} Q32,${-40 + bob} 18,${-18 + bob}" fill="none" stroke-width="5"/><path d="M18,${-62 + bob} Q32,${-40 + bob} 18,${-18 + bob}" fill="none" stroke="${WOOD}" stroke-width="2.5"/>` +
      `<path d="M18,${-62 + bob} L${18 - pull},${-40 + bob} L18,${-18 + bob}" fill="none" stroke="#efe6d0" stroke-width="1"/><path d="M${6 - pull},${-40 + bob} H30" stroke="#efe6d0" stroke-width="1.6"/><path d="M30,${-43 + bob} l5,3 l-5,3 Z" fill="${STEEL}"/>`;
    const quiver = (bob) => `<path d="M-13,${-48 + bob} l7,2 l-3,20 l-7,-2 Z" fill="#7a4a24"/><path d="M-12,${-50 + bob} l1,-6 M-9,${-49 + bob} l2,-6" stroke="#efe6d0" stroke-width="1.6"/>`;
    return shadow(62, 24) + wrap(human(58, GROUND, { tunic: '#3d7a3a', sleeve: '#3d7a3a', legs: '#6b5137', head: head.hood('#2f5f2c'), back: quiver, front: bow }, f));
  },
  // Horseman
  (f) =>
    shadow(62, 44) +
    wrap(horse(60, GROUND, { coat: '#9a6534' }, f) + human(58, GROUND - 24 + [0, -1.5, 0, -1.5][f], { mounted: true, scale: 1.15, tunic: '#b8452f', legs: '#6b5137', head: head.cap('#8a5a2c'), back: spear(-8, 62), front: armTo(16, -34) }, f)),
  // Legion
  (f) => {
    const scutum = (bob) => `<rect x="2" y="${-46 + bob}" width="15" height="30" rx="3" fill="#b3261e"/><path d="M9.5,${-46 + bob} v30 M2,${-31 + bob} h15" stroke="#e8c14a" stroke-width="1.8"/><circle cx="9.5" cy="${-31 + bob}" r="3" fill="${BRONZE}"/>`;
    const gladius = (bob, sway) => limb(-5, -42 + bob, -11, -32 + bob, SKIN, 4) + `<path d="M-12,${-33 + bob} l${-2 + sway},-20 l4,-4 l2,5 l-1,19 Z" fill="${STEEL}"/><path d="M-15,${-33 + bob} h8" stroke-width="3.5"/>`;
    const lorica = (bob) => `<path d="M-8,${-40 + bob} h17 M-8,${-35 + bob} h18 M-9,${-30 + bob} h19" stroke="rgba(0,0,0,0.45)" stroke-width="1.3"/>`;
    return shadow(62, 24) + wrap(human(60, GROUND, { tunic: STEEL, legs: '#b3261e', cape: '#b3261e', head: head.crest(BRONZE, '#c8281c'), chest: lorica, back: gladius, front: scutum }, f));
  },
  // War chariot
  (f) => {
    const rot = f * 0.45;
    const bowRider = (bob) => limb(5, -42 + bob, 16, -44 + bob, SKIN, 4) + `<path d="M17,${-60 + bob} Q28,${-44 + bob} 17,${-28 + bob}" fill="none" stroke-width="4.5"/><path d="M17,${-60 + bob} Q28,${-44 + bob} 17,${-28 + bob}" fill="none" stroke="${WOOD}" stroke-width="2.2"/><path d="M17,${-60 + bob} V${-28 + bob}" stroke="#efe6d0" stroke-width="1"/>`;
    return (
      shadow(64, 54) +
      wrap(
        human(30, GROUND - 16, { scale: 1.1, robe: true, tunic: '#e9dfc4', trim: '#2a8a8a', head: head.tiara('#2a6fa0'), front: bowRider }, f) +
          `<path d="M12,${GROUND - 34} q2,18 30,16 v-12 q-14,2 -16,-12 Z" fill="#c9973f"/><path d="M14,${GROUND - 26} q8,10 26,8" fill="none" stroke="#7a2a1a" stroke-width="2.5"/>` +
          `<path d="M40,${GROUND - 20} L74,${GROUND - 32}" stroke-width="3"/>` +
          wheel(28, GROUND - 13, 13, rot, '#7a4a24') +
          horse(88, GROUND, { coat: '#d9c7a0', dark: '#8a7a5a', scale: 0.95, saddle: '#2a8a8a' }, f),
      )
    );
  },
  // Immortal
  (f) =>
    shadow(62, 24) +
    wrap(
      human(60, GROUND, { robe: true, tunic: '#6a3a8f', trim: '#e8c14a', sleeve: '#e8c14a', head: head.turban('#e8c14a'), beard: '#2a1a12', back: spear(-10, 72, '#e8c14a'), front: (bob) => `<path d="M3,${-44 + bob} q11,-2 12,10 q1,12 -12,14 q-5,-12 0,-24 Z" fill="#b98a4a"/><path d="M5,${-38 + bob} h9 M4,${-32 + bob} h11 M5,${-26 + bob} h9" stroke="#6b4a2a" stroke-width="1.4"/>` }, f),
    ),
  // Phalanx (hoplite)
  (f) =>
    shadow(62, 26) +
    wrap(
      human(60, GROUND, { tunic: BRONZE, legs: SKIN, boots: BRONZE, cape: '#2a5f9a', head: head.corinthian(BRONZE, '#c8281c'), back: spear(-10, 84), front: (bob) => roundShield(9, -32, 14.5, BRONZE, '#7a2a1a')(bob) + `<path d="M3,${-36 + bob} l6,9 l6,-9" fill="none" stroke="#7a2a1a" stroke-width="2.2"/>` }, f),
    ),
  // Catapult
  (f) => {
    const ang = [-28, -24, -28, -32][f];
    return (
      shadow(64, 48) +
      wrap(
        `<path d="M22,${GROUND - 14} h82 v7 h-82 Z" fill="${WOOD}"/>` +
          `<path d="M58,${GROUND - 14} L66,${GROUND - 62} h6 L84,${GROUND - 14} h-7 L69,${GROUND - 50} L64,${GROUND - 14} Z" fill="#7a4a24"/>` +
          `<g transform="rotate(${ang} 69 ${GROUND - 54})"><path d="M26,${GROUND - 57} h66 v6 h-66 Z" fill="${WOOD}"/><path d="M18,${GROUND - 62} q8,-6 14,2 l-2,8 h-10 Z" fill="#5b3a1c"/><circle cx="25" cy="${GROUND - 64}" r="6" fill="#9a958c"/><rect x="86" y="${GROUND - 62}" width="12" height="16" rx="2" fill="#6f6a62"/></g>` +
          `<path d="M40,${GROUND - 14} L62,${GROUND - 40}" stroke-width="2.5"/>` +
          wheel(34, GROUND - 9, 10, f * 0.3) +
          wheel(92, GROUND - 9, 10, f * 0.3),
      )
    );
  },
  // Knight
  (f) => {
    const lance = (bob, sway) => limb(5, -42 + bob, 14, -32 + bob, STEEL, 4) + `<path d="M-14,${-26 + bob} L${44 + sway},${-58 + bob}" stroke-width="5"/><path d="M-14,${-26 + bob} L${44 + sway},${-58 + bob}" stroke="#d9c7a0" stroke-width="2.4"/><path d="M${42 + sway},${-61 + bob} l9,-2 l-6,7 Z" fill="${STEEL}"/><path d="M${30},${-52 + bob} l-2,-12 l12,5 Z" fill="#c8281c"/>`;
    return (
      shadow(62, 46) +
      wrap(horse(60, GROUND, { coat: '#d8d8dc', dark: '#77777f', cloth: '#2a4f9a', barding: STEEL }, f) + human(58, GROUND - 24 + [0, -1.5, 0, -1.5][f], { mounted: true, scale: 1.15, tunic: STEEL, sleeve: STEEL, legs: STEEL, boots: '#77777f', head: head.helm('#c8281c'), front: both(lance, roundShield(2, -34, 8, '#2a4f9a', '#e8c14a')) }, f))
    );
  },
  // Cannon
  (f) => {
    const puff = [0.15, 0.4, 0.65, 0.35][f];
    return (
      shadow(62, 46) +
      wrap(
        `<path d="M18,${GROUND - 4} L58,${GROUND - 22} l4,5 L26,${GROUND} Z" fill="#7a4a24"/>` +
          `<g transform="rotate(-10 60 ${GROUND - 26})"><path d="M34,${GROUND - 35} h66 l3,3 v10 l-3,3 h-66 q-6,-8 0,-16 Z" fill="${BRONZE}"/><path d="M50,${GROUND - 35} v16 M84,${GROUND - 35} v16 M98,${GROUND - 36} v18" stroke-width="2.6"/><path d="M38,${GROUND - 31} h56" stroke="rgba(255,255,255,0.45)" stroke-width="2"/><circle cx="32" cy="${GROUND - 27}" r="4" fill="#8a6424"/></g>` +
          wheel(58, GROUND - 16, 16, f * 0.2, '#6b4424', 10) +
          `<g stroke="none" fill="rgba(235,235,235,${puff})"><circle cx="${112 + f}" cy="${GROUND - 44 - f * 3}" r="${5 + f}"/><circle cx="${118 + f}" cy="${GROUND - 50 - f * 4}" r="${3 + f}"/></g>`,
      )
    );
  },
  // Rifleman
  (f) => {
    const musket = (bob, sway) =>
      limb(5, -42 + bob, 13, -30 + bob, '#2a4f9a', 4) +
      `<g transform="rotate(${sway} 8 ${-30 + bob})"><path d="M-4,${-18 + bob} L22,${-66 + bob}" stroke-width="6"/><path d="M-4,${-18 + bob} L10,${-44 + bob}" stroke="#6b4424" stroke-width="3.6"/><path d="M10,${-44 + bob} L22,${-66 + bob}" stroke="#555d68" stroke-width="2.4"/><path d="M22,${-66 + bob} l5,-10" stroke="${STEEL}" stroke-width="1.8"/></g>`;
    const straps = (bob) => `<path d="M-7,${-45 + bob} L9,${-24 + bob} M8,${-45 + bob} L-8,${-24 + bob}" stroke="#f1ecdf" stroke-width="2.4"/>`;
    return shadow(62, 24) + wrap(human(60, GROUND, { tunic: '#2a4f9a', sleeve: '#2a4f9a', legs: '#e9e4d6', boots: '#22201d', head: head.shako('#22201d'), chest: straps, front: musket }, f));
  },
  // Cavalry
  (f) => {
    const saber = (bob, sway) => limb(5, -42 + bob, 15, -46 + bob, '#2f5a3a', 4) + `<path d="M15,${-46 + bob} q${10 + sway},-14 ${6 + sway},-30" fill="none" stroke-width="4.5"/><path d="M15,${-46 + bob} q${10 + sway},-14 ${6 + sway},-30" fill="none" stroke="${STEEL}" stroke-width="2"/><path d="M11,${-45 + bob} h8" stroke-width="3.5"/>`;
    return shadow(62, 44) + wrap(horse(60, GROUND, { coat: '#3a2a22', dark: '#18110d', saddle: '#2f5a3a' }, f) + human(58, GROUND - 24 + [0, -1.5, 0, -1.5][f], { mounted: true, scale: 1.15, tunic: '#2f5a3a', sleeve: '#2f5a3a', legs: '#d9c7a0', boots: '#22201d', head: head.brim('#22201d'), front: saber }, f));
  },
  // Tank
  (f) => {
    const jig = [0, -0.8, 0, 0.8][f];
    let treads = '';
    for (let x = 22 + ((f * 3) % 12); x < 106; x += 12) treads += `<path d="M${x},${GROUND - 21} v3 M${x - 4},${GROUND - 1} v-3" stroke-width="2.2"/>`;
    let wheels = '';
    for (let x = 32; x <= 96; x += 16) wheels += `<circle cx="${x}" cy="${GROUND - 10}" r="6.5" fill="#5a5f4a"/><circle cx="${x}" cy="${GROUND - 10}" r="1.6" fill="${OUT}"/>`;
    return (
      shadow(64, 52, 7) +
      wrap(
        `<g transform="translate(0,${jig})"><path d="M22,${GROUND - 26} l8,-14 h56 l22,14 Z" fill="#6f7a4a"/><path d="M36,${GROUND - 40} l5,-14 h30 l8,14 Z" fill="#7d8a55"/><path d="M76,${GROUND - 49} h40 v5 h-39 Z" fill="#5a6440"/><rect x="114" y="${GROUND - 51}" width="7" height="9" fill="#4a5236"/><path d="M48,${GROUND - 54} v-7 h10 v7" fill="#5a6440"/><path d="M30,${GROUND - 40} h56" stroke="rgba(255,255,255,0.3)" stroke-width="1.6"/><path d="M52,${GROUND - 40} l4,-10 h8" fill="none" stroke-width="1.3"/></g>` +
          `<rect x="16" y="${GROUND - 22}" width="96" height="23" rx="11.5" fill="#3a3d33"/>` +
          wheels +
          treads +
          `<g stroke="none" fill="rgba(210,210,210,${[0.5, 0.35, 0.2, 0.4][f]})"><circle cx="${14 - f * 2}" cy="${GROUND - 34 - f * 3}" r="${4 + f}"/></g>`,
      )
    );
  },
  // Artillery
  (f) => {
    const recoil = [0, 0, -4, -1.5][f];
    return (
      shadow(62, 48) +
      wrap(
        `<path d="M10,${GROUND - 2} L54,${GROUND - 18} l3,5 L16,${GROUND + 2} Z" fill="#4f5a3c"/>` +
          `<g transform="rotate(-24 62 ${GROUND - 24})"><g transform="translate(${recoil},0)"><path d="M44,${GROUND - 30} h74 v7 h-74 Z" fill="#59624a"/><rect x="116" y="${GROUND - 32}" width="8" height="11" rx="1.5" fill="#3f4636"/></g><path d="M40,${GROUND - 34} h34 v14 h-34 Z" fill="#6b7655"/></g>` +
          `<path d="M66,${GROUND - 50} l10,-4 v40 l-10,4 Z" fill="#7d8a62"/>` +
          wheel(58, GROUND - 14, 14, f * 0.15, '#3f4636', 10) +
          (f === 2 ? `<g stroke="none"><circle cx="118" cy="${GROUND - 62}" r="7" fill="rgba(255,214,120,0.85)"/><circle cx="122" cy="${GROUND - 66}" r="4" fill="rgba(255,255,255,0.8)"/></g>` : ''),
      )
    );
  },
  // Diplomat
  (f) => {
    const scroll = (bob, sway) => limb(5, -42 + bob, 15, -36 + bob, '#f1ecdf', 4) + `<g transform="rotate(${sway * 3} 16 ${-36 + bob})"><rect x="13" y="${-48 + bob}" width="7" height="22" rx="3" fill="#f3ead0"/><path d="M13,${-37 + bob} h7" stroke="#b3261e" stroke-width="2.5"/></g>`;
    return shadow(62, 24) + wrap(human(60, GROUND, { robe: true, tunic: '#f1ecdf', trim: '#7a2a8f', sleeve: '#f1ecdf', cape: '#7a2a8f', head: head.hair('#bfbfc4'), beard: '#bfbfc4', front: scroll }, f));
  },
  // Caravan
  (f) => shadow(62, 44) + wrap(camel(60, GROUND, f)),
];

module.exports = {
  units: () => sheet({ cols: 4, rows: UNITS.length, fw: FW, fh: FH, draw: (col, row) => UNITS[row](col) }),
};
