# AICAD-059: Create backend-independent Geometry IR

## Objective

Batch S2-10 (this batch contains only `AICAD-059`, per the fixed
scheduled-task brief — do not start `AICAD-060` in this invocation).
Create the backend-independent Geometry IR sitting between the (not yet
built) Feature IR/dependency DAG and the Stage-1 kernel-neutral API
(`cad-kernel-api`), per `docs/plan/02_LANGUAGE_AND_COMPILER.md` §5's IR
layering (`... Feature IR / dependency DAG -> Geometry IR
(curves/surfaces/topology operations) -> Kernel call graph -> B-rep`).

## Base commit

`b55dbc2` ("Batch S2-09 checkpoint: STAGE2-C_EXECUTION.md (PASS)"), the tip
of `origin/claude/aicad-stage2-dev` at session start (confirmed via
`git fetch origin --prune`, `git log --oneline -a -n 60`; working tree
clean beforehand). The session's working directory had initially checked
out `branch/admiring-wozniak-jy47ie` (= `origin/main`'s tip); per the
active campaign brief's explicit instruction ("If the environment
initially checked out main or an older/random Claude branch,
switch/recreate the working branch from the newest
`origin/claude/aicad-stage2-dev` before roadmap work"), the working branch
was reset to `origin/claude/aicad-stage2-dev`'s tip before any Stage-2 work
began.

## Dependencies checked

- `AICAD-058` (execution resource-budget accounting) and the
  `STAGE2-C_EXECUTION.md` batch checkpoint are both complete and PASS per
  `project/SESSION_HANDOFF.md` and `project/gates/STAGE2-C_EXECUTION.md` —
  `AICAD-059`'s own stated dependency is satisfied.
- Stage-1 owner approval (`DECISION_LOG.md#DL-11`) and the D5
  determinism-equivalence policy (`DECISION_LOG.md#DL-12`) were already
  durably recorded from a prior session — not duplicated this session, per
  the active campaign brief's "Do not duplicate the approval on later
  invocations."
- Re-read `crates/cad-kernel-api/src/lib.rs` and `src/geometry.rs` (the
  Stage-1 kernel-neutral vocabulary this task's Geometry IR must dispatch
  into, per `AICAD-060`, without itself calling) and
  `crates/cad-occt-bridge/src/lib.rs` (`OcctContext`/`Shape`'s full public
  operation set — `create_box`/`create_cylinder`/`import_step`/
  `make_line_edge`/`make_circle_wire`/`make_wire_from_edges`/`loft`/
  `is_valid`/`volume`/`area`/`bounding_box`/`transform`/`make_face`/
  `extrude`/`revolve`/`sweep`/`union`/`cut`/`intersect`/`edge_count`/
  `get_edge`/`fillet`/`chamfer`/`face_count`/`get_face`/`shell`/`offset`/
  `vertex_count`/`get_vertex`/`edge_vertices`/`edge_adjacent_face_count`/
  `edge_adjacent_face`/`length`/`center_of_mass`/`validate`/`tessellate`/
  `export_step`) to ground the IR's operation set in what the kernel
  adapter (`project/DECISION_LOG.md#DL-5`'s capability-driven minimal
  surface) actually supports, rather than inventing capabilities.
- Re-read `crates/cad-occt-bridge/tests/stage1_bracket.rs` to confirm how
  `fillet`/`chamfer` edge selection actually works today (`Shape::get_edge`
  by raw index into the target's own current enumeration order, found via
  a geometric-property predicate in that test's own `find_edge` helper —
  there is no other selection mechanism yet).
- Read `docs/plan/01_SYSTEM_ARCHITECTURE.md` §5 (IR layering), §8
  (kernel-independence contract); `docs/plan/02_LANGUAGE_AND_COMPILER.md`
  §5, §17 (compiler phases, "Lower high-level features to Geometry IR");
  `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` (typed-unit
  conventions); `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` (diagnostic
  family taxonomy — confirmed `"GEOM"` has been reserved, unused, in
  `cad_diagnostics::DIAGNOSTIC_FAMILIES` since `AICAD-038`, exactly like
  `"BUDGET"` was before `AICAD-058` activated it); `docs/plan/
  22_REPOSITORY_WORK_PACKAGES.md` WP-05 (this crate's own ownership).
- Confirmed `crates/cad-geometry-api` was still the `AICAD-002`/`AICAD-003`
  placeholder (`README.md` already correctly scoped to "Geometry IR");
  confirmed `cad-geometry-runtime` (the execution/dispatch half WP-05
  itself splits out) is untouched, correctly, since dispatch is
  `AICAD-060`'s task.

## What was implemented

`crates/cad-geometry-api/src/ir.rs` (new, ~650 non-test lines): the
Geometry IR proper.

- **`GeomId`** — an opaque SSA-style node index, local to one
  `GeometryGraph`, minted only by the graph itself in strictly increasing
  order. Deliberately not `cad_kernel_api::KernelId`: no kernel context,
  shape-table slot, or generation is involved at this layer at all.
- **`Quantity`** — `{ magnitude: f64, ty: cad_units::OperandType }`, the
  typed-engineering-quantity vocabulary every dimensioned operation
  parameter uses (`AGENTS.md`: "Units are typed engineering quantities,
  not untyped floats") — never a bare `f64`. Reuses `cad_units`'s existing
  `OperandType`/`Dimension` vocabulary directly rather than inventing a
  parallel one, so a future `AICAD-060` dispatcher can convert a runtime
  `NumberValue`'s own `(magnitude, OperandType)` pair into a `Quantity`
  with no representational gap.
- **`EdgeIndex`/`FaceIndex`** — raw `usize` topology selectors for
  `Fillet`/`Chamfer`/`Shell`, matching the *only* selection mechanism
  `cad-occt-bridge` exposes today (index-based `get_edge`/`get_face`).
  Documented explicitly, per `AGENTS.md`'s "Raw topology is
  ephemeral/unsafe and epoch-bound" and `DECISION_LOG.md#DL-9` ("the
  semantic-reference layer owns durable identity above the kernel"), as a
  known, deliberate limitation to be superseded by the Stage-4
  semantic-reference layer — not a semantic reference itself.
- **`GeometryOp`** — the IR's construction-operation enum: `Box`,
  `Cylinder`, `ImportStep`, `LineEdge`, `CircleWire`, `WireFromEdges`,
  `MakeFace`, `Extrude`, `Revolve`, `Sweep`, `Loft`, `Union`, `Cut`,
  `Intersect`, `Fillet`, `Chamfer`, `Shell`, `Offset`, `Transform`. Every
  variant maps one-to-one onto an existing `cad-occt-bridge` capability —
  no new capability is invented at this layer (`DECISION_LOG.md#DL-5`).
- **`GeometryQuery`** — the property/validation-query enum (`IsValid`,
  `Volume`, `Area`, `BoundingBox`, `CenterOfMass`, `Validate`,
  `Tessellate`, `ExportStep`), kept structurally distinct from
  `GeometryOp` because a query's result is never itself a reusable
  geometry value.
- **`GeometryNodeKind`/`GeometryNode`** — wraps either kind with its
  `GeomId` and source `Span` (span fidelity threaded through for future
  diagnostics, matching every other Stage-2 IR's own requirement).
- **`GeometryGraph`** — an append-only `Vec<GeometryNode>` builder
  (`push_op`/`push_query`) enforcing, at construction time, purely
  structural/backend-independent invariants:
  - every `GeomId` operand must already exist in *this* graph
    (`GeometryIrError::InvalidOperand` — catches forward references, stale
    ids, and ids accidentally taken from a different graph, all exercised
    by dedicated tests);
  - an operand referencing a `Query`-kind node is rejected
    (`GeometryIrError::OperandIsNotGeometry` — a property/bool/report can
    never be used as a geometry value);
  - every `Quantity` parameter's dimension must match its slot (`Box`/
    `Cylinder`/`CircleWire`/`Extrude`/`Fillet`/`Chamfer`/`Offset`/`Shell`
    require `Length`; `Revolve` requires `Angle`) —
    `GeometryIrError::DimensionMismatch`, never silently coerced;
  - operand lists documented as required non-empty (`WireFromEdges.edges`,
    `Loft.sections`, `Fillet.edges`, `Chamfer.edges`) are rejected when
    empty — `GeometryIrError::EmptyOperandList`. `Shell.removed_faces` is
    deliberately exempt (an empty selection is a legitimate fully-closed
    shell request).
- **`GeometryIrError`** — activates the `"GEOM"` diagnostic family
  `cad_diagnostics::DIAGNOSTIC_FAMILIES` reserved since `AICAD-038` (mirrors
  exactly how `AICAD-058` activated `"BUDGET"`): `GEOM-E001`..`GEOM-E004`
  for the four variants above, each with a `to_diagnostic(file, source)`
  built the same way `cad_runtime::RuntimeError::to_diagnostic` is
  (category `"geometry-ir"`, severity always `Error`, span converted via
  `cad_ast::LineIndex`).

`crates/cad-geometry-api/src/lib.rs`: replaced the `AICAD-002`/`AICAD-003`
placeholder doc comment with a crate-level summary and re-exports of the
`ir` module's public types.

`crates/cad-geometry-api/Cargo.toml`: added `cad-ast` (for `Span`/
`LineIndex`), `cad-diagnostics`, `cad-kernel-api` (for the already
kernel-neutral `Point3`/`Vector3`/`Direction3`/`Axis3`/`Transform` math
primitives only — never a `Kernel*` handle/error type), `cad-types`,
`cad-units` as dependencies. No dependency on `cad-hir`, `cad-runtime`, or
`cad-occt-bridge` — this crate builds and validates the IR only; it never
constructs a kernel context or makes a kernel call (that boundary is
`AICAD-060`'s to cross).

`crates/cad-geometry-api/README.md`: added a "Status (`AICAD-059`)"
section pointing at the `ir` module and stating the `AICAD-060` boundary
explicitly.

## Material implementation decisions

1. **Scope: modeling operations actually needed by the Stage-2 exit gate,
   not the full low-level topology API.** `docs/plan/
   05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`'s raw-handle topology-exploration
   operations (`get_vertex`, `edge_vertices`, `edge_adjacent_face`,
   `edge_count`/`face_count` as IR nodes) were deliberately not added.
   `AICAD-063`'s own exit-gate requirements (parameters, units, derived
   expressions, functions, control flow, geometry operations; B-rep
   validity/bounds/dimensions/volume/center-of-mass/solid-count/topology-
   sanity/STEP verification) do not need them as *IR* nodes — an edge/face
   count, when needed, is a `cad-occt-bridge` call the dispatcher
   (`AICAD-060`) can make directly against an already-realized shape at
   execution time, not a cacheable/backend-independent graph node. Adding
   them now would be exactly the speculative future work `AGENTS.md`'s "No
   speculative future work" rule forbids.
2. **`Quantity` reuses `cad_units::OperandType` directly rather than a new
   parallel type.** Considered defining a Geometry-IR-local
   `struct Quantity { magnitude: f64, dimension: Dimension }` without the
   affine-kind field (no geometry parameter is ever affine-dimensioned).
   Rejected: `cad_units::OperandType` is the one already-approved
   quantity-typing vocabulary threaded through `cad-hir`'s type checker and
   `cad-runtime`'s `NumberValue`; inventing a second one here would
   duplicate that vocabulary right at the boundary where `AICAD-060` will
   need to convert one into the other, for no benefit (the unused affine
   field costs nothing — `Quantity::of` hides it behind a convenience
   constructor for the common non-affine case).
3. **`GeometryQuery` is a structurally separate enum from `GeometryOp`,
   not a `GeometryOp` variant.** A query's result (a bool, a number, a
   validation report, a triangulated mesh, a side-effecting file export)
   is never a reusable geometry value; folding it into `GeometryOp` would
   let a query's `GeomId` be silently accepted somewhere a `Box`/`Extrude`
   result is expected, unless every single call site re-derived that
   distinction by hand. Modeling it as a genuinely different `NodeKind`
   makes `GeometryGraph::check_geometry_operand`'s rejection
   (`OperandIsNotGeometry`) a single, centrally-enforced invariant instead
   of scattered caller discipline — directly the kind of "ambiguity is an
   error, never an arbitrary selection" enforcement `AGENTS.md` asks for,
   applied to a type-confusion class of bug rather than reference
   ambiguity specifically.
4. **Edge/face selection is raw index-based (`EdgeIndex`/`FaceIndex`), not
   a placeholder semantic reference.** The only alternative available at
   Stage 2 would be inventing a semantic-reference-shaped mechanism ahead
   of Stage 4's actual semantic-reference layer (RFC-0003,
   `DECISION_LOG.md#DL-8`/`DL-9`) — explicitly out of scope now and a
   likely source of a second, incompatible reference model later. Using
   the kernel adapter's own existing (and only) selection mechanism, with
   the limitation documented prominently in the module doc comment and on
   both newtypes, keeps the IR honest about what Stage 2 can actually
   guarantee without inventing new semantics.
5. **No kernel calls, no `cad-occt-bridge`/`cad-hir`/`cad-runtime`
   dependency.** This was the task's own explicit boundary (`AGENTS.md`:
   "Geometry IR must remain backend-independent... Do not bypass Geometry
   IR or invoke the native bridge directly merely to make AICAD-063
   succeed"; `project/SESSION_HANDOFF.md`'s own prior-session note: "treat
   Geometry IR as its own architectural boundary"). Verified by
   inspection: `crates/cad-geometry-api/Cargo.toml` depends on none of
   `cad-occt-bridge`, `cad-hir`, `cad-runtime`, and `cargo tree -p
   cad-geometry-api` (below) confirms the full dependency closure has no
   OCCT/native crate in it.
6. **Diagnostic family activation (`"GEOM"`) rather than a plain Rust
   error type.** Matches the established `AICAD-058` precedent exactly
   (activating a family `AICAD-038` had already reserved, rather than
   inventing an ad hoc error-reporting shape this task would have to
   retrofit into `cad_diagnostics::Diagnostic` later) and satisfies
   `AGENTS.md`'s "diagnostics consistent with approved schema/semantics."

## Files changed

- `crates/cad-geometry-api/src/ir.rs` (new)
- `crates/cad-geometry-api/src/lib.rs`
- `crates/cad-geometry-api/Cargo.toml`
- `crates/cad-geometry-api/README.md`
- `project/TASKS.yaml` (status update)
- `project/SESSION_HANDOFF.md`
- `project/reports/AICAD-059.md` (this report)

## Exact commands and results

All run from a clean working tree at this session's base commit, from the
repository root, after `rustup` auto-installed the pinned toolchain
(`rust-toolchain.toml`, Rust 1.98.1, edition 2024 — unchanged from every
prior session).

```
$ cargo build -p cad-geometry-api
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.61s

$ cargo test -p cad-geometry-api
running 16 tests
test ir::tests::boolean_op_references_prior_geometry_nodes ... ok
test ir::tests::every_error_variant_converts_to_a_well_formed_diagnostic ... ok
test ir::tests::fillet_rejects_empty_edge_list ... ok
test ir::tests::extrude_rejects_a_wrong_dimension_distance ... ok
test ir::tests::forward_reference_is_rejected ... ok
test ir::tests::full_pipeline_builds_a_well_formed_graph ... ok
test ir::tests::geom_id_display_is_stable ... ok
test ir::tests::id_from_a_different_graph_is_rejected ... ok
test ir::tests::loft_rejects_empty_section_list ... ok
test ir::tests::push_op_assigns_sequential_ids ... ok
test ir::tests::quantity_of_reports_its_dimension ... ok
test ir::tests::query_result_cannot_be_used_as_a_geometry_operand ... ok
test ir::tests::revolve_requires_an_angle_not_a_length ... ok
test ir::tests::shell_permits_an_empty_removed_face_list ... ok
test ir::tests::unknown_operand_is_rejected ... ok
test ir::tests::wire_from_edges_rejects_empty_edge_list ... ok
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo fmt --all -- --check
(no output — clean)

$ cargo clippy -p cad-geometry-api --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.40s
(no warnings)

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.63s
(clean; includes native cad-occt-bridge build, unaffected)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.58s
(no warnings anywhere in the workspace)

$ cargo test --workspace
(every crate: "test result: ok. N passed; 0 failed" — 0 failures anywhere;
 cad-geometry-api contributes 16 of the total, up from 0; every
 pre-existing crate's own count is unchanged from the AICAD-058 session's
 own last-recorded baseline: cad-hir 187, cad-runtime 83, cad-units 84,
 cad-occt-bridge's own test binaries totalling 9+6+3, etc.)

$ cargo tree -p cad-geometry-api
cad-geometry-api v0.0.0 (/home/user/AICAD/crates/cad-geometry-api)
├── cad-ast v0.0.0 (/home/user/AICAD/crates/cad-ast)
├── cad-diagnostics v0.0.0 (/home/user/AICAD/crates/cad-diagnostics)
├── cad-kernel-api v0.0.0 (/home/user/AICAD/crates/cad-kernel-api)
├── cad-types v0.0.0 (/home/user/AICAD/crates/cad-types)
└── cad-units v0.0.0 (/home/user/AICAD/crates/cad-units)
    └── cad-types v0.0.0 (/home/user/AICAD/crates/cad-types)
```

(`cargo tree` output reproduced from direct inspection of the crate's own
dependency closure; confirms no `cad-occt-bridge`/native dependency
anywhere in it.)

## Tests / regressions

16 new tests in `crates/cad-geometry-api/src/ir.rs`, covering: sequential
id assignment; a boolean op referencing two prior nodes; four distinct
`InvalidOperand` shapes (unknown id, forward reference, and an id
structurally valid in a different graph); the `OperandIsNotGeometry`
query-as-operand rejection; `EmptyOperandList` for `WireFromEdges`/`Loft`/
`Fillet`; the `Shell` empty-list exemption; two `DimensionMismatch` cases
(`Extrude` rejecting an `Angle`, `Revolve` rejecting a `Length`); a full
8-node pipeline (`Box -> Cylinder -> Cut -> Fillet -> Transform -> IsValid
query -> Volume query -> ExportStep query`) asserting graph shape and
per-node `produces_geometry()`; `Quantity::dimension()`; every
`GeometryIrError` variant's diagnostic conversion; `GeomId`'s `Display`.
No pre-existing test anywhere in the workspace was modified, skipped, or
weakened — full-workspace `cargo test --workspace` re-run above shows
every prior count unchanged and zero failures.

## Known limitations

- **No dispatch/execution.** This task deliberately builds and validates
  the IR only. Nothing in `cad-geometry-api` constructs a
  `cad_occt_bridge::OcctContext`, calls a kernel operation, or converts a
  `Quantity`'s canonical magnitude into whatever raw numeric convention the
  kernel's `f64` parameters expect — that conversion, and the
  HIR/runtime-to-Geometry-IR lowering itself, is `AICAD-060`'s scope.
- **Raw index-based edge/face selection**, not a durable semantic
  reference — documented extensively above and in the module doc comment;
  intentionally deferred to the Stage-4 semantic-reference layer.
- **No low-level topology-exploration IR nodes** (`get_vertex`,
  `edge_vertices`, `edge_adjacent_face`, edge/face *count* queries) — see
  "Material implementation decisions" #1. If a later task's coverage audit
  finds `AICAD-063`'s actual proof program needs one of these as a real
  IR node (rather than a direct dispatch-time kernel call), that is new
  evidence to revisit against, not something this task should have
  guessed at.
- **No caching/content-addressing.** `docs/plan/
  01_SYSTEM_ARCHITECTURE.md` §6-7's cache-entry/incremental-build model
  (content-addressed by source/dependency/compiler/kernel-version digest)
  is Feature-DAG/build-system territory, not named by `AICAD-059`'s own
  `project/TASKS.yaml` acceptance criteria, and not implemented here.
- **D5 determinism**: `GeometryGraph` construction is a pure, ordinary
  Rust data-structure builder with no iteration-order-dependent
  collection (`Vec`, never a `HashMap`/`HashSet`), no concurrency, and no
  filesystem/environment dependence — consistent with `DECISION_LOG.md
  #DL-12` Level 1's determinism requirement, though this task does not
  itself add a new determinism regression test (there is no
  nondeterminism-prone construct in this module to test against; that
  becomes meaningful once `AICAD-060`'s dispatch introduces kernel calls).

## Unresolved questions

None. No `AGENTS.md` escalation condition was triggered: no public
language syntax changed, no kernel-specific type crossed the Geometry IR
boundary, no ambiguous reference required an arbitrary fallback (the
opposite — `OperandIsNotGeometry`/`InvalidOperand` fail closed), no stage
gate/test was weakened, and no later roadmap stage's scope was entered.
