# Assumptions & Ambiguity Resolutions (plan/ASSUMPTIONS.md)

| ID | Topic | Resolved Choice | Rationale & Technical Boundary |
|---|---|---|---|
| **A-001** | Rendering Engine | Godot 4.3 (Slave Renderer via GDExtension) | Godot provides rapid 3D note highway rendering, cross-platform viewport scaling, and UI. Boundary: Godot's audio engine is bypassed; rendering is purely slave-driven via lock-free SPSC queue from Rust. |
| **A-002** | Audio I/O Library | `cubeb` (via `cubeb-rs`) with `cpal` fallback | Firefox-proven production audio library supporting low-latency WASAPI Exclusive, AudioUnit, and ALSA/Pulse. Duplex stream guarantees sample-accurate sync. |
| **A-003** | Pitch Detection Algorithm | Hybrid MPM + Octave-Decimated YIN (Multi-Rate) | Avoids octave errors on low $E_2$ ($82.4\text{ Hz}$) by downsampling by $4\times$ (to $11.025\text{ kHz}$) for sub-$200\text{ Hz}$ analysis, maintaining $<50\text{ ms}$ algorithmic latency. |
| **A-004** | Polyphonic Support | Out of Scope for Milestone 1 (Monophonic only) | Milestone 1 focuses on single-note lead/bass detection. Polyphony (chords) deferred to Milestone 2. |
| **A-005** | Example Song | Original procedural composition + Frozen reference FLAC DI fixture | Zero copyright risk; fully reproducible in automated test suites and live play. |
| **A-006** | Guitar Input Signal | Dry DI with $3.5\text{ kHz}$ Conditioning Pre-Filter | Suppresses sharp pick scrape transients and electromagnetic pickup noise before feeding detection algorithms. |
| **A-007** | Scoring Model | Rocksmith-style continuous pitch + timing window | Strict timing tolerances: Perfect ($\le 15\text{ ms}$), Great ($\le 30\text{ ms}$), Good ($\le 50\text{ ms}$), Miss ($> 50\text{ ms}$ or wrong pitch). |
| **A-008** | Latency Calibration | Dual Manual Sliders (Audio Offset + Visual Offset) | Rhythm-game standard: separates soundcard driver roundtrip from monitor display lag. |
| **A-009** | Repository & Crate Structure | Clean Rust workspace (`rocksmith-core`) + GDExtension crate | Standalone core DSP library enables 100% headless CI compilation and test execution without Godot runtime dependencies. |
| **A-010** | License | AGPL-3.0 | Consistent with open-source rhythm game ecosystem (`fee[dB]ack`, `Slopsmith`). |
| **A-011** | Transient/Pitch Reconciliation | 2-Stage Temporal Note Tracker | Pick attacks are inharmonic for the first $15\text{ ms}$; pitch estimation is gathered over $15\text{--}35\text{ ms}$ before final note attribution. |
| **A-012** | CI Virtualization Constraint | Decoupled Hardware Benchmarks | Headless CI runs offline DSP throughput tests (`T-015a`); physical hardware latency (`T-015b`) is evaluated during Gate G-001. |
