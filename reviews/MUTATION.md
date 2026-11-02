# Mutation testing design — priced three-tier, one aggregator (v3.9)

**Repository:** `qualitycoding/Rawksmyth` — primary language: Rust. **Tool:** Mull
(C/C++/LLVM) where applicable; a language-appropriate equivalent (Stryker /
mutmut / cargo-mutants) per repo toolchain, pinned after existence checks.

## Pricing (recorded at amendment time)

- Baseline fast-test time: estimated from repository size (~0 source files)
  — **estimate, to be confirmed by the first CI run** (D-905).
- Estimated mutants: set by the first incremental run; post-condition requires
  a non-empty parseable report and >= 10 mutants (>= 1 with the small-scope
  D-ID recorded).
- Incremental job budget: 20 min; scheduled full-run budget: 120 min;
  deterministic module sharding if the full run exceeds budget.

## Trigger strategy (three tiers)

1. **Incremental** on pull requests and default-branch pushes, scoped to
   changed mutable files; no changed mutable files → explicit `not-applicable`.
2. **Scheduled full run** (weekly) with `qualitycoding` as named owner and a failure
   notification/issue.
3. **workflow_dispatch** on demand. Changes to killing tests or mutation config
   force a full run.

## Aggregator and score gate

One always-running aggregator job is the only required mutation status; it
fails unless the mutation job passed or was legitimately not applicable; no
`paths` filtering skips a required workflow. The score gate parses the tool
report (not exit status), enforces the report/mutant-count post-conditions, and
has a demonstrated below-floor failing fixture recorded in CI. After the
first real full run the floor is set five points below the measured score;
survivors are converted to tests or written class-grouped justifications.

## Status

Design recorded; first run and floor calibration **pending in CI**
(honest-evidence substitution D-905 — not claimed as locally passed).
