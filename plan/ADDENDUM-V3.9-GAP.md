# Addendum v3.9 gap ledger

**Repository:** `qualitycoding/Rawksmyth`
**Baseline:** `v3.1`  **Target:** `v3.9`
**Audited source commit:** `9d43955e82cccb0cece7162b193dfddd0ab32284`
**Source branch tip at implementation:** `d7fce2f2834159130f45bc5d53f0c9b6812654bb` (addendum publication commit on top of the audited commit; deviation D-901)

## Gap table (from ADDENDUM-PLAN-V3.9.md)

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

Every capability not listed above is treated as present in the baseline and is
re-verified by the final mechanical consistency check. Product scope, public
interfaces, frozen expected behaviour, performance targets and human gates
are unchanged (D-900).
