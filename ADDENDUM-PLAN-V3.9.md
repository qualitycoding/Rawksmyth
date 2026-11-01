# Addendum plan — planning protocol v3.9

**Status: READY FOR ADDENDUM IMPLEMENTATION**  
**Repository:** `qualitycoding/Rawksmyth`  
**Planning source branch:** `gen-20260923T152727Z-rocksmith-clone`  
**Audited source commit:** `9d43955e82cccb0cece7162b193dfddd0ab32284`  
**Current planning baseline:** **v3.1**  
**Target protocol:** **v3.9**, `qualitycoding/skills` commit `ef99285404419153dd5f9aaae14f14bb53604a62`  
**Active profiles:** `software, computational`; `software.deploys = false`  
**Planning root(s):** `plan`

## Purpose and non-negotiable scope

Upgrade the repository's existing planning to protocol v3.9 without executing the product plan. Preserve product scope, interfaces, frozen expected behaviour and existing human decisions. Where no protocol-based plan exists, create the missing v3.9 planning layout from the README, source and existing tests; record unavoidable defaults as A/D IDs rather than asking questions. This addendum is normative together with the base plan and stops after the planning amendment is pushed.

The canonical cross-repository controller is `qualitycoding/skills/protocols/planning-v3.9-addendum-control.md`. The implementing agent must follow its source-drift, push, recovery and reporting rules.

## Audited version gap

| Introduced | Missing capability | Amendment step |
|---|---|---|
| v3.2 | Lessons, knowledge, R0 prior-knowledge loading, retrospective and closing store push | `learning` step below |
| v3.3 | Periodic full-context checkpoints and zero-context resumption | `checkpoint` step below |
| v3.4 | Assumption/premise audit plus traditional, adversarial and optional mutation review steps | `assumption` step below |
| v3.5 | Honest verification, command/pin evidence and complete deviation logging | `honest` step below |
| v3.6 | Environment-class probe, ephemeral protections and immediate push verification | `ephemeral` step below |
| v3.7 | Separate Lessons and knowledge stores throughout intake, checkpoints and G-003 | `split` step below |
| v3.8 | Intake-derived versioned CI platform matrix with explicit coverage or waivers | `ci` step below |
| v3.9 | Priced three-tier mutation CI with one aggregator and a report-based score gate | `mutation` step below |

Capabilities not listed are treated as present but still receive the final consistency check. A discovered false baseline claim is added to the gap ledger and repaired; it is not ignored merely because this table omits it.

## Global execution contract

1. Work only on `impl-v39-addendum-20261102` created from the audited source commit. Never commit directly to the planning source branch during implementation.
2. After every successful step, commit all artifacts, push, and verify the remote branch SHA equals local HEAD. On rejection, fetch and rebase onto the implementation branch only; a changed planning source requires a fresh gap review.
3. Append every attempt to `EXECUTION_LOG.md`; record substitutions in `DEVIATIONS.md` and corrections in `lessons/`. Never record credentials or personal data.
4. Use protocol v3.9 and `skills/mutation-testing-in-ci` at the source commit above. Do not weaken frozen tests or report unrun checks as passed. No deployment, release, package publication or live external service action is authorised.
5. Steps are sequential because they share plan files and checkpoint state. Resume at the first status row not marked `pass` after validating artifact hashes.

## Steps

### S-ADD-000 Verify source, access and resumable workspace
- Tier: Sonnet
- Profile: software (and computational where already active)
- Depends on: none
- Inputs: current planning branch, this addendum, protocol v3.9 at `ef99285404419153dd5f9aaae14f14bb53604a62`, and the files named by each action
- Actions:
  1. Confirm `git rev-parse HEAD` equals `9d43955e82cccb0cece7162b193dfddd0ab32284` on `gen-20260923T152727Z-rocksmith-clone`. If not, stop this repository, write `BLOCKED.md` with the observed SHA, and do not merge or overwrite newer work.
  2. Create implementation branch `impl-v39-addendum-20261102` from that exact commit; if it already exists, resume from its tip and reconcile the last status entry instead of repeating completed work.
  3. Verify push, workflow and pull-request permissions and reachability of GitHub, required package registries and CI artifact storage. Record results without recording credentials.
  4. Probe whether the execution environment is stable or ephemeral. On ephemeral environments, keep all work on the durable mount, restage tools at turn start, restore executable bits, and move long builds to CI.
  5. Create `ADDENDUM-V3.9-STATUS.md` with one row per applicable step, initial state `pending`, and the source/target protocol versions.
- Outputs: `ADDENDUM-V3.9-STATUS.md`, `.checkpoints/state.json`, `BLOCKED.md` only on source/access failure
- Evidence produced: upgrade evidence in `ADDENDUM-V3.9-STATUS.md`; no product-result evidence
- Done when: source SHA and permissions match, the branch is resumable, and every applicable step is registered; every output exists, is non-empty, and the step entry records commit hash and verification result
- Checkpoint: record this step, outputs and SHA-256 hashes in `.checkpoints/state.json`
- On failure: record the failure and any correction as a lesson; use the most reversible fix; halt in `BLOCKED.md` if frozen behaviour, security, data integrity, public interfaces or an external action would change
- Gate: none
- Relevant decisions/claims: D-900
- Lessons applied: none
- Exclusive resources: the repository working branch and `.checkpoints/state.json`

### S-ADD-001 Freeze the upgrade contract and gap ledger
- Tier: Sonnet
- Profile: software (and computational where already active)
- Depends on: S-ADD-000
- Inputs: current planning branch, this addendum, protocol v3.9 at `ef99285404419153dd5f9aaae14f14bb53604a62`, and the files named by each action
- Actions:
  1. Preserve the existing product plan and frozen tests. Add `plan/ADDENDUM-V3.9-GAP.md` (for multiple plan roots, place it at each root) listing baseline `v3.1`, target `v3.9`, source commit `9d43955e82cccb0cece7162b193dfddd0ab32284`, and only the gaps in this addendum.
  2. Add decision D-900: this is a planning-only amendment; product scope, public interfaces, expected test behaviour, performance targets and human gates do not change except for the mandatory split G-003 store semantics.
  3. Add decisions D-901 through D-909 for each resolved ambiguity. Use existing assumptions before inventing new ones; absent evidence defaults to no added platform, no release, and an explicit CI waiver rather than a silent support claim.
  4. Update the active profile record to `software, computational` and `software.deploys = false` only where the current plan lacks that record; otherwise preserve the existing values and explain any difference as D-901.
  5. Add this addendum to HANDOFF reading order immediately after HANDOFF and before the base PLAN. State that later protocol amendments take precedence only where the gap ledger says so.
- Outputs: `plan/ADDENDUM-V3.9-GAP.md`, `plan/DECISIONS.md`, `plan/PROFILE.md`, `HANDOFF.md`
- Evidence produced: upgrade evidence in `ADDENDUM-V3.9-STATUS.md`; no product-result evidence
- Done when: the gap table matches this addendum, D-900 forbids product rescope, and every planning root points to the addendum; every output exists, is non-empty, and the step entry records commit hash and verification result
- Checkpoint: record this step, outputs and SHA-256 hashes in `.checkpoints/state.json`
- On failure: record the failure and any correction as a lesson; use the most reversible fix; halt in `BLOCKED.md` if frozen behaviour, security, data integrity, public interfaces or an external action would change
- Gate: none
- Relevant decisions/claims: D-900
- Lessons applied: none
- Exclusive resources: the repository working branch and `.checkpoints/state.json`

### S-ADD-010 Lessons, knowledge, R0 prior-knowledge loading, retrospective and closing store push
- Tier: Sonnet
- Profile: software (and computational where already active)
- Depends on: S-ADD-001
- Inputs: current planning branch, this addendum, protocol v3.9 at `ef99285404419153dd5f9aaae14f14bb53604a62`, and the files named by each action
- Actions:
  1. Create or complete `research/PRIOR_KNOWLEDGE.md`, `lessons/`, and `knowledge/`. Read active `applies:all` plus matching domain/technology entries from qualitycoding/Lessons and qualitycoding/knowledge; record commit SHAs, prevention rules and exact application points.
  2. Add Rules 11, 12 and 13 verbatim to HANDOFF. Require one file per correction/fact, the Appendix A/B schemas, secret redaction, recurrence handling and immediate commits.
  3. Add or complete S-RETRO and S-KNOW as the final product-plan steps. Ensure every correction becomes a lesson and every new subject fact becomes a knowledge item before closing.
  4. Add G-003 to `plan/GATES.md` if absent, with `push-to-proposed`, `push-to:` and `do-not-push` responses and bundle fallback. Do not execute the product closing steps during this amendment.
- Outputs: `research/PRIOR_KNOWLEDGE.md`, `lessons/`, `knowledge/`, HANDOFF, PLAN and GATES updates
- Evidence produced: upgrade evidence in `ADDENDUM-V3.9-STATUS.md`; no product-result evidence
- Done when: R0 sources and applications are recorded, HANDOFF contains the persistence rules, and PLAN ends with S-RETRO then S-KNOW; every output exists, is non-empty, and the step entry records commit hash and verification result
- Checkpoint: record this step, outputs and SHA-256 hashes in `.checkpoints/state.json`
- On failure: record the failure and any correction as a lesson; use the most reversible fix; halt in `BLOCKED.md` if frozen behaviour, security, data integrity, public interfaces or an external action would change
- Gate: none
- Relevant decisions/claims: D-900–D-980 as assigned
- Lessons applied: all matching R0 lesson IDs
- Exclusive resources: the repository working branch and `.checkpoints/state.json`

### S-ADD-020 Periodic full-context checkpoints and zero-context resumption
- Tier: Sonnet
- Profile: software (and computational where already active)
- Depends on: S-ADD-010
- Inputs: current planning branch, this addendum, protocol v3.9 at `ef99285404419153dd5f9aaae14f14bb53604a62`, and the files named by each action
- Actions:
  1. Upgrade `.checkpoints/state.json` to the complete v3.9 schema: run/branch/sequence, profiles, phase pointers, completed/pending/N/A, artifact hashes, execution_context, active_variables, tier substitutions, open items, store status and UTC timestamp.
  2. Add atomic write/rename and snapshots under `.checkpoints/snapshots/`; checkpoint every three planner actions or three minutes, before external operations, and at every sub-phase boundary.
  3. Write the zero-context resume procedure in HANDOFF: select newest valid state/snapshot, verify every artifact hash, restore variables/context, reconcile in-flight side effects and continue idempotently.
  4. Reconcile existing legacy checkpoint files by reference; do not delete their history.
- Outputs: `.checkpoints/state.json`, snapshot directory and HANDOFF resume procedure
- Evidence produced: upgrade evidence in `ADDENDUM-V3.9-STATUS.md`; no product-result evidence
- Done when: state parses as JSON, contains every v3.9 field, hashes all amendment artifacts, and a dry resume identifies exactly one next step; every output exists, is non-empty, and the step entry records commit hash and verification result
- Checkpoint: record this step, outputs and SHA-256 hashes in `.checkpoints/state.json`
- On failure: record the failure and any correction as a lesson; use the most reversible fix; halt in `BLOCKED.md` if frozen behaviour, security, data integrity, public interfaces or an external action would change
- Gate: none
- Relevant decisions/claims: D-900–D-980 as assigned
- Lessons applied: all matching R0 lesson IDs
- Exclusive resources: the repository working branch and `.checkpoints/state.json`

### S-ADD-030 Assumption/premise audit plus traditional, adversarial and optional mutation review steps
- Tier: Sonnet
- Profile: software (and computational where already active)
- Depends on: S-ADD-020
- Inputs: current planning branch, this addendum, protocol v3.9 at `ef99285404419153dd5f9aaae14f14bb53604a62`, and the files named by each action
- Actions:
  1. Create `premortem/ASSUMPTION_AUDIT.md`. Audit every A-ID, premise behind every D-ID, every single-source/inferred claim and every frozen-test premise; assign confirmed, falsified or unverifiable with checked evidence.
  2. For each falsified load-bearing item, add the required lesson and corrected knowledge item, trace dependants, amend from the earliest affected phase, re-freeze only affected tests, and rerun cold read plus one pre-mortem round. Convert unverifiable items into mitigated risks.
  3. Ensure the product PLAN contains S-REVIEW then S-ADVERSARIAL after implementation and before packaging/release/S-RETRO. Create `reviews/CODE_REVIEW.md` and `reviews/ADVERSARIAL_REVIEW.md` as honest pending templates if reviews have not run; never mark them passed.
  4. Ensure S-MUTATE exists or a reasoned opt-out D-ID exists; the later v3.9 step defines its CI design.
- Outputs: assumption audit, risk/lesson/knowledge amendments, review templates and PLAN review steps
- Evidence produced: upgrade evidence in `ADDENDUM-V3.9-STATUS.md`; no product-result evidence
- Done when: every in-scope premise has a verdict, falsified items link to L/K/D records, and both review steps and artifacts are present without false pass claims; every output exists, is non-empty, and the step entry records commit hash and verification result
- Checkpoint: record this step, outputs and SHA-256 hashes in `.checkpoints/state.json`
- On failure: record the failure and any correction as a lesson; use the most reversible fix; halt in `BLOCKED.md` if frozen behaviour, security, data integrity, public interfaces or an external action would change
- Gate: none
- Relevant decisions/claims: D-900–D-980 as assigned
- Lessons applied: all matching R0 lesson IDs
- Exclusive resources: the repository working branch and `.checkpoints/state.json`

### S-ADD-040 Honest verification, command/pin evidence and complete deviation logging
- Tier: Sonnet
- Profile: software (and computational where already active)
- Depends on: S-ADD-030
- Inputs: current planning branch, this addendum, protocol v3.9 at `ef99285404419153dd5f9aaae14f14bb53604a62`, and the files named by each action
- Actions:
  1. Inspect every plan-specified command and CI workflow. Remove failure-swallowing idioms on load-bearing checks; bounded retries must end in visible failure. Add artifact existence and parse/sanity post-conditions to every producing step.
  2. Make every measurable budget gating. Name runner OS/CPU/core class for performance targets, bootstrap missing regression baselines, and require human sign-off before relaxing a target.
  3. Build a command/pin evidence matrix in `plan/ENVIRONMENT.md`: every pin has a registry/tag existence result and every literal command/flag/config has a committed spike output or a locator in the pinned version documentation.
  4. Create or complete `DEVIATIONS.md`; log every substitution when made. Add the retrospective diff against ENVIRONMENT so unexplained manifest/workflow differences fail closure.
- Outputs: ENVIRONMENT evidence matrix, corrected PLAN/workflows, DEVIATIONS and post-condition checks
- Evidence produced: upgrade evidence in `ADDENDUM-V3.9-STATUS.md`; no product-result evidence
- Done when: no load-bearing swallowed failure remains, all artifacts/budgets gate, and every command and pin has version-matched evidence; every output exists, is non-empty, and the step entry records commit hash and verification result
- Checkpoint: record this step, outputs and SHA-256 hashes in `.checkpoints/state.json`
- On failure: record the failure and any correction as a lesson; use the most reversible fix; halt in `BLOCKED.md` if frozen behaviour, security, data integrity, public interfaces or an external action would change
- Gate: none
- Relevant decisions/claims: D-900–D-980 as assigned
- Lessons applied: all matching R0 lesson IDs
- Exclusive resources: the repository working branch and `.checkpoints/state.json`

### S-ADD-050 Environment-class probe, ephemeral protections and immediate push verification
- Tier: Sonnet
- Profile: software (and computational where already active)
- Depends on: S-ADD-040
- Inputs: current planning branch, this addendum, protocol v3.9 at `ef99285404419153dd5f9aaae14f14bb53604a62`, and the files named by each action
- Actions:
  1. Record the environment-class probe and evidence in state and ENVIRONMENT; unknown Kimi-local environments default to ephemeral.
  2. Add durable-mount, turn-start restaging/exec-bit restoration, resumable checkpointed builds and dependency-ordering invariants to all applicable long steps.
  3. Right-size local steps to minutes. Move heavy instrumented builds, benchmarks and fuzzing to named CI jobs; mark locally impossible dimensions blocked with the CI equivalent, never green.
  4. Restate Global Rule 6 in HANDOFF: commit and push after every successful chunk on all platforms, then compare local HEAD with `git ls-remote`; retry transient failures with backoff and report any mismatch.
- Outputs: ENVIRONMENT/HANDOFF ephemeral rules, resilient PLAN steps and push-verification rule
- Evidence produced: upgrade evidence in `ADDENDUM-V3.9-STATUS.md`; no product-result evidence
- Done when: every long step names checkpoint/resume/durable state and every successful chunk has an immediate push plus remote-SHA verification; every output exists, is non-empty, and the step entry records commit hash and verification result
- Checkpoint: record this step, outputs and SHA-256 hashes in `.checkpoints/state.json`
- On failure: record the failure and any correction as a lesson; use the most reversible fix; halt in `BLOCKED.md` if frozen behaviour, security, data integrity, public interfaces or an external action would change
- Gate: none
- Relevant decisions/claims: D-900–D-980 as assigned
- Lessons applied: all matching R0 lesson IDs
- Exclusive resources: the repository working branch and `.checkpoints/state.json`

### S-ADD-060 Separate Lessons and knowledge stores throughout intake, checkpoints and G-003
- Tier: Sonnet
- Profile: software (and computational where already active)
- Depends on: S-ADD-050
- Inputs: current planning branch, this addendum, protocol v3.9 at `ef99285404419153dd5f9aaae14f14bb53604a62`, and the files named by each action
- Actions:
  1. Set the lesson store to `qualitycoding/Lessons` and knowledge store to `qualitycoding/knowledge` in intake assumptions and `.checkpoints/state.json`; record each commit read and readability separately.
  2. Update R0/PRIOR_KNOWLEDGE to read lessons from Lessons and facts from knowledge using their respective matching rules.
  3. Update G-003/S-KNOW to push the two collections separately, regenerate each store index, retry moved remotes three times per store, and create separate bundle fallbacks.
  4. Update HANDOFF and retrospective wording so no step treats lessons and knowledge as one combined destination.
- Outputs: ASSUMPTIONS, state, PRIOR_KNOWLEDGE, GATES, PLAN and HANDOFF split-store amendments
- Evidence produced: upgrade evidence in `ADDENDUM-V3.9-STATUS.md`; no product-result evidence
- Done when: both stores are named independently everywhere and G-003 has separate push, index, retry and fallback paths; every output exists, is non-empty, and the step entry records commit hash and verification result
- Checkpoint: record this step, outputs and SHA-256 hashes in `.checkpoints/state.json`
- On failure: record the failure and any correction as a lesson; use the most reversible fix; halt in `BLOCKED.md` if frozen behaviour, security, data integrity, public interfaces or an external action would change
- Gate: none
- Relevant decisions/claims: D-900–D-980 as assigned
- Lessons applied: all matching R0 lesson IDs
- Exclusive resources: the repository working branch and `.checkpoints/state.json`

### S-ADD-070 Intake-derived versioned CI platform matrix with explicit coverage or waivers
- Tier: Sonnet
- Profile: software (and computational where already active)
- Depends on: S-ADD-060
- Inputs: current planning branch, this addendum, protocol v3.9 at `ef99285404419153dd5f9aaae14f14bb53604a62`, and the files named by each action
- Actions:
  1. Write the repository CI matrix decision as D-970: Rust audio application: ubuntu-24.04, windows-2022, macos-15; ubuntu-24.04-arm if claimed; alpine for portable crates; oldest-supported Rust (MSRV) job; hardware tests remain explicitly separated.
  2. Map every intake-declared OS, architecture and runtime bound to a CI job in `plan/TRACEABILITY.md`, or to a specific D-ID waiver. Use `fail-fast: false`; do not use `-latest` runner labels.
  3. Existence-check every runner label, action and tool pin at planning time and record the evidence in ENVIRONMENT/SOURCES. Add ARM64, libc-variant and oldest-supported-toolchain jobs by default or explicit waivers.
  4. Keep mutation out of the platform matrix. Ensure CI checks cannot swallow failures and assert all expected reports/artifacts.
- Outputs: D-970, ENVIRONMENT CI matrix, TRACEABILITY platform map and workflow/plan updates
- Evidence produced: upgrade evidence in `ADDENDUM-V3.9-STATUS.md`; no product-result evidence
- Done when: every supported platform maps to a versioned job or waiver, fail-fast is false, and all labels/pins have existence evidence; every output exists, is non-empty, and the step entry records commit hash and verification result
- Checkpoint: record this step, outputs and SHA-256 hashes in `.checkpoints/state.json`
- On failure: record the failure and any correction as a lesson; use the most reversible fix; halt in `BLOCKED.md` if frozen behaviour, security, data integrity, public interfaces or an external action would change
- Gate: none
- Relevant decisions/claims: D-900–D-980 as assigned
- Lessons applied: all matching R0 lesson IDs
- Exclusive resources: the repository working branch and `.checkpoints/state.json`

### S-ADD-080 Priced three-tier mutation CI with one aggregator and a report-based score gate
- Tier: Sonnet
- Profile: software (and computational where already active)
- Depends on: S-ADD-070
- Inputs: current planning branch, this addendum, protocol v3.9 at `ef99285404419153dd5f9aaae14f14bb53604a62`, and the files named by each action
- Actions:
  1. Record D-980 tool/scope decision: cargo-mutants for Rust. Pin the tool and config after existence checks; scope all mutable production modules whose correctness is primarily protected by tests.
  2. Measure and commit baseline fast-test time, generated mutant count and estimated full/incremental job time in `reviews/MUTATION.md`. Budgets: 20 minutes for incremental CI and 120 minutes for scheduled full CI; if full exceeds budget, shard deterministically by module rather than silently dropping mutants.
  3. Use three triggers: changed-file incremental runs on pull requests and default-branch pushes; a scheduled full run with `qualitycoding` as named owner and failure issue/notification; and workflow_dispatch. Changes to killing tests or mutation config force a full run. No changed mutable files yields explicit `not-applicable`.
  4. Create one always-running aggregator as the only required mutation status. It fails unless the mutation job passed or was legitimately not applicable; never use paths filtering to skip a required workflow.
  5. Compute score from the tool report, not exit status. Require a non-empty parseable report and at least 10 mutants; if the whole scope yields fewer than 10, require at least 1 and record the small-scope D-ID. Demonstrate once that a below-floor fixture fails.
  6. After the first real full run, set the floor five percentage points below the measured score (not below 0). Do not weaken it later without a human gate. List the fast tests and every omission as a D-ID; convert survivors into tests or written class-grouped justifications.
- Outputs: D-980, pinned mutation config/workflow, reviews/MUTATION.md, aggregator and score-gate evidence
- Evidence produced: upgrade evidence in `ADDENDUM-V3.9-STATUS.md`; no product-result evidence
- Done when: pricing and mutant count are recorded, all three triggers plus aggregator exist, the report gate has post-conditions and a proven failing case, and the floor follows the first-run rule; every output exists, is non-empty, and the step entry records commit hash and verification result
- Checkpoint: record this step, outputs and SHA-256 hashes in `.checkpoints/state.json`
- On failure: record the failure and any correction as a lesson; use the most reversible fix; halt in `BLOCKED.md` if frozen behaviour, security, data integrity, public interfaces or an external action would change
- Gate: none
- Relevant decisions/claims: D-900–D-980 as assigned
- Lessons applied: all matching R0 lesson IDs
- Exclusive resources: the repository working branch and `.checkpoints/state.json`

### S-ADD-090 Re-freeze traceability and run independent plan review
- Tier: Sonnet
- Profile: software (and computational where already active)
- Depends on: S-ADD-080
- Inputs: current planning branch, this addendum, protocol v3.9 at `ef99285404419153dd5f9aaae14f14bb53604a62`, and the files named by each action
- Actions:
  1. Update TRACEABILITY so every new protocol requirement maps to its artifact, step and verification. Update the PLAN status header to protocol v3.9 and include the addendum in the normative plan.
  2. Run the semantic cold read with a fresh-context reviewer who did not author the amendment and a separate mechanical reviewer. Resolve every question in artifacts; if fresh agents are unavailable, record that substitution as a D-ID and High risk rather than claiming independence.
  3. Run one pre-mortem round over all changed steps, including CI platform coverage, mutation cost/gating, store separation, integrity and resume. Repeat to zero Critical/High, maximum five rounds; otherwise mark BLOCKED — HUMAN DECISION REQUIRED.
  4. Recompute FROZEN_MANIFEST only if an amendment legitimately changed a frozen artifact; never change expected product behaviour to make the upgrade pass. Record the reason and amendment D-ID.
  5. From the repository root run `python3 /path/to/skills/protocols/v39_addendum_verify.py final --repo . --plan-root plan` after replacing `/path/to/skills` with the actual skills clone. For Stream-of-consonance-vst run it again with `--plan-root vst/plan`.
- Outputs: TRACEABILITY, PLAN status, cold-read and pre-mortem records, updated manifest only when required, verifier output
- Evidence produced: upgrade evidence in `ADDENDUM-V3.9-STATUS.md`; no product-result evidence
- Done when: both reviews return zero unresolved items or an honest blocked status, traceability is complete, and the mechanical verifier exits 0; every output exists, is non-empty, and the step entry records commit hash and verification result
- Checkpoint: record this step, outputs and SHA-256 hashes in `.checkpoints/state.json`
- On failure: record the failure and any correction as a lesson; use the most reversible fix; halt in `BLOCKED.md` if frozen behaviour, security, data integrity, public interfaces or an external action would change
- Gate: none
- Relevant decisions/claims: D-900
- Lessons applied: none
- Exclusive resources: the repository working branch and `.checkpoints/state.json`

### S-ADD-100 Close the amendment and persist learning
- Tier: Sonnet
- Profile: software (and computational where already active)
- Depends on: S-ADD-090
- Inputs: current planning branch, this addendum, protocol v3.9 at `ef99285404419153dd5f9aaae14f14bb53604a62`, and the files named by each action
- Actions:
  1. Write `ADDENDUM-RETROSPECTIVE.md` from EXECUTION_LOG, DEVIATIONS, gate/challenge records, reviews and the final diff. Confirm every substitution and correction has its required record.
  2. Mark each applicable row in ADDENDUM-V3.9-STATUS.md `pass` with commit and evidence; no `pending`, `fail` or unapproved `blocked` row may remain in a READY amendment.
  3. Commit and push the implementation branch, verify the remote SHA equals local HEAD, and hand the branch/SHA back to the central controller. Do not merge, release, deploy or execute product plan steps.
  4. At G-003 request separate Lessons/knowledge destinations. Push only after sign-off; otherwise use the documented bundle fallback. This gate holds learning-store publication, not completion of the repository amendment.
- Outputs: retrospective, completed status log, remote branch and G-003 record
- Evidence produced: upgrade evidence in `ADDENDUM-V3.9-STATUS.md`; no product-result evidence
- Done when: all applicable steps pass, remote matches local, no product execution occurred, and G-003 is recorded; every output exists, is non-empty, and the step entry records commit hash and verification result
- Checkpoint: record this step, outputs and SHA-256 hashes in `.checkpoints/state.json`
- On failure: record the failure and any correction as a lesson; use the most reversible fix; halt in `BLOCKED.md` if frozen behaviour, security, data integrity, public interfaces or an external action would change
- Gate: none
- Relevant decisions/claims: D-900
- Lessons applied: none
- Exclusive resources: the repository working branch and `.checkpoints/state.json`


## Completion

The amendment is complete only when S-ADD-100 has a verified remote commit, the central verifier passes for every planning root, all new Critical/High risks are resolved, and the controller row is updated. G-003 may still be awaiting permission to publish learning stores; that does not authorise product implementation.
