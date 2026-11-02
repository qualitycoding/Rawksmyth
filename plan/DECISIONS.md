
## Addendum v3.9 decisions (2026-11-02)

- **D-900** — Planning-only amendment. Product scope, public interfaces,
  expected test behaviour, performance targets and human gates do not change,
  except the mandatory split G-003 lesson/knowledge store semantics.
- **D-901** — Source-branch drift: the planning source branch tip `d7fce2f28341`
  is the rollout phase's `plan: add protocol v3.9 addendum` commit, a direct
  child of the audited source commit `9d43955e82cc`. The implementation
  branch descends from `d7fce2f28341`, which descends from the audited commit;
  no newer planning was overwritten or rebased away.
- **D-902** — No platform expansion: no new OS/architecture/runtime support is
  claimed; CI coverage is recorded as plan-level jobs/waivers only.
- **D-903** — No release, deployment or package publication is authorised or
  planned by this amendment.
- **D-904** — Ambiguities resolved from existing plan artifacts and repository
  evidence only; defaults follow the addendum's no-added-platform / explicit
  waiver rule.
- **D-905** — Local build and test execution is not available in the ephemeral
  sandbox; the CI matrix jobs are the authoritative execution venue (honest
  substitution; see DEVIATIONS.md).
- **D-906** — Fresh-context independent review is unavailable in this
  environment; the cold read is recorded as a substitution with High risk
  (S-ADD-090), not claimed as an independent pass.
- **D-980** — Mutation testing design: see `reviews/MUTATION.md` (priced
  three-tier trigger strategy, one aggregator job, report-based score gate).
