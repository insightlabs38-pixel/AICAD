# AICAD-060: Implement HIR/runtime geometry dispatch into Geometry IR/kernel API

## Status: partially complete — escalated (`project/OWNER_DECISIONS.md#D18`)

This task's title has two halves. This session fully implemented and
tested the half that needs no owner ruling, and escalated the half that
does, rather than silently inventing a language mechanism. `AICAD-060`
therefore stays open; the next invocation to touch Batch S2-11 must resume
it (not start `AICAD-061`) once `D18` is ruled on, following the exact
`D16`/`D17` precedent (`AICAD-056`/`AICAD-057` stayed open across their own
escalations).

## Objective

Per `project/TASKS.yaml`'s `AICAD-060` entry: implement HIR/runtime
geometry dispatch into the Geometry IR (`AICAD-059`) and kernel API
(`cad-kernel-api`/`cad-occt-bridge`, Stage 1), per RFC-0002's architecture-
layer diagram (`Geometry IR -> Kernel call graph -> B-rep`).

## Base commit

`06eb780` ("AICAD-059: Create backend-independent Geometry IR"), the tip
of `origin/claude/aicad-stage2-dev` at session start.

## Research performed before implementing

A dedicated research pass (recorded in full in this session's own
transcript) mapped, with exact signatures:

1. `crates/cad-runtime/src/{interp,value,error}.rs` — the `Interpreter`,
   `Value`/`NumberValue`, call-dispatch mechanism, and `ResourceBudget`.
2. `crates/cad-hir/src/{hir,ids,prelude,typeck}.rs` — `HirItem`/
   `BindingKind` shapes and the existing `List`/`Range`/`Result`/`Optional`
   precedents for compiler-builtin constructs.
3. `crates/cad-occt-bridge/src/lib.rs` — the complete `OcctContext`/`Shape`
   public API (every method signature).
4. `crates/cad-kernel-api` — the kernel-neutral math/handle vocabulary.
5. `crates/cad-geometry-api/src/ir.rs` — already-committed Geometry IR
   (read directly, not via the research pass).
6. Workspace-wide greps confirming no partial dispatch work already
   existed, and that no geometry-builtin name was pre-reserved anywhere.
7. `project/reports/AICAD-059.md` in full.
8. `docs/plan/02_LANGUAGE_AND_COMPILER.md` /
   `03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` for any documented builtin-
   function catalogue (none exists — only illustrative example syntax).

The key finding: **calling a function today unconditionally requires a
real AICAD-source `HirBlock` body** (`HirItem::Fn::body` is a mandatory
field; `Interpreter::run_fn_body` always executes it via `exec_block`).
The only existing "callee dispatched without running a `HirBlock`" is
`BindingKind::EnumVariant`, which is enum-construction-specific. No
`Builtin`/`NativeFn`/intrinsic-function mechanism exists anywhere in the
workspace. This is the same structural shape of gap `D16` (`AICAD-056`)
and `D17` (`AICAD-057`) each found — "the language cannot yet construct
the kind of value this task needs" — except here it is a *callable*, not a
*type*, that is missing a construction mechanism.

## What was implemented

### 1. `crates/cad-geometry-runtime` — the Geometry IR -> kernel dispatcher

Populated the previously-empty `AICAD-002`/`AICAD-003` placeholder crate
(per RFC-0002 §2's own architecture table: "Geometry runtime |
`cad-geometry-api`, `cad-geometry-runtime`" — the natural home for the
*runtime* half of the geometry pipeline, distinct from `cad-geometry-api`'s
pure, kernel-independent IR):

- **`dispatch` module**: `dispatch_graph(&GeometryGraph, &OcctContext) ->
  Result<GraphResults, DispatchError>` walks every node of an
  already-validated `GeometryGraph` in order (a single forward pass
  suffices — SSA/append-only, only-backward-references, per
  `cad_geometry_api::ir`'s own design) and calls the matching
  `OcctContext`/`Shape` operation for all 18 `GeometryOp` variants and all
  8 `GeometryQuery` variants. `EdgeIndex`/`FaceIndex` selectors
  (`Fillet`/`Chamfer`/`Shell`) are resolved to real edge/face `Shape`s via
  `Shape::get_edge`/`get_face` immediately before the operation that
  consumes them, in the same dispatch pass, and never cached across nodes
  — matching `AGENTS.md`'s "Raw topology is ephemeral/unsafe and
  epoch-bound" and `cad-occt-bridge`'s own "intended for immediate use as
  a selector, not for storage" documentation.
  `DispatchError` (`GEOM-E005` kernel-operation-failed, `GEOM-E006`
  graph-invariant-violated — the latter defensive-only, unreachable for
  any graph built through `GeometryGraph::push_op`/`push_query`'s own
  validation) converts to a `cad_diagnostics::Diagnostic` mirroring
  `GeometryIrError::to_diagnostic`'s exact pattern.
- **`bridge` module**: `number_value_to_quantity(&NumberValue) ->
  Option<Quantity>` — a direct, lossless field copy. Both
  `cad_runtime::value::NumberValue` and `cad_geometry_api::Quantity`
  independently chose to store `(magnitude: f64, ty: cad_units::
  OperandType)` with the magnitude in canonical units (metres for
  `Length`, radians for `Angle`), so no unit-system reconciliation is
  needed — `AICAD-059`'s own report already predicted this ("so a future
  `AICAD-060` dispatcher can convert a runtime `NumberValue`'s own
  `(magnitude, OperandType)` pair into a `Quantity` with no
  representational gap"). Returns `None` for a non-dimensional `Scalar`
  value, since no `GeometryOp`/`GeometryQuery` parameter slot accepts one.

**Quantity -> kernel `f64` convention (a decision, not an escalation):**
this dispatcher passes a `Quantity`'s canonical magnitude straight through
as the kernel's raw `f64` parameter. `cad-occt-bridge`'s operations are
unit-agnostic (OCCT itself does not have a unit system; existing tests
pick arbitrary magnitudes), so there is no established alternative
convention to reconcile against, and this choice is invisible to public
language semantics — an AICAD program author who writes `5mm` never
observes which internal float convention the kernel received. This is an
internal implementation detail, not a public semantic decision, so it was
made autonomously rather than escalated.

### 2. `crates/cad-geometry-api/src/ir.rs` — a real bug found and fixed

`GeometryQuery::Tessellate` (added by `AICAD-059`, already committed)
carried only one `deflection: Quantity` field, but `Shape::tessellate(&self,
linear_deflection: f64, angular_deflection: f64)` needs both a linear and
an angular deflection tolerance. This was discovered while implementing
this dispatcher's `Tessellate` arm — the IR literally could not describe a
valid `tessellate` call. Fixed per `AGENTS.md`'s "when a bug is found...
fix the root cause": split the field into `linear_deflection: Quantity`
(`Dimension::Length`) and `angular_deflection: Quantity`
(`Dimension::Angle`), updated `GeometryGraph::push_query`'s own dimension
validation for both, and added a regression test
(`tessellate_requires_a_length_linear_and_angle_angular_deflection`)
covering the correct case and both wrong-dimension cases. No existing test
referenced the old one-field shape (`AICAD-059`'s own test suite never
exercised `Tessellate`), so this is a clean additive fix with zero
breakage.

### 3. `project/OWNER_DECISIONS.md#D18` — the escalated question

**What was *not* implemented, and why:** giving an `.aicad` program a way
to actually *invoke* `box(...)`/`cylinder(...)`/etc. — i.e., the
"HIR/runtime -> Geometry IR" trigger, as opposed to the "Geometry IR ->
kernel API" half implemented above. This needs a new binding/dispatch
mechanism `cad-hir`/`cad-runtime` do not have today (see "key finding"
above), which is both an `AGENTS.md`/`project/TASKS.yaml` "public syntax/
semantics must change beyond an approved RFC" and "an unresolved
architecture alternative must be selected" escalation trigger — this task
does not silently invent one. Full audit findings and three live options
(a new `BindingKind::GeometryIntrinsic` mirroring `BindingKind::
EnumVariant`'s own precedent; reusing/extending RFC-0002 §4's already-
frozen `unsafe geometry` blocks; deferring language-surface invocation
further) are recorded in `project/OWNER_DECISIONS.md#D18`, without
deciding among them.

## Tests

- `crates/cad-geometry-api/src/ir.rs`: 1 new regression test (the
  `Tessellate` dimension split), 17 total in this crate (up from 16).
- `crates/cad-geometry-runtime/src/bridge.rs`: 2 tests (dimensional
  conversion round-trip; scalar value correctly does not convert).
- `crates/cad-geometry-runtime/src/dispatch.rs`: 7 tests, all dispatching
  against a real `cad_occt_bridge::OcctContext` (never a render-only
  check, per `AGENTS.md`'s evidence rule):
  - `full_pipeline_dispatches_a_notched_filleted_box_through_the_kernel` —
    `Box`/`Cut`/`Fillet`/`Transform`, then `IsValid`/`Validate`/`Volume`/
    `BoundingBox`/`CenterOfMass`/`ExportStep` (through the dispatcher's own
    query path, not a direct `Shape::export_step` call), then a *second*,
    independent `OcctContext` reimports that exact STEP file via a fresh
    graph's `ImportStep` op and confirms it is a valid B-rep. Volume/
    bounding-box assertions are closed-form/analytic, with tolerances
    documented and justified (the fillet's own small, un-derived volume/
    bbox perturbation, not a claimed exact match).
  - `cylinder_volume_matches_the_closed_form_formula` — exact to 1e-6
    relative.
  - `extruded_circle_wire_matches_the_closed_form_volume` — `CircleWire` ->
    `MakeFace` -> `Extrude`, volume within 1e-3 relative of πr²h.
  - `wire_from_edges_builds_a_square_face_with_the_expected_area` — four
    `LineEdge`s -> `WireFromEdges` -> `MakeFace`, area exact to 1e-6
    relative.
  - `tessellate_produces_a_nonempty_mesh`.
  - `kernel_error_surfaces_as_a_well_formed_dispatch_error_diagnostic` — a
    structurally-valid-but-kernel-rejected graph (negative box dimension)
    surfaces as `DispatchError::Kernel` with a well-formed `GEOM-E005`
    diagnostic.
  - `graph_invariant_violation_is_reported_rather_than_panicking` — the
    defensive-only `GraphInvariantViolated` path.

## Exact commands and results

```
$ cargo build -p cad-geometry-api -p cad-geometry-runtime
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.51s

$ cargo test -p cad-geometry-api -p cad-geometry-runtime
cad-geometry-api:    17 passed; 0 failed
cad-geometry-runtime: 9 passed; 0 failed
(includes a real OCCT STEP export/reimport round trip)

$ cargo fmt --all -- --check
(clean, after one `cargo fmt --all` pass over the two new files)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.66s
(clean, zero warnings)

$ cargo test --workspace
Every crate: test result: ok, 0 failed anywhere.
(cad-hir 187, cad-geometry-api 17, cad-geometry-runtime 9, cad-units 75,
cad-runtime 83, cad-occt-bridge unit tests 119 + its three separate test
binaries 9+6+3, cad-types 49, cad-ast 20, cad-diagnostics 10, cad-lexer 23,
cad-parser 29, cad-kernel-api 84 — all unchanged from the AICAD-059
session's own baseline except the two crates this session touched.)
```

## Known limitations

- `AICAD-060` is not complete: the language-surface invocation mechanism
  is escalated as `D18`, not implemented. `AICAD-061` should not begin
  until `D18` resolves and `AICAD-060` resumes and completes.
- The `GraphInvariantViolated` dispatch-error path is exercised only via a
  direct unit test bypassing `GeometryGraph`'s own public API (there is no
  way to construct a genuinely invariant-violating graph through
  `push_op`/`push_query` — that is the point of those functions' own
  validation).
- No wall-clock/resource-budget accounting was added for kernel dispatch
  calls (`AICAD-058`'s own module doc comment already flagged this as
  "the earliest any geometry-op-shaped budget could mean anything" —
  scoping that is left to whichever task actually wires runtime execution
  into this dispatcher, once `D18` resolves, since only then would a
  running program's own resource consumption include geometry-op calls).

## Unresolved questions

`project/OWNER_DECISIONS.md#D18`, in full — see that entry.
