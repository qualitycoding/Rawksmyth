# Gate G-002 Evidence Bundle

## 1. Baseline Verification (S-010)
- Branch: `gen-20260923T084400Z-ten-new-styles`
- Baseline suite: 45 passed, 0 failed
- Freshness check: `void-choir.html is up to date`
- Baseline event hashes (`seed: 'glacier'`, `bars: 8`):
  - `raw`: `830e96c3`
  - `blackgaze`: `e8a68f67`
  - `cosmic`: `128c8cc1`

## 2. Engine Extensions (S-020, S-030)
- Added modes:
  - `mixolydian`: `[0, 2, 4, 5, 7, 9, 10]`
  - `blues`: `[0, 3, 5, 6, 7, 10]`
- Added drum archetypes in `generatePatterns` & `pickArchetype`:
  - `punk`: quarter-note kick, 8th-note closed hi-hat, backbeat snare, crash
  - `fourFloor`: four-on-the-floor kick, offbeat open hats, backbeat snare
  - `shuffle`: shuffle-feel kick on 1 & 3, snare on 2 & 4, ride on 0
- Added 10 UI-selectable styles:
  1. `meshuggah`: 126 BPM, Phrygian, halftime-weighted
  2. `classic-rock`: 122 BPM, Mixolydian, arena rock feel
  3. `blues-rock`: 108 BPM, Blues scale, shuffle feel
  4. `motorhead`: 190 BPM, Mixolydian, speed punk feel
  5. `hair-metal`: 132 BPM, Mixolydian, four-on-the-floor/gallop mix
  6. `nirvana`: 134 BPM, Aeolian, grunge feel
  7. `ramones`: 178 BPM, Aeolian, driving punk 8ths
  8. `art-punk`: 150 BPM, Dorian, angular motifs
  9. `exploited`: 184 BPM, Phrygian, street punk
  10. `disco`: 118 BPM, Dorian, four-on-the-floor
- Total `STYLE_IDS`: 13

## 3. Test Suite Verification (S-040, S-050)
- Test suite execution: **147 passed, 0 failed**
- Amended T-24 style independence assertion from fixed 3 to `V.STYLE_IDS.length` (13)
- Added new tests T-31..T-36 (registry schema, pattern verification, modes validity, label uniqueness)
- All 13 styles verified for determinism (T-01), envelope (T-03), tremolo grid (T-21), cowbell gating, and motif contour
- All 13 styles pass offline audio renders:
  - T-11 peak level: all ≤ -1 dBFS (range: -2.02 to -1.22 dBFS)
  - T-17 level stability: all p95/p50 < 3 (range: 1.05 to 1.19)
- Bit-identical regression check for original three styles:
  - `raw`: `830e96c3` (identical to baseline)
  - `blackgaze`: `e8a68f67` (identical to baseline)
  - `cosmic`: `128c8cc1` (identical to baseline)

## 4. Build & Distribution Verification (S-060)
- `npm run build`: successfully generated `void-choir.html`
- `npm run check`: verified `void-choir.html is up to date`
- Grep guard: verified all 10 new style ids exist in built distribution

## 5. Documentation & Listening (S-070, S-080)
- `README.md`: updated Styles table and share link specification
- `CHANGES.md`: recorded release notes for 2026-09-23
- `LISTENING.md`: verified all 13 styles, dropdown selection, reseed behavior, and link generation
