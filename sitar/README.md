# Sitar VST & Physical Modeling Audio Synthesizer

A physical modeling virtual instrument and DSP synthesizer reproducing the acoustics of the Indian Sitar, built in **pure C++20** with optional **JUCE 7** VST3/Standalone plugin targets.

## Features

- **Karplus-Strong Digital Waveguide with Sub-Sample Allpass Interpolation**: Eliminates tuning bias across the 4-octave playing register with strict phase compensation.
- **Jawari Bridge (Non-Linear Curved Bone Saddle)**: Rolling boundary collision model with soft compressive contact damping and harmonic enrichment.
- **13 Sympathetic Resonators (Tarab Strings)**: High-Q 2nd-order biquad resonators tuned to Indian Classical Ragas (Yaman, Bhairav, Kafi, Darbari, Bilawal, Todi) excited purely through acoustic bridge coupling.
- **Chikari Rhythm Drone Strings**: Dedicated high-pitch steel drone strings tuned to Sa, Pa, Tar Sa, and Kharaj Sa for driving rapid Jhala phrases.
- **Meend Portamento Controller**: Anti-zipper microtonal lateral string deflection with deterministic slew-rate smoothing.
- **JUCE-Free Parameter Registry**: 15 normalized and discrete parameters cleanly mappable to JUCE `AudioProcessorValueTreeState` (APVTS) or any host environment.
- **Zero-Dependency Test Suite**: 19 frozen tests verifying tuning accuracy, sample rate invariance, harmonic generation, loop stability, and real-time CPU performance (< 5% load).

## Building with CMake

```bash
# Configure build
cmake -B build -DCMAKE_BUILD_TYPE=Release

# Build test runner and DSP static library
cmake --build build --config Release

# Run the 19 verified tests
./build/sitar_tests
```

## Running the Verification Test Suite

All 19 tests run offline without external dependencies:

```text
=========================================================
  SITAR VST — VERIFIED FROZEN DSP TEST SUITE (C++20)     
=========================================================
[T001] Karplus-Strong Pitch Tuning Accuracy across MIDI 36-72 ... PASS (22.3 ms)
[T011] Sample Rate Invariance at 44.1k, 48k, and 96k ... PASS (8.0 ms)
[T002] Jawari Buzz Harmonic Generation (High-Frequency Enrichment) ... PASS (65 µs)
[T006] Jawari Bridge Asymmetric Boundary Condition & Curvature ... PASS (< 1 µs)
[T013] Mizrab Hardness Dynamic Brightness & Spectral Envelope ... PASS (27 µs)
[T003] Tarab Sympathetic String Resonant Ringing upon Harmonic Excitation ... PASS (112 µs)
[T007] Tarab Q Factor Ringing Decay Scaling ... PASS (6 µs)
[T014] Tarab Coupling Complete Isolation at Zero ... PASS (< 1 µs)
[T005] Meend Continuous Pitch Deflection Monotonicity & Anti-Zipper ... PASS (93 µs)
[T017] Meend Slew Rate Ordering (Fast vs Slow Portamento Glide Times) ... PASS (238 µs)
[T012] Parameter Metadata Registry Bounds & Range Invariants ... PASS (< 1 µs)
[T018] Parameter Tag Identifier Uniqueness across Registry ... PASS (8 µs)
[T004] Pluck Position Comb Filtering & Harmonic Timbre Variation ... PASS (42 µs)
[T008] Karplus-Strong Loop Numerical Stability & Energy Boundedness ... PASS (5.6 ms)
[T009] Chikari Drone String Tunings & Harmonic Sa-Pa Ratios ... PASS (192 µs)
[T010] Indian Classical Raga Scale Interval Definitions ... PASS (1 µs)
[T015] SitarSynth Stereo Audio Block Rendering & Spatial Diffusion ... PASS (42 µs)
[T016] SitarSynth Voice Handling & Legato Meend Transition ... PASS (26 µs)
[T019] Real-Time CPU Budget (< 5% Single-Thread Load at 44.1 kHz) ... PASS (8.5 ms)
---------------------------------------------------------
Results: 19 passed, 0 failed, 19 total
---------------------------------------------------------
```

## Repository Structure

```
src/
├── dsp/
│   ├── MathUtils.hpp            # Interpolation, pitch-to-freq conversions
│   ├── DelayLine.hpp            # Power-of-two circular buffer with allpass fractional delay
│   ├── OnePoleFilter.hpp        # Lowpass filter & exact phase delay compensation
│   ├── JawariBridge.hpp         # Asymmetric non-linear curved bone rolling contact
│   ├── KarplusStrongString.hpp  # Melody string physical waveguide engine
│   ├── SympatheticResonator.hpp # 13-string Tarab sympathetic resonator bank
│   ├── ChikariStrings.hpp       # Rhythm drone strings with Sa-Pa harmonic ratios
│   ├── MeendController.hpp      # Anti-zipper lateral pitch bend portamento
│   ├── Parameters.hpp           # JUCE-free parameter definitions and registry
│   └── SitarSynth.hpp           # Complete stereo voice synthesizer engine
└── plugin/
    ├── PluginProcessor.h/cpp    # JUCE AudioProcessor & APVTS integration
    └── PluginEditor.h/cpp       # JUCE AudioProcessorEditor GUI
tests/
    ├── test_harness.hpp         # Zero-dependency test runner & autocorr estimator
    ├── test_ks_tuning.cpp       # T-001 & T-011
    ├── test_jawari_buzz.cpp     # T-002, T-006, T-013
    ├── test_sympathetic.cpp     # T-003, T-007, T-014
    ├── test_meend.cpp           # T-005, T-017
    ├── test_parameters.cpp      # T-012, T-018
    ├── test_synth_engine.cpp    # T-004, T-008, T-009, T-010, T-015, T-016, T-019
    └── main.cpp                 # Test executable entry point
```
