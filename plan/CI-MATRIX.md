# CI platform matrix (v3.9, intake-derived)

**Repository:** `qualitycoding/Rawksmyth` — primary language: Rust.

Intake declares no additional OS/architecture/runtime bounds beyond the default
Linux CI runner; per D-902/D-904 no platform expansion is claimed.

| Platform bound | CI job | Waiver |
|---|---|---|
| ubuntu-24.04 x86_64 | `build-test` (pinned label `ubuntu-24.04`) | — |
| oldest supported toolchain | `oldest-toolchain` job (pinned) | — |
| ARM64 | `linux-arm64` job (pinned label) | — |
| libc variant | `musl` container job | — |

Matrix fail-fast disabled. Runner labels and action pins are checked for
existence at plan freeze; every intake-declared platform maps to a versioned
job or a recorded waiver. Long builds and mutation runs execute in CI, not in
ephemeral local sandboxes (D-905).
