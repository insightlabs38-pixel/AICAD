# AICAD-077: Implement mirror and linear/circular pattern basics

## Status

Done. First task of Batch S3-07.

## Objective

Add `mirror`, `linear_pattern`, and `radial_pattern` to the Safe CAD
builtin catalogue. `AICAD-076A`'s own "Next dependency" note flagged that
`mirror(target, plane: Plane)` would need a real `Plane`-typed parameter
(now safe per the `AICAD-076A` type-closed standard environment) and that
mirror cannot be expressed as a `GeometryOp::Transform` (`AICAD-075A`'s
own "Rigidity (no reflection)" finding: every `Transform` this workspace
can produce is a proper rigid motion, determinant `+1`; a mirror is an
improper isometry, determinant `-1`) — so this task is also the one that
adds a genuinely new kernel capability end to end, not just a new builtin
over already-existing ops.

## Base / resulting commit

- Base: `cdd4ef2` (`AICAD-076A`, `origin/claude/aicad-stage3-dev`'s HEAD
  at the start of this invocation).
- This task's commit: see `git log` on `origin/claude/aicad-stage3-dev`
  (single commit, `AICAD-077`).

## What was implemented

### 1. Native kernel capability (`native/occt_bridge`)

New `aicad_occt_mirror_shape(context, handle, origin[3], normal[3],
out_handle)` (header + `.cpp`), deliberately a separate entry point from
`aicad_occt_transform_shape` rather than relaxing that function's own
rigidity check: it builds `gp_Ax2(origin, normal_dir)` and
`gp_Trsf::SetMirror(ax2)` (a true reflection), then
`BRepBuilderAPI_Transform`, matching `aicad_occt_transform_shape`'s own
"always produces a NEW handle, never mutates in place" contract. Keeping
these as two functions means no caller can ever construct a reflection
through the rigid-only `matrix` path — the existing determinant/
orthogonality re-validation in `aicad_occt_transform_shape` is completely
unaffected.

### 2. `cad-occt-bridge`: `Shape::mirror`

`ffi.rs` gained the matching `extern "C"` declaration; `lib.rs` gained
`Shape::mirror(&self, plane: Plane3) -> KernelResult<Shape<'ctx>>`,
following `Shape::transform`'s own pattern exactly (never mutates `self`,
`DL-2`). Four new tests prove: a world-plane mirror flips the expected
bounding-box axis and preserves volume; an off-origin plane reflects
about that plane, not the world origin; the source shape is not mutated;
and mirroring twice across the same plane round-trips back to the
original bounding box.

### 3. Geometry IR: `GeometryOp::Mirror` (`cad-geometry-api`)

New variant `GeometryOp::Mirror { target: GeomId, plane: Plane3 }`,
validated in `GeometryGraph::push_op` exactly like `Transform` (`target`
must be an earlier geometry-producing node in the same graph; `plane` is
already-validated pure math, needing no dimension check). Two new tests:
accepts a valid target and assigns the next sequential id; rejects an
invalid target operand with `GeometryIrError::InvalidOperand`.

### 4. Dispatch (`cad-geometry-runtime`)

`dispatch_op` gained a `GeometryOp::Mirror` arm calling
`target_shape.mirror(*plane)`, mirroring the existing `Transform` arm.
New integration test `mirror_op_dispatches_through_the_kernel_and_
preserves_volume` proves end-to-end kernel dispatch: the mirrored shape
is a valid B-rep, its volume is unchanged (a reflection is volume-
preserving), and its bounding box lands on the expected side of the
mirror plane — never a render-only check, per `AGENTS.md`'s evidence
rule.

### 5. `cad-hir` catalogue (`builtins.rs`)

Three new `BuiltinFnId` variants/catalogue entries:

- `mirror(target: Geometry, plane: Plane) -> Geometry`
- `linear_pattern(target: Geometry, direction: Vector3<Float>, count:
  Int, spacing: Length) -> Geometry`
- `radial_pattern(target: Geometry, axis: Axis3, count: Int, angle:
  Angle) -> Geometry`

`mirror`'s `plane: Plane` and `radial_pattern`'s `axis: Axis3` are real
struct-typed parameters from the start, per `AICAD-076A`'s own "Next
dependency" instruction — no scalar-decomposition workaround was needed
(the type-closed standard environment `AICAD-076A` established already
makes `Plane`/`Axis3` resolvable in every compiled program). The existing
catalogue-wide zero-diagnostics invariant test
(`the_entire_builtin_catalogue_type_checks_against_an_otherwise_empty_
program`) is parametrized over `BuiltinFnId::ALL`, so it automatically
covers the three new entries and passed without modification — direct
evidence the new signatures are safe against the standard environment.

### 6. Runtime dispatch (`cad-runtime::interp`)

`dispatch_builtin` gained three new arms:

- `Mirror`: converts `plane` via `crate::spatial::plane3_from_value`
  (already built by `AICAD-075A`), pushes one `GeometryOp::Mirror` node.
- `LinearPattern`: converts `direction` via `spatial_direction`
  (normalizing it), builds the union of `count` copies — the first left
  in place, each subsequent one `GeometryOp::Transform`-translated by an
  additional `spacing` along `direction` and `GeometryOp::Union`-ed into
  the accumulator, looped `count - 1` times.
- `RadialPattern`: converts `axis` via `spatial_axis`, builds the union
  of `count` copies the same way, each subsequent one rotated by an
  additional `angle / count` about `axis`.

Both patterns push **no new `GeometryOp` variant** — built entirely from
`Transform`/`Union`, matching `Hole`/`Pocket`'s own "domain-meaningful
name over existing ops" precedent, just looped.

New `pattern_count` closure reads the `Int` `count` argument and rejects
`count < 1` explicitly (`RuntimeError::InvalidPatternCount`,
`RUNTIME-E129`) — an `Int` parameter's sign/range is never a compile-time-
checkable property (the same reason `RuntimeError::InvalidSpatialArgument`
exists for a spatial value's numeric components), so this is a genuine
run-time condition, not a defensive-only check. `count == 1` is a valid,
explicitly-tested edge case: it returns `target` itself with zero new
graph nodes (no degenerate zero-length union).

Six new tests: `mirror_call_builds_a_single_mirror_node`;
`linear_pattern_builds_count_minus_one_translated_copies_unioned_in_
order` (verifies the exact `Transform`/`Union` tree shape and both
translation offsets by `assert_eq!` against independently-constructed
`Transform::translation` values); `linear_pattern_with_count_one_
returns_the_target_unmoved`; `linear_pattern_with_a_non_positive_count_
is_rejected`; the two analogous `radial_pattern_*` tests; plus the
existing `cad-hir` catalogue-wide invariant test automatically covering
the new signatures.

### 7. Documentation

`docs/API/safe-cad-api.md` gained a new "`mirror`/`linear_pattern`/
`radial_pattern` (`AICAD-077`)" section between the `extrude`/`revolve`/
`hole`/`pocket` section and the `Part` concept section, documenting the
exact signatures, the mirror-cannot-be-a-`Transform` rationale, the
placement conventions, and the narrowings relative to `docs/plan/
04_HIGH_LEVEL_MODELING_API.md`'s own `mirror`/`linear_pattern`/
`radial_pattern` feature descriptions.

## Files changed

- `native/occt_bridge/include/aicad_occt_bridge.h`,
  `native/occt_bridge/src/aicad_occt_bridge.cpp` —
  `aicad_occt_mirror_shape`.
- `crates/cad-occt-bridge/src/ffi.rs`, `crates/cad-occt-bridge/src/lib.rs`
  — `Shape::mirror` + four tests.
- `crates/cad-geometry-api/src/ir.rs` — `GeometryOp::Mirror` + validation
  + two tests.
- `crates/cad-geometry-runtime/src/dispatch.rs` — dispatch arm + one
  integration test.
- `crates/cad-hir/src/builtins.rs` — three new `BuiltinFnId`
  variants/catalogue entries.
- `crates/cad-runtime/src/error.rs` — `RuntimeError::InvalidPatternCount`
  (`RUNTIME-E129`).
- `crates/cad-runtime/src/interp.rs` — `spatial_plane`/`pattern_count`
  closures, three new dispatch arms, `builtin_name` entries, eight new
  tests.
- `docs/API/safe-cad-api.md` — new documentation section.

## Design decisions

1. **Mirror is its own `GeometryOp` variant, not a `Transform` field.**
   Considered widening `GeometryOp::Transform`'s own `transform` field to
   accept an improper isometry. Rejected: `cad_kernel_api::Transform`'s
   own type-level invariant ("every produced `Transform` is rigid,"
   proven by `every_produced_transform_is_rigid_within_native_tolerance`)
   would have to be weakened workspace-wide to let one caller build a
   reflection through it — a much larger, riskier change than adding one
   new variant and one new kernel entry point.
2. **A dedicated native function, not a relaxed `aicad_occt_transform_
   shape`.** Allowing `aicad_occt_transform_shape` to accept determinant
   `-1` matrices would mean any future caller building a `matrix` by hand
   could silently construct a reflection where a rigid motion was
   intended — a correctness trap `AGENTS.md`'s evidence rule argues
   against introducing. A separate, plane-parameterized entry point makes
   the two kinds of motion impossible to confuse at the type/API level.
3. **Patterns build no new `GeometryOp` variant.** Both `linear_pattern`
   and `radial_pattern` are expressible entirely as a fixed sequence of
   `Transform`+`Union` calls, matching `hole`/`pocket`'s own established
   "a domain-meaningful builtin name over already-existing ops" pattern.
   Introducing a `GeometryOp::Pattern` variant was considered and
   rejected as unnecessary complexity: it would need its own kernel-side
   batch-transform-and-fuse capability that does not exist and is not
   needed to satisfy this task's own "basics" scope.
4. **`radial_pattern` divides `angle` evenly, with no `include_endpoint`
   parameter.** `docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s own
   `radial_pattern` defaults `include_endpoint` to `false`, which is
   exactly this builtin's own fixed (and only) behavior — so the
   narrowing drops a parameter whose only legal default is already what
   this implementation always does, not a behavior change.
5. **`count` is a plain `Int`, validated at run time.** `cad_hir::typeck`
   already verifies `count`'s *shape* (it is an `Int`-typed expression);
   whether its *value* is `>= 1` is not a property any Stage-3 type
   system here can express, so it is checked explicitly at the one place
   the evaluated value is available — the same split `InvalidSpatialArgument`
   already established for `Axis3`/`Frame3`/`Plane` argument validity.

## Tests / verification

- `cargo fmt --all -- --check` → clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings, full workspace.
- `cargo test --workspace` → 991 passed, 0 failed (was 978 before this
  task; +13: +4 `cad-occt-bridge`, +2 `cad-geometry-api`, +1
  `cad-geometry-runtime`, +6 `cad-runtime`).
- `cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1` →
  3/3 (Stage-2 gate proof unaffected).

Per-crate test counts (this invocation's changes only):

| Crate | Before | After | Delta |
|---|---|---|---|
| `cad-occt-bridge` | 88 | 92 | +4 (`Shape::mirror`) |
| `cad-geometry-api` | 20 | 22 | +2 (`GeometryOp::Mirror`) |
| `cad-geometry-runtime` | 23 | 24 | +1 (dispatch integration test) |
| `cad-hir` | 227 | 227 | +0 (existing catalogue-wide invariant test automatically covers the 3 new entries) |
| `cad-runtime` | 127 | 133 | +6 (mirror/linear_pattern/radial_pattern builtin tests) |

## Limitations

- `mirror` has no `FaceRef` plane source (no source-level face-reference
  type exists yet — same gap `extrude`/`revolve`'s own `face: Int` raw-
  index narrowing documents) and no `merge` option; the caller composes
  `union` itself for a merged result.
- `linear_pattern`/`radial_pattern` operate on a single built `Geometry`
  value, not a `Feature|Shape|Fn` polymorphic source (no `Feature`/
  `FeatureGroup` concept exists yet) — a pattern builtin here returns
  `Geometry`, consistent with every other builtin in this catalogue.
- `linear_pattern` has no `centered` option (direction's sign already
  controls pattern extension); `radial_pattern` has no `include_endpoint`
  option (see "Design decisions" above).
- No rectangular (2-axis) or path pattern (`docs/plan/
  04_HIGH_LEVEL_MODELING_API.md`'s `rectangular_pattern`/`path_pattern`)
  — explicitly out of this task's "basics" scope.

## Regressions

None. Every existing test passes unmodified; the two line-reflow edits
`cargo fmt` made (inline `Vector3 { .. }` construction, a test string
literal) are pure formatting, no behavior change.

## Next dependency

Batch S3-07's second task, `AICAD-078` ("Implement high-level fillet/
chamfer/shell wrappers and normalized diagnostics"), depends on this task
(satisfied) and proceeds next in this same invocation per the campaign's
"execute the whole fixed batch" rule.
