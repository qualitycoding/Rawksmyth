#!/usr/bin/env node
/* Builds index.html (the single-file app) from its two sources:
 *   void-choir.template.html   the UI, with a <script src="void-choir-engine.js"> tag
 *   void-choir-engine.js       the audio and music engine
 *
 *   node build.js           write index.html
 *   node build.js --check   exit 1 if index.html is out of date (writes nothing)
 *
 * No dependencies. Line endings are normalised to \n so the check also passes
 * on Windows checkouts that convert them. */
'use strict';
const fs = require('fs');
const path = require('path');

const TEMPLATE = path.join(__dirname, 'void-choir.template.html');
const ENGINE = path.join(__dirname, 'void-choir-engine.js');
const OUT = path.join(__dirname, 'index.html');
const TAG = '<script src="void-choir-engine.js"></script>';
const BANNER = '<!-- GENERATED FILE: edit void-choir.template.html or void-choir-engine.js, then run `npm run build`. -->\n';

const norm = (s) => s.replace(/\r\n/g, '\n');

function build() {
  const template = norm(fs.readFileSync(TEMPLATE, 'utf8'));
  const engine = norm(fs.readFileSync(ENGINE, 'utf8')).replace(/\n+$/, '');
  if (template.split(TAG).length !== 2) {
    throw new Error('void-choir.template.html must contain exactly one ' + TAG);
  }
  if (/<\/script/i.test(engine)) {
    throw new Error('void-choir-engine.js contains "</script", which would end the inline script early');
  }
  /* Function replacers, not strings: a string replacement would treat "$&", "$1" and
     similar sequences in the engine (for example inside template literals) as patterns. */
  const html = template.replace(TAG, () => '<script>\n' + engine + '\n</script>');
  return /^<!DOCTYPE html>\n/i.test(html)
    ? html.replace(/^(<!DOCTYPE html>\n)/i, (m) => m + BANNER)
    : BANNER + html;
}

if (require.main === module) {
  try {
    const built = build();
    if (process.argv.includes('--check')) {
      const current = fs.existsSync(OUT) ? norm(fs.readFileSync(OUT, 'utf8')) : null;
      if (current === built) { console.log('index.html is up to date'); process.exit(0); }
      console.error('index.html is out of date. Run: npm run build');
      process.exit(1);
    }
    fs.writeFileSync(OUT, built);
    console.log('wrote index.html (' + built.length + ' characters)');
  } catch (e) {
    console.error('build failed: ' + e.message);
    process.exit(1);
  }
}

module.exports = { build, OUT, norm };
