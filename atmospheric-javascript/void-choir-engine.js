/* VOID CHOIR — engine core.
   DOM-free and side-effect-free on load, so the same file backs both the
   instrument (index.html) and the frozen test harness (tests.html).
   Plain script, not an ES module: ES modules do not load over file://. */
(function (global) {
  'use strict';

  /* The pitch at which each guitar voice's brightness sweep was hand-tuned.
     The excitation is anchored here: at these notes the sound is what it was,
     and at every other pitch it is the same waveform transposed. Confirm or
     change to the note the original timbre was actually derived from. */
  var REF_MIDI = { lead: 64, rhythm: 40 };

  var EPS = 1e-5;                 /* no envelope target may ever reach 0 */
  var MIN_SCHED_AHEAD = 0.02;     /* never schedule in the past */
  var MAX_VOICES = 160;

  function clamp(v, a, b) { return v < a ? a : (v > b ? b : v); }
  function lerp(a, b, t) { return a + (b - a) * t; }
  function midiToFreq(m) { return 440 * Math.pow(2, (m - 69) / 12); }

  /* ================= SEEDED RNG ================= */
  function xmur3(str) {
    var h = 1779033703 ^ str.length;
    for (var i = 0; i < str.length; i++) {
      h = Math.imul(h ^ str.charCodeAt(i), 3432918353);
      h = (h << 13) | (h >>> 19);
    }
    return function () {
      h = Math.imul(h ^ (h >>> 16), 2246822507);
      h = Math.imul(h ^ (h >>> 13), 3266489909);
      return (h ^= h >>> 16) >>> 0;
    };
  }
  function mulberry32(a) {
    return function () {
      a |= 0; a = (a + 0x6D2B79F5) | 0;
      var t = Math.imul(a ^ (a >>> 15), 1 | a);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }
  function makeRng(seed) { return mulberry32(xmur3(String(seed))()); }

  /* ================= THEORY ================= */
  var MODES = {
    aeolian:  [0, 2, 3, 5, 7, 8, 10],
    phrygian: [0, 1, 3, 5, 7, 8, 10],
    dorian:   [0, 2, 3, 5, 7, 9, 10],
    mixolydian: [0, 2, 4, 5, 7, 9, 10],
    blues:      [0, 3, 5, 6, 7, 10]
  };

  /* Three seeds, each a real point in the subgenre rather than a knob preset.
     progression entries are [semitones above tonic, triad quality]. */
  var STYLES = {
    raw: {
      id: 'raw', label: 'SECOND WAVE \u2014 RAW',
      bpm: 186, tonic: 40, stepsPerChord: 32, mode: 'phrygian',
      progression: [[0, 'min'], [10, 'maj'], [8, 'maj'], [10, 'maj']],
      tremPerBeat: 4,
      archetypeWeights: { blast: 0.62, gallop: 0.30, halftime: 0.06, sparse: 0.02, punk: 0, fourFloor: 0, shuffle: 0 },
      allowCowbell: false,
      reverb: { seconds: 1.0, damp: 2600, mix: 0.10 },
      drive: { rhythm: 74, lead: 82 },
      mix: { drums: 72, gtrchord: 70, gtrtrem: 62, bass: 56, pads: 14, drones: 20, scifi: 6 },
      motif: { len: 16, span: 7, leapChance: 0.10 },
      cymBase: 330, hatTone: 8200, kickClick: 1.25, padOct: 0, droneChance: 0.12, scifiDensity: 8
    },
    blackgaze: {
      id: 'blackgaze', label: 'BLACKGAZE \u2014 SYMPHONIC',
      bpm: 142, tonic: 40, stepsPerChord: 64, mode: 'aeolian',
      progression: [[8, 'maj'], [10, 'maj'], [0, 'min'], [3, 'maj']],
      tremPerBeat: 4,
      archetypeWeights: { blast: 0.30, gallop: 0.34, halftime: 0.30, sparse: 0.06, punk: 0, fourFloor: 0, shuffle: 0 },
      allowCowbell: false,
      reverb: { seconds: 3.4, damp: 4200, mix: 0.42 },
      drive: { rhythm: 58, lead: 64 },
      mix: { drums: 60, gtrchord: 66, gtrtrem: 66, bass: 52, pads: 52, drones: 38, scifi: 18 },
      motif: { len: 16, span: 9, leapChance: 0.22 },
      cymBase: 290, hatTone: 7400, kickClick: 0.7, padOct: 0, droneChance: 0.30, scifiDensity: 22
    },
    cosmic: {
      id: 'cosmic', label: 'ATMOSPHERIC \u2014 COSMIC',
      bpm: 112, tonic: 36, stepsPerChord: 64, mode: 'aeolian',
      progression: [[0, 'min'], [8, 'maj'], [3, 'maj'], [10, 'maj']],
      tremPerBeat: 4,
      archetypeWeights: { blast: 0.18, gallop: 0.22, halftime: 0.40, sparse: 0.20, punk: 0, fourFloor: 0, shuffle: 0 },
      allowCowbell: false,
      reverb: { seconds: 4.8, damp: 5200, mix: 0.56 },
      drive: { rhythm: 46, lead: 52 },
      mix: { drums: 52, gtrchord: 58, gtrtrem: 60, bass: 50, pads: 66, drones: 58, scifi: 34 },
      motif: { len: 16, span: 7, leapChance: 0.16 },
      cymBase: 260, hatTone: 6800, kickClick: 0.5, padOct: 12, droneChance: 0.55, scifiDensity: 40
    },
    meshuggah: {
      id: 'meshuggah', label: 'MESHUGGAH \u2014 OBLIQUE',
      bpm: 126, tonic: 32, stepsPerChord: 32, mode: 'phrygian',
      progression: [[0, 'min'], [10, 'maj'], [8, 'maj'], [10, 'maj']],
      tremPerBeat: 4,
      archetypeWeights: { blast: 0.10, gallop: 0.10, halftime: 0.55, sparse: 0.25, punk: 0, fourFloor: 0, shuffle: 0 },
      allowCowbell: false,
      reverb: { seconds: 1.2, damp: 2800, mix: 0.12 },
      drive: { rhythm: 80, lead: 74 },
      mix: { drums: 68, gtrchord: 76, gtrtrem: 60, bass: 62, pads: 8, drones: 18, scifi: 6 },
      motif: { len: 16, span: 5, leapChance: 0.12 },
      cymBase: 300, hatTone: 8000, kickClick: 1.4, padOct: 0, droneChance: 0.15, scifiDensity: 6
    },
    'classic-rock': {
      id: 'classic-rock', label: 'CLASSIC ROCK \u2014 ARENA',
      bpm: 122, tonic: 40, stepsPerChord: 32, mode: 'mixolydian',
      progression: [[0, 'maj'], [7, 'maj'], [5, 'maj'], [7, 'maj']],
      tremPerBeat: 4,
      archetypeWeights: { blast: 0, gallop: 0.30, halftime: 0.40, sparse: 0.10, punk: 0, fourFloor: 0, shuffle: 0.20 },
      allowCowbell: false,
      reverb: { seconds: 1.8, damp: 3400, mix: 0.22 },
      drive: { rhythm: 55, lead: 60 },
      mix: { drums: 62, gtrchord: 66, gtrtrem: 64, bass: 58, pads: 30, drones: 20, scifi: 8 },
      motif: { len: 16, span: 7, leapChance: 0.20 },
      cymBase: 330, hatTone: 8200, kickClick: 0.9, padOct: 0, droneChance: 0.10, scifiDensity: 8
    },
    'blues-rock': {
      id: 'blues-rock', label: 'BLUES ROCK \u2014 SHUFFLE',
      bpm: 108, tonic: 38, stepsPerChord: 32, mode: 'blues',
      progression: [[0, 'maj'], [5, 'maj'], [0, 'maj'], [7, 'maj']],
      tremPerBeat: 4,
      archetypeWeights: { blast: 0, gallop: 0, halftime: 0.30, sparse: 0.15, punk: 0, fourFloor: 0, shuffle: 0.55 },
      allowCowbell: false,
      reverb: { seconds: 1.6, damp: 3200, mix: 0.25 },
      drive: { rhythm: 52, lead: 58 },
      mix: { drums: 58, gtrchord: 62, gtrtrem: 66, bass: 56, pads: 22, drones: 14, scifi: 6 },
      motif: { len: 16, span: 8, leapChance: 0.28 },
      cymBase: 320, hatTone: 8000, kickClick: 0.6, padOct: 0, droneChance: 0.10, scifiDensity: 6
    },
    motorhead: {
      id: 'motorhead', label: 'MOT\u00d6RHEAD \u2014 SPEED',
      bpm: 190, tonic: 38, stepsPerChord: 32, mode: 'mixolydian',
      progression: [[0, 'maj'], [10, 'maj'], [8, 'maj'], [10, 'maj']],
      tremPerBeat: 4,
      archetypeWeights: { blast: 0.20, gallop: 0.30, halftime: 0, sparse: 0, punk: 0.50, fourFloor: 0, shuffle: 0 },
      allowCowbell: false,
      reverb: { seconds: 0.9, damp: 2600, mix: 0.10 },
      drive: { rhythm: 78, lead: 72 },
      mix: { drums: 72, gtrchord: 70, gtrtrem: 62, bass: 64, pads: 6, drones: 10, scifi: 4 },
      motif: { len: 16, span: 6, leapChance: 0.15 },
      cymBase: 310, hatTone: 8400, kickClick: 1.3, padOct: 0, droneChance: 0.06, scifiDensity: 4
    },
    'hair-metal': {
      id: 'hair-metal', label: '80s HAIR METAL \u2014 ARENA',
      bpm: 132, tonic: 40, stepsPerChord: 64, mode: 'mixolydian',
      progression: [[8, 'maj'], [10, 'maj'], [0, 'maj'], [5, 'maj']],
      tremPerBeat: 4,
      archetypeWeights: { blast: 0, gallop: 0.35, halftime: 0.10, sparse: 0, punk: 0.20, fourFloor: 0.35, shuffle: 0 },
      allowCowbell: false,
      reverb: { seconds: 2.6, damp: 4600, mix: 0.35 },
      drive: { rhythm: 62, lead: 70 },
      mix: { drums: 64, gtrchord: 64, gtrtrem: 68, bass: 56, pads: 34, drones: 20, scifi: 12 },
      motif: { len: 16, span: 9, leapChance: 0.30 },
      cymBase: 300, hatTone: 7800, kickClick: 1.0, padOct: 12, droneChance: 0.18, scifiDensity: 14
    },
    nirvana: {
      id: 'nirvana', label: 'NIRVANA \u2014 GRUNGE',
      bpm: 134, tonic: 40, stepsPerChord: 32, mode: 'aeolian',
      progression: [[0, 'min'], [8, 'maj'], [3, 'maj'], [10, 'maj']],
      tremPerBeat: 4,
      archetypeWeights: { blast: 0, gallop: 0.10, halftime: 0.55, sparse: 0.05, punk: 0.30, fourFloor: 0, shuffle: 0 },
      allowCowbell: false,
      reverb: { seconds: 2.2, damp: 4000, mix: 0.30 },
      drive: { rhythm: 68, lead: 64 },
      mix: { drums: 66, gtrchord: 70, gtrtrem: 62, bass: 58, pads: 26, drones: 16, scifi: 10 },
      motif: { len: 16, span: 7, leapChance: 0.22 },
      cymBase: 280, hatTone: 7600, kickClick: 1.1, padOct: 0, droneChance: 0.20, scifiDensity: 12
    },
    ramones: {
      id: 'ramones', label: 'RAMONES \u2014 PUNK',
      bpm: 178, tonic: 40, stepsPerChord: 32, mode: 'aeolian',
      progression: [[0, 'min'], [8, 'maj'], [10, 'maj'], [8, 'maj']],
      tremPerBeat: 4,
      archetypeWeights: { blast: 0, gallop: 0.20, halftime: 0, sparse: 0.05, punk: 0.75, fourFloor: 0, shuffle: 0 },
      allowCowbell: false,
      reverb: { seconds: 1.1, damp: 2800, mix: 0.14 },
      drive: { rhythm: 70, lead: 64 },
      mix: { drums: 68, gtrchord: 70, gtrtrem: 60, bass: 60, pads: 10, drones: 8, scifi: 4 },
      motif: { len: 16, span: 5, leapChance: 0.10 },
      cymBase: 330, hatTone: 8600, kickClick: 1.2, padOct: 0, droneChance: 0.05, scifiDensity: 4
    },
    'art-punk': {
      id: 'art-punk', label: 'ART PUNK \u2014 AVANT',
      bpm: 150, tonic: 40, stepsPerChord: 32, mode: 'dorian',
      progression: [[0, 'min'], [1, 'maj'], [0, 'min'], [10, 'maj']],
      tremPerBeat: 4,
      archetypeWeights: { blast: 0.10, gallop: 0, halftime: 0.15, sparse: 0.30, punk: 0.45, fourFloor: 0, shuffle: 0 },
      allowCowbell: false,
      reverb: { seconds: 1.4, damp: 3000, mix: 0.20 },
      drive: { rhythm: 66, lead: 60 },
      mix: { drums: 62, gtrchord: 66, gtrtrem: 62, bass: 56, pads: 24, drones: 14, scifi: 16 },
      motif: { len: 16, span: 10, leapChance: 0.34 },
      cymBase: 300, hatTone: 8200, kickClick: 1.0, padOct: 0, droneChance: 0.22, scifiDensity: 18
    },
    exploited: {
      id: 'exploited', label: 'EXPLOITED \u2014 STREET PUNK',
      bpm: 184, tonic: 38, stepsPerChord: 32, mode: 'phrygian',
      progression: [[0, 'min'], [10, 'maj'], [8, 'maj'], [10, 'maj']],
      tremPerBeat: 4,
      archetypeWeights: { blast: 0.20, gallop: 0.10, halftime: 0, sparse: 0, punk: 0.70, fourFloor: 0, shuffle: 0 },
      allowCowbell: false,
      reverb: { seconds: 0.8, damp: 2400, mix: 0.08 },
      drive: { rhythm: 76, lead: 70 },
      mix: { drums: 72, gtrchord: 72, gtrtrem: 62, bass: 60, pads: 6, drones: 8, scifi: 4 },
      motif: { len: 16, span: 5, leapChance: 0.12 },
      cymBase: 310, hatTone: 8400, kickClick: 1.35, padOct: 0, droneChance: 0.05, scifiDensity: 4
    },
    disco: {
      id: 'disco', label: 'DISCO \u2014 FOUR ON THE FLOOR',
      bpm: 118, tonic: 38, stepsPerChord: 64, mode: 'dorian',
      progression: [[0, 'min'], [3, 'maj'], [8, 'maj'], [10, 'maj']],
      tremPerBeat: 2,
      archetypeWeights: { blast: 0, gallop: 0, halftime: 0, sparse: 0.10, punk: 0, fourFloor: 0.85, shuffle: 0.05 },
      allowCowbell: false,
      reverb: { seconds: 2.0, damp: 4400, mix: 0.30 },
      drive: { rhythm: 40, lead: 46 },
      mix: { drums: 66, gtrchord: 46, gtrtrem: 40, bass: 62, pads: 48, drones: 20, scifi: 20 },
      motif: { len: 16, span: 7, leapChance: 0.24 },
      cymBase: 340, hatTone: 9200, kickClick: 0.8, padOct: 12, droneChance: 0.12, scifiDensity: 20
    }
  };
  var STYLE_IDS = ['raw', 'blackgaze', 'cosmic',
    'meshuggah', 'classic-rock', 'blues-rock', 'motorhead', 'hair-metal',
    'nirvana', 'ramones', 'art-punk', 'exploited', 'disco'];

  var DRUMS = ['kick', 'snare', 'chat', 'ohat', 'tom', 'ride', 'crash', 'cowbell'];
  var STEPS = 16;

  function buildChords(style) {
    return style.progression.map(function (p) {
      var root = style.tonic + p[0];
      return { root: root, third: root + (p[1] === 'maj' ? 4 : 3), fifth: root + 7 };
    });
  }

  /* ================= COMPOSER (pure, no audio) ================= */
  function pickArchetype(style, rng) {
    var r = rng(), acc = 0, keys = ['blast', 'gallop', 'halftime', 'sparse', 'punk', 'fourFloor', 'shuffle'];
    for (var i = 0; i < keys.length; i++) {
      acc += style.archetypeWeights[keys[i]] || 0;
      if (r < acc) return keys[i];
    }
    return 'sparse';
  }

  /* A real melodic line: mostly stepwise, occasional leap, anchored to the
     tonic at both ends. Replaces the old 22%-chance random walk that repeated
     the same pitch ~78% of the time. */
  function generateMotif(style, rng) {
    var m = style.motif, out = [], deg = 0, i;
    for (i = 0; i < m.len; i++) {
      out.push(deg);
      var leap = rng() < m.leapChance;
      var mag = leap ? 2 + Math.floor(rng() * 3) : 1;
      var dir = rng() < 0.5 ? -1 : 1;
      if (deg >= m.span) dir = -1;
      if (deg <= -m.span) dir = 1;
      deg = clamp(deg + dir * mag, -m.span, m.span);
    }
    out[0] = 0;
    out[Math.floor(m.len / 2)] = 4;   /* half-way lift onto the fifth */
    return out;
  }

  function degreeToMidi(style, chordRoot, degree) {
    var scale = MODES[style.mode], n = scale.length;
    var oct = Math.floor(degree / n);
    var idx = ((degree % n) + n) % n;
    return chordRoot + 24 + scale[idx] + oct * 12;
  }

  function emptyPattern() {
    var p = {};
    DRUMS.forEach(function (d) { p[d] = new Array(STEPS).fill(false); });
    return p;
  }

  function generatePatterns(style, archetype, intensity, rng) {
    var d = emptyPattern(), s;
    if (archetype === 'blast') {
      for (s = 0; s < STEPS; s++) {
        if (s % 2 === 0) d.kick[s] = true; else d.snare[s] = true;
        d.chat[s] = intensity > 0.2 || s % 2 === 0;
        if (intensity > 0.65 && s % 2 === 1) d.kick[s] = true;
      }
      d.crash[0] = true;
    } else if (archetype === 'gallop') {
      for (s = 0; s < STEPS; s++) {
        d.kick[s] = (s % 4 !== 3) || intensity > 0.75;
        d.chat[s] = (s % 2 === 0) || intensity > 0.5;
        if (s % 8 === 4) d.snare[s] = true;
      }
      d.ride[0] = true;
      if (intensity > 0.6) d.crash[8] = true;
    } else if (archetype === 'halftime') {
      for (s = 0; s < STEPS; s += 4) {
        d.kick[s] = true;
        d.kick[(s + 2) % STEPS] = intensity > 0.4;
        d.chat[s] = true;
        d.ohat[(s + 2) % STEPS] = intensity > 0.6;
      }
      d.snare[8] = true;
      if (intensity > 0.5) d.tom[14] = true;
    } else if (archetype === 'punk') {
      for (s = 0; s < STEPS; s++) {
        d.kick[s] = (s % 4 === 0);
        d.chat[s] = (s % 2 === 0);
        d.snare[s] = (s === 4 || s === 12);
      }
      if (intensity > 0.75) { d.kick[2] = d.kick[6] = d.kick[10] = d.kick[14] = true; }
      d.crash[0] = true;
      if (intensity > 0.5) d.crash[8] = true;
    } else if (archetype === 'fourFloor') {
      for (s = 0; s < STEPS; s++) {
        d.kick[s] = (s % 4 === 0);
        d.ohat[s] = (s % 2 === 1);
        d.chat[s] = (s % 2 === 0) && intensity < 0.8;
        d.snare[s] = (s === 4 || s === 12);
      }
      d.crash[0] = true;
    } else if (archetype === 'shuffle') {
      d.kick[0] = true;
      d.kick[8] = intensity > 0.45;
      d.snare[4] = true;
      d.snare[12] = true;
      d.chat[0] = d.chat[4] = d.chat[8] = d.chat[12] = true;
      d.ohat[6] = d.ohat[14] = intensity > 0.6;
      d.ride[0] = true;
    } else {
      d.kick[0] = true;
      if (intensity > 0.3) d.snare[8] = true;
      d.ride[0] = d.ride[6] = true;
      if (intensity > 0.5) d.ride[10] = true;
      if (intensity > 0.4) { d.chat[4] = true; d.chat[12] = true; }
    }
    for (s = 0; s < STEPS; s++) {
      if (intensity > 0.55 && s % 4 === 3 && rng() < intensity - 0.4) d.tom[s] = true;
    }
    if (!style.allowCowbell) d.cowbell.fill(false);
    return d;
  }

  /* createComposer is the single source of musical truth. The audio scheduler
     and the determinism tests both read events from here, so they cannot drift. */
  function createComposer(seed, styleId) {
    var style = STYLES[styleId] || STYLES.cosmic;
    var composeRng = makeRng(seed + '|' + style.id + '|compose');
    var humanRng = makeRng(seed + '|' + style.id + '|human');
    var chords = buildChords(style);
    var archetype = pickArchetype(style, composeRng);
    var motif = generateMotif(style, composeRng);
    var stepDur = 60 / style.bpm / 4;

    function chordIndexAt(globalStep) {
      return Math.floor(globalStep / style.stepsPerChord) % chords.length;
    }

    /* Tremolo notes are indexed off the absolute step grid, so there is no
       free-running tremolo clock left to fall behind and burst-fire. */
    function tremRange(globalStep, perBeat) {
      return [Math.floor(globalStep * perBeat / 4),
              Math.floor((globalStep + 1) * perBeat / 4)];
    }

    function eventsForStep(globalStep, patterns, opts) {
      opts = opts || {};
      var perBeat = opts.tremPerBeat || style.tremPerBeat;
      var intensity = opts.intensity == null ? 0.7 : opts.intensity;
      var step = globalStep % STEPS;
      var cIdx = chordIndexAt(globalStep);
      var chord = chords[cIdx];
      var ev = [];
      var i;

      for (i = 0; i < DRUMS.length; i++) {
        var id = DRUMS[i];
        if (patterns[id] && patterns[id][step]) {
          ev.push({ type: 'drum', voice: id, offset: 0,
                    vel: lerp(0.84, 1.0, humanRng()), punch: lerp(0.8, 1.25, intensity) });
        }
      }

      if (step % 8 === 0) {
        /* Power chord: root, fifth, octave. Duration now stops short of the
           next strike, so chord voices no longer stack two-deep. */
        ev.push({ type: 'chord', offset: 0, cIdx: cIdx,
                  midis: [chord.root, chord.fifth, chord.root + 12],
                  dur: stepDur * 7.2 });
      }

      if (step % 4 === 0 || step % 8 === 6) {
        ev.push({ type: 'bass', offset: 0, midi: chord.root - 12, dur: stepDur * 3 });
      }

      var r = tremRange(globalStep, perBeat);
      for (var k = r[0]; k < r[1]; k++) {
        ev.push({
          type: 'trem',
          offset: (k * 4 / perBeat - globalStep) * stepDur,
          midi: degreeToMidi(style, chord.root, motif[k % motif.length]),
          dur: (60 / style.bpm / perBeat) * 0.92,
          amp: lerp(0.17, 0.25, humanRng()),
          jitter: lerp(-0.003, 0.003, humanRng())
        });
      }

      if (globalStep % style.stepsPerChord === 0) {
        ev.push({ type: 'pad', offset: 0, cIdx: cIdx,
                  dur: stepDur * style.stepsPerChord });
        if (humanRng() < style.droneChance) {
          ev.push({ type: 'drone', offset: 0, cIdx: cIdx,
                    dur: stepDur * style.stepsPerChord * lerp(0.8, 1.6, humanRng()) });
        }
      }
      return ev;
    }

    return {
      style: style, chords: chords, archetype: archetype, motif: motif,
      stepDur: stepDur, steps: STEPS,
      chordIndexAt: chordIndexAt,
      makePatterns: function (intensity) {
        return generatePatterns(style, archetype, intensity, makeRng(seed + '|' + style.id + '|drums'));
      },
      eventsForStep: eventsForStep
    };
  }

  /* Flat, hashable event list — the determinism contract (T-01/T-02/T-24).
     Deliberately defined over events, not PCM: oscillator and compressor
     implementations differ between browsers, so PCM equality is untestable. */
  function buildEventList(opts) {
    var c = createComposer(opts.seed, opts.styleId);
    var patterns = c.makePatterns(opts.intensity == null ? 0.7 : opts.intensity);
    var bars = opts.bars || 8;
    var out = [];
    for (var g = 0; g < bars * STEPS; g++) {
      var ev = c.eventsForStep(g, patterns, { intensity: opts.intensity });
      for (var i = 0; i < ev.length; i++) {
        var e = ev[i];
        out.push([g, e.type, e.voice || e.midi || (e.midis ? e.midis.join('.') : e.cIdx),
                  Math.round((e.offset || 0) * 1e5)].join(':'));
      }
    }
    return out;
  }
  function hashEvents(list) {
    var h = 5381;
    var s = list.join('|');
    for (var i = 0; i < s.length; i++) h = ((h * 33) ^ s.charCodeAt(i)) >>> 0;
    return h.toString(16);
  }

  /* ================= AUDIO PRIMITIVES ================= */
  function makeNoise(ctx, sec, rng) {
    var len = Math.floor(ctx.sampleRate * sec);
    var buf = ctx.createBuffer(1, len, ctx.sampleRate);
    var d = buf.getChannelData(0);
    for (var i = 0; i < len; i++) d[i] = rng() * 2 - 1;
    return buf;
  }

  /* Synthesized room: exponentially decaying, damped noise. One convolver on a
     shared send — never one per voice, which is what kills mobile CPU. */
  function makeImpulse(ctx, seconds, damp, rng) {
    var sr = ctx.sampleRate, len = Math.max(1, Math.floor(sr * seconds));
    var buf = ctx.createBuffer(2, len, sr);
    var a = Math.exp(-2 * Math.PI * damp / sr);
    for (var ch = 0; ch < 2; ch++) {
      var d = buf.getChannelData(ch), lp = 0;
      for (var i = 0; i < len; i++) {
        var n = rng() * 2 - 1;
        lp = n * (1 - a) + lp * a;
        var pos = i / len;
        var envv = Math.pow(1 - pos, 2.4) * (pos < 0.004 ? pos / 0.004 : 1);
        d[i] = lp * envv;
      }
    }
    return buf;
  }

  /* Asymmetric saturation — even harmonics, tube-ish. This is now the ONLY
     curve generator; the old symmetric one silently overwrote it at init. */
  function createAsymmetricDistortion(amount) {
    var n = 8192, curve = new Float32Array(n);
    for (var i = 0; i < n; i++) {
      var x = (i * 2) / n - 1;
      curve[i] = x < 0 ? -Math.tanh(-x * amount * 0.65) : Math.tanh(x * amount * 1.35);
    }
    return curve;
  }

  function curveHash(curve) {
    var h = 5381;
    for (var i = 0; i < curve.length; i += 37) {
      h = ((h * 33) ^ Math.round(curve[i] * 1e6)) >>> 0;
    }
    return h.toString(16);
  }

  /* ================= MEASURED CABINET =================
     Fixed-frequency response fitted to the reference recording (power-synth-guitar-buzz):
     46 dB of structure that a hand-tuned filter stack missed (steep low cut under ~90 Hz,
     dip near 340 Hz, notches near 1.07 and 1.72 kHz, bright to 3.5 kHz+). Fitted against this
     engine's own pitch-invariant excitation, so the pair reproduces the patch. Held-out
     error 5.8 dB across pitches over 45 Hz-10 kHz (recording noise floor 3-4 dB; the previous
     hand-tuned filters: 10+ dB). Notes measured: 58-104 Hz only. Above 10 kHz: rolled off. */
  var CAB = { hz: [45.0, 48.6, 52.4, 56.5, 61.0, 65.8, 71.0, 76.7, 82.7, 89.3, 96.3, 103.9, 112.2, 121.0, 130.6, 140.9, 152.1, 164.1, 177.1, 191.1, 206.2, 222.5, 240.1, 259.1, 279.6, 301.7, 325.5, 351.3, 379.1, 409.0, 441.4, 476.3, 513.9, 554.6, 598.4, 645.8, 696.8, 751.9, 811.4, 875.6, 944.8, 1019.5, 1100.2, 1187.2, 1281.0, 1382.3, 1491.6, 1609.6, 1736.9, 1874.2, 2022.5, 2182.4, 2355.0, 2541.2, 2742.2, 2959.0, 3193.0, 3445.5, 3718.0, 4012.0, 4329.3, 4671.6, 5041.0, 5439.7, 5869.8, 6334.0, 6834.9, 7375.4, 7958.7, 8588.0, 9267.2, 10000.0],
              db: [-40.3, -40.5, -40.7, -40.9, -41.1, -37.7, -31.3, -24.0, -17.7, -11.8, -9.6, -9.6, -6.1, -2.7, -2.0, -3.3, -4.6, -4.4, 1.8, 5.5, 5.4, 3.6, 4.8, 6.0, 1.5, -6.1, -8.3, -15.0, -9.0, -9.6, -0.8, 6.9, 1.1, 3.4, 9.3, 5.1, -0.1, 1.7, -2.6, -0.0, -6.2, -22.7, -16.2, -3.4, -0.8, 3.9, 2.9, 0.6, -14.2, 2.5, 8.2, 10.7, -0.1, -0.1, 14.7, 11.1, -0.1, 21.8, 8.8, 15.0, 18.8, 17.1, 22.1, 23.3, 19.1, 21.9, 17.9, 16.4, 15.2, 15.4, 13.2, 17.1] };
  /* hf scales the lift above 4.6 kHz: 0 = hold the 4.6 kHz value flat, 1 = fully fitted. */
  var CAB_SETTINGS = { hf: 1.0 };
  var CAB_GAIN_DB = { rhythm: 2.0, lead: 0 };   /* loudness match to the previous cabinet, set by measurement */

  function fftInPlace(re, im, inverse) {
    var n = re.length, i, j = 0, k, m, t, bit;
    for (i = 1; i < n; i++) {
      bit = n >> 1;
      for (; j & bit; bit >>= 1) j ^= bit;
      j ^= bit;
      if (i < j) { t = re[i]; re[i] = re[j]; re[j] = t; t = im[i]; im[i] = im[j]; im[j] = t; }
    }
    for (m = 2; m <= n; m <<= 1) {
      var ang = (inverse ? 2 : -2) * Math.PI / m, wr = Math.cos(ang), wi = Math.sin(ang);
      for (i = 0; i < n; i += m) {
        var cr = 1, ci = 0;
        for (k = 0; k < m / 2; k++) {
          var a = i + k, b = a + m / 2;
          var vr = re[b] * cr - im[b] * ci, vi = re[b] * ci + im[b] * cr;
          re[b] = re[a] - vr; im[b] = im[a] - vi; re[a] += vr; im[a] += vi;
          var nc = cr * wr - ci * wi; ci = cr * wi + ci * wr; cr = nc;
        }
      }
    }
    if (inverse) for (i = 0; i < n; i++) { re[i] /= n; im[i] /= n; }
  }

  function cabTable(f) {
    var hz = CAB.hz, db = CAB.db, n = hz.length;
    if (f <= hz[0]) return db[0] - 24 * Math.log2(hz[0] / Math.max(f, 1));
    if (f >= hz[n - 1]) return db[n - 1] - (f > 10000 ? 18 * Math.log2(f / 10000) : 0);
    var j = 0; while (j < n - 2 && hz[j + 1] < f) j++;
    var t = (Math.log(f) - Math.log(hz[j])) / (Math.log(hz[j + 1]) - Math.log(hz[j]));
    return db[j] * (1 - t) + db[j + 1] * t;
  }
  function cabDb(f) {
    var v = cabTable(f);
    if (f > 4600 && CAB_SETTINGS.hf !== 1) { var base = cabTable(4600); v = base + CAB_SETTINGS.hf * (v - base); }
    return v;
  }

  /* Minimum-phase FIR from the log-magnitude table (real-cepstrum method). Built for the
     context's own sample rate, so the response is right at 44.1, 48 or 96 kHz. */
  var cabIrCache = {};
  function makeCabIR(sr, gainDb) {
    var key = sr + '|' + gainDb + '|' + CAB_SETTINGS.hf; if (cabIrCache[key]) return cabIrCache[key];
    var N = 8192, half = N / 2, i;
    var re = new Float64Array(N), im = new Float64Array(N), peak = -1e9, tmp = new Float64Array(half + 1);
    for (i = 0; i <= half; i++) { tmp[i] = cabDb(Math.max(i * sr / N, 1)); if (tmp[i] > peak) peak = tmp[i]; }
    for (i = 0; i <= half; i++) {
      var v = (Math.max(tmp[i], peak - 90) + gainDb) * Math.LN10 / 20;
      re[i] = v; if (i > 0 && i < half) re[N - i] = v;
    }
    fftInPlace(re, im, true);                                        /* real cepstrum */
    var cr = new Float64Array(N), ci = new Float64Array(N);
    cr[0] = re[0]; for (i = 1; i < half; i++) cr[i] = 2 * re[i]; cr[half] = re[half];
    fftInPlace(cr, ci, false);                                       /* log spectrum, minimum phase */
    for (i = 0; i < N; i++) { var m = Math.exp(cr[i]); var ph = ci[i]; cr[i] = m * Math.cos(ph); ci[i] = m * Math.sin(ph); }
    fftInPlace(cr, ci, true);
    var taps = 2048, ir = new Float32Array(taps), fade = 512;
    for (i = 0; i < taps; i++) {
      var w = i < taps - fade ? 1 : Math.pow(Math.cos(0.5 * Math.PI * (i - (taps - fade)) / fade), 2);
      ir[i] = cr[i] * w;
    }
    cabIrCache[key] = ir; return ir;
  }

  /* ================= GUITAR AMP ================= */
  function makeGuitarAmp(ctx, dest, isLead) {
    var input = ctx.createGain(); input.gain.value = 0.5;

    /* Stable node directly after the clipper: the pitch-invariant excitation. */
    var tap = ctx.createGain();

    /* Measured cabinet (see CAB). Sits after the clipper, so the fixed-Hz colouring
       is applied to the same excitation shape at every pitch. */
    var cab = ctx.createConvolver();
    cab.normalize = false;
    var ir = makeCabIR(ctx.sampleRate, CAB_GAIN_DB[isLead ? 'lead' : 'rhythm']);
    var irBuf = ctx.createBuffer(1, ir.length, ctx.sampleRate);
    irBuf.getChannelData(0).set(ir);
    cab.buffer = irBuf;

    var post = ctx.createGain(); post.gain.value = isLead ? 0.35 : 0.40;
    tap.connect(cab); cab.connect(post); post.connect(dest);

    /* The spec's curve setter throws InvalidStateError once a curve has been
       assigned; desktop browsers tolerate reassignment but strict runtimes do
       not. So changing drive swaps in a fresh shaper rather than overwriting
       the curve, and the amount is quantised so a knob drag cannot spawn a
       node per input event. */
    var shaper = null, shaperAmount = null;
    var amp = {
      input: input, post: post, tap: tap, cab: cab, isLead: isLead, distortion: null,
      setDrive: function (norm) {
        var amount = Math.round(lerp(6, 34, clamp(norm, 0, 1)) * 2) / 2;
        if (amount === shaperAmount) return;
        var ws = ctx.createWaveShaper();
        ws.curve = createAsymmetricDistortion(amount);
        ws.oversample = '4x';
        input.connect(ws); ws.connect(tap);
        if (shaper) {
          try { input.disconnect(shaper); shaper.disconnect(tap); } catch (e) { /* already gone */ }
        }
        shaper = ws; shaperAmount = amount;
        this.distortion = ws;
        this.post.gain.value = (isLead ? 0.35 : 0.40) * lerp(1.25, 0.72, clamp(norm, 0, 1));
      }
    };
    amp.setDrive(isLead ? 0.55 : 0.40);
    return amp;
  }

  /* ================= VOICES ================= */
  function guitarPick(ctx, chain, t, o) { t = Math.max(0, t);
    var f = Math.max(20, o.freq), dur = Math.max(0.02, o.dur), amp = Math.max(EPS, o.amp);
    var isLead = !!o.isLead;

    /* Pluck brightness sweep, scaled with the note. A lowpass whose cutoff is
       proportional to f0 has the same gain on harmonic k at every pitch, so the
       waveform reaching the clipper is the same shape transposed. That is what
       keeps the distorted harmonic balance from drifting with pitch: measured at
       the clipper output, the fixed-Hz version varied by 10-14 dB across pitch.
       Anchored to the original constants at the reference pitch. */
    var refF = midiToFreq(o.refMidi != null ? o.refMidi : (isLead ? REF_MIDI.lead : REF_MIDI.rhythm));
    var pitchScale = f / refF, ceil = ctx.sampleRate * 0.45;
    var startCut = clamp((isLead ? 7500 : 4500) * pitchScale, 20, ceil);
    var endCut = clamp((isLead ? 2200 : 1800) * pitchScale, 20, ceil);
    var vf = ctx.createBiquadFilter();
    vf.type = 'lowpass';
    vf.frequency.setValueAtTime(startCut, t);
    vf.frequency.exponentialRampToValueAtTime(endCut, t + Math.min(0.12, dur));

    if (isLead && o.noise) {
      var pn = ctx.createBufferSource(); pn.buffer = o.noise;
      var pf = ctx.createBiquadFilter();
      pf.type = 'bandpass'; pf.frequency.value = clamp(f * 8, 1800, 5200); pf.Q.value = 3;
      var pe = ctx.createGain();
      pe.gain.setValueAtTime(Math.max(EPS, amp * 0.5), t);
      pe.gain.exponentialRampToValueAtTime(EPS, t + 0.008);
      pn.connect(pf); pf.connect(pe); pe.connect(vf);
      pn.start(t); pn.stop(t + 0.01);
    }

    var body = ctx.createGain();
    body.gain.setValueAtTime(EPS, t);
    body.gain.linearRampToValueAtTime(amp, t + 0.004);
    body.gain.setValueAtTime(amp, t + Math.max(0.01, dur * 0.7));
    body.gain.exponentialRampToValueAtTime(EPS, t + dur + 0.05);

    var osc = ctx.createOscillator(); osc.type = 'sawtooth';
    osc.frequency.setValueAtTime(f, t);
    var oscB = null;
    if (!isLead) {
      oscB = ctx.createOscillator(); oscB.type = 'sawtooth';
      oscB.detune.value = 10; oscB.frequency.setValueAtTime(f, t);
      var mixB = ctx.createGain(); mixB.gain.value = o.detuneMix != null ? o.detuneMix : 0.55;
      oscB.connect(mixB); mixB.connect(body);
      if (o.vib) o.vib.connect(osc.detune);
    } else if (o.vib) {
      var vd = ctx.createGain();
      vd.gain.setValueAtTime(0, t);
      vd.gain.setValueAtTime(0, t + 0.08);
      vd.gain.linearRampToValueAtTime(1, t + 0.2);
      o.vib.connect(vd); vd.connect(osc.detune);
    }

    osc.connect(body); body.connect(vf); vf.connect(chain.input);
    var end = t + dur + 0.06;
    osc.start(t); osc.stop(end);
    if (oscB) { oscB.start(t); oscB.stop(end); }
    return osc;
  }

  function bassNote(ctx, dest, t, o) {
    var f = midiToFreq(o.midi), dur = o.dur, drive = clamp(o.drive / 100, 0, 1);
    var env = ctx.createGain();
    env.gain.setValueAtTime(EPS, t);
    env.gain.exponentialRampToValueAtTime(0.6, t + 0.008);
    env.gain.setValueAtTime(0.55, t + dur * 0.7);
    env.gain.exponentialRampToValueAtTime(EPS, t + dur);
    env.connect(dest);

    var n = 2048, k = 1 + drive * 8, arr = new Float32Array(n);
    for (var i = 0; i < n; i++) arr[i] = Math.tanh(((i * 2) / n - 1) * k);
    var sh = ctx.createWaveShaper(); sh.curve = arr; sh.oversample = '2x';
    var pre = ctx.createGain(); pre.gain.value = lerp(1.5, 7, drive);
    var lp = ctx.createBiquadFilter(); lp.type = 'lowpass';
    lp.frequency.value = 600 + drive * 1400;
    pre.connect(sh); sh.connect(lp); lp.connect(env);

    [[f, 'sawtooth', 0.5], [f / 2, 'sawtooth', 0.4], [f / 2 * 1.5, 'triangle', 0.2]]
      .forEach(function (v) {
        var osc = ctx.createOscillator(); osc.type = v[1];
        osc.frequency.setValueAtTime(v[0], t);
        var g = ctx.createGain(); g.gain.value = v[2];
        osc.connect(g); g.connect(pre);
        osc.start(t); osc.stop(t + dur + 0.03);
      });
  }

  /* Six mutually inharmonic partials, the classic metallic-percussion trick.
     The old code ran 3-4 squares each through a bandpass centred on its own
     frequency, which reduces a square to near-sine: a bell, not a cymbal. */
  var CYM_RATIOS = [1, 1.4471, 1.6170, 1.9265, 2.5028, 2.6637];

  function cymbal(ctx, dest, t, o) {
    var sum = ctx.createGain(); sum.gain.value = 1 / CYM_RATIOS.length;
    CYM_RATIOS.forEach(function (r) {
      var osc = ctx.createOscillator();
      osc.type = 'square'; osc.frequency.value = o.base * r;
      osc.connect(sum); osc.start(t); osc.stop(t + o.decay * 1.2 + 0.05);
    });
    var bp = ctx.createBiquadFilter();
    bp.type = 'bandpass'; bp.frequency.value = o.tone; bp.Q.value = 0.25;
    var hp = ctx.createBiquadFilter();
    hp.type = 'highpass'; hp.frequency.value = o.tone * 0.62;
    if (o.noise) {
      var wash = ctx.createBufferSource(); wash.buffer = o.noise;
      var wg = ctx.createGain(); wg.gain.value = 0.75;
      wash.connect(wg); wg.connect(sum);
      wash.start(t); wash.stop(t + Math.min(o.decay * 1.2, 1.9));
    }
    sum.connect(bp); bp.connect(hp);

    /* Two-stage decay: fast metallic splash over a long shimmering tail. */
    var fast = ctx.createGain();
    fast.gain.setValueAtTime(Math.max(EPS, o.vol), t);
    fast.gain.exponentialRampToValueAtTime(EPS, t + o.decay * 0.14);
    var tail = ctx.createGain();
    tail.gain.setValueAtTime(Math.max(EPS, o.vol * 0.7), t);
    tail.gain.exponentialRampToValueAtTime(Math.max(EPS, o.vol * 0.002), t + o.decay * 1.2);
    hp.connect(fast); hp.connect(tail);
    fast.connect(dest); tail.connect(dest);
  }

  function makeVoices(ctx, res) {
    var noise = res.noise;
    function hat(t, o, open) {
      var s = ctx.createBufferSource(); s.buffer = noise;
      var bp = ctx.createBiquadFilter();
      bp.type = 'bandpass'; bp.frequency.value = o.tone; bp.Q.value = 1.1;
      var hp = ctx.createBiquadFilter();
      hp.type = 'highpass'; hp.frequency.value = o.tone * 0.7;
      var g = ctx.createGain();
      var dur = open ? 0.36 : 0.045;
      g.gain.setValueAtTime(EPS, t);
      g.gain.linearRampToValueAtTime(Math.max(EPS, o.vel * (open ? 0.24 : 0.30)), t + 0.0012);
      g.gain.exponentialRampToValueAtTime(EPS, t + dur);
      s.connect(bp); bp.connect(hp); hp.connect(g); g.connect(res.drumBus);
      s.start(t); s.stop(t + dur + 0.05);
    }

    return {
      kick: function (t, o) {
        var p = o.punch * o.vel, dest = res.drumBus;
        var g = ctx.createGain(); g.connect(dest);
        g.gain.setValueAtTime(Math.max(EPS, 1.0 * p), t);
        g.gain.exponentialRampToValueAtTime(EPS, t + 0.30);
        var osc = ctx.createOscillator(); osc.type = 'sine';
        osc.frequency.setValueAtTime(190, t);
        osc.frequency.exponentialRampToValueAtTime(48, t + 0.055);
        osc.connect(g); osc.start(t); osc.stop(t + 0.32);

        var b = ctx.createOscillator(); b.type = 'triangle';
        b.frequency.setValueAtTime(62, t);
        var bg = ctx.createGain();
        bg.gain.setValueAtTime(Math.max(EPS, 0.34 * p), t);
        bg.gain.exponentialRampToValueAtTime(EPS, t + 0.18);
        b.connect(bg); bg.connect(dest); b.start(t); b.stop(t + 0.2);

        /* Trigger click — the part that makes a kick audible under a wall of
           tremolo guitar. Style-scaled: raw wants it, cosmic does not. */
        var cl = ctx.createBufferSource(); cl.buffer = noise;
        var chp = ctx.createBiquadFilter();
        chp.type = 'highpass'; chp.frequency.value = 2600;
        var cg = ctx.createGain();
        cg.gain.setValueAtTime(Math.max(EPS, 0.42 * p * res.style.kickClick), t);
        cg.gain.exponentialRampToValueAtTime(EPS, t + 0.010);
        cl.connect(chp); chp.connect(cg); cg.connect(dest);
        cl.start(t); cl.stop(t + 0.02);
      },

      snare: function (t, o) {
        var p = o.punch * o.vel, dest = res.drumBus;
        /* Crack band, deliberately bounded above so the centroid sits in the
           snare register rather than up in hiss. */
        var n1 = ctx.createBufferSource(); n1.buffer = noise;
        var hp = ctx.createBiquadFilter(); hp.type = 'highpass'; hp.frequency.value = 1700;
        var lp = ctx.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.value = 5200;
        var lp2 = ctx.createBiquadFilter(); lp2.type = 'lowpass'; lp2.frequency.value = 5200;
        var g1 = ctx.createGain();
        g1.gain.setValueAtTime(EPS, t);
        g1.gain.linearRampToValueAtTime(Math.max(EPS, 0.92 * p), t + 0.0012);
        g1.gain.exponentialRampToValueAtTime(EPS, t + 0.155);
        n1.connect(hp); hp.connect(lp); lp.connect(lp2); lp2.connect(g1); g1.connect(dest);
        n1.start(t); n1.stop(t + 0.18);

        var n2 = ctx.createBufferSource(); n2.buffer = noise;
        var bp = ctx.createBiquadFilter();
        bp.type = 'bandpass'; bp.frequency.value = 240; bp.Q.value = 1.2;
        var g2 = ctx.createGain();
        g2.gain.setValueAtTime(Math.max(EPS, 0.62 * p), t);
        g2.gain.exponentialRampToValueAtTime(EPS, t + 0.09);
        n2.connect(bp); bp.connect(g2); g2.connect(dest);
        n2.start(t); n2.stop(t + 0.11);

        [185, 232].forEach(function (f, i) {
          var osc = ctx.createOscillator(); osc.type = 'triangle';
          osc.frequency.setValueAtTime(f, t);
          var g = ctx.createGain();
          g.gain.setValueAtTime(Math.max(EPS, (i ? 0.26 : 0.38) * p), t);
          g.gain.exponentialRampToValueAtTime(EPS, t + 0.075);
          osc.connect(g); g.connect(dest); osc.start(t); osc.stop(t + 0.09);
        });
      },

      chat: function (t, o) { hat(t, { tone: res.style.hatTone, vel: o.vel }, false); },
      ohat: function (t, o) { hat(t, { tone: res.style.hatTone * 0.92, vel: o.vel }, true); },

      tom: function (t, o) {
        var p = o.punch * o.vel;
        var osc = ctx.createOscillator(); osc.type = 'sine';
        osc.frequency.setValueAtTime(205, t);
        osc.frequency.exponentialRampToValueAtTime(96, t + 0.18);
        var g = ctx.createGain();
        g.gain.setValueAtTime(Math.max(EPS, 0.66 * p), t);
        g.gain.exponentialRampToValueAtTime(EPS, t + 0.26);
        osc.connect(g); g.connect(res.drumBus); osc.start(t); osc.stop(t + 0.28);

        var n = ctx.createBufferSource(); n.buffer = noise;
        var ng = ctx.createGain();
        ng.gain.setValueAtTime(Math.max(EPS, 0.18 * p), t);
        ng.gain.exponentialRampToValueAtTime(EPS, t + 0.02);
        n.connect(ng); ng.connect(res.drumBus); n.start(t); n.stop(t + 0.03);
      },

      ride: function (t, o) {
        cymbal(ctx, res.drumBus, t,
          { base: res.style.cymBase * 1.18, tone: 7200, decay: 1.5, vol: 0.40 * o.vel, noise: res.noise });
      },
      crash: function (t, o) {
        cymbal(ctx, res.drumBus, t,
          { base: res.style.cymBase, tone: 5200, decay: 2.8, vol: 0.70 * o.vel, noise: res.noise });
      },
      cowbell: function (t, o) {
        [562, 845].forEach(function (f, i) {
          var osc = ctx.createOscillator(); osc.type = 'square'; osc.frequency.value = f;
          var g = ctx.createGain();
          g.gain.setValueAtTime(Math.max(EPS, 0.20 * o.vel), t);
          g.gain.exponentialRampToValueAtTime(EPS, t + (i ? 0.24 : 0.12));
          var bp = ctx.createBiquadFilter();
          bp.type = 'bandpass'; bp.frequency.value = 2600; bp.Q.value = 1.2;
          osc.connect(g); g.connect(bp); bp.connect(res.drumBus);
          osc.start(t); osc.stop(t + 0.3);
        });
      }
    };
  }

  /* ================= GRAPH ================= */
  var CHANNEL_IDS = ['drums', 'gtrchord', 'gtrtrem', 'bass', 'pads', 'drones', 'scifi'];

  function createGraph(ctx, styleId, seed) {
    var style = STYLES[styleId] || STYLES.cosmic;
    var rng = makeRng(seed + '|ir');

    var destination = ctx.destination;

    /* Master: gentle bus control, then a true limiter. The old chain put a
       -16 dB / 4:1 compressor across the whole mix, so every blast beat
       pumped the guitars and pads down with it. */
    var limiter = ctx.createDynamicsCompressor();
    limiter.threshold.value = -6; limiter.ratio.value = 20;
    limiter.attack.value = 0.001; limiter.release.value = 0.05; limiter.knee.value = 0;

    /* A compressor is not a true limiter: it has a knee and a finite attack, so
       transients still overshoot. A tanh stage after it bounds the output
       mathematically, which is what keeps the true-peak ceiling honest. */
    var softClip = ctx.createWaveShaper();
    var sc = new Float32Array(4096);
    for (var q = 0; q < 4096; q++) sc[q] = Math.tanh(((q * 2) / 4096 - 1) * 1.6);
    softClip.curve = sc; softClip.oversample = '2x';

    var outGain = ctx.createGain(); outGain.gain.value = 0.85;
    var analyser = ctx.createAnalyser(); analyser.fftSize = 2048;
    var master = ctx.createGain(); master.gain.value = 0.72;
    master.connect(limiter); limiter.connect(softClip);
    softClip.connect(outGain); outGain.connect(analyser); analyser.connect(destination);

    var convolver = ctx.createConvolver();
    convolver.buffer = makeImpulse(ctx, style.reverb.seconds, style.reverb.damp, rng);
    var reverbReturn = ctx.createGain(); reverbReturn.gain.value = style.reverb.mix;
    convolver.connect(reverbReturn); reverbReturn.connect(master);

    var channels = {};
    CHANNEL_IDS.forEach(function (id) {
      var gain = ctx.createGain();
      gain.gain.value = Math.pow((style.mix[id] || 50) / 100, 2) * 0.9;
      var mute = ctx.createGain();
      var send = ctx.createGain();
      send.gain.value = id === 'drums' ? 0.5 : (id === 'bass' ? 0.12 : 0.85);
      gain.connect(mute); mute.connect(master); mute.connect(send); send.connect(convolver);
      channels[id] = { gain: gain, mute: mute, send: send, muted: false };
    });

    /* No compressor on the drum bus or the amps. A DynamicsCompressorNode
       delays its output (measured ~20 ms on one runtime, implementation
       specific), and only the master compressor sits across every source, so
       adding one to a subset shifts those instruments late against the rest. */
    var drumBus = ctx.createGain();
    drumBus.connect(channels.drums.gain);

    var amps = {}, vibOscs = [];
    ['gtrchord', 'gtrtrem'].forEach(function (id, i) {
      var vib = ctx.createOscillator(); vib.type = 'sine';
      vib.frequency.value = i ? 6.2 : 5.5;
      var vibAmt = ctx.createGain(); vibAmt.gain.value = i ? 5 : 4;
      vib.connect(vibAmt); vib.start(); vibOscs.push(vib);
      var amp = makeGuitarAmp(ctx, channels[id].gain, id === 'gtrtrem');
      amp.vib = vibAmt;
      amps[id] = amp;
    });
    amps.gtrchord.setDrive(style.drive.rhythm / 100);
    amps.gtrtrem.setDrive(style.drive.lead / 100);

    var padFilter = ctx.createBiquadFilter();
    padFilter.type = 'lowpass'; padFilter.Q.value = 0.8;
    padFilter.frequency.value = style.id === 'raw' ? 1400 : 2600;
    padFilter.connect(channels.pads.gain);

    var res = {
      ctx: ctx, style: style, master: master, analyser: analyser,
      channels: channels, drumBus: drumBus, amps: amps,
      padFilter: padFilter, convolver: convolver, reverbReturn: reverbReturn,
      noise: makeNoise(ctx, 2, makeRng(seed + '|noise')),
      noiseLong: makeNoise(ctx, 4, makeRng(seed + '|noiseL')),
      voiceCount: 0
    };
    res.voices = makeVoices(ctx, res);
    /* A rebuilt graph must not leave the old one running: free-running vibrato
       oscillators and a live path to the destination would accumulate on every
       style or seed change. Fade, then disconnect and stop. */
    res.dispose = function () {
      var now = ctx.currentTime;
      try { master.gain.setTargetAtTime(0, now, 0.03); } catch (e) { /* closed */ }
      setTimeout(function () {
        vibOscs.forEach(function (o) { try { o.stop(); } catch (e) { /* stopped */ } });
        try { master.disconnect(); analyser.disconnect(); reverbReturn.disconnect(); } catch (e) { /* gone */ }
      }, 4000);
    };
    return res;
  }

  function padChord(res, chord, t, dur) {
    var ctx = res.ctx;
    var env = ctx.createGain();
    env.gain.setValueAtTime(EPS, t);
    env.gain.linearRampToValueAtTime(0.11, t + dur * 0.35);
    env.gain.linearRampToValueAtTime(EPS, t + dur * 1.02);
    env.connect(res.padFilter);
    var oct = res.style.padOct;
    [chord.root + 12 + oct, chord.third + 12 + oct, chord.fifth + 12 + oct,
     chord.root + 24, chord.fifth + 24].forEach(function (m) {
      [-6, 5].forEach(function (det) {
        var osc = ctx.createOscillator(); osc.type = 'sawtooth';
        osc.frequency.value = midiToFreq(m); osc.detune.value = det;
        osc.connect(env); osc.start(t); osc.stop(t + dur * 1.05);
      });
    });
  }

  function droneNote(res, chord, t, dur, rng) {
    var ctx = res.ctx;
    var mod = ctx.createOscillator();
    mod.frequency.value = Math.max(0.02, res.modRate || 0.15);
    var modG = ctx.createGain(); modG.gain.value = 6;
    mod.connect(modG);
    var env = ctx.createGain();
    env.gain.setValueAtTime(EPS, t);
    env.gain.linearRampToValueAtTime(0.09, t + 2);
    env.gain.linearRampToValueAtTime(EPS, t + dur);
    env.connect(res.channels.drones.gain);
    [chord.root, chord.root + 7].forEach(function (m) {
      var osc = ctx.createOscillator();
      osc.type = rng() < 0.5 ? 'sine' : 'triangle';
      osc.frequency.value = midiToFreq(m + 12);
      osc.detune.value = lerp(-9, 9, rng());
      modG.connect(osc.frequency); osc.connect(env);
      osc.start(t); osc.stop(t + dur + 0.1);
    });
    mod.start(t); mod.stop(t + dur + 0.1);
  }

  /* ================= EXPORT ================= */
  global.VoidChoir = {
    EPS: EPS, MIN_SCHED_AHEAD: MIN_SCHED_AHEAD, MAX_VOICES: MAX_VOICES,
    STEPS: STEPS, DRUMS: DRUMS, REF_MIDI: REF_MIDI, CAB: CAB, CAB_SETTINGS: CAB_SETTINGS, makeCabIR: makeCabIR, cabDb: cabDb, CHANNEL_IDS: CHANNEL_IDS,
    STYLES: STYLES, STYLE_IDS: STYLE_IDS, MODES: MODES,
    clamp: clamp, lerp: lerp, midiToFreq: midiToFreq,
    makeRng: makeRng, buildChords: buildChords,
    createComposer: createComposer, generatePatterns: generatePatterns,
    emptyPattern: emptyPattern, generateMotif: generateMotif,
    buildEventList: buildEventList, hashEvents: hashEvents,
    createAsymmetricDistortion: createAsymmetricDistortion, curveHash: curveHash,
    makeNoise: makeNoise, makeImpulse: makeImpulse,
    makeGuitarAmp: makeGuitarAmp, guitarPick: guitarPick, bassNote: bassNote,
    cymbal: cymbal, makeVoices: makeVoices, createGraph: createGraph,
    padChord: padChord, droneNote: droneNote
  };
})(typeof globalThis !== 'undefined' ? globalThis : this);
