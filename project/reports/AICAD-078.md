# AICAD-078: Implement high-level fillet/chamfer/shell wrappers and normalized diagnostics

## Status

Done. Second and final task of Batch S3-07.

## Objective

Two parts, per the task title:

1. Complete the dress-up-feature Safe CAD catalogue by adding `shell` —
   `fillet`/`chamfer` already existed (Stage 2); `GeometryOp::Shell`/
   `Shape::shell`/`aicad_occt_shell` have existed since `AICAD-026`/
   `AICAD-059`/`AICAD-060`, but no `BuiltinFnId` wrapper was ever wired to
   them, the one dress-up feature `cad_hir::builtins`'s own "Stage-2
   catalogue scope" module note explicitly left out.
2. "Normalized diagnostics" — `project/DECISION_LOG.md#DL-18` (D10,
   diagnostic code/schema stability policy) was ruled specifically so it
   would be in force before this task, per the campaign brief's own
   explicit requirement (`DL-18`'s "Affected RFCs/tasks" entry). This
   task brings the repository's existing diagnostic-emitting code into
   line with that now-resolved policy: fixing stale "D10 is still open"
   doc comments, fixing a genuine pre-existing `GEOM`-family code
   collision a cross-crate audit found, and adding real-diagnostic schema
   conformance evidence.

## Base / resulting commit

- Base: `16a2f38` (`AICAD-077`, `origin/claude/aicad-stage3-dev`'s HEAD
  at the start of this task within the same invocation).
- This task's commit: see `git log` on `origin/claude/aicad-stage3-dev`
  (single commit, `AICAD-078`).

## What was implemented

### 1. `shell` Safe CAD builtin

New `BuiltinFnId::Shell` / catalogue entry: `shell(target: Geometry,
removed_faces: List<Int>, thickness: Length) -> Geometry`. Dispatches to
the already-existing `GeometryOp::Shell` — no new IR variant, no new
kernel capability. `removed_faces` reuses `fillet`/`chamfer`'s own raw-
index-selection convention (a plain `List<Int>`, new `face_indices`
closure in `cad_runtime::interp::dispatch_builtin` mirroring the existing
`edge_indices`), and unlike `fillet`/`chamfer`'s `edges`, an empty
`removed_faces` list is explicitly permitted (a fully closed shell,
matching `GeometryOp::Shell`'s own existing contract).

**Sign-convention finding, fixed at the builtin boundary.**
`Shape::shell`'s own already-established kernel convention
(`crates/cad-occt-bridge`'s `shell_hollowed_box_matches_analytic_volume`
test) is "negative thickness hollows inward, positive builds material
outward" — discovered when this task's own end-to-end test first failed
with a positive-thickness source literal. Rather than exposing that sign
convention to Safe CAD source (which would be surprising: an ordinary
positive `Length` reads as "hollow inward" to any author, matching
`docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s own `inward: Bool = true`
default), `dispatch_builtin`'s `Shell` arm negates the evaluated
magnitude before building the `GeometryOp::Shell` node. No new
parameter was added for this — it is the one fixed behavior, not a
caller-visible option, matching `linear_pattern`/`radial_pattern`'s own
narrowing precedent of dropping an optional plan parameter whose default
is the builtin's own fixed behavior.

Four new tests: `shell_call_builds_a_single_shell_node_with_the_given_
removed_faces` (structural, `cad-runtime`, asserts the stored node's
`thickness.magnitude` is the *negated* source literal); `shell_call_
permits_an_empty_removed_face_list` (`cad-runtime`); and
`shell_builtin_end_to_end_hollows_a_box_with_one_face_removed`
(`cad-geometry-runtime`, a real kernel-backed proof: hollows a cube with
one face removed and checks the resulting volume against the closed-form
remaining-material formula — a cube was deliberately chosen so the
expected volume is invariant to which of the cube's six faces the kernel
happens to enumerate as index 0, sidestepping `GeometryOp::Shell`'s own
already-documented raw-index/kernel-enumeration-order limitation without
weakening the check).

### 2. Normalized diagnostics

**Stale "D10 still open" doc comments fixed**, now that `DL-18` resolves
D10: `crates/cad-diagnostics/src/lib.rs` (module doc comment and
`DIAGNOSTIC_FAMILIES`'s own doc comment), `crates/cad-runtime/src/
error.rs` (module doc comment and `RuntimeError::code`'s own doc
comment), `crates/cad-units/src/arithmetic.rs`
(`DimensionalArithmeticError::code`'s own doc comment), `crates/cad-cli/
src/build.rs` (a code comment explaining the CLI's own out-of-range
`PARSE-E900` choice), and `specs/schemas/diagnostic.schema.json`'s own
`description` field (previously "Field/code stability policy is NOT
covered by this schema — see project/OWNER_DECISIONS.md D10 (open)").

**Schema versioned, per `DL-18`'s own requirement** ("Machine-readable
diagnostic schemas ... are versioned"): added a `"$comment"` field to
`specs/schemas/diagnostic.schema.json` recording it as schema version 1,
with an instruction to bump it and record any future compatibility-
breaking change in `project/DECISION_LOG.md`. `$comment` is a standard
JSON Schema annotation-only keyword; `cad_diagnostics::schema::Schema`'s
own minimal validator (which only inspects `type`/`required`/
`properties`/`items`/`enum`) ignores it entirely — confirmed by the full
`schema_conformance.rs` suite still passing unmodified.

**A genuine `GEOM`-family code collision found and fixed.** A cross-crate
audit of every literal diagnostic code in the workspace (the kind of
check `DL-18`'s stability policy now makes actually matter) found that
`cad_feature_graph::FeatureGraphError` (`UnresolvedGeometryInput`/
`MalformedBuiltinCall`, introduced by `AICAD-068`/`069`) and
`cad_geometry_runtime::dispatch::DispatchError` (`Kernel`/
`GraphInvariantViolated`, introduced earlier by `AICAD-060`) had
independently claimed the identical two codes — `GEOM-E005` and
`GEOM-E006` — for four entirely unrelated conditions, with no shared
registry to have caught it. Fixed by renumbering the younger assignment
(`FeatureGraphError`, Stage 3) to the next free `GEOM` codes,
`GEOM-E007`/`GEOM-E008` (`GEOM-E001`-`004` = `GeometryIrError`,
`E010`-`015` = `SketchIrError`, `E020`-`024` = `SketchLoweringError`, all
already in use and left untouched). This is ordinary bug-fixing, not a
`DL-18` violation or an owner escalation: neither collision had been
included in any completed Stage gate's own frozen evidence at the time of
this fix (`project/gates/STAGE3-A_PARAMETRIC_GRAPH.md`'s own text citing
`GEOM-E005`/`GEOM-E006` describes `FeatureGraphError`'s pre-this-task
behavior faithfully as of that gate's own sealed commit and is left
unedited, as a gate's historical record should be — the renumbering is
recorded here, in the task that made it, not retrofitted into an already-
passed checkpoint).

**Real-diagnostic schema conformance evidence added**, per `AGENTS.md`'s
evidence rule (normalizing diagnostics should demonstrate real producers
stay conformant, not merely assert a shape). Two new tests in
`crates/cad-diagnostics/tests/schema_conformance.rs`:
`a_real_geometry_ir_error_diagnostic_conforms_to_the_schema` (builds an
actual `cad_geometry_api::GeometryIrError::EmptyOperandList`'s own
`to_diagnostic()` output and validates it against the checked-in schema
— not a hand-built `GEOM-E###` stand-in) and `the_renumbered_feature_
graph_codes_do_not_collide_with_dispatch_error_codes` (a direct
uniqueness check over every `GEOM`-family code this workspace currently
defines, so a future reintroduced collision fails loudly here rather than
waiting to be found by another cross-crate audit). `cad-diagnostics`
gained `cad-ast`/`cad-geometry-api` as `[dev-dependencies]` only (no
normal-dependency cycle: `cad-geometry-api`'s own lib depends on
`cad-diagnostics`, but `cad-diagnostics`'s lib still depends on nothing —
only its *test* binary now also depends on `cad-geometry-api`, which
Cargo permits).

## Files changed

- `crates/cad-hir/src/builtins.rs` — `BuiltinFnId::Shell` + catalogue
  entry.
- `crates/cad-runtime/src/interp.rs` — `face_indices` closure, `Shell`
  dispatch arm (with the thickness-negation fix), `builtin_name` entry,
  two new tests, one existing test's assertion corrected for the negation.
- `crates/cad-geometry-runtime/src/dispatch.rs` — one new kernel-backed
  end-to-end test.
- `crates/cad-diagnostics/src/lib.rs`, `Cargo.toml` — doc comments
  updated; `cad-ast`/`cad-geometry-api` dev-dependencies added.
- `crates/cad-diagnostics/tests/schema_conformance.rs` — two new tests.
- `crates/cad-runtime/src/error.rs`, `crates/cad-units/src/arithmetic.rs`,
  `crates/cad-cli/src/build.rs` — stale D10-provisional doc
  comments/code comments updated.
- `crates/cad-feature-graph/src/graph.rs` — `GEOM-E005`/`GEOM-E006` ->
  `GEOM-E007`/`GEOM-E008` (collision fix), doc comment, one test
  assertion.
- `specs/schemas/diagnostic.schema.json` — `description` updated; new
  `$comment` schema-version annotation.
- `docs/API/safe-cad-api.md` — new `shell` documentation section.

## Design decisions

1. **`shell` always hollows inward; no `inward` parameter.** See "What
   was implemented" above — the alternative (exposing `Shape::shell`'s
   raw sign convention directly, or adding an `inward: Bool` parameter)
   was rejected as unnecessary: this catalogue's own established pattern
   is to drop an optional plan parameter entirely when one fixed behavior
   already matches that parameter's own documented default, not to thread
   it through as a real parameter.
2. **Fix the code collision by renumbering, not by any DL-18 escalation.**
   Considered treating the found collision as a `DL-18`-governed "durable
   code" conflict needing an owner ruling. Rejected: `DL-18`'s own policy
   only protects a code already used by a *merged commit this policy
   covers going forward*; both colliding assignments predate `DL-18`
   itself, and neither had been cited by a completed Stage gate as frozen
   evidence (`STAGE3-A_PARAMETRIC_GRAPH.md`'s own citation describes
   `FeatureGraphError`'s pre-fix behavior, not a promise that exact code
   stays `GEOM-E005` forever). Renumbering the objectively younger
   assignment to a free code is ordinary bug-fixing — exactly the kind of
   problem a "normalize diagnostics" task exists to catch before `DL-18`
   would have made fixing it harder.
3. **No new required field on every `Diagnostic` instance for schema
   versioning.** `DL-18` requires the *schema* be versioned, not that
   every diagnostic instance carry a version number. A `$comment`
   annotation on the schema file itself satisfies this with no change to
   `cad_diagnostics::Diagnostic`'s own shape, `to_json()` output, or any
   existing conformance test — the smallest correct solution, not a
   speculative versioning mechanism no current consumer needs.
4. **`cad-ast`/`cad-geometry-api` as `cad-diagnostics` dev-dependencies,
   not a new crate.** Considered a separate `cad-diagnostics-tests`-style
   integration crate to avoid touching `cad-diagnostics`'s own
   `Cargo.toml`. Rejected as unneeded complexity: Cargo's dev-dependency
   mechanism exists precisely for this "a leaf crate's own test suite
   needs a real consumer crate, which depends on the leaf crate normally"
   pattern, and does not introduce a build-graph cycle (only
   `cad-diagnostics`'s test binaries gain the dependency, never its own
   library target).

## Tests / verification

- `cargo fmt --all -- --check` → clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings, full workspace.
- `cargo test --workspace` → 996 passed, 0 failed (was 991 after
  `AICAD-077`; +5: +2 `cad-runtime` (`shell` builtin), +1
  `cad-geometry-runtime` (`shell` end-to-end), +2 `cad-diagnostics`
  (schema-conformance/collision evidence)).
- `cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1` →
  3/3 (Stage-2 gate proof unaffected).
- `cargo test -p cad-feature-graph` → 34/34 (the renumbered-code test
  assertion updated and passing).

Per-crate test counts (this invocation's changes only):

| Crate | Before | After | Delta |
|---|---|---|---|
| `cad-runtime` (lib) | 133 | 135 | +2 (`shell` builtin) |
| `cad-geometry-runtime` (lib) | 23 | 24 | +1 (`shell` end-to-end) |
| `cad-diagnostics` (lib + `schema_conformance.rs`) | 30 | 32 | +2 (real-diagnostic conformance + collision check) |
| `cad-feature-graph` (lib) | 34 | 34 | +0 (one existing assertion updated, not a new test) |

## Limitations

- `shell` has no `join: OffsetJoin` parameter (`docs/plan/
  04_HIGH_LEVEL_MODELING_API.md`'s own `shell` signature) — OCCT's
  default corner-join behavior is used unconditionally, matching
  `offset`'s own existing identical narrowing.
- This task fixed the one `GEOM`-family collision its own audit found; it
  did not re-audit every other family (`RUNTIME`, `BUDGET`, `CONSTRAINT`,
  ...) for a similar problem beyond the spot-checks in "What was
  implemented" — a full cross-family audit was judged out of this task's
  own bounded "normalize the new Stage-3 diagnostics" scope, not a gap
  left silently.
- No CLI/tooling-facing change (e.g. a new `cad-cli` command surfacing
  the diagnostic family/code registry) — `DL-18` itself required no such
  tooling, only the stability policy and versioned-schema commitments
  this task satisfies directly.

## Regressions

None new. The one pre-existing bug this task fixed (the `GEOM-E005`/
`GEOM-E006` collision) is documented above, not hidden as a "regression"
of this task's own making — it predates this task and this task is what
found and fixed it.

## Next dependency

Batch S3-07 (`AICAD-077`, `AICAD-078`) is now complete. Per `project/
CURRENT_STAGE.md`'s fixed batch list, the next batch is S3-08
(`AICAD-079`, "Implement named semantic outputs baseline + ordinary-part
examples + AI benchmark seed"), depends on `AICAD-078` (satisfied).
`project/gates/STAGE3-C_MODELING.md` is prepared only after Batches
S3-06/S3-07/S3-08 all complete — S3-06/S3-07 are now both done; S3-08
remains. Per the campaign brief ("Each invocation works on exactly ONE
fixed batch"), this invocation stops here at the end of S3-07 rather than
continuing into S3-08.
