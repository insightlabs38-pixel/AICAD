# AICAD

AICAD is an AI-native CAD language, compiler, parametric modeling runtime, and geometry stack.

The repository has completed Stage 3 (parametric foundations and standard modeling). Stage-4 semantic-reference work has **not** begun on this transition branch.

## Documentation

- [`docs/user/`](docs/user/) — how to use the currently implemented AICAD product surface.
- [`docs/developer/`](docs/developer/) — how the current compiler/runtime/geometry system works and how to develop it.
- [`project/`](project/) — how AICAD itself is being developed: active stage/task governance, owner decisions, archived implementation evidence, gates, transitions, and roadmap planning.
- [`specs/`](specs/) and [`rfcs/`](rfcs/) — normative/architecture specification surfaces according to their own status.
- [`docs/plan/`](docs/plan/) — the frozen original implementation-plan bundle. This is historical/internal foundation planning retained at its legacy path because Stage-0..3 evidence references it extensively; it is not normal user/developer documentation.

## Active development control

Normal implementation/transition startup context is curated around:

- `CLAUDE.md`
- `AGENTS.md`
- `project/CURRENT_STAGE.md`
- `project/TASKS.yaml`
- `project/OWNER_DECISIONS.md`
- `project/DECISION_LOG.md`
- `project/SESSION_HANDOFF.md`
- current-stage reports/gates only when relevant

Completed Stage-0..3 reports and gate evidence are preserved under `project/reports/archive/` and `project/gates/archive/`; they should be retrieved for archaeology/review rather than recursively loaded during routine startup.

## Current implemented surface

By the end of Stage 3 the repository contains a Rust workspace implementing the AICAD frontend/type/HIR/runtime stack, explicit units and spatial types, runtime-backed Safe CAD functions, parameter and feature-DAG infrastructure, sketch/constraint foundations, backend-neutral geometry IR/runtime dispatch, OCCT adapter integration, validation, a build CLI, and Stage-3 standard modeling operations. See `docs/user/` for the supported user-facing surface and `docs/developer/` for architecture details.

Semantic references, the Stage-4 resolver, and post-100 roadmap capabilities remain future work until their respective tasks/owner gates are completed.

## Git and attribution policy

Repository-required commit identity is `insightlabs38-pixel <insightlabs38@gmail.com>`. Do not bypass repository hooks or add AI/session attribution metadata.
