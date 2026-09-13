# Permanent silent-misselection regressions

Every discovered Stage-4 `SILENT_WRONG` case becomes a minimized permanent
regression here. Use one `<id>.json` record plus the smallest referenced model
and perturbation files needed to reproduce it.

Each JSON record must contain:

- `schema_version: 1`;
- stable `id`;
- `model` — path to the minimized baseline/model input;
- `perturbation` — path or precise description of the upstream edit;
- `intended_target` — human/semantic description of what must be tracked;
- `actual_outcome` — what the defective implementation selected/reported;
- `failure_classification: "SILENT_WRONG"`;
- non-empty `evidence` — candidates, lineage/query evidence, diagnostics, or
  other data sufficient to reconstruct why the selection was wrong.

`scripts/ci/semantic_ref_harness.py validate` checks every JSON record in this
directory. Resolver-specific replay is added by AICAD-080+ once the real
resolver interface exists; no fake resolver API is introduced by AICAD-079C.
