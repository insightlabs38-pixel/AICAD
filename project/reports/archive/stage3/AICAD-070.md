# AICAD-070: Create safe language-facing geometry types

## Status

Done.

## Objective

`docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s Safe CAD feature catalogue
(`box`, `cylinder`, `plate`, `revolve`, ...) is written throughout against
a small vocabulary of geometry value types — `Vector2<Length>`/
`Vector3<Length>`, `Point2`/`Point3`, `Axis3`, `Frame3` — that Stage 2's own
catalogue never needed (Stage 2's `box`/`cylinder`/`transform` all take
plain scalar `Length` parameters; `docs/API/safe-cad-api.md`'s own
`transform` entry explains why: "no AICAD source-level vector/axis/rotation
type exists yet"). This task gives Safe CAD source that vocabulary, first
task of Batch S3-03.

## Base / resulting commit

- Base: `project/gates/STAGE3-A_PARAMETRIC_GRAPH.md`'s own commit on
  `origin/claude/aicad-stage3-dev` (Batch S3-02 checkpoint).
- This task's commit: see `git log` on `origin/claude/aicad-stage3-dev`
  (single commit, `AICAD-070`).

## What this task found before implementing

Auditing `cad_hir::typeck` and `cad_runtime` before writing anything found
the actual gap was **not** "no way to declare these types" — general
generic `struct` declaration syntax (`struct Pair<T, U> { ... }`,
`AICAD-057B`) and full struct-construction/field-access *type checking*
(`Checker::check_struct_construction`/`check_field_access`) already existed
and already worked for any user-declared struct, generic or not. The real
gap was one crate lower: `cad_runtime::value::Value` had no struct-instance
variant at all (its own module doc comment documented this explicitly: "no
Stage-2 batch task title yet owns giving struct construction a runtime
value"), so `Interpreter::call`'s `BindingKind::Struct` case fell through
to `RuntimeError::NotCallable`, and `HirExpr::Field` unconditionally
returned `RuntimeError::Unsupported`. Declaring `Vector3<Length>` etc. as
inert types with no way to actually construct or read one would have been
hollow, so this task's real content is completing that already-approved
general struct-value machinery, then declaring the six geometry types on
top of it.

## Files changed

- `crates/cad-runtime/src/value.rs` — new `Value::Struct { ty: BindingId,
  fields: Vec<(String, Value)> }` and `Value::Part { binding: BindingId,
  fields: Vec<(String, Value)> }` (the latter is `AICAD-071`'s, added here
  since both variants land in the same enum edit — see that task's own
  report). `kind_name`/`operand_type` updated for both.
- `crates/cad-runtime/src/interp.rs`:
  - `Interpreter` gains a `structs: HashMap<BindingId, &'a HirItem>` index
    (new `index_structs`, mirroring the existing `fns`/`index_fns` exactly,
    including recursion into `part` bodies).
  - `Interpreter::call`'s `BindingKind::Struct` arm now calls new
    `Interpreter::construct_struct` (positional/named argument matching
    against the struct's own declared field order) instead of falling
    through to `NotCallable`.
  - `HirExpr::Field` evaluation now looks up the field by name on a
    `Value::Struct`/`Value::Part` receiver (`RuntimeError::UnknownField` if
    absent — defensive for `Struct`, since `check_field_access` already
    guarantees this at compile time; genuinely reachable for `Part`, which
    has no compile-time field check yet) instead of unconditionally
    returning `Unsupported`.
- `crates/cad-runtime/src/error.rs` — new `RuntimeError::
  StructConstructionArgumentShape`/`RuntimeError::UnknownField`
  (`RUNTIME-E126`/`RUNTIME-E127`); updated the now-stale doc comments on
  `NotCallable`/`Unsupported` that previously documented the struct-value
  gap this task closes.
- `crates/cad-hir/src/geometry_types.rs` (new) — `GEOMETRY_TYPES_SOURCE`
  (six `struct` declarations: `Vector2<T>`, `Vector3<T>`, `Point2`,
  `Point3`, `Axis3`, `Frame3`) and `with_geometry_types`, mirroring
  `crate::prelude::with_prelude`'s exact mechanism (parse fixed AICAD
  source text, prepend to a user program) and rationale.
- `crates/cad-hir/src/lib.rs` — exports `geometry_types`/
  `GEOMETRY_TYPES_SOURCE`/`with_geometry_types`; module doc comment
  updated.
- `docs/API/safe-cad-api.md` — new "Stage-3 additions" section documenting
  the six types and their deliberate scope limits; status line updated
  from "Stage 2, authoritative for the current Stage-2 window" to "Stage 2
  (authoritative baseline) plus Stage 3 additions."

## Design decisions

1. **A struct's own type-parameter instantiation is erased at runtime**,
   exactly like `Value::List`'s element type already is (per that variant's
   own pre-existing doc comment). `Value::Struct::ty` is the struct's
   *declaring* `BindingId`, not a per-instantiation identity — a
   `Vector3<Length>` and a `Vector3<Mass>` share one identical runtime
   shape. This follows the crate's own established precedent rather than
   inventing a new runtime-generics representation.
2. **Field order is declaration order, not construction order** — mirrors
   `VariantPayload::Record`'s own identical, already-documented convention
   (a field's value is always looked up by name, so order has no observable
   effect; kept for symmetry with that precedent, not because it matters
   operationally).
3. **Kept as a separate module/file from `crate::prelude`**, not folded
   into `PRELUDE_SOURCE`. `Result<T,E>`/`Optional<T>` are `D17`'s
   general-purpose language machinery; these six types are Stage-3
   Safe-CAD-specific. A caller wanting both composes `with_prelude` and
   `with_geometry_types` itself — order does not matter, per `with_prelude`'s
   own "not load-bearing" note, which applies identically here.
4. **Not wired into any builtin signature.** No existing Stage-2 builtin
   (`box`/`cylinder`/`transform`) was changed to accept a
   `Vector3<Length>`/`Point3`/`Frame3` parameter, and `AICAD-071`'s new
   `plate` builtin deliberately stays scalar-only too. Changing `box`/
   `cylinder`'s existing signatures risks the already-proven `AICAD-063`
   Stage-2 gate fixture for no requirement any current task actually
   states; `Frame3`/`Axis3` are passive data shapes only — establishing a
   coherent axis/frame/rotation *semantics* for revolve/transform/mirror/
   circular-pattern is `AICAD-075A`'s own explicitly assigned task (per the
   active campaign brief), not this one's. Documented explicitly in
   `docs/API/safe-cad-api.md` and `geometry_types.rs`'s own module doc
   comment rather than silently left implicit.
5. **No `CheckedType::Part`/typeck changes in this task.** Struct
   construction/field-access type checking already existed before this
   task (see "What this task found" above) — this task only completed the
   *runtime* side for structs. The `Value::Part` variant and its own
   (deliberately narrower, typeck-untouched) scope belong to `AICAD-071`'s
   own report.

## Test coverage

- `crates/cad-runtime/src/interp.rs` (6 new tests): named-argument struct
  construction produces the correct `Value::Struct`; positional-argument
  construction matches declared field order even when the caller's own
  argument order differs from field-name reading order; field access reads
  a constructed field's value; a *generic* struct instantiated via an
  explicit `let` type annotation (`Pair<Float, Bool>`) constructs and
  reads correctly, proving instantiation-erasure does not affect
  correctness. (The pre-existing `struct_construction_is_not_yet_supported`
  test, which asserted the old `NotCallable` failure, was rewritten into
  two positive tests — named and positional construction — now that the
  behavior it documented is implemented; this is a corrected assertion of
  fixed behavior, not a weakened test.)
- `crates/cad-hir/src/geometry_types.rs` (5 new tests): the fixed source
  text itself parses/lowers/type-checks cleanly alone; `with_geometry_types`
  prepends exactly six struct items before the user's own; user code
  constructs and reads a `Point3` field; user code constructs a `Frame3`
  from nested `Point3`/`Vector3<Float>` constructions; `Vector2<Length>`
  and `Vector2<Float>` are genuinely distinct types (assigning one where
  the other is declared is a real type error) — proving generic
  instantiation is enforced at the type level even though it is erased at
  runtime.

## Exact verification commands/results

```
cargo fmt --all -- --check
```
→ clean (after one `cargo fmt --all` pass to normalize this task's own new
code).

```
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
→ zero warnings across the full workspace.

```
cargo test --workspace
```
→ 0 failures across every crate with tests. `cad-hir`: 202 (was 197 before
this task — 5 new in `geometry_types.rs`; `cad-hir`'s pre-existing generic
catalogue tests already cover `AICAD-071`'s later `plate` addition with no
further new test needed there). `cad-runtime`: 104 immediately after this
task's own struct-value work (101 pre-existing minus the one rewritten test
plus 4 new positive ones); see `AICAD-071`'s own report for the further
+6 (part execution + `plate` dispatch) bringing the crate to its final 110.

```
cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1
```
→ 3/3 (Stage-2 gate proof unaffected — no existing builtin signature or
runtime behavior changed).

## Limitations / explicit non-goals

- No builtin function consumes any of the six new types yet (see "Design
  decisions" #4).
- `Frame3`/`Axis3` carry no operational semantics (no "rotate by this
  frame" operation exists) — purely passive data, by design; `AICAD-075A`
  owns giving them one.
- Struct *pattern* destructuring (`match p { Point3 { x, y, z } => ... }`)
  remains unimplemented — an already-documented, pre-existing Stage-2 scope
  boundary (`project/reports/AICAD-053.md`) this task does not touch.
- No new workspace dependency was added.

## Regressions

None found; `cargo test --workspace` and the Stage-2 end-to-end gate both
pass unchanged. The one test whose assertion changed
(`struct_construction_is_not_yet_supported` -> two positive tests) changed
because the behavior it documented was itself the target of this task, not
because coverage was removed.

## Next dependency

`AICAD-071` ("Implement part concept plus box/cylinder/plate standard
helpers"), the second and final task of Batch S3-03 — see its own report
for how it builds on this task's `Value::Struct`/field-access machinery
(reusing the identical `(name, Value)` field shape for `Value::Part`).
