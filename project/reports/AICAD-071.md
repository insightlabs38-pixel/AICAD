# AICAD-071: Implement part concept plus box/cylinder/plate standard helpers

## Status

Done.

## Objective

Give `part { ... }` bodies real execution semantics (previously a pure,
inert declaration — `HirItem::Part` was silently skipped by every
interpreter pass) and extend the Safe CAD standard-function catalogue with
`plate`, per `AGENTS.md`'s Stage-3 mission text: "a `Part` concept built on
the existing D18 runtime-backed-function mechanism." Second and final task
of Batch S3-03, depends on `AICAD-070`'s `Value::Struct`/field-access
machinery.

## Base / resulting commit

- Base: `AICAD-070`'s own commit on `origin/claude/aicad-stage3-dev`.
- This task's commit: see `git log` on `origin/claude/aicad-stage3-dev`
  (single commit, `AICAD-071`).

## Scoping this task before implementing

"The Part concept" is used inconsistently across the plan bundle at very
different levels of ambition: a plain `part Name { ... }` item-scope
declaration (the only form the current, frozen grammar actually has —
`cad_ast::item::Item::Part { name, items }`, no parameter list on the
header, no call/instantiation syntax); a `-> Part` high-level return type
(`docs/plan/04...`'s `gear`/`enclosure` features); and, in the (explicitly
non-compilable, aspirational, cross-many-future-stages) `docs/plan/
18_REFERENCE_EXAMPLES.md`/`examples/assemblies/stage0_paper_example.aicad`
paper examples, a fully parameterized, instantiable, composable unit
(`instance bracket = Bracket();`, `constraint { ... }` blocks inside a
part, `expose`d named outputs, assembly `mate`s) — none of which exist
anywhere in the actual Stage-2 grammar or type checker today.

Building the full paper-example vocabulary now would mean inventing
`constraint`/`expose`/`instance`/`assembly`/`mate` syntax with no RFC and
no Stage-3 batch task naming them (`AGENTS.md` escalation trigger: "public
syntax/semantics must change beyond an approved RFC"), and would reach
squarely into assemblies/configurations — explicitly listed in `project/
CURRENT_STAGE.md`'s "Not allowed yet" for Stage 3. This task instead scopes
"the Part concept" to exactly what the *existing* `part { ... }` grammar
already supports today: giving that already-parseable, already-lowered,
already-type-checked (its own nested items were already type-checked by
`AICAD-052`/`053`'s `register_type_names`/`collect_signatures` recursing
into `HirItem::Part`) construct a real runtime effect for the first time.

## Files changed

- `crates/cad-runtime/src/value.rs` — `Value::Part { binding: BindingId,
  fields: Vec<(String, Value)> }` (added alongside `AICAD-070`'s
  `Value::Struct` in the same enum edit; see that task's own report for
  the shared diff).
- `crates/cad-runtime/src/interp.rs`:
  - `Interpreter::run_top_level` now executes each top-level `HirItem::
    Part` via new `Interpreter::eval_part_body`, binding the resulting
    `Value::Part` into `globals` under the part's own `BindingId` (was:
    silently skipped, alongside `fn`/`struct`/`enum`/`import`).
  - New `Interpreter::eval_part_body`: evaluates a part's own top-level
    `let`/`const`/`param`-with-default items, in source order, into one
    shared nested scope (mirrors `run_top_level`'s own naive pass, and
    reuses the identical `Signal` conversion `eval_top_level_value` already
    established), collecting `(name, value)` pairs. A `param` with no
    default is excluded from the result, mirroring `run_top_level`'s own
    pre-existing "left unpopulated" convention for the identical case at
    top level. Nested `fn`/`struct`/`enum`/`part`/`import` items inside the
    body are skipped (see "Design decisions" below).
  - New `Interpreter::global(binding) -> Option<&Value>` — a public
    accessor for a top-level binding's current value (previously private/
    untestable directly; existing tests always went through
    `call_by_name`'s indirect route). Needed because there is no
    `.`-syntax source access to a part's own outputs yet (see below), so
    this is the only way a caller (a test, a future `cad-cli`) can observe
    what a part produced.
  - `run_top_level_parametric` is unchanged (still skips `HirItem::Part`)
    — documented explicitly as a deliberate scope cut, not an
    inconsistency: wiring parametric-rebuild support into `part` execution
    has no forcing task or evidence yet.
- `crates/cad-hir/src/builtins.rs` — new `BuiltinFnId::Plate` variant and
  catalogue entry: `plate(width: Length, depth: Length, thickness: Length)
  -> Geometry`. `BuiltinFnId::ALL` grew from 8 to 9 entries.
- `crates/cad-runtime/src/interp.rs` (`dispatch_builtin`) — `BuiltinFnId::
  Plate` dispatches to the identical `GeometryOp::Box` construction `box`
  itself uses (`width/depth/thickness` -> `dx/dy/dz`); `builtin_name` gained
  the `"plate"` arm.
- `docs/API/safe-cad-api.md` — "Stage-3 additions" section (shared edit
  with `AICAD-070`'s report) documents `plate`'s exact signature/rationale
  and the Part concept's scope.

## Design decisions

1. **No parameterized part instantiation (`Bracket()`), no `.`-syntax
   output access (`Bracket.body`).** Neither exists in the grammar/type
   checker today (see "Scoping this task" above) and adding either is a
   genuinely separate, larger decision (a `CheckedType::Part`, a
   `struct_fields`-equivalent table for parts, and/or new call-site
   instantiation semantics) with no forcing task naming it. Building only
   the runtime execution half — while leaving these two clearly documented
   as explicit non-goals rather than silently limited — matches this
   codebase's own established precedent (`D16`/`D17`/`D18` each drew an
   identical line between "what this task's own forcing evidence requires"
   and "what a future task should decide").
2. **A part's own body is evaluated exactly once, in source order, into
   one flat nested scope** — not lazily, not memoized across multiple
   `run_top_level` calls (there is only ever one), and not recursively
   into nested `part`-in-`part` bodies (skipped, matching `fn`/`struct`/
   `enum`/`import`'s own precedent — no evidence anywhere requires nested
   part execution yet, and skipping keeps this task's own diff minimal and
   auditable rather than guessing at a nesting semantics no example
   exercises).
3. **`Value::Part` is a separate variant from `Value::Struct`, not a reuse
   of it**, even though both carry an identical `(name, Value)` field
   shape. A `part` is not a `struct` declaration: there is no `cad_hir::
   typeck::struct_fields` entry for a part's own binding and no
   `CheckedType::Part`, so treating a part's own runtime value as if it
   were a genuine struct instance would be misleading to any future reader
   pattern-matching on `Value::Struct::ty` expecting a real
   `BindingKind::Struct`. `HirExpr::Field` evaluation still handles both
   uniformly (one `match` arm, `Value::Struct { fields, .. } | Value::Part
   { fields, .. }`) since the field-lookup-by-name operation itself is
   identical.
4. **`plate` reuses `GeometryOp::Box` verbatim rather than adding a new
   Geometry IR variant.** A flat rectangular plate is geometrically
   identical to a box with `dz = thickness` — `cad_geometry_api::ir`
   already documents its own scope cut against adding IR surface with no
   forcing requirement ("Scope cut: no low-level topology-exploration
   ops"), and the same reasoning applies here: no new capability is needed,
   only a second Safe-CAD-source name for an existing one. This keeps the
   change entirely inside `cad-hir`/`cad-runtime`, touching neither
   `cad-geometry-api` nor `cad-geometry-runtime`/`cad-occt-bridge`.
5. **`plate` stays scalar (`width`/`depth`/`thickness: Length`), not
   `size: Vector2<Length>`.** `AICAD-070`'s new geometry types exist, but
   wiring one into a builtin signature is a separate, explicit non-goal of
   that task (see its own report, "Design decisions" #4) — `plate` follows
   that same boundary rather than being the first builtin to cross it
   unreviewed. `corner_radius`/`frame`/`centered` (present in `docs/plan/
   04...`'s own `plate` signature) are omitted for the same reason
   `transform`'s own translate-only narrowing was: `corner_radius` needs
   low-level wire/face construction outside this catalogue's current
   scope, and `frame`/`center` placement composes with an ordinary
   `transform` call instead of a new parameter.

## Test coverage

All in `crates/cad-runtime/src/interp.rs`, new:

- `part_body_executes_and_exposes_named_outputs` — a part with a `param`
  (using its default) and a derived `let` executes in order; both appear
  in the resulting `Value::Part::fields` with correct canonical-unit
  magnitudes.
- `part_body_can_call_safe_cad_builtins_and_expose_geometry` — a part's
  `let` calling `box(...)` produces a `Value::Geometry` field and appends
  exactly one node to the interpreter's shared `GeometryGraph` — the
  concrete proof that a part body is "built on the existing D18
  runtime-backed-function mechanism," not a separate invocation path.
- `a_param_with_no_default_is_left_out_of_a_part_s_exposed_fields` — mirrors
  `run_top_level`'s own pre-existing "left unpopulated" convention inside a
  part body specifically.
- `a_part_not_yet_run_top_level_ed_has_no_global_value` — `Interpreter::
  global` returns `None` before `run_top_level` runs (mirrors the
  pre-existing `top_level_const_referenced_before_run_top_level_is_unbound`
  test's own convention, for the new accessor).
- `a_part_body_can_call_a_fn_declared_inside_the_same_part` — a part-local
  `fn` is callable from that same part's own `let` expressions (`index_fns`'
  pre-existing recursion into `part` bodies already made this possible;
  this confirms it still holds now that the body around it actually
  executes). Note: a part-declared `fn` is **not** callable from sibling
  top-level code — confirmed while writing this test (an earlier draft
  assumed otherwise and failed to lower with `UNRESOLVED_BINDING`, TYPE-
  E410): `index_fns`/`collect_signatures` recurse into parts for
  type-checking/dispatch purposes, but name-binding scope itself does not
  leak a part's inner names to sibling declarations. Pre-existing behavior,
  unchanged by this task, not re-tested further here since it is outside
  this task's own scope.
- `plate_call_dispatches_to_a_box_geometry_node` — `plate(40mm, 25mm,
  3mm)` produces exactly the `GeometryOp::Box { dx: 0.040, dy: 0.025,
  dz: 0.003 }` node `box`'s own equivalent test expects for its own
  arguments, confirming the verbatim reuse.

## Exact verification commands/results

```
cargo fmt --all -- --check
```
→ clean.

```
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
→ zero warnings across the full workspace.

```
cargo test --workspace
```
→ 0 failures across every crate with tests. `cad-runtime`: 110 (104 after
`AICAD-070`'s own struct-value work, +6 here: 5 part-execution tests + 1
`plate` dispatch test). `cad-hir`: unchanged at 202 (the pre-existing
generic `catalogue_has_exactly_one_entry_per_builtin_fn_id`/
`every_catalogue_name_is_unique` tests already cover the new `Plate`
variant with no new test needed).

```
cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1
```
→ 3/3 (Stage-2 gate proof unaffected — `box`/`cylinder`/`transform`/
`union`/`cut`/`intersect`/`fillet`/`chamfer` signatures/dispatch are
untouched by this task).

## Limitations / explicit non-goals

- No parameterized part instantiation call syntax (`Bracket()`) — see
  "Design decisions" #1.
- No `.`-syntax source-level access to a part's own named outputs
  (`Bracket.body`) — same.
- No nested `part`-in-`part` execution — see "Design decisions" #2.
- `run_top_level_parametric` does not execute `part` bodies (unchanged by
  this task) — parametric rebuild support for parts is unscoped future
  work, not silently broken.
- `plate` has no `corner_radius`/`frame`/`center`/`centered` parameter —
  see "Design decisions" #5.
- No new workspace dependency was added.

## Regressions

None found; `cargo test --workspace` and the Stage-2 end-to-end gate both
pass unchanged.

## Next dependency

Batch S3-03 is now complete (`AICAD-070`, `AICAD-071`). Per `project/
CURRENT_STAGE.md`'s fixed batch order, no checkpoint gate lands at the end
of this particular batch — the next checkpoint (`STAGE3-B_SKETCH_
CONSTRAINTS.md`) lands after Batch S3-05. The next batch is **S3-04**
(`AICAD-072`, "Create minimal sketch entity IR"), which should read
`docs/plan/04_HIGH_LEVEL_MODELING_API.md` and `project/DECISION_LOG.md
#DL-19` (D3 — sketch entity/object model) before starting.
