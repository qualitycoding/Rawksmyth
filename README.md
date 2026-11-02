# Rawksmyth

A Rocksmith-style guitar learning game. You play a real electric guitar into an audio interface, the game
listens, decides whether each note on the chart was played correctly and on time, and scores it.

> **Status: the headless core works on synthetic audio. There is no game, no graphics and no live audio yet.**
> The note-checking engine, the chart format, the scoring and a headless session runner are implemented and
> pass about 90 tests, but all of them run on generated test signals, which are cleaner than a real guitar.
> Nothing has been tried with a real guitar or a real audio interface.

## What works (Rust workspace)

| Crate | What it does |
|---|---|
| `gtcore` | Pure logic with no hardware: the clock and drift model, an onset detector (SuperFlux), the note verifier (pitch, octave and ringing checks), the evidence pipeline, a chart parser, scoring, the highway layout (where each note sits on screen), and a session logger |
| `gtaudio` | WAV reading, a lock-free ring buffer, the real-time callback core and a file-based audio backend |
| `gtapp` | The headless application: song loading, an engine and a session runner. Binary `gt-headless` |
| `tests/fixtures/gen` | A seeded generator of plucked-string (Karplus-Strong) test audio |

An example song is in `assets/songs/example/` (`chart.json`, `guitar.wav`, `backing.wav`). Run the example
without any hardware:

```bash
cargo run --release -p gtapp --bin gt-headless -- assets/songs/example --log session.json
```

Run the tests (needs Rust 1.83):

```bash
cargo test --workspace
sha256sum -c tests/FROZEN_MANIFEST.sha256     # checks that the frozen tests are unchanged
```

## Not built

- The live audio backend (cubeb duplex), including real latency measurement. The tool for that is a stand-alone
  spike in `research/spikes/spike-cubeb-duplex`, only partly run.
- The Godot 4 front end (`gtbridge` and `godot-project/`): no graphics, highway rendering or settings UI.
- Live multi-threaded wiring, performance hardening and macOS microphone permissions.
- Validation with a real guitar. Known weak spot: several notes ringing together make the verifier reject good
  notes at faster tempos (about 1.6 % of notes in one synthetic test at 0.30 s spacing).
- Legato notes are not supported, and the parser rejects them.

`HANDOFF.md` lists these limits and the deliberate deviations from the plan in full. The plan is in `plan/PLAN.md`.

## Other branches

`rusty-dev` is a separate line of work: a TypeScript and Vite front end with a Rust crate (`crates/rocksmith-core`)
and web audio code. It is not part of the workspace described above.

## Related projects

- [audio-to-rocksmith](https://github.com/qualitycoding/audio-to-rocksmith): a planned tool that turns a guitar or bass stem into a Rocksmith 2014 custom song.

## Licence

Apache License 2.0. See [LICENSE](LICENSE).
