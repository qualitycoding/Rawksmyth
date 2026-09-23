# Calibration Offset Verification Report

## Overview
Pursuant to Decision D-007, the engine decouples soundcard roundtrip buffer latency (Audio Offset) from visual display refresh lag (Visual Offset).

## Offset Verification Checklist
- [x] **Audio Offset Range:** Adjusts within `[-200, +200] ms` smoothly without causing AudioBuffer underruns or buffer rebuild stalls.
- [x] **Visual Offset Range:** Shifts highway note gem perspective position predictably along the $y$-axis relative to the hit laser line.
- [x] **Metronome Sync:** Audio click tone aligns with the visual beat flash when both offsets are calibrated to workstation baseline.
- [x] **Bounds Safety:** Values exceeding `[-200, +200] ms` are clamped by `ClockSynchronizer::set_audio_offset_ms` and `set_visual_offset_ms` (verified in `T-019`).
