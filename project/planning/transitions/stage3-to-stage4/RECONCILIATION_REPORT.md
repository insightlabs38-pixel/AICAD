# Stage 3 → Stage 4 reconciliation report

Status: **COMPLETE FOR OWNER REVIEW** on `claude/aicad-stage4-transition`.

This is an information-architecture, documentation, governance-record, and historical-evidence reconciliation only. It does **not** initialize Stage-4 implementation and does not implement AICAD-080.

## 1. Frozen source and branch provenance

The transition branch was created directly from the owner-directed Stage-3 `main` head:

- repository: `insightlabs38-pixel/AICAD`
- source branch: `main`
- exact source commit: `15fc5a37e4382de717e426ccc5317a491be264cd`
- source commit type: Stage-3 merge commit
- final Stage-3 incremental-remediation parent included by that merge: `99fb0d3b17000b0a1c6a1a3c175ea16f0d450180`
- transition branch: `claude/aicad-stage4-transition`

The final remediation commit records the production ParamModel/FeatureGraph incremental-rebuild integration repair and the final Stage-3 test rerun. No transition commit rewrites that implementation lineage.

## 2. Source branches/artifacts inspected

### `planning/aicad-post100-audit`

Present and inspected. Source tip at reconciliation time:

`fcf5038161b696714ab6dda8d41fa40d7d85e9d0`

The audit was originally performed against `claude/aicad-stage3-dev` at `d82bf82e53a98f3117d3dc17707f1756822e593d` (AICAD-074). Its 14 files were imported **without rewriting their contents** into:

`project/planning/roadmap/post100/`

The imported README still states the original branch/revision and clearly says the audit is planning-only/non-normative. None of the provisional Stage-5/6/7 task IDs were copied into `project/TASKS.yaml`; no AICAD-101+ IDs were assigned; no audit owner decision was resolved.

### `claude/aicad-docs-dev`

The requested branch was not present in the connected repository branch set, and no matching PR source branch was available. The transition therefore does not invent or attribute documentation to that branch. Current documentation was reconciled from owner-approved Stage-3 `main` and verified implementation surfaces. This absence is also recorded in `DEFERRED_TRANSITION_ITEMS.md`.

## 3. Transition planning structure established

Created:

- `project/planning/README.md`
- `project/planning/transitions/stage3-to-stage4/README.md`
- `project/planning/transitions/stage3-to-stage4/RECONCILIATION_REPORT.md`
- `project/planning/transitions/stage3-to-stage4/DEFERRED_TRANSITION_ITEMS.md`
- `project/planning/roadmap/post100/` (frozen imported audit)

The planning index explicitly separates internal planning from user/developer documentation.

`docs/plan/` remains at its legacy location in this pass. It is now explicitly classified by current indexes as frozen internal/foundation planning, not normal user/developer documentation. Physical relocation was deferred because Stage-0..3 task metadata, reports, gates, code comments, and RFC-era evidence refer to those paths extensively; moving it now would create broad path-only churn and risk historical archaeology for little active-context benefit.

## 4. Completed development evidence archived without deletion

### Reports

Original Stage-0..3 report blobs were preserved under:

- `project/reports/archive/stage0/`
- `project/reports/archive/stage1/`
- `project/reports/archive/stage2/`
- `project/reports/archive/stage3/`

This includes:

- orientation evidence;
- AICAD-001..014 (Stage 0);
- AICAD-015..037 (Stage 1);
- AICAD-038..064 plus Stage-2 subtasks such as 057A..057F (Stage 2);
- AICAD-064A..079B plus the final incremental-remediation report (Stage 3);
- Stage-0 and Stage-1 independent review documents.

The archive uses the original blob SHA for each historical document. Example verification performed during this pass: archived `AICAD-050.md` retains original blob `fbeb71e7233d62f296ed7318fa4fc6fed9a688f6`.

To avoid breaking the many frozen `project/TASKS.yaml` and historical-document references, former `project/reports/<name>` paths are retained as small compatibility pointers directing readers to the corresponding archive stage. This intentionally avoids rewriting hundreds of historical references while moving the evidence itself out of routine context.

### Gates

Original completed gate/checkpoint blobs were preserved under:

- `project/gates/archive/stage0/`
- `project/gates/archive/stage1/`
- `project/gates/archive/stage2/`
- `project/gates/archive/stage3/`

Former completed-gate paths use compatibility pointers. Two transition-relevant gate surfaces remain directly available:

- `project/gates/stage-0-4-traceability-matrix.md` — still spans active Stage-4 traceability;
- `project/gates/stage-3-gate.md` — retained directly for the immediate Stage-3→4 transition, with an exact archived snapshot also under `archive/stage3/`.

Archival indexes were added under both reports and gates. Archiving does not change recommendations or confer owner approval.

## 5. Documentation classification and reconciliation

The repository now has three explicit documentation classes.

### User documentation

Current user documentation is under `docs/user/`:

- `README.md`
- `getting-started/README.md`
- `getting-started/installation.md`
- `getting-started/first-part.md`
- `language/README.md`
- `language/types-and-units.md`
- `language/functions-and-control-flow.md`
- `language/parameters.md`
- `modeling/README.md`
- `modeling/primitives-and-booleans.md`
- `modeling/features.md`
- `modeling/transforms-and-patterns.md`
- `modeling/sketches-and-constraints.md`
- `cli/README.md`
- `examples/README.md`
- `troubleshooting/README.md`

These documents describe the implemented Stage-3 surface only. Stage-4 semantic references are explicitly described as future work, not as available functionality.

The current modeling pages are grounded in the final Stage-3 runtime catalogue (17 runtime-backed modeling functions), and the CLI page is grounded in `crates/cad-cli/src/main.rs`/`Cargo.toml` (`cad build <path.aicad> [--json] [--output <path>] [--name <binding>[.<field>]]`).

### Developer documentation

Current developer/contributor documentation is under `docs/developer/`:

- `README.md`
- `architecture/README.md`
- `architecture/system-overview.md`
- `architecture/repository-layout.md`
- `compiler-runtime/README.md`
- `compiler-runtime/frontend.md`
- `compiler-runtime/hir-and-typechecking.md`
- `compiler-runtime/runtime.md`
- `geometry/README.md`
- `geometry/geometry-ir.md`
- `geometry/safe-cad-api.md`
- `kernel/README.md`
- `kernel/occt-boundary.md`
- `parametrics/README.md`
- `parametrics/parameters-and-feature-dag.md`
- `parametrics/incremental-rebuild.md`
- `constraints/README.md`
- `testing/README.md`
- `contributing/README.md`

The existing detailed Safe CAD API was moved intact (same blob `0abb7d30ea104407c80115d2c1b6ef8c31dd0450`) into `docs/developer/geometry/safe-cad-api.md`. The old `docs/API/safe-cad-api.md` path remains a compatibility pointer.

Developer docs describe the current layer boundaries rather than task chronology: frontend/HIR/runtime, feature DAG/incremental model, backend-neutral Geometry IR/runtime, kernel-neutral adapter, OCCT bridge, validation, and sketch constraint architecture.

### Internal development/history

Internal development material is under `project/`: active governance at top-level plus archived reports/gates, transition records, and roadmap audits.

The pre-transition placeholder READMEs from `docs/API`, `docs/architecture`, `docs/language`, and `docs/package-authoring` were preserved under:

`project/planning/transitions/stage3-to-stage4/legacy-docs/`

Their former locations now point readers to current docs or explicitly mark future package authoring as not an implemented Stage-3 feature.

`docs/site/` remains a legacy empty scaffold and is explicitly identified by `docs/README.md` as non-authoritative rather than being silently deleted.

## 6. Navigation and agent-context hygiene

Created/updated concise navigation at:

- repository `README.md`;
- `docs/README.md`;
- `docs/user/README.md`;
- `docs/developer/README.md`;
- `project/planning/README.md`;
- `project/reports/archive/README.md`;
- `project/gates/archive/README.md`;
- `project/gates/README.md`.

The root README no longer claims the repository is an unimplemented Stage-0 skeleton.

`AGENTS.md` received only a minimal organizational/context-hygiene clarification: routine agents should not recursively load archived reports/gates, the post-100 audit, or old implementation diaries. Historical evidence is retrieved when a task/regression/gate/architecture question actually needs it. No mission/non-negotiable/stage-gate semantic policy was weakened.

## 7. Reference/path strategy

Rather than rewriting historical evidence, this pass preserves old high-volume report/gate paths as compatibility pointers. Therefore:

- existing `project/TASKS.yaml` report paths remain navigable;
- frozen historical documents that cite old report/gate paths do not become hard dead links;
- original historical contents live under per-stage archives;
- current docs/indexes point directly to the new user/developer/archive/planning locations.

The old Safe CAD API path also remains a compatibility pointer to its new developer-doc location.

This strategy avoids changing normative/historical content merely to maintain navigation.

## 8. Conflicts and stale state found

### Governance-state mismatch — resolved during the documentation second pass

At transition startup, the owner-provided transition directive stated that Stage 3 was completed, owner-approved, and merged to `main`. The code lineage confirmed the Stage-3 merge and included the final remediation parent. `project/TASKS.yaml` also marked AICAD-079B `done` and left AICAD-080+ `todo`.

At that point, `project/CURRENT_STAGE.md`, `project/SESSION_HANDOFF.md`, and the recorded decision log still represented the pre-approval Stage-3 state. The documentation second pass reconciled that already-issued approval without inventing a new architecture decision:

- DL-22 records the owner-issued Stage-3 approval;
- the DL-22 commit preserved DL-1 through DL-21 and appended the approval record;
- `project/CURRENT_STAGE.md` now records the active Stage-3 → Stage-4 transition rather than claiming Stage 3 still awaits approval;
- `project/SESSION_HANDOFF.md` points to the transition branch while keeping AICAD-080 explicitly todo.

This record correction does **not** initialize Stage-4 implementation and does not authorize AICAD-080. The former governance mismatch is therefore removed from the deferred-work queue.

### Canonical language-spec completeness

The post-100 audit notes missing claimed language-spec artifacts (semantics/types/diagnostics). This is substantive normative cleanup and remains deferred; no spec file was created/reinterpreted here.

### Legacy foundation-plan location

`docs/plan/` is semantically internal planning but physically remains under `docs/` for reference-preservation reasons described above. Current navigation makes the classification explicit.

## 9. Commit structure

First reconciliation pass:

1. `f5dfa39ab197b819f907150bd88d40b09411ccac` — transition scaffold;
2. `e88cfeca663e1ab04c54c22fd9c10ab7efd7f11b` — frozen post-100 planning import;
3. `dbefe18d4352d55d75b2a8d9ab79eb599ef52713` — archive Stage-0/1 development evidence;
4. `62c0eb8ade117ceb8fd3982f6ab2253deb97ea68` — archive Stage-2/3 development evidence;
5. `b40713ab4e9a515b43e44a99d2db89bfa46b0dd0` — archive completed Stage-0..3 gate evidence;
6. `acd02671816ff9fe6a5b4c67d6739d07be7e8242` — establish user/developer documentation split;
7. `570c91ed77df8112dc5da91f6d5577b10dfa9c3b` — clarify routine agent context hygiene;
8. `380c25f4bf095f63e17a9872d912037d1d7ff355` — finalize first-pass reconciliation report.

Documentation/governance second pass:

9. `810ab245fde3c4df948df233371b2d5b152d0de7` — replace the root README with the current post-Stage-3 landing page;
10. `4c615633f107919c37d2d948ed3bfe4e85b47f99` — populate the current Stage-3 user guide;
11. `5dd74c9a627794012f523872e1e1f0468e45f93f` — populate the current developer architecture manual;
12. `59781eaf275c4b0b792954a1fc37892700d903b8` — record the already-issued Stage-3 approval as DL-22 while preserving prior decision history;
13. `0f03f06754c34396aaf53cbe4a26082620992ad6` — align current transition state and handoff after Stage-3 approval.

The final documentation-correctness/bookkeeping remediation is intentionally one coherent commit after `0f03f067...`.

All commits use repository identity `insightlabs38-pixel <insightlabs38@gmail.com>` as shown by GitHub commit metadata. No AI/session attribution metadata was added.

## 10. Validation performed

### Structural / scope validation

The first-pass remote compare of exact Stage-3 base `15fc5a37e4382de717e426ccc5317a491be264cd` to the then-current transition branch verified that the archival/reorganization work was strictly ahead of the correct merge base and did not change production/spec/CI/task/governance files.

The later documentation/governance pass intentionally changed only current documentation plus the three governance records described in Section 8. The final documentation-correctness remediation changes only current documentation and transition-record files. Across these documentation follow-ups:

- no `crates/` production file changed;
- no `native/` file changed;
- no `specs/` or `rfcs/` file changed;
- no `.github`/CI workflow file changed;
- no `project/TASKS.yaml` or `project/OWNER_DECISIONS.md` content changed;
- no AICAD-080 implementation or semantic-reference resolver/code was introduced;
- no post-100 draft was promoted or made normative.

Spot checks from the first pass confirmed archived documents retain their original blob identities and the post-100 audit README retains source blob `0ba78ef48897a3345d387fc4f2e43de9223ba975` with its AICAD-074 audit basis/non-normative status intact.

### Documentation correctness audit

The current `README.md`, `docs/user/`, and `docs/developer/` population was audited specifically for `sketch`, constraints/profiles, `FaceRef`/`EdgeRef`/`VertexRef`, semantic/topology/persistent references, named outputs, face/edge indices, and selection wording.

The authoritative distinction now documented is:

- Stage 3 **does implement internally** sketch IR/entity identity, solver-independent constraint semantics, numerical solving, solved-profile validation, exact solved-profile-to-face lowering, and profile-driven modeling infrastructure;
- the current `.aicad` parser/compiler/runtime does **not** expose a supported direct `sketch { ... }` (or equivalent) source-authoring construct;
- current source-level `extrude`/`revolve` operate on existing `Geometry` plus raw face indices rather than authored `Sketch`/`Profile` source values;
- Stage 3 has feature identity/provenance, named source/model outputs, operation-local lineage where applicable, and raw/index-based topology selectors;
- those mechanisms are **not** durable `VertexRef` / `EdgeRef` / `WireRef` / `FaceRef` / `ShellRef` / `SolidRef` identity across topology-changing regeneration;
- durable, fail-closed semantic topology-reference resolution remains Stage-4 work.

The audit found no executable user-guide example that relies on unsupported sketch source syntax. Current occurrences of `sketch { ... }` are explicit warnings that the syntax is not supported. The guide's recommended examples were checked against the branch files `examples/brackets/stage3_l_bracket.aicad`, `examples/plates/stage3_bearing_mount.aicad`, and `examples/brackets/stage2_mounting_plate.aicad`; those examples use the implemented Safe CAD source surface and explicitly caveat raw topology indices where used.

### Stage-4 implementation guard

No AICAD-080 implementation, semantic-reference resolver/code, production geometry/kernel behavior, language implementation, normative semantics, or CI expansion was introduced. The Stage-4 benchmark corpus that already existed on `main` was not modified.

### Documentation tooling

No active MkDocs configuration was found. The existing `docs/site/` tree is placeholder scaffolding, not an active documentation build source, so there was no documentation generator/build command to run.

### Rust / local Git commands

The sandbox cannot resolve/reach `github.com`, so it cannot clone/materialize the repository into a complete local checkout. The connected GitHub repository interface was therefore used for authoritative reads/writes and remote diff validation.

No Rust/build-facing path changed in this documentation correctness remediation, so no behavioral test change is required. The final Stage-3 remediation already recorded a clean full workspace suite on the implementation lineage, but this report does **not** claim that historical run as a fresh transition-branch test execution.

Where a checkout-dependent validation cannot be executed truthfully in this environment, it is reported as unavailable rather than inferred from historical results.

## 11. Intentionally deferred work

See `DEFERRED_TRANSITION_ITEMS.md`. Most importantly, this reconciliation does not:

- rewrite normative specs/RFC semantics;
- resolve post-100 audit owner decisions;
- expand Stage-4 CI/CD;
- initialize Stage-4 implementation work;
- implement AICAD-080+;
- assign AICAD-101+ IDs;
- make any post-100 draft normative;
- implement Stage-5+ functionality;
- physically relocate the deeply referenced frozen `docs/plan/` tree.

## 12. Final transition state

The repository now has the intended conceptual separation:

- `docs/user/` — current user-facing documentation;
- `docs/developer/` — current developer/contributor architecture documentation;
- `project/` — active development governance and internal development history;
- `project/reports/archive/` — preserved completed implementation evidence;
- `project/gates/archive/` — preserved completed gate evidence;
- `project/planning/roadmap/post100/` — frozen, non-normative future-roadmap audit/drafts;
- `project/planning/transitions/stage3-to-stage4/` — this reconciliation record;
- `docs/plan/` — explicitly classified legacy-location frozen foundation planning pending any later path-only migration.

Nothing important was intentionally destroyed because it became old. Historical evidence was copied using its original Git blobs into archival destinations; compatibility pointers preserve old high-volume paths. The active documentation surface is now audience-oriented, current feature boundaries are explicit, and the agent startup surface excludes recursive historical reading.

The branch is ready for owner review. **Do not merge automatically. Do not begin CI/CD expansion. Do not begin AICAD-080.**
