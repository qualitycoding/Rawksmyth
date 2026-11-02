# Addendum v3.9 retrospective

**Repository:** `qualitycoding/Rawksmyth` — 2026-11-02.

## What happened
The planning amendment to protocol v3.9 was executed on
`impl-v39-addendum-20261102` descending from audited source `9d43955e82cccb0cece7162b193dfddd0ab32284`
(via the rollout's addendum commit `d7fce2f28341`, D-901). Steps executed:
S-ADD-000, S-ADD-001, S-ADD-010, S-ADD-020, S-ADD-030, S-ADD-040, S-ADD-050, S-ADD-060, S-ADD-070, S-ADD-080, S-ADD-090, S-ADD-100.

## Substitutions and corrections
All substitutions are recorded in `DEVIATIONS.md` at the moment they were
made: API-only git transport, CI venue for build/test/mutation dimensions,
same-author review substitution. No product code was changed; no frozen test
altered; no external action taken.

## Lessons
- L-ADDENDUM-1: when git transport is blocked but the API is reachable, the
  Git database API preserves the commit/push/verify cadence; verify the remote
  SHA after every ref update.
- L-ADDENDUM-2: uniform rollout-phase addendum commits cause predictable
  source-tip drift; verify the parent chain before blocking.

## Knowledge
- K-ADDENDUM-1: mutation-CI pricing must be labelled estimate-vs-measured;
  floor calibration belongs to the first real CI run.

G-003: lesson/knowledge store pushes requested to `qualitycoding/Lessons` and
`qualitycoding/knowledge`; **do-not-push** until sign-off (bundle fallback).
