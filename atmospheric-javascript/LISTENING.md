# Void Choir — Listening Pass (S-080)

All styles verified with seed `glacier`.

- raw: ok | tuning: none
- blackgaze: ok | tuning: none
- cosmic: ok | tuning: none
- meshuggah: ok | tuning: none
- classic-rock: ok | tuning: none
- blues-rock: ok | tuning: none
- motorhead: ok | tuning: none
- hair-metal: ok | tuning: none
- nirvana: ok | tuning: none
- ramones: ok | tuning: none
- art-punk: ok | tuning: none
- exploited: ok | tuning: none
- disco: ok | tuning: none

## Functional UI Verification
- Style dropdown correctly populates all 13 styles in canonical order (V.STYLE_IDS).
- `?style=<id>` selects the target style on load, falling back to `cosmic` if invalid.
- Changing style in dropdown applies `applyStyleMix()`, updates tempo/summary, and restarts audio via `reseed()`.
- "Copy link" generates URL containing `?style=<id>&seed=<seed>`.
