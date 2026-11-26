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

// ---- ships: a hull bobbing on the waterline -------------------------------------------------
function ship(f, roll, inner) {
  const bob = [0, -1.5, 0, 1.5][f];
  const tilt = [-roll, 0, roll, 0][f];
  const wave = [0, 4, 8, 4][f];
  return wrap(
    `<g transform="translate(0,${bob}) rotate(${tilt} 64 ${GROUND - 14})">${inner}</g>` +
      `<path d="M${-4 + wave},${GROUND - 1} q8,-6 16,0 t16,0 t16,0 t16,0 t16,0 t16,0 t16,0 t16,0" fill="none" stroke="rgba(225,242,255,0.9)" stroke-width="3"/>` +
      `<path d="M108,${GROUND - 8} q10,-2 14,6" fill="none" stroke="rgba(255,255,255,0.85)" stroke-width="2.5"/>`,
  );
}
const smoke = (x, y, f) =>
  `<g stroke="none" fill="rgba(70,70,76,${[0.75, 0.6, 0.45, 0.6][f]})"><circle cx="${x - f * 3}" cy="${y - f * 3}" r="${5 + f}"/><circle cx="${x - 8 - f * 4}" cy="${y - 9 - f * 4}" r="${4 + f}"/></g>`;
const turret = (x, y, k) =>
  `<path d="M${x - 9 * k},${y} v${-8 * k} h${16 * k} l${3 * k},${8 * k} Z" fill="#6b727d"/><path d="M${x + 6 * k},${y - 5 * k} h${16 * k}" stroke-width="${3.5 * k}"/><path d="M${x + 6 * k},${y - 5 * k} h${16 * k}" stroke="#4f5660" stroke-width="${1.6 * k}"/>`;

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
  // Transport boat: shown for any land unit while it is at sea
  (f) => {
    const bob = [0, -2, 0, 2][f];
    const tilt = [-2, 0, 2, 0][f];
    const flutter = [0, 2, 4, 2][f];
    return wrap(
      `<g transform="translate(0,${bob}) rotate(${tilt} 64 ${GROUND - 14})">` +
        `<path d="M62,${GROUND - 24} V${GROUND - 92}" stroke-width="5"/><path d="M62,${GROUND - 24} V${GROUND - 92}" stroke="${WOOD}" stroke-width="2.5"/>` +
        `<path d="M66,${GROUND - 88} q${30 + flutter},14 ${34},48 h-34 Z" fill="#efe6d0"/><path d="M66,${GROUND - 70} h24 M66,${GROUND - 54} h32" stroke="rgba(0,0,0,0.2)" stroke-width="1.5"/>` +
        `<path d="M58,${GROUND - 84} q-${20 + flutter},12 -26,40 h26 Z" fill="#d9c7a0"/>` +
        `<path d="M62,${GROUND - 92} l14,4 l-14,5 Z" fill="#c8281c"/>` +
        `<circle cx="40" cy="${GROUND - 30}" r="5.5" fill="${SKIN}"/><circle cx="84" cy="${GROUND - 30}" r="5.5" fill="${SKIN}"/>` +
        `<path d="M14,${GROUND - 26} h100 q-6,22 -24,24 h-52 q-18,-2 -24,-24 Z" fill="#8b5a2b"/><path d="M18,${GROUND - 18} h92" stroke="#5b3a1c" stroke-width="2"/><path d="M14,${GROUND - 26} h100" stroke="#c9973f" stroke-width="3"/>` +
        `</g>` +
        `<path d="M8,${GROUND - 1} q8,-6 16,0 t16,0 t16,0 t16,0 t16,0 t16,0 t16,0" fill="none" stroke="rgba(220,240,255,0.9)" stroke-width="3"/>`,
    );
  },
  // ---- warships: drawn on the waterline with a bow wave ----
  // Galley
  (f) => {
    const sweep = [-14, -4, 10, -2][f];
    let oars = '';
    for (let x = 34; x <= 84; x += 10) oars += `<path d="M${x},${GROUND - 20} l${sweep * 0.45},20" stroke-width="4"/><path d="M${x},${GROUND - 20} l${sweep * 0.45},20" stroke="#c9a06a" stroke-width="1.8"/>`;
    return ship(f, 2,
      `<path d="M60,${GROUND - 26} V${GROUND - 84}" stroke-width="5"/><path d="M60,${GROUND - 26} V${GROUND - 84}" stroke="${WOOD}" stroke-width="2.5"/>` +
      `<path d="M40,${GROUND - 78} h40 q${4 + f % 2 * 2},18 0,34 h-40 q-${4 + f % 2 * 2},-16 0,-34 Z" fill="#efe6d0"/><path d="M40,${GROUND - 66} h40 M40,${GROUND - 54} h40" stroke="#b3261e" stroke-width="4"/>` +
      `<path d="M12,${GROUND - 28} q6,-8 14,-6 h72 l18,-6 l-6,14 q-8,16 -24,18 h-52 q-16,-2 -22,-20 Z" fill="#8b5a2b"/><path d="M18,${GROUND - 22} h84" stroke="#c9973f" stroke-width="3"/>` +
      `<path d="M112,${GROUND - 26} l10,3 l-10,4 Z" fill="${BRONZE}"/>` + oars);
  },
  // Frigate
  (f) => {
    const fl = [0, 2, 4, 2][f];
    const mast = (x, h, w) => `<path d="M${x},${GROUND - 28} V${GROUND - h}" stroke-width="4.5"/><path d="M${x},${GROUND - 28} V${GROUND - h}" stroke="${WOOD}" stroke-width="2"/>` +
      `<path d="M${x - w},${GROUND - h + 6} h${w * 2} q${3 + fl},${(h - 40) / 2} 0,${h - 44} h${-w * 2} q${3 + fl},${-(h - 44) / 2} 0,${-(h - 44)} Z" fill="#f3ecd8"/><path d="M${x - w},${GROUND - (h + 44) / 2 + 3} h${w * 2}" stroke="rgba(0,0,0,0.25)" stroke-width="1.5"/>`;
    return ship(f, 2,
      mast(36, 82, 13) + mast(62, 98, 16) + mast(88, 84, 13) +
      `<path d="M62,${GROUND - 98} l${14 + fl},4 l-${14 + fl},5 Z" fill="#c8281c"/>` +
      `<path d="M96,${GROUND - 34} L122,${GROUND - 44}" stroke-width="3"/>` +
      `<path d="M10,${GROUND - 36} h18 v6 h74 l12,-6 l-6,18 q-6,16 -22,18 h-56 q-16,-2 -20,-36 Z" fill="#7a4a24"/>` +
      `<path d="M16,${GROUND - 24} h92" stroke="#e8c14a" stroke-width="3"/>` +
      [34, 50, 66, 82].map((x) => `<rect x="${x}" y="${GROUND - 18}" width="7" height="6" fill="#1d1611" stroke="none"/>`).join(''));
  },
  // Ironclad
  (f) => ship(f, 1,
    smoke(58, GROUND - 62, f) +
    `<path d="M52,${GROUND - 34} v-24 h12 v24 Z" fill="#33363d"/><path d="M50,${GROUND - 58} h16" stroke-width="4"/>` +
    `<path d="M30,${GROUND - 22} l8,-16 h46 l8,16 Z" fill="#59606b"/>` +
    [44, 58, 72].map((x) => `<rect x="${x}" y="${GROUND - 32}" width="7" height="6" fill="#1d1611" stroke="none"/>`).join('') +
    `<path d="M8,${GROUND - 22} h112 l-8,12 q-6,8 -18,10 h-64 q-14,-2 -22,-22 Z" fill="#3f454e"/><path d="M12,${GROUND - 18} h104" stroke="#8d93a0" stroke-width="2"/>` +
    `<path d="M96,${GROUND - 22} v-22" stroke-width="3"/><path d="M96,${GROUND - 44} l12,3 l-12,4 Z" fill="#c8281c"/>`),
  // Destroyer
  (f) => ship(f, 1,
    smoke(64, GROUND - 60, f) +
    `<path d="M58,${GROUND - 30} v-26 h10 v26 Z" fill="#6b727d"/>` +
    `<path d="M70,${GROUND - 30} v-18 h18 l4,18 Z" fill="#aab2bd"/><path d="M74,${GROUND - 42} h12" stroke="#1d1611" stroke-width="3"/>` +
    `<path d="M79,${GROUND - 48} v-18 M72,${GROUND - 60} h14" stroke-width="2.5"/>` +
    turret(30, GROUND - 28, 1) + turret(100, GROUND - 28, 1) +
    `<path d="M4,${GROUND - 26} h106 l14,-6 l-10,20 q-6,10 -18,12 h-74 q-12,-2 -18,-26 Z" fill="#8a929d"/><path d="M8,${GROUND - 16} h106" stroke="#5a616b" stroke-width="2"/><path d="M8,${GROUND - 21} h108" stroke="#d5dbe2" stroke-width="1.6"/>`),
  // Battleship
  (f) => ship(f, 0,
    smoke(52, GROUND - 74, f) + smoke(66, GROUND - 72, (f + 2) % 4) +
    `<path d="M47,${GROUND - 34} v-34 h10 v34 Z M61,${GROUND - 34} v-32 h10 v32 Z" fill="#59606b"/>` +
    `<path d="M72,${GROUND - 34} v-30 h16 l6,30 Z" fill="#aab2bd"/><path d="M76,${GROUND - 54} h12 M76,${GROUND - 46} h14" stroke="#1d1611" stroke-width="2.5"/>` +
    `<path d="M80,${GROUND - 64} v-24 M72,${GROUND - 78} h16" stroke-width="2.5"/>` +
    turret(22, GROUND - 30, 1.35) + turret(100, GROUND - 30, 1.35) + turret(38, GROUND - 38, 1.1) +
    `<path d="M2,${GROUND - 30} h112 l12,-6 l-8,22 q-6,12 -20,14 h-76 q-14,-2 -20,-30 Z" fill="#7d8590"/><path d="M6,${GROUND - 16} h110" stroke="#4f5660" stroke-width="2.5"/><path d="M6,${GROUND - 24} h112" stroke="#d5dbe2" stroke-width="1.6"/>`),
  // ---- aircraft: flying above their shadow ----
  // Fighter
  (f) => {
    const bob = [0, -3, 0, 3][f];
    return `<ellipse cx="64" cy="${GROUND + 2}" rx="${30 - Math.abs(bob)}" ry="5" fill="rgba(0,0,0,0.22)"/>` + wrap(
      `<g transform="translate(0,${bob})">` +
      `<path d="M14,54 l-4,-22 l12,2 l10,18 Z" fill="#6f7a4a"/>` +
      `<path d="M14,52 C40,44 84,44 106,54 C112,58 112,64 104,66 C76,72 36,70 16,62 Z" fill="#7d8a55"/>` +
      `<path d="M70,48 q10,-12 22,-2 q-6,6 -22,2 Z" fill="#bfe6f5"/>` +
      `<path d="M40,60 l34,2 l-22,22 h-14 Z" fill="#5a6440"/>` +
      `<circle cx="52" cy="57" r="6" fill="#f1ecdf"/><circle cx="52" cy="57" r="3" fill="#c8281c" stroke="none"/>` +
      `<path d="M106,58 h8" stroke-width="4"/>` +
      `<ellipse cx="116" cy="58" rx="3" ry="${[20, 14, 20, 12][f]}" fill="rgba(220,220,220,0.55)" stroke="rgba(29,22,17,0.5)" stroke-width="1.2"/>` +
      `</g>`);
  },
  // Bomber
  (f) => {
    const bob = [0, -2, 0, 2][f];
    return `<ellipse cx="64" cy="${GROUND + 2}" rx="${40 - Math.abs(bob)}" ry="6" fill="rgba(0,0,0,0.22)"/>` + wrap(
      `<g transform="translate(0,${bob})">` +
      `<path d="M8,50 l-2,-24 l12,2 l12,20 Z" fill="#59624a"/><path d="M4,46 h22" stroke-width="4"/><path d="M4,46 h22" stroke="#6b7655" stroke-width="2"/>` +
      `<path d="M8,48 C36,40 92,40 112,50 C120,54 120,62 110,64 C80,72 34,70 12,60 Z" fill="#6b7655"/>` +
      `<path d="M96,44 q12,0 20,8 q-10,4 -22,0 Z" fill="#bfe6f5"/><path d="M64,43 q8,-8 16,-1" fill="#bfe6f5"/>` +
      `<path d="M30,56 l52,2 l-26,32 h-18 Z" fill="#4f5a3c"/>` +
      `<rect x="54" y="62" width="30" height="13" rx="6" fill="#3f4636"/>` +
      `<ellipse cx="88" cy="68" rx="2.6" ry="${[16, 10, 16, 9][f]}" fill="rgba(220,220,220,0.55)" stroke="rgba(29,22,17,0.5)" stroke-width="1.2"/>` +
      `<circle cx="36" cy="54" r="6" fill="#f1ecdf"/><circle cx="36" cy="54" r="3" fill="#2a4f9a" stroke="none"/>` +
      `<path d="M112,56 h8" stroke-width="4"/>` +
      `<ellipse cx="121" cy="56" rx="2.6" ry="${[14, 9, 14, 8][f]}" fill="rgba(220,220,220,0.55)" stroke="rgba(29,22,17,0.5)" stroke-width="1.2"/>` +
      `</g>`);
  },
  // ---- Great people (not trainable: born in cities). A soft golden aura marks them out. ----
  // Great General: crested helm, red cloak, sword raised
  (f) => greatPerson(f, { tunic: '#8a1f1f', legs: '#5a3a2a', cape: '#c22d2d', head: head.crest(BRONZE, '#e8c14a'),
    front: (bob) => limb(5, -42 + bob, 16, -60 + bob, SKIN, 4) + `<path d="M16,${-60 + bob} L24,${-84 + bob}" stroke-width="5"/><path d="M16,${-60 + bob} L24,${-84 + bob}" stroke="${STEEL}" stroke-width="2.5"/><path d="M11,${-62 + bob} h10" stroke-width="3"/>` }),
  // Great Merchant: turban and fine robe, a heavy purse
  (f) => greatPerson(f, { robe: true, tunic: '#2f6f8f', trim: '#e8c14a', sleeve: '#2f6f8f', head: head.turban('#e8c14a'), beard: '#3a2a1a',
    front: (bob, sway) => armTo(14, -30)(bob) + `<g transform="rotate(${sway * 4} 15 ${-30 + bob})"><path d="M8,${-28 + bob} q7,-8 14,0 q4,12 -7,13 q-11,-1 -7,-13 Z" fill="#d9a83a"/><path d="M10,${-29 + bob} h10" stroke-width="2.2"/></g>` }),
  // Great Scientist: scholar's robe, grey beard, a glowing flask
  (f) => greatPerson(f, { robe: true, tunic: '#3b4a8a', trim: '#f1ecdf', sleeve: '#3b4a8a', head: head.hair('#c8c8cc'), beard: '#c8c8cc',
    front: (bob) => armTo(15, -44)(bob) + `<path d="M12,${-58 + bob} h7 v6 l6,10 q1,4 -3,4 h-13 q-4,0 -3,-4 l6,-10 Z" fill="#dff3f7"/><path d="M8,${-41 + bob} q7,-5 15,0 l2,3 q-1,1 -3,1 h-13 q-2,0 -3,-1 Z" fill="#5fd38a" stroke="none"/>` }),
  // Great Artist: beret, violet smock, palette and brush
  (f) => greatPerson(f, { tunic: '#7a2a8f', legs: '#3a2f4a', head: head.cap('#c2185b'),
    front: (bob, sway) => armTo(16, -38)(bob) + `<ellipse cx="18" cy="${-36 + bob}" rx="11" ry="7" fill="#c89b62"/><g stroke="none"><circle cx="13" cy="${-38 + bob}" r="2" fill="#d43c2c"/><circle cx="19" cy="${-40 + bob}" r="2" fill="#2a6fd4"/><circle cx="24" cy="${-36 + bob}" r="2" fill="#e8c14a"/><circle cx="16" cy="${-33 + bob}" r="2" fill="#3fae5a"/></g><path d="M22,${-46 + bob + sway} l9,-12" stroke-width="2.5"/>` }),
  // Great Engineer: work coat, brass helmet, hammer over the shoulder
  (f) => greatPerson(f, { tunic: '#8b5a2b', legs: '#4a3a2a', boots: '#3a2a1a', head: head.cap('#d9a441'),
    front: (bob) => limb(5, -42 + bob, 14, -52 + bob, SKIN, 4) + `<path d="M14,${-52 + bob} L4,${-74 + bob}" stroke-width="5"/><path d="M14,${-52 + bob} L4,${-74 + bob}" stroke="${WOOD}" stroke-width="2.5"/><rect x="-6" y="${-82 + bob}" width="18" height="9" rx="2" fill="#7a8088" transform="rotate(-25 4 ${-78 + bob})"/>` }),
  // ---- the later defenders (rows after the great people) ----
  // Pikeman
  (f) => {
    const pike = (bob, sway) => armTo(13, -34, '#6a7a8f')(bob) + spear(14, 98)(bob, sway);
    return shadow(62, 24) + wrap(human(60, GROUND, { tunic: '#6a7a8f', sleeve: '#6a7a8f', legs: '#8b2f2f', trim: '#e8c14a', head: head.helm('#e8c14a'), back: roundShield(-6, -34, 10, '#8b2f2f'), front: pike }, f));
  },
  // Musketman
  (f) => {
    const musket = (bob, sway) =>
      limb(5, -42 + bob, 13, -30 + bob, '#b3261e', 4) +
      `<g transform="rotate(${sway} 8 ${-30 + bob})"><path d="M-2,${-16 + bob} L18,${-70 + bob}" stroke-width="6"/><path d="M-2,${-16 + bob} L8,${-44 + bob}" stroke="#7a4a24" stroke-width="3.6"/><path d="M8,${-44 + bob} L18,${-70 + bob}" stroke="#4a4f58" stroke-width="2.4"/></g>`;
    const sash = (bob) => `<path d="M-7,${-45 + bob} L9,${-24 + bob}" stroke="#f1ecdf" stroke-width="3"/>`;
    return shadow(62, 24) + wrap(human(60, GROUND, { tunic: '#b3261e', sleeve: '#b3261e', legs: '#f1ecdf', boots: '#22201d', head: head.brim('#22201d'), chest: sash, front: musket }, f));
  },
  // Infantry
  (f) => {
    const rifle = (bob, sway) =>
      limb(5, -42 + bob, 14, -32 + bob, '#5f6b3a', 4) +
      `<g transform="rotate(${sway} 8 ${-30 + bob})"><path d="M-6,${-22 + bob} L30,${-52 + bob}" stroke-width="5.5"/><path d="M-6,${-22 + bob} L12,${-37 + bob}" stroke="#6b4424" stroke-width="3.4"/><path d="M12,${-37 + bob} L30,${-52 + bob}" stroke="#3f444a" stroke-width="2.2"/></g>`;
    const pack = (bob) => `<rect x="-14" y="${-46 + bob}" width="8" height="16" rx="2" fill="#4a5230"/>`;
    return shadow(62, 24) + wrap(human(60, GROUND, { tunic: '#5f6b3a', sleeve: '#5f6b3a', legs: '#5f6b3a', boots: '#2a241c', head: head.cap('#4a5230'), back: pack, front: rifle }, f));
  },
  // Mech. Infantry: a tracked carrier with a soldier in the hatch
  (f) => {
    const jig = [0, -0.8, 0, 0.8][f];
    let wheels = '';
    for (let x = 30; x <= 98; x += 17) wheels += `<circle cx="${x}" cy="${GROUND - 10}" r="6.5" fill="#4a4f44"/><circle cx="${x}" cy="${GROUND - 10}" r="1.6" fill="${OUT}"/>`;
    let treads = '';
    for (let x = 22 + ((f * 3) % 12); x < 106; x += 12) treads += `<path d="M${x},${GROUND - 21} v3 M${x - 4},${GROUND - 1} v-3" stroke-width="2.2"/>`;
    return (
      shadow(64, 52, 7) +
      wrap(
        `<g transform="translate(0,${jig})"><path d="M18,${GROUND - 22} v-26 h70 l24,10 v16 Z" fill="#7a7f5c"/><path d="M24,${GROUND - 38} h56" stroke="rgba(255,255,255,0.3)" stroke-width="1.6"/>` +
          `<path d="M40,${GROUND - 48} v-6 h14 v6" fill="#5f6447"/><path d="M58,${GROUND - 52} h30 v4 h-30 Z" fill="#4a4f38"/>` +
          `<circle cx="47" cy="${GROUND - 60}" r="6" fill="${SKIN}"/><path d="M41,${GROUND - 61} a6,6 0 0 1 12,0 Z" fill="#4a5230"/></g>` +
          `<rect x="16" y="${GROUND - 22}" width="96" height="23" rx="11.5" fill="#33362e"/>` + wheels + treads,
      )
    );
  },
  // Modern Armor
  (f) => {
    const jig = [0, -0.8, 0, 0.8][f];
    let wheels = '';
    for (let x = 28; x <= 100; x += 12) wheels += `<circle cx="${x}" cy="${GROUND - 9}" r="5.5" fill="#55585a"/>`;
    let treads = '';
    for (let x = 20 + ((f * 3) % 10); x < 110; x += 10) treads += `<path d="M${x},${GROUND - 20} v3" stroke-width="2"/>`;
    return (
      shadow(64, 54, 7) +
      wrap(
        `<g transform="translate(0,${jig})"><path d="M14,${GROUND - 22} l10,-16 h76 l16,16 Z" fill="#b8a77a"/><path d="M34,${GROUND - 38} l6,-12 h40 l10,12 Z" fill="#a8976a"/>` +
          `<path d="M84,${GROUND - 46} h40 v4 h-40 Z" fill="#6f6650"/><path d="M50,${GROUND - 50} v-6 h12 v6" fill="#8f8460"/><path d="M24,${GROUND - 38} h76" stroke="rgba(255,255,255,0.35)" stroke-width="1.6"/></g>` +
          `<rect x="12" y="${GROUND - 21}" width="104" height="22" rx="11" fill="#3a3a36"/>` + wheels + treads +
          (f === 2 ? `<g stroke="none"><circle cx="126" cy="${GROUND - 44}" r="5" fill="rgba(255,214,120,0.85)"/></g>` : ''),
      )
    );
  },
  // ---- filling the gaps (rows after the late defenders) ----
  // Scout: light pack, spyglass raised
  (f) => {
    const glass = (bob, sway) => limb(5, -42 + bob, 12, -52 + bob, '#8a6a3a', 4) + `<g transform="rotate(${-20 + sway * 2} 12 ${-52 + bob})"><rect x="10" y="${-56 + bob}" width="18" height="5" rx="1.5" fill="#c9973f"/><rect x="26" y="${-57 + bob}" width="4" height="7" fill="#8a6a3a"/></g>`;
    const pack = (bob) => `<rect x="-15" y="${-46 + bob}" width="9" height="15" rx="3" fill="#7a5a34"/>`;
    return shadow(62, 24) + wrap(human(58, GROUND, { tunic: '#8a6a3a', sleeve: '#8a6a3a', legs: '#5a4a32', head: head.brim('#5a4a32'), back: pack, front: glass }, f));
  },
  // Longbowman: tall bow, hooded
  (f) => {
    const pull = [0, 2, 4, 2][f];
    const bow = (bob) =>
      limb(5, -42 + bob, 17, -40 + bob, '#2f5f2c', 4) +
      `<path d="M18,${-74 + bob} Q36,${-40 + bob} 18,${-6 + bob}" fill="none" stroke-width="5"/><path d="M18,${-74 + bob} Q36,${-40 + bob} 18,${-6 + bob}" fill="none" stroke="#6b4424" stroke-width="2.5"/>` +
      `<path d="M18,${-74 + bob} L${18 - pull},${-40 + bob} L18,${-6 + bob}" fill="none" stroke="#efe6d0" stroke-width="1"/><path d="M${6 - pull},${-40 + bob} H32" stroke="#efe6d0" stroke-width="1.6"/><path d="M32,${-43 + bob} l5,3 l-5,3 Z" fill="${STEEL}"/>`;
    const quiver = (bob) => `<path d="M-13,${-50 + bob} l7,2 l-3,22 l-7,-2 Z" fill="#5b3a1c"/><path d="M-12,${-52 + bob} l1,-7 M-9,${-51 + bob} l2,-7 M-6,${-50 + bob} l2,-6" stroke="#efe6d0" stroke-width="1.6"/>`;
    return shadow(62, 24) + wrap(human(56, GROUND, { tunic: '#2f6a3a', sleeve: '#2f6a3a', legs: '#5a4a32', trim: '#c9973f', head: head.hood('#24502a'), back: quiver, front: bow }, f));
  },
  // Galleon: deep hull, three masts with square sails
  (f) => {
    const fl = [0, 2, 4, 2][f];
    const mast = (x, h, w) => `<path d="M${x},${GROUND - 34} V${GROUND - h}" stroke-width="4.5"/><path d="M${x},${GROUND - 34} V${GROUND - h}" stroke="${WOOD}" stroke-width="2"/>` +
      `<path d="M${x - w},${GROUND - h + 6} h${w * 2} q${2 + fl},16 0,30 h${-w * 2} q${2 + fl},-14 0,-30 Z" fill="#efe2c2"/><path d="M${x},${GROUND - h + 10} v22" stroke="#b3261e" stroke-width="5"/>`;
    return ship(f, 2,
      mast(34, 80, 12) + mast(60, 94, 15) + mast(86, 78, 11) +
      `<path d="M60,${GROUND - 94} l${12 + fl},4 l-${12 + fl},5 Z" fill="#e8c14a"/>` +
      `<path d="M8,${GROUND - 44} h22 v10 h62 l8,-12 h18 l-8,24 q-6,20 -26,22 h-56 q-18,-2 -20,-44 Z" fill="#6b3f1e"/>` +
      `<path d="M14,${GROUND - 24} h96" stroke="#e8c14a" stroke-width="3"/>` +
      [30, 46, 62, 78].map((x) => `<rect x="${x}" y="${GROUND - 18}" width="6" height="5" fill="#1d1611" stroke="none"/>`).join(''));
  },
  // Submarine: low hull, conning tower, periscope
  (f) => ship(f, 0,
    `<path d="M54,${GROUND - 22} v-16 h22 l4,16 Z" fill="#4f5a64"/><path d="M64,${GROUND - 38} v-${14 + [0, 2, 4, 2][f]} h6" fill="none" stroke-width="2.5"/>` +
    `<path d="M6,${GROUND - 12} q4,-12 22,-12 h76 q18,0 20,10 q-2,8 -20,10 h-76 q-18,0 -22,-8 Z" fill="#5f6b76"/><path d="M14,${GROUND - 12} h96" stroke="#3a434c" stroke-width="2"/><path d="M4,${GROUND - 14} l-6,-6 v14 Z" fill="#3a434c"/>`),
  // Engineers: hard hat and shovel
  (f) => {
    const ang = [-40, -10, 20, -10][f];
    const tool = (bob) =>
      limb(5, -42 + bob, 14, -34 + bob, '#d98a2b', 4) +
      `<g transform="rotate(${ang} 14 ${-34 + bob})"><path d="M14,${-30 + bob} v-34" stroke-width="5"/><path d="M14,${-30 + bob} v-34" stroke="${WOOD}" stroke-width="2.5"/><path d="M8,${-64 + bob} h12 v8 q-6,6 -12,0 Z" fill="${STEEL}"/></g>`;
    const hat = (hx, hy) => `<path d="M${hx - 8},${hy - 1} a8,8 0 0 1 16,0 Z" fill="#f2c230"/><path d="M${hx - 11},${hy - 1} h22" stroke-width="3.5"/><path d="M${hx - 11},${hy - 1} h22" stroke="#f2c230" stroke-width="1.6"/>`;
    return shadow(60, 24) + wrap(human(58, GROUND, { tunic: '#d98a2b', sleeve: '#d98a2b', legs: '#3f4f6a', boots: '#2a241c', head: hat, front: tool }, f));
  },
  // Radar Artillery: tracked rocket launcher with a radar dish
  (f) => {
    const jig = [0, -0.8, 0, 0.8][f];
    let wheels = '';
    for (let x = 30; x <= 98; x += 17) wheels += `<circle cx="${x}" cy="${GROUND - 10}" r="6.5" fill="#4a4f44"/>`;
    const spin = [0, 30, 60, 30][f];
    return (
      shadow(64, 52, 7) +
      wrap(
        `<g transform="translate(0,${jig})"><path d="M18,${GROUND - 22} v-16 h88 l8,16 Z" fill="#6b7655"/>` +
          `<g transform="rotate(-28 70 ${GROUND - 40})"><rect x="50" y="${GROUND - 54}" width="60" height="16" rx="2" fill="#59624a"/><path d="M54,${GROUND - 46} h52" stroke="#3f4636" stroke-width="2"/>` +
          [56, 70, 84, 98].map((x) => `<circle cx="${x}" cy="${GROUND - 50}" r="2.5" fill="#d43c2c" stroke="none"/>`).join('') + `</g>` +
          `<path d="M30,${GROUND - 38} v-16" stroke-width="2.5"/><ellipse cx="30" cy="${GROUND - 58}" rx="${11 - spin / 10}" ry="6" fill="#c4ccd6"/></g>` +
          `<rect x="16" y="${GROUND - 22}" width="96" height="23" rx="11.5" fill="#33362e"/>` + wheels,
      )
    );
  },
  // Missionary: robed, with a staff topped by a sun disc and a book
  (f) => {
    const staff = (bob, sway) => limb(5, -42 + bob, 13, -38 + bob, '#efe6d0', 4) + `<path d="M14,2 L${14 + sway},${-78 + bob}" stroke-width="4.5"/><path d="M14,2 L${14 + sway},${-78 + bob}" stroke="${WOOD}" stroke-width="2.2"/><circle cx="${14 + sway}" cy="${-84 + bob}" r="6" fill="#e8c14a"/><circle cx="${14 + sway}" cy="${-84 + bob}" r="2.5" fill="#f7e7a8" stroke="none"/>`;
    const book = (bob) => `<rect x="-14" y="${-40 + bob}" width="10" height="13" rx="1.5" fill="#7a2a2a"/><path d="M-12,${-37 + bob} h6" stroke="#e8c14a" stroke-width="1.6"/>`;
    return shadow(62, 24) + wrap(human(58, GROUND, { robe: true, tunic: '#efe6d0', trim: '#c9973f', sleeve: '#efe6d0', head: head.hood('#d8ccb0'), beard: '#8a6a4a', back: book, front: staff }, f));
  },
  // Nuclear Missile: on its launch cradle, venting
  (f) => {
    const vent = [0.5, 0.35, 0.2, 0.35][f];
    return (
      shadow(64, 34, 6) +
      wrap(
        `<path d="M40,${GROUND - 2} l12,-14 h24 l12,14 Z" fill="#5a6440"/>` +
          `<path d="M56,${GROUND - 14} V${GROUND - 92} q8,-18 16,0 V${GROUND - 14} Z" fill="#e8e4d8"/>` +
          `<path d="M56,${GROUND - 70} h16 M56,${GROUND - 64} h16" stroke="#c8281c" stroke-width="3"/>` +
          `<path d="M56,${GROUND - 34} h16" stroke="#e8c14a" stroke-width="6"/><path d="M56,${GROUND - 34} h16" stroke="#1d1611" stroke-width="2" stroke-dasharray="3 3"/>` +
          `<path d="M56,${GROUND - 14} l-8,6 v-18 Z M72,${GROUND - 14} l8,6 v-18 Z" fill="#9aa3ad"/>` +
          `<g stroke="none" fill="rgba(220,220,220,${vent})"><circle cx="${50 - f * 2}" cy="${GROUND - 6}" r="${6 + f}"/><circle cx="${80 + f * 2}" cy="${GROUND - 6}" r="${6 + f}"/></g>`,
      )
    );
  },
  // Transport: high-sided troopship, bridge aft, deck cargo and a crane
  (f) => ship(f, 1,
    smoke(92, GROUND - 66, f) +
    `<path d="M86,${GROUND - 40} v-22 h12 v22 Z" fill="#59606b"/>` +
    `<path d="M80,${GROUND - 40} v-14 h30 l4,14 Z" fill="#e3e6ea"/><path d="M84,${GROUND - 48} h22" stroke="#1d1611" stroke-width="3" stroke-dasharray="4 3"/>` +
    `<path d="M30,${GROUND - 40} v-34 l22,14" fill="none" stroke-width="3"/><path d="M52,${GROUND - 60} v12" stroke-width="1.5"/>` +
    `<rect x="18" y="${GROUND - 52}" width="18" height="12" fill="#6b7a4a"/><rect x="40" y="${GROUND - 48}" width="22" height="8" fill="#8a6a3a"/><rect x="62" y="${GROUND - 52}" width="14" height="12" fill="#6b7a4a"/>` +
    `<path d="M4,${GROUND - 40} h110 l12,-8 l-10,26 q-6,18 -22,20 h-72 q-14,-2 -18,-38 Z" fill="#7d6b5a"/>` +
    `<path d="M8,${GROUND - 14} h108" stroke="#4a3a2c" stroke-width="2.5"/>` +
    [18, 32, 46, 60, 74, 88, 102].map((x) => `<circle cx="${x}" cy="${GROUND - 28}" r="2.6" fill="#f1e3a0" stroke="none"/>`).join('')),
  // Carrier: long flat deck with an island and a parked plane
  (f) => {
    const blink = f % 2 === 0 ? '#ffd24a' : '#c8281c';
    return ship(f, 0,
      `<path d="M84,${GROUND - 38} v-24 h14 l3,24 Z" fill="#8a929d"/><path d="M86,${GROUND - 54} h12" stroke="#1d1611" stroke-width="2.5"/>` +
      `<path d="M92,${GROUND - 62} v-14 M86,${GROUND - 72} h12" stroke-width="2.5"/><circle cx="92" cy="${GROUND - 78}" r="2.5" fill="${blink}" stroke="none"/>` +
      `<g transform="translate(34 ${GROUND - 44})"><path d="M0,0 h26 l6,3 l-6,3 h-26 Z" fill="#c4ccd6"/><path d="M10,3 l4,-10 h4 l-2,10 Z M10,3 l4,10 h4 l-2,-10 Z" fill="#aab2bd"/><path d="M0,3 l-4,-6 h4 Z" fill="#aab2bd"/></g>` +
      `<path d="M-2,${GROUND - 38} h122 l6,4 h-6 l-8,20 q-6,14 -20,16 h-70 q-14,-2 -18,-26 Z" fill="#7d8590"/>` +
      `<path d="M4,${GROUND - 36} h112" stroke="#e8e4d8" stroke-width="1.6" stroke-dasharray="7 5"/><path d="M6,${GROUND - 18} h110" stroke="#4f5660" stroke-width="2.5"/>`);
  },
  // Cartel Gunmen: dark jacket, ball cap and shades, a short submachine gun held low
  (f) => {
    const gun = (bob, sway) =>
      limb(5, -42 + bob, 14, -34 + bob, '#2b2b30', 4) +
      `<g transform="rotate(${sway * 2} 12 ${-32 + bob})"><path d="M4,${-32 + bob} h22" stroke-width="5"/><path d="M4,${-32 + bob} h22" stroke="#3a3d42" stroke-width="2.6"/><path d="M12,${-31 + bob} v9" stroke-width="3.5"/><path d="M12,${-31 + bob} v9" stroke="#2a2a2e" stroke-width="1.6"/></g>`;
    const shades = (hx, hy) =>
      `<path d="M${hx - 8.5},${hy - 2} a8.5,8.5 0 0 1 17,0 Z" fill="#1f1f24"/><path d="M${hx - 1},${hy - 2} h12" stroke-width="3"/><path d="M${hx - 1},${hy - 2} h12" stroke="#1f1f24" stroke-width="1.6"/>` +
      `<rect x="${hx - 1}" y="${hy + 1}" width="9" height="3.5" rx="1.2" fill="#111" stroke="none"/>`;
    return shadow(62, 24) + wrap(human(60, GROUND, { tunic: '#2b2b30', sleeve: '#2b2b30', legs: '#3b4a66', boots: '#141414', trim: '#b8892e', head: shades, front: gun }, f));
  },
  // Insurgents: scarf over the face, drab fatigues, a rifle slung forward
  (f) => {
    const rifle = (bob, sway) =>
      limb(5, -42 + bob, 14, -32 + bob, '#7a6a4a', 4) +
      `<g transform="rotate(${sway} 8 ${-30 + bob})"><path d="M-6,${-24 + bob} L30,${-50 + bob}" stroke-width="5.5"/><path d="M-6,${-24 + bob} L10,${-36 + bob}" stroke="#6b4424" stroke-width="3.4"/><path d="M10,${-36 + bob} L30,${-50 + bob}" stroke="#3f444a" stroke-width="2.2"/></g>`;
    const scarf = (hx, hy) =>
      `<path d="M${hx - 9},${hy - 1} q-1,-11 9,-11 q10,0 9,11 Z" fill="#5a4e3a"/><path d="M${hx - 8},${hy + 1} h16 v5 q-8,4 -16,0 Z" fill="#8a3a2e"/><path d="M${hx - 8},${hy + 3} h16" stroke="#6e2a20" stroke-width="1.2"/>`;
    const pouch = (bob) => `<rect x="-14" y="${-44 + bob}" width="7" height="12" rx="2" fill="#4e4630"/>`;
    return shadow(62, 24) + wrap(human(60, GROUND, { tunic: '#7a6a4a', sleeve: '#7a6a4a', legs: '#5e5440', boots: '#2a241c', head: scarf, back: pouch, front: rifle }, f));
  },
];

// ---- unique units: each people's own version of an ordinary unit (order matches UnitType) --------
// The base unit's figure, with a gold crest and a pennant in the unit's own colour.
const crest = (colour) =>
  wrap(
    `<path d="M10,8 h18 v12 q0,10 -9,14 q-9,-4 -9,-14 Z" fill="#e3b341"/><path d="M14,12 h10 v7 q0,6 -5,8 q-5,-2 -5,-8 Z" fill="${colour}"/>` +
      `<path d="M108,6 v34" stroke-width="2.5"/><path d="M108,8 l14,5 l-14,5 Z" fill="${colour}"/>`,
  );
// Rows of the base units' figures (the boat drawn at row 19 shifts later rows by one).
const UNIQUE_BASES = [6, 9, 5, 5, 7, 8, 11, 6, 11, 38, 11, 11, 2, 3, 11, 5, 11, 21, 33, 14, 14, 34, 15, 13, 33, 14, 14, 14, 34, 14, 33, 34];
const UNIQUE_COLOURS = ['#c8281c', '#2a6fb0', '#d9a441', '#7a3a8f', '#e0c060', '#5a3a8f', '#8a8f98', '#2f9a5a', '#b07a4e', '#c83a3a', '#33363f', '#6b4a2a', '#3a8a5a', '#e08a2a', '#d9a441', '#5b3a1c', '#b3261e', '#22305a', '#2a2f5a', '#8f1f2f', '#4f7a3a', '#1f3f7a', '#4a4a4a', '#a01c1c', '#2e8b3a', '#c9a227', '#c41e2e', '#1a7a4a', '#5a6b3a', '#1a1a1a', '#8a5a2a', '#1f6a3a'];
UNIQUE_BASES.forEach((base, i) => {
  const draw = UNITS[base];
  UNITS.push((f) => draw(f) + crest(UNIQUE_COLOURS[i]));
});

// ---- Civilization III's later units (after the unique units: order matches UnitType) ----------
// Each is its forerunner's figure with an emblem in a round badge (unique units wear a gold crest).
const badge = (inner) =>
  wrap(`<circle cx="18" cy="18" r="13" fill="#f4efe2"/>`) + `<g transform="translate(18 18)">${inner}</g>`;
const row = (unitIndex) => (unitIndex >= 19 ? unitIndex + 1 : unitIndex); // the boat row shifts later rows
const LATER = [
  // Crusader: a knight under the red cross
  [11, badge(`<path d="M-3,-9 h6 v6 h6 v6 h-6 v6 h-6 v-6 h-6 v-6 h6 Z" fill="#c8281c"/>`)],
  // Medieval Infantry: a swordsman with a kite shield
  [6, badge(`<path d="M-7,-8 h14 q0,12 -7,18 q-7,-6 -7,-18 Z" fill="#6c7480" stroke="#22262b" stroke-width="1.5"/><path d="M0,-8 v18 M-7,-2 h14" stroke="#d9c7a0" stroke-width="2"/>`)],
  // Trebuchet: a catapult with a counterweight
  [10, badge(`<path d="M-8,8 L8,-8" stroke="#6b4424" stroke-width="3"/><rect x="-11" y="4" width="8" height="7" fill="#55595e" stroke="#22262b" stroke-width="1.2"/><circle cx="8" cy="-8" r="3" fill="#8a8a8a"/>`)],
  // Caravel: a smaller ocean ship, lateen-rigged
  [38, badge(`<path d="M-9,4 h18 l-4,5 h-10 Z" fill="#8b5a2b"/><path d="M0,4 V-9 L8,2 Z" fill="#efe6d0" stroke="#5a3a1a" stroke-width="1.2"/>`)],
  // Jet Fighter: a swept-wing jet
  [24, badge(`<path d="M-10,2 L10,0 L-10,-2 Z M-2,0 L-8,-8 L-4,0 L-8,8 Z" fill="#5a6470" stroke="#22262b" stroke-width="1.2"/>`)],
  // Stealth Bomber: a dark flying wing
  [25, badge(`<path d="M-11,4 L0,-7 L11,4 L6,2 L3,5 L0,2 L-3,5 L-6,2 Z" fill="#1d1f24"/>`), 'stealth'],
  // AEGIS Cruiser: a destroyer with a radar array
  [22, badge(`<rect x="-6" y="-8" width="12" height="12" fill="#5a6470" stroke="#22262b" stroke-width="1.2"/><path d="M-3,-5 h6 v6 h-6 Z" fill="#9fd8ff"/><path d="M-9,8 h18" stroke="#22262b" stroke-width="2"/>`)],
  // Nuclear Submarine: a submarine with the trefoil
  [39, badge(`<circle r="9" fill="#f2d23c"/><circle r="2.2" fill="#1d1a0c"/><path d="M0,0 L-4.5,-7.8 A9,9 0 0 1 4.5,-7.8 Z M0,0 L9,0 A9,9 0 0 1 4.5,7.8 Z M0,0 L-4.5,7.8 A9,9 0 0 1 -9,0 Z" fill="#1d1a0c" transform="scale(0.85)"/>`)],
  // Paratrooper: infantry under a parachute
  [33, badge(`<path d="M-10,-1 a10,9 0 0 1 20,0 Z" fill="#e8e2cf" stroke="#4a4a3a" stroke-width="1.2"/><path d="M-10,-1 L0,9 L10,-1 M0,-10 V9" stroke="#4a4a3a" stroke-width="1"/>`)],
  // Cruise Missile: the missile's figure with a target badge
  [43, badge(`<circle r="8" fill="none" stroke="#c8281c" stroke-width="2"/><circle r="3" fill="#c8281c"/><path d="M-11,0 h5 M6,0 h5 M0,-11 v5 M0,6 v5" stroke="#c8281c" stroke-width="2"/>`)],
];
const DARKEN = `<filter id="stealthy"><feColorMatrix type="matrix" values="0.35 0 0 0 0.03  0 0.35 0 0 0.03  0 0 0.38 0 0.05  0 0 0 1 0"/></filter>`;
LATER.forEach(([base, emblem, look]) => {
  const draw = UNITS[row(base)];
  UNITS.push((f) => (look === 'stealth' ? `<defs>${DARKEN}</defs><g filter="url(#stealthy)">${draw(f)}</g>` : draw(f)) + emblem);
});

/** A great person: a figure standing in a softly pulsing golden aura. */
function greatPerson(f, o) {
  const glow = [0.28, 0.36, 0.28, 0.2][f];
  return (
    `<ellipse cx="62" cy="${GROUND - 44}" rx="34" ry="48" fill="rgba(255,214,90,${glow})"/>` +
    shadow(62, 24) +
    wrap(human(60, GROUND, o, f))
  );
}

module.exports = {
  // Two units a row (four frames each), so the sheet stays within a browser's 8192-pixel texture
  // limit; frame f of unit u is still cell u * 4 + f in reading order.
  units: () =>
    sheet({ cols: 8, rows: Math.ceil(UNITS.length / 2), fw: FW, fh: FH, draw: (col, row) => (UNITS[row * 2 + Math.floor(col / 4)] || (() => ''))(col % 4) }),
};
