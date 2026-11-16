#!/usr/bin/env node
// Synthesises the sound effects in assets/sounds/ as small mono WAV files.
// Usage: node tools/audio/generate.cjs [sound-name ...]   (no dependencies)
// The noise generator is seeded, so running it again produces identical files.
const path = require('path');
const fs = require('fs');

const RATE = 22050;

/** Deterministic white noise in [-1, 1]. */
function noise(seed) {
  let s = seed >>> 0 || 1;
  return () => {
    s ^= s << 13;
    s ^= s >>> 17;
    s ^= s << 5;
    return ((s >>> 0) / 0xffffffff) * 2 - 1;
  };
}

/** A buffer of `seconds` of silence. */
const silence = (seconds) => new Float32Array(Math.round(seconds * RATE));

/** Mixes `source` into `into` starting at `at` seconds. */
function mix(into, source, at = 0, gain = 1) {
  const start = Math.round(at * RATE);
  for (let i = 0; i < source.length && start + i < into.length; i++) into[start + i] += source[i] * gain;
  return into;
}

/** Exponential decay envelope: 1 at the start, ~0 after `seconds`. */
const decay = (i, seconds) => Math.exp((-6.9 * i) / (seconds * RATE));

/** A sine partial with a fast attack and exponential decay; `glide` multiplies the pitch by the end. */
function tone(freq, seconds, { glide = 1, attack = 0.003, shape = 'sine' } = {}) {
  const out = silence(seconds);
  let phase = 0;
  for (let i = 0; i < out.length; i++) {
    const t = i / out.length;
    phase += (2 * Math.PI * freq * Math.pow(glide, t)) / RATE;
    const wave = shape === 'triangle' ? (2 / Math.PI) * Math.asin(Math.sin(phase)) : Math.sin(phase);
    out[i] = wave * Math.min(1, i / (attack * RATE)) * decay(i, seconds);
  }
  return out;
}

/** Noise through a one-pole low-pass whose cutoff (Hz) follows `cutoff(t)`, shaped by `envelope(t)`. */
function filteredNoise(seconds, seed, cutoff, envelope) {
  const out = silence(seconds);
  const rand = noise(seed);
  let low = 0;
  for (let i = 0; i < out.length; i++) {
    const t = i / out.length;
    const a = 1 - Math.exp((-2 * Math.PI * cutoff(t)) / RATE);
    low += a * (rand() - low);
    out[i] = low * envelope(t);
  }
  return out;
}

/** Scales to the given peak and fades the last few milliseconds so nothing clicks. */
function finish(samples, peak = 0.8) {
  let max = 0;
  for (const s of samples) max = Math.max(max, Math.abs(s));
  const gain = max > 0 ? peak / max : 0;
  const fade = Math.min(samples.length, Math.round(0.004 * RATE));
  for (let i = 0; i < samples.length; i++) {
    const tail = samples.length - i;
    samples[i] *= gain * (tail < fade ? tail / fade : 1);
  }
  return samples;
}

const sounds = {
  // Interface button: a short, soft wooden tick.
  click: () => {
    const out = silence(0.06);
    mix(out, tone(1500, 0.035, { glide: 0.7 }), 0, 0.7);
    mix(out, filteredNoise(0.02, 3, () => 4000, (t) => 1 - t), 0, 0.5);
    return finish(out, 0.45);
  },
  // Selecting a unit or city: two quick rising notes, like a tap on a map table.
  select: () => {
    const out = silence(0.2);
    mix(out, tone(587, 0.09, { shape: 'triangle' }), 0, 0.8);
    mix(out, tone(880, 0.12, { shape: 'triangle' }), 0.055, 0.8);
    mix(out, tone(1760, 0.05), 0.055, 0.12);
    return finish(out, 0.55);
  },
  // Troops on the march: three muffled footfalls.
  march: () => {
    const out = silence(0.46);
    [0, 0.14, 0.28].forEach((at, n) => {
      mix(out, filteredNoise(0.12, 11 + n, (t) => 900 - 600 * t, (t) => Math.exp(-7 * t)), at, 1);
      mix(out, tone(85 - n * 5, 0.1, { glide: 0.6 }), at, 0.9);
    });
    return finish(out, 0.7);
  },
  // A ship getting under way: a swell of water.
  sail: () => finish(filteredNoise(0.6, 21, (t) => 350 + 500 * Math.sin(Math.PI * t), (t) => Math.sin(Math.PI * t) ** 1.5), 0.6),
  // An aircraft: a rising whoosh.
  fly: () => finish(filteredNoise(0.55, 31, (t) => 400 + 2600 * t * t, (t) => Math.sin(Math.PI * Math.min(1, t * 1.15)) ** 2), 0.55),
  // Combat: two metallic clashes over a thud.
  battle: () => {
    const out = silence(0.55);
    [0, 0.17].forEach((at, n) => {
      for (const [freq, gain] of [[523, 0.5], [1307, 0.35], [2290, 0.25], [3711, 0.15]]) {
        mix(out, tone(freq * (1 + n * 0.06), 0.3), at, gain);
      }
      mix(out, filteredNoise(0.08, 41 + n, () => 5000, (t) => 1 - t), at, 0.9);
    });
    mix(out, tone(70, 0.25, { glide: 0.5 }), 0, 1);
    return finish(out, 0.75);
  },
};

/** 16-bit PCM mono WAV. */
function wav(samples) {
  const data = Buffer.alloc(samples.length * 2);
  samples.forEach((s, i) => data.writeInt16LE(Math.round(Math.max(-1, Math.min(1, s)) * 32767), i * 2));
  const header = Buffer.alloc(44);
  header.write('RIFF', 0);
  header.writeUInt32LE(36 + data.length, 4);
  header.write('WAVE', 8);
  header.write('fmt ', 12);
  header.writeUInt32LE(16, 16);
  header.writeUInt16LE(1, 20); // PCM
  header.writeUInt16LE(1, 22); // mono
  header.writeUInt32LE(RATE, 24);
  header.writeUInt32LE(RATE * 2, 28);
  header.writeUInt16LE(2, 32);
  header.writeUInt16LE(16, 34);
  header.write('data', 36);
  header.writeUInt32LE(data.length, 40);
  return Buffer.concat([header, data]);
}

const outDir = path.join(__dirname, '..', '..', 'assets', 'sounds');
fs.mkdirSync(outDir, { recursive: true });
const wanted = process.argv.slice(2);
for (const [name, build] of Object.entries(sounds)) {
  if (wanted.length && !wanted.includes(name)) continue;
  const file = wav(build());
  fs.writeFileSync(path.join(outDir, `${name}.wav`), file);
  console.log(`${name}.wav ${(file.length / 1024).toFixed(1)} KiB`);
}
