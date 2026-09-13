# Repository layout

The repository contains both active implementation and intentionally retained development history. Keep those roles separate when navigating it.

## Runtime implementation

### Compiler/language crates

- `crates/cad-lexer` — lexical analysis.
- `crates/cad-parser` — parser and syntax diagnostics.
- `crates/cad-ast` — source-level AST/span structures.
- `crates/cad-hir` — lowering, bindings, HIR, type checking, standard types, Safe CAD catalogue, sketch semantic model.
- `crates/cad-types` / `crates/cad-units` — primitive/engineering dimensions and unit semantics.
- `crates/cad-runtime` — HIR interpretation, parameter model, values, runtime budgets, RuntimeBuiltin execution.
- `crates/cad-compiler` / `crates/cad-diagnostics` — compiler orchestration/support and stable diagnostic representation.

### Parametric and constraint crates

- `crates/cad-feature-graph` — modeling feature DAG, cache keys, dirty propagation, source provenance.
- `crates/cad-constraints` — sketch-constraint IR, solver abstraction/implementation, solved-value application.

`crates/cad-references` and `crates/cad-query` exist in the workspace as roadmap-owned crates, but Stage-4 semantic-reference implementation has not begun on this transition branch. Their presence must not be mistaken for a completed user feature.

### Geometry/kernel crates

- `crates/cad-geometry-api` — backend-independent Geometry IR and graph.
- `crates/cad-geometry-runtime` — Geometry IR realization, sketch/profile lowering, full/incremental graph dispatch.
- `crates/cad-kernel-api` — kernel-neutral geometry values/operation contract.
- `crates/cad-occt-bridge` — Rust-side OCCT adapter and native build integration.
- `crates/cad-validation` — geometric comparison/validation policy support.
- `native/occt_bridge` — only C++ layer that includes OCCT implementation headers.

### Tooling

- `crates/cad-cli` — current `cad build` command plus in-process `ParametricBuildSession` library orchestration.
- `crates/cad-lsp`, `crates/cad-agent-tools`, and several later-stage domain crates are workspace-reserved for future stages; inspect actual source before assuming capability.

## Language and architecture references

- `specs/` — specification artifacts according to each file's status.
- `rfcs/` — architecture/language RFCs.
- `docs/developer/` — current implementation documentation.
- `docs/user/` — current user-facing documentation.
- `docs/plan/` — frozen original plan bundle retained at its historical path. It is not the current manual.

## Development control/history

- `project/CURRENT_STAGE.md` — active transition/stage state.
- `project/TASKS.yaml` — bounded task queue through AICAD-100.
- `project/DECISION_LOG.md` — owner-approved decisions and stage approvals.
- `project/OWNER_DECISIONS.md` — unresolved/escalated owner questions.
- `project/reports/archive/` — completed Stage-0..3 task evidence.
- `project/gates/archive/` — completed gate/checkpoint evidence.
- `project/planning/` — transition records and future planning/audits.

Routine development should load current code and current docs first. Retrieve archived reports when a specific implementation claim or historical decision needs evidence; do not recursively treat the archive as startup context.
