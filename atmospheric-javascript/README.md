# atmospheric-javascript

javascript engine that produces continuous stream of "music"

**Void Choir** is a generative engine that plays an endless stream of atmospheric black metal in your browser. Nothing is pre-recorded: drums, guitars, bass, pads, drones and sound effects are all synthesised live with the Web Audio API, and the whole app is one HTML file with no build step.

**Try it:** [qualitycoding.github.io/atmospheric-javascript](https://qualitycoding.github.io/atmospheric-javascript/), or download `index.html` and open it from disk.

## Using it

Press **BEGIN TRANSMISSION**. Browsers only allow audio after a click, so nothing plays until you do.

**Style and seed** (top of the page)
- **Style** picks the subgenre (see below).
- **Seed** is any text. The same style and seed always give the same piece: same chords, melody and drum pattern.
- **New seed** picks a random one. **Copy link** copies a URL that reproduces the current style and seed.

**Sequencer.** A 16-step grid with eight drum rows (kick, snare, closed and open hat, tom, ride, crash, cowbell). Click cells to edit the pattern. Your edits stay until you change the style or seed, or move the blast-intensity knob, any of which rewrites the pattern.

**Mixer.** Seven channels, each with a knob, a level fader and a mute button:

| Channel | Knob |
|---|---|
| Drumkit | Blast intensity (how busy the pattern is) |
| Rhythm guitar | Drive |
| Lead guitar | Drive, plus a tremolo-picking speed in notes per beat |
| Bass guitar | Overdrive |
| Synth pads | Filter cutoff |
| Drones | Modulation rate |
| Sci-fi ambience | Density |

## Styles

| Style | Tempo | Mode | Character |
|---|---|---|---|
| Second Wave, raw | 186 BPM | Phrygian | Mostly blast beats, short dry room, hard-driven guitars |
| Blackgaze, symphonic | 142 BPM | Aeolian | Mixed blast, gallop and half-time, long reverb, prominent pads |
| Atmospheric, cosmic | 112 BPM | Aeolian | Mostly half-time and sparse drums, very long reverb, pads and drones forward |
| Meshuggah, oblique | 126 BPM | Phrygian | Half-time-weighted, dry room, maximum drive, low tonic |
| Classic Rock, arena | 122 BPM | Mixolydian | Mixed gallop and half-time, moderate reverb, major-key riffs |
| Blues Rock, shuffle | 108 BPM | Blues scale | Shuffle-feel drums, lead forward, warm drive |
| Motörhead, speed | 190 BPM | Mixolydian | Punk-weighted drums, heavy drive, dry room, loud bass |
| 80s Hair Metal, arena | 132 BPM | Mixolydian | Four-on-floor/gallop mix, long reverb, lead forward |
| Nirvana, grunge | 134 BPM | Aeolian | Half-time/punk mix, crunchy rhythm guitar forward |
| Ramones, punk | 178 BPM | Aeolian | Driving eighths, backbeat, short dry room |
| Art Punk, avant | 150 BPM | Dorian | Angular motifs, sparse/punk mix, prominent sci-fi ambience |
| Exploited, street punk | 184 BPM | Phrygian | Fast punk beats, high drive, very dry room |
| Disco, four on the floor | 118 BPM | Dorian | Offbeat open hats, kick quarters, bass forward, cleaner guitars |

Each style sets its own tempo, chord progression, drum-feel weighting, reverb, guitar drive and mix balance. Changing style rebuilds the audio and restarts from the top.

## Share links

The page reads these URL parameters:

| Parameter | Values | Effect |
|---|---|---|
| `style` | `raw`, `blackgaze`, `cosmic`, `meshuggah`, `classic-rock`, `blues-rock`, `motorhead`, `hair-metal`, `nirvana`, `ramones`, `art-punk`, `exploited`, `disco` | Starting style |
| `seed` | any text | Starting seed |
| `cabhf` | `0` to `1` | Scales the high-frequency lift of the guitar cabinet: `0` is flat above 4.6 kHz, `1` is fully fitted (default) |

Example: `?style=blackgaze&seed=glacier` appended to the app's URL, or `index.html?style=blackgaze&seed=glacier` when opened from disk.

Reproducibility is at the level of the music (which notes and drum hits are played), not sample-exact audio, because different browsers render audio slightly differently.

## How it works

- **Composer.** A seeded random generator drives everything, so the music is a pure function of style and seed. There is no `Math.random` in the engine.
- **Guitars.** Sawtooth oscillators go through a note-tracked brightness filter, an asymmetric clipper, then a measured cabinet response. Keeping the distorted tone the same shape at every pitch is what stops the timbre drifting as notes change.
- **Cabinet.** The cabinet response was fitted to a real recording of a synth guitar (six pitches, 58 to 104 Hz) and is applied as a minimum-phase impulse response built at load time, so it is correct at any sample rate. See [CHANGES.md](CHANGES.md) for the method, the measurements and the caveats.
- **Drums.** Synthesised: a pitch-swept kick with a click, a two-band snare, cymbals built from six inharmonic partials plus noise, and a shared synthesised room reverb.
- **Master.** A limiter with a soft clip, so peaks stay under the ceiling.
- **Robustness.** Voices are counted and capped so a long session can't pile up audio nodes, timers are cleared on stop, and the scheduler widens its lookahead when the tab is in the background.

## Development

`index.html` is a **generated file**. It is built from two sources: `void-choir.template.html` (the interface) and `void-choir-engine.js` (the audio and music engine). Edit those, then rebuild. Don't edit `index.html` by hand; the next build would overwrite it.

```
npm install          # once, for the test dependency
npm run build        # regenerate index.html from its sources
npm run check        # exit 1 if index.html is out of date
npm test             # the full test suite (includes the same freshness check)
```

The build script needs only Node, with no dependencies. It fails with a clear message if the template loses its engine tag, or if the engine contains `</script`, which would end the inline script early.

The suite renders audio offline (via `node-web-audio-api`) and runs 45 checks: same seed gives the same music; guitar harmonic balance is stable across pitch; drum attack, decay and brightness; peak level; the cabinet response at 44.1, 48 and 96 kHz; timing alignment between instruments; how closely the guitar matches the reference recording; and that `index.html` matches its sources.

| Path | Purpose |
|---|---|
| `index.html` | The app, generated by `build.js` (engine inlined). Serve or open this one |
| `void-choir.template.html` | Source of the interface; the build inlines the engine into it |
| `void-choir-engine.js` | Source of the audio and music engine, also loaded directly by the tests |
| `build.js` | Generates `index.html` (`--check` verifies it is current) |
| `tests/` | Test suite, helpers and the reference measurements it compares against |
| `sample-analysis/` | Which parts of the reference recording were used and the harmonic levels measured from them |
| `CHANGES.md` | What changed and why, corrections made along the way, and what is still unverified |

## Known limitations

- The tests check measurable properties. They cannot say whether it sounds good, and the cabinet's strong lift above 2.5 kHz in particular needs a listen; use `cabhf` to adjust it.
- The guitar cabinet was measured on low notes (58 to 104 Hz). It is applied to the lead guitar too, which plays much higher, so that is an extrapolation.
- Fonts load from Google Fonts; offline, the page falls back to system fonts.

## License

Apache-2.0. See [LICENSE](LICENSE).
