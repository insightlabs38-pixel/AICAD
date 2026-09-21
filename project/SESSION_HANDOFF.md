# Session Handoff

## Canonical state

**Stage 4 is complete, owner-approved, and merged.**

- Stage-4 merged implementation: `564e6790b67bf2a5b489bca2003a5084a959e937` (PR #13).
- Final Stage-4 task: `AICAD-100A`.
- Final Stage-4 gate: `project/gates/stage-4-gate.md`.
- Owner approval: 2026-09-16, recorded by `project/approvals/STAGE4_OWNER_APPROVAL.md` and `DL-34`.

**The Stage-5 queue is approved and implementation is authorized.**

- Owner approval: 2026-09-18, recorded by `project/approvals/STAGE5_QUEUE_OWNER_APPROVAL.md` and `DL-35`.
- Canonical Stage-5 development branch: `claude/aicad-stage5-dev`, created from the approved merged head `df9334634e09a9793191ea5de40f0aad12032589`.

The active branch is `claude/aicad-stage5-dev`. It is the Stage-5 development branch.

## What the transition finalized

Stage 5 has an exact executable queue, now approved for implementation:

- `AICAD-101..AICAD-130` in `project/TASKS.yaml`;
- fixed batches `S5-00..S5-10` in `project/planning/transitions/stage4-to-stage5/STAGE5_FINAL_BATCHES.md`;
- transition/reconciliation rationale in `TRANSITION_REPORT.md`;
- current public/developer documentation and maintained examples synchronized in `DOCUMENTATION_AND_EXAMPLES_SYNC.md`.

Stage-4 carry-forward work is explicit in S5-00: nested `part` behavior, Area/spatial source construction, remaining source-query vocabulary/lowering, and production-path source-query completeness. Automatic fingerprint recovery remains disabled under D7.

Stage 6 and Stage 7 remain complete but provisional queues; they are not current product commitments.

## Maintained example invariant

The current user-facing example inventory is in `examples/README.md`. Every ACTIVE example is registered in `crates/cad-cli/tests/active_examples.rs`.

Beginning with Stage 5:

- public language/modeling changes update affected ACTIVE examples in the same task/batch;
- each major checkpoint adds/refreshes representative examples;
- stale examples are updated or explicitly archived;
- stress/benchmark fixtures stay out of the primary learning path.

## Stage-5 batch sequence

1. S5-00 / AICAD-101..104 — Stage-4 carry-forward language/query completeness.
2. S5-01 / AICAD-105..106 — runtime/query/tolerance foundations.
3. S5-02 / AICAD-107..108 — programmable feature/provenance + geometry value/IR foundations.
4. S5-03 / AICAD-109..112 — curves + Checkpoint A.
5. S5-04 / AICAD-113..116 — surfaces.
6. S5-05 / AICAD-117..118 — intersection/projection/distance + Checkpoint B.
7. S5-06 / AICAD-119..121 — topology construction/healing/inspection.
8. S5-07 / AICAD-122..124 — controlled raw geometry/edit/adoption.
9. S5-08 / AICAD-125..126 — lineage/reference integrity + Checkpoint C.
10. S5-09 / AICAD-127..129 — realistic/adversarial/example campaign.
11. S5-10 / AICAD-130 — final Stage-5 owner gate.

## Non-negotiable invariants carried forward

D2 functional/value semantics; D5 deterministic equivalence separation; D6 kernel-neutral public semantics; D7 fail-closed references/no automatic fingerprint recovery; D18 ordinary RuntimeBuiltin calls; D20-D25 type/catalogue/safe-raw/query/tolerance/provenance rules. Raw handles remain epoch-bound and never durable identity. Topology-changing Stage-5 operations must emit resolver-consumable lineage.

## Current stop rule and exact next action

Batch `S5-08` (`AICAD-125..126`) is complete — see `project/reports/
AICAD-125.md`/`AICAD-126.md` (superseded detail retired from this file per
its own "current state, not an appended diary" convention).

**Batch `S5-09` is in progress. `AICAD-127`/`128` are done; `AICAD-129` is
next** (depends on both, satisfied). See `project/reports/AICAD-127.md`/
`AICAD-128.md` for full detail. Summary:

- `AICAD-127` froze `project/benchmarks/stage5_freeform_corpus/` (4
  `public` + 3 `held_out` fixtures, each with declared exact/closed-form
  checked evidence and tolerance domain), proven by
  `crates/cad-cli/tests/stage5_freeform_corpus.rs` (7/7 passing).
- **Central finding**: building the corpus's originally-planned
  positive-path fixtures discovered that `make_edge`/
  `make_face_on_surface` (topology construction, `AICAD-119`) only bridge
  the original Stage-2 analytic families (`Circle`/`Arc`/trimmed-`Line`;
  `Plane`/`Cylinder`/`Cone`/`Sphere`/`Torus`) into real kernel B-rep
  topology — a Bezier/B-spline curve (`AICAD-110`) or surface
  (`AICAD-114`), or any `trim_surface` result (`AICAD-115`), is rejected
  with an explicit `UNSUPPORTED_TOPOLOGY_CONSTRUCTION` diagnostic, never
  silently. `AICAD-110`/`114`/`115` gave `.aicad` real freeform curve/
  surface **values** (construction/evaluation/trim/intersection/
  projection/distance, already kernel-free), but `AICAD-119`'s topology
  dispatch was never extended to accept them. Recorded as a non-decision
  item in `project/OWNER_DECISIONS.md` (ordinary future implementation
  work, not an architecture ruling) and as this corpus's own README
  "Discovered capability gap" section. Every `public/` fixture proves the
  freeform geometry's exact value-level math instead of a face/solid it
  cannot yet construct; `held_out/06`/`07` turn the gap itself into
  adversarial evidence that the rejection is explicit.

- `AICAD-128` ran 8 adversarial cases (`crates/cad-cli/tests/
  stage5_adversarial_campaign.rs`) against the categories this task's
  acceptance list names: near-degenerate + multi-scale circular faces
  (1nm/1km radius, area exact to ~1e-16 relative error), tangent spheres
  (`intersect_surfaces` fails the *whole build* explicitly with
  `GeometricQueryFailed`, not an empty `List` — recorded precisely, not
  assumed), periodic circle wraparound (exact past +/-2*pi), a sew
  tolerance boundary (0.001mm gap merges to 7 edges/6 vertices; 0.1mm gap
  stays at 8/8 — the tolerance is a real threshold), a self-intersecting
  B-spline's closest-point query (one real inspectable `List`), a 64-edge
  polygon (~30ms, exact area), two independent builds of `AICAD-127`'s
  own `02` fixture (bit-identical areas — D5 Level 1/2 determinism), and
  a malformed B-spline knot vector (rejected explicitly at construction,
  never a panic). **No reproducible defect found** — every case is
  either a correct positive result or an explicit structured failure;
  two of the eight cases' initial assertions were themselves corrected
  mid-task once real kernel behavior diverged from the initial
  assumption (see `AICAD-128.md`'s own "Real findings" section).

All required checks pass as of `AICAD-128`: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
the full `cargo test --workspace` (1826 passed, 0 failed — 1818 after
`AICAD-127` + 8 new adversarial tests).

Next: `AICAD-129` (core-vs-library boundary, learnability/inspectability,
maintained-examples audit) — depends on `AICAD-127`/`128` (satisfied).

Do not perform another broad architecture audit during Stage-5 -> Stage-6 promotion unless actual Stage-5 evidence invalidates a material provisional assumption.
