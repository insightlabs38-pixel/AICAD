# Final Stage-5 execution plan

Status: **FINAL QUEUE FOR OWNER REVIEW — NOT YET IMPLEMENTATION-AUTHORIZED**  
Stage: 5 — advanced geometry  
Approved Stage-4 base: `35547025bbe32f350bcdaf2c1556ed2482a2cada` (`564e6790b67bf2a5b489bca2003a5084a959e937` merged Stage-4 code + owner approval record)  
Canonical executable task IDs: `AICAD-101` through `AICAD-130`

This file is the batch view of the exact Stage-5 tasks appended to `project/TASKS.yaml`. `project/TASKS.yaml` remains the executable task authority. No task in this file is permission to implement Stage 5 before the transition is owner-reviewed/merged and `claude/aicad-stage5-dev` is created from that exact merged HEAD.

## Reconciliation principles

The old `POST100-S5-*` draft was decomposition input, not a fixed queue. Final Stage 4 materially changed its assumptions: AICAD-100A resolved D31, made persistent-reference scope explicit, supplied production evidence for all non-fingerprint construction strategies, completed the Rust production predicate surface, promoted `query { ... }` into real source syntax/lowering, and proved the public corpus through production paths. Those completed items are not re-created as Stage-5 work.

Four real Stage-4 carry-forwards are promoted explicitly into the Stage-5 prelude: nested `part` behavior may no longer execute silently inert; the Area/source-value and spatial-construction gaps needed by queries must be closed; the remaining source vocabulary/lowering for already-real Stage-4 predicates must be completed; and the completed source query vocabulary must receive production-path regression/integration proof. Automatic fingerprint recovery is intentionally **not** among them. Unscoped whole-session search remains an explicitly broad low-level operation rather than receiving magical repair.

All Stage-5 work is constrained by D2, D5, D6, D7, D18, and D20-D25. In particular: no OCCT type crosses the public/HIR boundary; no new compiler geometry intrinsic is introduced merely for convenience; tolerance domains remain distinct; raw handles are never durable identity; topology-changing operations emit lineage; ambiguity fails closed; and programmability may not erase feature/dependency/provenance visibility.

## Fixed batches

### S5-00 — Stage-4 carry-forward language/query completeness

**AICAD-101 — Make nested `part` semantics explicit and non-silent.**  
Acceptance intent: every grammatically accepted nested `part` either executes with coherent recursive lexical/scoped semantics through interpreter, feature graph, build/name collection, provenance, and query lowering, or is rejected before execution with a stable structured diagnostic until supported. Silent inert execution is forbidden. D31 remains authoritative: supported part nesting is a scope boundary, not a feature-visibility barrier.

**AICAD-102 — Complete dimensional/spatial source-value construction needed by Stage-5 queries.**  
Acceptance intent: close the Area-dimension/source-literal gap and make the required `Point3`/vector/direction/axis/frame value construction available through ordinary typed, unit-safe AICAD source semantics. No backend-specific public fields or kernel types are introduced.

**AICAD-103 — Complete `.aicad` query vocabulary/lowering for the Stage-4 production predicate surface.**  
Acceptance intent: source forms can express the already-implemented Stage-4 predicates that still require nested semantic references, spatial values such as `Point3`/`Frame3`, or other source-side operands. Lowering produces real `cad_query`/`cad_references` values; malformed/unsupported forms fail structurally rather than being ignored or injected by tests.

**AICAD-104 — Prove production-path source-query completeness and preserve fail-closed behavior.**  
Acceptance intent: real `.aicad` fixtures exercise the completed query vocabulary through parse/HIR/lowering, `ParametricBuildSession`, source-reference registration, resolver/cardinality handling, and `cad refs check`; nested-reference, ambiguity, broken-reference, and cardinality cases are permanent regressions. `GeometricFingerprint` remains automatic-recovery-disabled under D7.

Batch exit: all four tasks complete; source examples use only supported syntax; no unresolved Stage-4 execution-completeness limitation needed by Stage 5 remains hidden.

### S5-01 — Runtime/tolerance/query foundations

**AICAD-105 — Scale the closed RuntimeBuiltin catalogue and kernel-backed query evaluation.**  
Acceptance intent: implement D21/D23 without opening native registration. Runtime-backed advanced functions remain ordinary typed calls from a deterministic, type-closed first-party catalogue; source evaluation may demand-realize kernel query results with dependency/cache/resource/evidence accounting and no uncontrolled I/O or compiler-special syntax.

**AICAD-106 — Implement typed Stage-5 operation/approximation tolerance policy primitives.**  
Acceptance intent: modeling/construction and approximation policy are explicit, dimensionally typed and evidence-carrying, distinct from representation validity, solver, verification, and D5 equivalence tolerances. No value is silently widened or inherited from another domain.

### S5-02 — Programmable feature/provenance and geometry value/IR foundations

**AICAD-107 — Preserve feature identity, dependencies and provenance through ordinary language abstractions.**  
Acceptance intent: supported user functions, helper/part abstractions, repeated calls and supported control flow remain visible to feature/dependency/provenance and incremental-rebuild systems under D25. Stable call/source identity is deterministic or unsupported cases fail explicitly; abstraction may not erase semantic-reference support.

**AICAD-108 — Define kernel-neutral advanced geometry, result, report and execution-IR families.**  
Acceptance intent: curve/surface/topology/query/report values are AICAD-owned and backend-neutral; public source/HIR values do not mirror OCCT classes; internal execution IR remains independently evolvable; failures/results are structured and serializable without native pointers.

### S5-03 — Curves + Checkpoint A

**AICAD-109 — Implement analytic curve construction/evaluation.**  
Exact kernel-neutral analytic curves, explicit domains, deterministic evaluation/tangents, structured degeneracy behavior.

**AICAD-110 — Implement Bezier, B-spline and NURBS curves.**  
Validated degrees/control points/knots/multiplicities/weights/periodicity; clamped/periodic/repeated-knot/rational cases; invalid combinations rejected before unsafe kernel behavior.

**AICAD-111 — Implement curve trimming, interpolation, offset, projection and distance operations.**  
Functional new-value semantics, explicit tolerance class, exact/approximation evidence, deterministic normalized multi-results, structured tangent/no-solution/degenerate cases.

**AICAD-112 — Checkpoint A: programmable curve stack.**  
A source-defined helper builds and queries nontrivial freeform curves through ordinary calls; feature/provenance/invalidation remain correct; representative ACTIVE teaching/realistic examples are executable in CI/equivalent automation; workspace checks pass.

### S5-04 — Surfaces

**AICAD-113 — Implement analytic surface construction/evaluation.**  
Kernel-neutral analytic families, explicit parameter domains/periodicity/singularities, deterministic evaluation/tangents/normals.

**AICAD-114 — Implement Bezier, B-spline and NURBS surfaces.**  
Validated freeform tensor-product/rational surface values; periodic/high-degree/rational cases; no native-kernel identity as data semantics.

**AICAD-115 — Implement the trimmed-surface semantic model.**  
Semantic boundary loops/regions with orientation/closure/parameter-space validity and lineage-ready identity; reparameterization may not silently change identity where equivalence is defined.

**AICAD-116 — Implement surface evaluation, derivative and offset operations.**  
Explicit conventions and tolerance policies; self-intersection/singularity/approximation outcomes are structured; operations are functional and provenance-carrying.

### S5-05 — Multi-solution geometric queries + Checkpoint B

**AICAD-117 — Implement intersection, projection and distance query families with explicit cardinality/ambiguity semantics.**  
Curve/surface combinations return typed multi-results with multiplicity/classification/effective-tolerance evidence; tangent/coincident/overlap/no-solution states are distinct; observation order is normalized backend-neutrally; single-result expectations never silently choose among multiple candidates.

**AICAD-118 — Checkpoint B: freeform surface/query stack.**  
At least one difficult freeform surface is constructed, trimmed, queried and inspected from source; tolerances/cardinality/order are machine-inspectable; provenance/incremental behavior remains intact; ACTIVE representative examples execute automatically; no OCCT leakage.

### S5-06 — General topology

**AICAD-119 — Implement general topology construction.**  
Validated vertex/edge/wire/face/shell/solid construction from approved geometry; orientation/closure/manifoldness/validity failures structured; semantic naming does not rely on raw enumeration order.

**AICAD-120 — Implement sewing/healing with explicit fidelity, tolerance and lineage evidence.**  
Returns geometry plus structured changes/tolerances/unresolved defects/lineage; no undocumented tolerance growth; input-to-output provenance retained.

**AICAD-121 — Implement deterministic topology traversal and inspection.**  
Kernel-neutral typed adjacency/orientation/geometry-association/hierarchy inspection; semantic/reference-oriented results where possible; raw enumeration explicitly ephemeral; deterministic observation boundaries.

### S5-07 — Raw geometry, editing and adoption

**AICAD-122 — Extend Stage-4 raw handles into the controlled Stage-5 raw/unsafe geometry tier.**  
AICAD-owned opaque epoch/context-bound values; deterministic stale/cross-context rejection before native dereference; no serialization as durable identity; safe/raw type surfaces visibly distinct under D22.

**AICAD-123 — Implement functional controlled topology editing.**  
Split/merge/replace/trim and approved edits return new values, invalidate prior raw epochs as specified, emit split/merge/deleted/modified/unchanged lineage, leave originals semantically unchanged on failure, and expose no alias-observable in-place mutation.

**AICAD-124 — Implement explicit validation/adoption from raw geometry to safe semantic geometry.**  
Only conforming raw results can be adopted; report includes validation, effective tolerances, lineage/provenance and required semantic output bindings; successful result requires no raw handle for ordinary safe use; missing/ambiguous bindings fail explicitly.

### S5-08 — Reference integrity + Checkpoint C

**AICAD-125 — Integrate Stage-5 topology-changing lineage with Stage-4 persistent-reference semantics.**  
Every topology-changing Stage-5 operation supplies resolver-consumable lineage. Adversarial split/merge/delete/edit/rebuild cases preserve intended references or fail `Ambiguous`/`Broken`; Stage-4 durability/scoping semantics remain compatible; fingerprint policy unchanged; silent-wrong count remains zero.

**AICAD-126 — Checkpoint C: safe/raw topology and semantic-reference integrity.**  
A real source-first fixture performs raw inspection/editing, validates/adopts, and continues in safe modeling; stale handles/native pointers never become artifacts/refs; reference perturbation metrics show no silent-wrong regression; healing/edit evidence is deterministic and inspectable.

### S5-09 — Realism, hardening and maintained examples

**AICAD-127 — Build the difficult freeform exact-geometry integration corpus.**  
Source-first fixtures combine spline curves/surfaces, trims, topology construction and controlled healing (including representative spline hook/variable-section sweep/twisted loft/blade-or-impeller-like/manually trimmed cases where applicable). Expected exact properties/topology/reference behavior are machine checked; no test-only native shortcuts.

**AICAD-128 — Run numerical/robustness adversarial and bounded performance/resource campaign.**  
Degenerate/near-singular/multi-scale/periodic/tangent/self-intersecting/invalid/resource-heavy cases produce no panic, UB, stale dereference, arbitrary selection or silent recovery. Failures are structured. Performance/resource evidence is collected where it materially informs the gate; optimizations may not weaken semantics or tolerances.

**AICAD-129 — Prove core-vs-library boundary, learnability/inspectability, and the maintained-examples contract.**  
Advanced models are reusable ordinary AICAD source abstractions; structured inspection exposes operations/refs/provenance/validation/healing/measurements; domain generators remain library concerns absent proven privileged need. Every ACTIVE example is executable/tested in CI or equivalent automation; examples are classified as teaching, realistic, or stress/benchmark fixtures; user-facing `examples/` primarily contains teaching/realistic examples; stale historical examples are archived, not presented as supported usage.

### S5-10 — Final Stage-5 owner gate

**AICAD-130 — Prepare the Stage-5 owner hard-gate packet.**  
Checkpoint A/B/C complete; difficult corpus/adversarial campaign and maintained examples pass; safe/raw/adoption/provenance/reference integrity demonstrated end-to-end; no OCCT/public-kernel leakage, geometry compiler intrinsic, global epsilon, provenance-erasing abstraction, raw-handle identity shortcut, or silent-wrong reference behavior remains; residual limitations are explicit. Agent may recommend pass/do-not-pass but may not approve Stage 5. Stage 6 stays provisional until owner approval.

## Batch dependency summary

`S5-00 -> S5-01 -> S5-02 -> S5-03(A) -> S5-04 -> S5-05(B) -> S5-06 -> S5-07 -> S5-08(C) -> S5-09 -> S5-10`.

Within that sequence, task-level dependencies in `project/TASKS.yaml` are authoritative and allow only deliberately safe overlap. Checkpoints and the final gate terminate their batch.

## Examples invariant beginning with Stage 5

Every ACTIVE example is a maintained product surface and must execute successfully in CI or an equivalent automated example suite. Teaching and realistic examples belong primarily under user-facing `examples/`; stress/reference fixtures belong primarily under testing/benchmark infrastructure. Each major checkpoint must add or update representative examples for newly supported public capability, and those examples must use actual supported `.aicad` syntax rather than Rust APIs or aspirational syntax. Historical useful examples are archived instead of silently remaining stale.
