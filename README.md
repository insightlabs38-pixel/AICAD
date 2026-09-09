# AICAD

AI-native CAD language, compiler, and geometry runtime. The design source
of truth is the plan bundle imported verbatim into `docs/plan/` (see
`project/reports/AICAD-001.md` for its checksum/version record); this
repository implements it stage by stage under `AGENTS.md` and
`project/CURRENT_STAGE.md`.

## Where to start

- `AGENTS.md` — the implementation-agent constitution (mission,
  non-negotiables, work loop, escalation rules).
- `project/CURRENT_STAGE.md` — the active roadmap stage and its exit gate.
- `project/TASKS.yaml` — the bounded task queue (currently AICAD-001..100,
  covering Stages 0-4).
- `project/OWNER_DECISIONS.md` — unresolved architecture/product decisions
  awaiting owner ruling; do not resolve these implicitly.
- `project/DECISION_LOG.md` — decisions the owner has approved.
- `project/reports/` — one evidence report per completed task
  (`project/reports/<task-id>.md`), plus the plan orientation pass
  (`project/reports/ORIENTATION_PASS.md`).
- `project/gates/` — stage exit-gate evidence packets.
- `docs/plan/` — the immutable, verbatim-imported implementation-plan
  bundle. Treat as read-only design reference; do not edit in place.

## Repository layout

This layout follows `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` §1. Every
top-level directory and crate carries its own `README.md` naming the plan
sections and work package (WP-01..WP-19) that own it — read those before
adding code so ownership boundaries in `docs/plan/22_REPOSITORY_WORK_PACKAGES.md`
stay intact.

```text
crates/            Rust workspace crates (compiler, runtime, geometry, CLI, ...)
native/            Native bridges (OCCT bridge), called only from crates/cad-occt-bridge
std/               Standard packages (core, modeling, fasteners, bearings, threads, gears)
editor/            Human IDE (app shell, viewport, inspectors, sketcher, debugger, diff viewer)
tree-sitter-aicad/ Editor-responsive grammar, kept in sync with crates/cad-parser
skills/            AI skill files (cad-core / cad-geometry / cad-engineering)
examples/          Example source library, by category
tests/             Cross-crate test suites, by concern
benchmarks/        Acceptance benchmarks, including the Stage-4 topology-naming hard gate
specs/             Canonical language grammar/semantics and JSON schemas
docs/              Authored documentation (docs/plan/ is the imported plan; the rest is ours)
project/           Control files: stage, tasks, owner decisions, reports, gates
```

No Rust workspace, native build, or source-level implementation exists yet
at this skeleton stage (Stage 0). Directories are present with ownership
documentation so later tasks land in the right place; see each directory's
`README.md` and `project/TASKS.yaml` for what actually builds them out.

## Git and attribution policy

See `CLAUDE.md`: all commits are authored/committed as
`insightlabs38-pixel <insightlabs38@gmail.com>` with no AI/Claude
attribution, enforced by the repository's Git hooks
(`core.hooksPath`). Do not bypass them.
