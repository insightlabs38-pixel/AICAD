# AICAD-076A: Make the RuntimeBuiltin catalogue's standard type environment type-closed

## Status

Done. Interstitial task inserted into Batch S3-06 by owner ruling
(`project/DECISION_LOG.md#DL-21`, resolving `project/
OWNER_DECISIONS.md#D20`) after `AICAD-076` found the underlying
architecture gap. Batch S3-06 (`AICAD-075A`, `AICAD-076`, `AICAD-076A`)
is now complete.

## Objective

`AICAD-076` discovered that a `BuiltinFnId` catalogue entry referencing
an unseeded `cad_hir::geometry_types` struct type broke every other
compiled program's own type-checking (149 unrelated `cad-hir` tests),
and shipped a temporary scalar-decomposed workaround for `revolve`/
`hole`/`pocket` rather than guess at a fix. The owner ruled (`DL-21`):
make the always-seeded builtin environment type-closed — every nominal
type a seeded builtin's signature needs must itself be seeded
unconditionally, the same way the builtins are. This task implements
that ruling, migrates `revolve`/`hole`/`pocket` back to real `Axis3`/
`Frame3`-typed signatures, and adds the owner-specified catalogue-wide
zero-diagnostics invariant test.

## Base / resulting commit

- Base: `8bf1a7c` (`AICAD-076`, `origin/claude/aicad-stage3-dev`'s HEAD
  at the start of this invocation).
- This task's commit: see `git log` on `origin/claude/aicad-stage3-dev`
  (single commit, `AICAD-076A`).

## What was implemented

### 1. The standard type environment (`crates/cad-hir/src/lower.rs`)

New `Lowerer::seed_standard_types(&mut self, user_items: &[cad_ast::Item]) -> Vec<HirItem>`,
called from `lower_program` unconditionally, alongside (and using the
identical placement convention as) `seed_builtins`:

- Scans `user_items` (the caller's own already-parsed program, before
  any composition) for `Item::Struct` names, by name.
- Parses `cad_hir::geometry_types::GEOMETRY_TYPES_SOURCE` (the same
  fixed constant `with_geometry_types` already used) and lowers every
  one of its struct declarations *except* any whose name the caller's
  own program already declares.
- Appended to the final `HirProgram::items` after the user's own lowered
  items and after `builtin_items` — `register_type_names`/
  `collect_signatures` (in `crate::typeck`) both run as a full-item-list
  pass after lowering completes, so seeding order among these three
  groups has no effect on resolution correctness; only "user items come
  first" needed preserving, since several pre-existing tests assert
  `result.program.items[0]`/`[1]` directly.

This makes `Point2`/`Point3`/`Vector2<T>`/`Vector3<T>`/`Axis3`/`Frame3`/
`Plane` resolvable in **every** compiled program, unconditionally — no
`with_geometry_types` composition required — closing exactly the gap
`AICAD-076` found.

### 2. Idempotence with `with_geometry_types` (`DL-21`'s own requirement)

`seed_standard_types`'s own "skip already-declared names" check is what
makes composing `with_geometry_types` on top of the new unconditional
seeding safe: if a caller still calls it (backward-compatible, now
optional), the caller's own `program.items` already contains all seven
struct declarations by the time `lower_program` runs, so
`seed_standard_types` finds all seven names already present and adds
none — never two distinct `BindingId`s nominally named the same standard
type. Proven directly by a new test (`cad-runtime`,
`with_geometry_types_composition_still_works_alongside_the_always_
seeded_standard_types`) exercising a real `revolve` call after explicit
`with_geometry_types` composition.

The one pre-existing edge case checked directly: `crates/cad-hir/src/
lower.rs`'s own `ordinary_non_generic_declarations_lower_with_an_empty_
type_param_list` test declares its own local `struct Point2 { x: Length }`
(a single-field shape, unrelated to the standard `Point2`). Because
`seed_standard_types` skips any name the user's own items already
declare, the user's own `Point2` remains the only one — confirmed by
this test still passing unmodified.

### 3. The catalogue-wide invariant test (`crates/cad-hir/src/builtins.rs`)

New test `the_entire_builtin_catalogue_type_checks_against_an_otherwise_
empty_program` (the owner's own explicitly requested invariant): parses
an empty (`""`) source string, runs it through `lower_program` +
`check_program`, and asserts zero diagnostics from both — proving the
*entire* `BuiltinFnId` catalogue resolves cleanly with no caller
composition at all, plus a per-catalogue-entry check that every declared
builtin actually seeded a real `Fn` binding (ruling out a vacuous pass
where a signature's parameters all silently resolved to `None`). This is
the targeted test the owner asked for, so a future bad signature is
caught here directly rather than by an unrelated suite exploding.

### 4. Migrated `revolve`/`hole`/`pocket` back to real struct types

- `revolve(target: Geometry, face: Int, axis: Axis3, angle: Angle) ->
  Geometry` — `axis` converted via `cad_runtime::spatial::
  axis3_from_value` (`AICAD-075A`), restoring the arbitrary-origin
  capability the Batch S3-06 scalar-decomposed workaround had
  temporarily dropped.
- `hole(target: Geometry, axis: Axis3, diameter: Length, depth: Length)
  -> Geometry` — `axis` converted the same way.
- `pocket(target: Geometry, frame: Frame3, width: Length, length:
  Length, depth: Length) -> Geometry` — `frame` converted via
  `cad_runtime::spatial::frame3_from_value`.

`extrude`'s own `direction: Vector3<Float>` parameter is unchanged (it
was never affected by the original gap — see "Design decisions" below).
`crates/cad-runtime/src/interp.rs`'s `dispatch_builtin` restored the
`spatial_axis`/`spatial_frame` closures (removed by `AICAD-076`'s own
workaround) alongside the already-present `spatial_direction`.

### 5. Documentation

- `crates/cad-hir/src/builtins.rs`'s "Why no `Axis3`/`Frame3`-typed
  parameter yet" module note (added by `AICAD-076`) replaced with "The
  standard type environment" note describing the resolved state.
- `crates/cad-hir/src/geometry_types.rs`'s own module doc comment and
  `with_geometry_types`'s own doc comment updated: the function is now
  an optional, idempotent, backward-compatible composition helper, not a
  required step.
- `docs/API/safe-cad-api.md`'s `extrude`/`revolve`/`hole`/`pocket`
  section rewritten for the final `Axis3`/`Frame3`-typed signatures and
  the standard-type-environment rationale.
- `project/OWNER_DECISIONS.md#D20` marked RESOLVED, `project/
  DECISION_LOG.md#DL-21` records the full ruling.
- `project/CURRENT_STAGE.md`'s fixed batch list updated to include
  `AICAD-076A` in Batch S3-06.

## Files changed

- `crates/cad-hir/src/lower.rs` — `Lowerer::seed_standard_types`,
  `lower_program` doc comment and call site.
- `crates/cad-hir/src/builtins.rs` — `Revolve`/`Hole`/`Pocket` doc
  comments and catalogue entries re-typed; module note replaced; new
  catalogue-wide invariant test.
- `crates/cad-hir/src/geometry_types.rs` — module and `with_geometry_
  types` doc comments updated for the now-optional role.
- `crates/cad-runtime/src/interp.rs` — `spatial_axis`/`spatial_frame`
  closures restored; `Revolve`/`Hole`/`Pocket` dispatch arms consume real
  `Axis3`/`Frame3` values; four existing tests updated to the new
  signatures (three now use plain `compiled`, not `compiled_with_
  geometry_types`, directly demonstrating the fix); one new idempotence
  test.
- `crates/cad-geometry-runtime/src/dispatch.rs` — `hole_builtin_end_to_
  end_bores_a_clean_through_hole` updated to the `Axis3`-typed signature
  and plain (non-composed) source.
- `docs/API/safe-cad-api.md`, `project/OWNER_DECISIONS.md`, `project/
  DECISION_LOG.md`, `project/CURRENT_STAGE.md`, `project/TASKS.yaml` —
  documentation/decision-record updates.

## Design decisions

1. **Seed by name-skip, not by mutating the caller's own AST.**
   Considered making `with_geometry_types` itself "smart" (detect
   already-seeded types and no-op) instead of adding new `Lowerer`
   machinery. Rejected: `with_geometry_types` operates on `cad_ast::
   Program` before any lowering/binding exists, so it cannot know
   whether `lower_program` will *later* seed these types — the
   idempotence check has to live at the point that actually decides
   whether to add a duplicate, which is `seed_standard_types` inside
   `lower_program` itself.
2. **Struct-name-based skip check, not a general "already bound"
   check.** `seed_standard_types` only inspects `Item::Struct` entries
   in `user_items` — sufficient because `GEOMETRY_TYPES_SOURCE` declares
   nothing but structs, and simpler/cheaper than resolving full binding
   identity before any binding pass has run.
3. **`extrude`'s `direction: Vector3<Float>` was not touched.** It was
   never part of the original problem (a `Generic` reference to a
   user-defined generic struct fails silently, not eagerly, when
   unresolved) and remains a perfectly valid, minimal choice for a
   direction argument — `AICAD-076A`'s own scope is closing the gap for
   `Named` struct references, not converting every parameter to a
   struct type merely because it now can be.
4. **No change to `cad_hir::typeck`'s eager-resolution architecture.**
   `DL-21` explicitly does not authorize lazy per-call-site resolution;
   this task's fix works entirely by ensuring the eagerly-resolved
   names exist, not by deferring when they are resolved.

## Tests / verification

- `cargo fmt --all -- --check` → clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings, full workspace (29 crates).
- `cargo test -p cad-hir` → 227 passed, 0 failed (was 226 before this
  task; +1, the catalogue-wide invariant test).
- `cargo test --workspace` → 978 passed, 0 failed across every crate
  (was 976 before this task; +1 `cad-hir`, +1 `cad-runtime`).
- `cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1` →
  3/3 (Stage-2 gate proof unaffected).
- The exact pre-existing edge case this task's own design needed to get
  right (a user program declaring its own unrelated `struct Point2`)
  was checked directly: `cad-hir`'s own `ordinary_non_generic_
  declarations_lower_with_an_empty_type_param_list` test still passes
  unmodified.

Per-crate test counts (this invocation's changes only):

| Crate | Before | After | Delta |
|---|---|---|---|
| `cad-hir` | 226 | 227 | +1 (catalogue-wide zero-diagnostics invariant) |
| `cad-runtime` | 126 | 127 | +1 (`with_geometry_types` idempotence) |
| `cad-geometry-runtime` | 22 unit + 4 integration | 22 unit + 4 integration | +0 (one existing test's source/doc comment updated, not a new test) |

Every other crate's count is unchanged from `AICAD-076`'s own
last-verified totals.

## Limitations

- `pocket`/`hole`/`revolve`'s own *feature* narrowings (no `ThroughAll`,
  no counterbore/countersink/thread, `pocket`/`extrude`/`revolve` still
  operate only on an existing solid's own face, never an arbitrary
  sketch profile) are unchanged from `AICAD-076` — this task fixed the
  *type-system* limitation only, not the feature-completeness ones,
  which were never in scope for either task.
- `Point2`/`Vector2<T>` are now also unconditionally seeded (they are
  part of `GEOMETRY_TYPES_SOURCE`) even though no current builtin uses
  them — an acceptable, minimal side effect of seeding the whole fixed
  standard-types source as one unit, matching `seed_builtins`'s own
  existing "seed the whole catalogue, not just what today's callers use"
  precedent.
- `with_geometry_types` remains in the crate's public API as a
  backward-compatible helper (per `DL-21`'s own instruction), even
  though no code in this repository still needs to call it — a future
  cleanup task could consider removing it once confirmed genuinely
  unused, but `DL-21` did not ask for that and this task does not do it.

## Regressions

None. Every existing test passes unmodified except the four `AICAD-076`
tests intentionally updated to the new signatures (three of which now
demonstrate the fix directly by using plain `compiled` instead of
`compiled_with_geometry_types`).

## Next dependency

Batch S3-06 (`AICAD-075A`, `AICAD-076`, `AICAD-076A`) is now complete.
Per `project/CURRENT_STAGE.md`'s updated fixed batch list, the next batch
is S3-07 (`AICAD-077`, `AICAD-078`), with no checkpoint required between
S3-06 and S3-07. `AICAD-077` ("Implement mirror and linear/circular
pattern basics") now depends on `AICAD-076A` (satisfied), not
`AICAD-076` directly, and should design `mirror(target, plane: Plane)`
and `radial_pattern`'s own axis using real struct-typed parameters from
the start (`plane3_from_value`/`axis3_from_value`, both already
available from `AICAD-075A`) — the type-closed standard environment this
task established makes that safe now, with no scalar-decomposition
workaround needed. `AICAD-077` is also the task that must add mirror's
own `Plane3`-based native/kernel-adapter operation, since mirror cannot
be expressed as a `Transform` (`AICAD-075A`'s own "Rigidity (no
reflection)" finding) and no such kernel capability exists yet at any
layer.
