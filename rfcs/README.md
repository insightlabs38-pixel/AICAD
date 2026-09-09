# AICAD RFCs

RFCs freeze foundational contracts before implementation, per
`docs/plan/15_IMPLEMENTATION_ROADMAP.md` Stage 0 and `AGENTS.md`'s
non-negotiable to prefer library solutions and require an RFC for compiler
intrinsics (RFC-0001 §6, ruling DL-7).

No directory for RFCs is specified in
`docs/plan/22_REPOSITORY_WORK_PACKAGES.md` §1's proposed layout; `rfcs/` at
the repository root was chosen for AICAD-007 following common practice
(e.g. Rust's own RFC process) rather than overloading `specs/language/`,
which holds the machine-checked grammar/semantics artifacts an RFC's
decisions get lowered into, not the decision record itself.

| RFC | Title | Status |
|---|---|---|
| [0001](0001-language-principles.md) | Language principles | Draft (Stage 0) |
| [0002](0002-geometry-runtime-kernel-abstraction.md) | Geometry/runtime and kernel abstraction | Draft (Stage 0) |
| [0003](0003-semantic-references.md) | Semantic references | Draft (Stage 0) |
| [0004](0004-units-type-system.md) | Units/type system | Draft (Stage 0) |
| [0005](0005-diagnostics.md) | Diagnostics | Draft (Stage 0) |

"Draft (Stage 0)" means: every architectural choice inside the RFC that
required an owner ruling has one, recorded in `project/DECISION_LOG.md` and
cited inline; the RFC document itself is submitted as part of the Stage-0
gate packet (`project/reports/AICAD-014.md`) for the owner's overall
stage-exit decision, not implicitly self-approved by being drafted.

An RFC does not resolve any decision still marked `open` in
`project/OWNER_DECISIONS.md` — where a section of the plan touches one, the
RFC says so explicitly and leaves it open rather than picking an answer.
