#!/usr/bin/env node
// Regenerates the PNG sprite atlases in assets/ from the SVG definitions in this folder.
// Usage: node tools/art/generate.cjs [sheet-name ...]   (needs Playwright + Chromium)
const path = require('path');
const fs = require('fs');
const { chromium } = require('playwright');

const sheets = {
  ...require('./terrain.cjs'),
  ...(fs.existsSync(path.join(__dirname, 'units.cjs')) ? require('./units.cjs') : {}),
  ...(fs.existsSync(path.join(__dirname, 'world.cjs')) ? require('./world.cjs') : {}),
  ...(fs.existsSync(path.join(__dirname, 'ui.cjs')) ? require('./ui.cjs') : {}),
};

(async () => {
  const outDir = path.join(__dirname, '..', '..', 'assets', 'sprites');
  fs.mkdirSync(outDir, { recursive: true });
  const wanted = process.argv.slice(2);
  const browser = await chromium.launch();
  for (const [name, build] of Object.entries(sheets)) {
    if (wanted.length && !wanted.includes(name)) continue;
    const { width, height, svg } = build();
    const page = await browser.newPage({ viewport: { width, height }, deviceScaleFactor: 1 });
    await page.setContent(`<!doctype html><body style="margin:0;background:transparent">${svg}</body>`);
    await page.screenshot({ path: path.join(outDir, `${name}.png`), omitBackground: true, clip: { x: 0, y: 0, width, height } });
    await page.close();
    console.log(`${name}.png ${width}x${height}`);
  }
  await browser.close();
})();
