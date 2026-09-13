# AICAD-060: Implement HIR/runtime geometry dispatch into Geometry IR/kernel API

## Status: COMPLETE

This task spanned two sessions. The first session implemented and tested
the `GeometryGraph -> kernel` dispatcher and the `NumberValue -> Quantity`
bridge, then escalated the open question of how `.aicad` source actually
*invokes* a geometry operation as `project/OWNER_DECISIONS.md#D18` (full
audit trail preserved there). The owner resolved `D18` (recorded as
`project/DECISION_LOG.md#DL-15`): AICAD supports a general, non-geometry-
specific "runtime-backed standard function" mechanism, and Safe CAD
geometry operations are its first concrete use. This session implemented
that mechanism and completed `AICAD-060`.

## Base commits

- First session: `06eb780` ("AICAD-059: Create backend-independent
  Geometry IR"), producing `19edf0d` ("AICAD-060: Implement Geometry IR ->
  kernel dispatch; escalate invocation surface as D18").
- This session: `19edf0d`, resuming from the owner's `D18` ruling.

## What the first session implemented (kept, not rewritten)

- `crates/cad-geometry-runtime::dispatch::dispatch_graph` — walks an
  already-validated `GeometryGraph` and calls the matching
  `cad_occt_bridge::OcctContext`/`Shape` operation for every `GeometryOp`/
  `GeometryQuery` variant, resolving `EdgeIndex`/`FaceIndex` to real
  edge/face `Shape`s at dispatch time.
- `crates/cad-geometry-runtime::bridge::number_value_to_quantity` — a
  direct field-copy conversion from a runtime `NumberValue` to a Geometry
  IR `Quantity`.
- A real bug fix in `AICAD-059`'s own `GeometryQuery::Tessellate` (missing
  angular-deflection field).

No part of this was discarded or rewritten this session, per the owner's
explicit instruction — the D18 ruling's own mechanism plugs into it
unchanged: this session's new `Interpreter`-owned `GeometryGraph` is
handed to the exact same `dispatch_graph` function.

## What this session implemented (resolving D18)

### 1. `FunctionImplementation`/`BuiltinFnId` (`crates/cad-hir`)

- `cad_hir::hir::FunctionImplementation` — `Aicad(HirBlock)` or
  `RuntimeBuiltin(BuiltinFnId)`. `HirItem::Fn::body`'s type changed from
  `HirBlock` to `FunctionImplementation` (the ruling's own preferred
  general shape — no geometry-specific `BindingKind`/`HirExpr` variant was
  added).
- `cad_hir::builtins` (new module) — `BuiltinFnId` (a closed, 8-variant
  enum: `Box`/`Cylinder`/`Transform`/`Union`/`Cut`/`Intersect`/`Fillet`/
  `Chamfer`) and `catalogue()`, the single authoritative name/parameter/
  return-type table both binding and the runtime read. See
  `docs/API/safe-cad-api.md` for the human-readable rendering and the
  exact Stage-2 signatures (including the two deliberate narrowings:
  `transform` is translate-only, `fillet`/`chamfer` select edges via a
  plain `List<Int>`).
- `cad_hir::lower::Lowerer::seed_builtins` — mints a real `BindingId` per
  catalogue entry, declares each name in the module scope *before* any
  user item is lowered (so ordinary call sites resolve to it, mirroring
  `crate::prelude::with_prelude`'s own "seed first" ordering), and appends
  the resulting synthetic `HirItem::Fn` nodes *after* the user's own items
  in the returned `HirProgram` (order doesn't affect
  `crate::typeck`'s forward-reference-friendly two-pass checking or
  `cad_runtime`'s `index_fns`, both of which scan the whole item list
  regardless of position — this ordering choice specifically keeps
  `result.program.items[0]` meaning the caller's own first item, exactly
  what every pre-existing test already assumed).
- `cad_hir::typeck`: added `CheckedType::Geometry` (a single opaque nominal
  type — Stage 2 does not distinguish solid/wire/face at the type-checker
  level), wired into `types_compatible`/`describe`/`resolve_type_ref`
  (`"Geometry"` resolves the same way `"Length"`/`"Int"` do); `check_item`'s
  `HirItem::Fn` arm now only calls `check_block` for an `Aicad` body (a
  `RuntimeBuiltin`'s signature is already fully resolved by
  `collect_signatures`, which needed no change at all — it already worked
  generically over `params`/`return_ty`, never touching `body`).

### 2. Execution (`crates/cad-runtime`)

- `cad_runtime::value::Value::Geometry(cad_geometry_api::GeomId)` — a new
  runtime value carrying only an opaque SSA-node index, never an OCCT
  object or kernel handle.
- `Interpreter` gained a `geometry: cad_geometry_api::GeometryGraph` field
  (new dependency: `cad-geometry-api`, a pure/kernel-independent crate —
  no OCCT/native dependency was added) and a public
  `geometry_graph(&self) -> &GeometryGraph` accessor.
- `Interpreter::run_fn_body` now matches on `FunctionImplementation`:
  `Aicad(block)` runs exactly as before; `RuntimeBuiltin(id)` dispatches to
  the new `Interpreter::dispatch_builtin`. Both paths are charged against
  the same `enter_call`/`exit_call` recursion-depth budget (`DL-15`:
  "cannot bypass AICAD execution budgets merely because their
  implementation is runtime-provided").
- `Interpreter::dispatch_builtin` reads the already-evaluated argument
  `Value`s from the call frame (bound to each parameter's own `BindingId`
  exactly like an ordinary function call), converts them into
  `cad_geometry_api` vocabulary (`Value::Number` -> `Quantity`,
  `Value::Geometry` -> `GeomId`, `Value::List` of `Value::Number` ->
  `Vec<EdgeIndex>`), and appends one `GeometryOp` node to `self.geometry`.
- New `RuntimeError` variants: `GeometryConstruction` (wraps a
  `cad_geometry_api::GeometryIrError`, reusing its `code()`/`title()`/
  `message()`/`span()` verbatim under the `GEOM` family — the same "reuse,
  not re-derivation" pattern `DimensionalArithmetic` already established
  for `cad_units`'s own errors; required making those four `GeometryIrError`
  methods `pub` in `cad-geometry-api`) and `BuiltinArgumentShape`
  (defensive-only: an argument's runtime kind didn't match what
  `cad_hir::typeck` already verified — unreachable for a type-checked
  program).

### 3. Documentation

- `docs/API/safe-cad-api.md` — the authoritative Stage-2 Safe CAD source
  API specification `DL-15` requires, rendering `cad_hir::builtins::
  catalogue` for humans and documenting every deliberate scope narrowing
  (`transform` translate-only; `export_step`/`import_step`/`tessellate`/
  raw topology/queries not source-visible in Stage 2) with rationale.
- `rfcs/0001-language-principles.md` §6a (new) — the general runtime-backed
  standard-function mechanism, and why it is not a `DL-7` compiler
  intrinsic.
- `rfcs/0002-geometry-runtime-kernel-abstraction.md` §3a (new) — how Safe
  CAD calls relate to the Tier B/Tier C distinction and the kernel
  boundary.

## Tests

- `crates/cad-hir` (8 new, all in `typeck.rs`): `box`/`cut`/`transform`
  resolve through ordinary binding and type-check with `Geometry` return
  type; `fillet` accepts a plain `List<Int>`; a dimensionally wrong
  argument (`box(1mm, 2kg, 3mm)`) and a non-`Geometry` argument to `union`
  are both reported with the ordinary `TYPE-E418` `ARGUMENT_TYPE_MISMATCH`
  code (no geometry-specific diagnostic); an undeclared name that looks
  like a builtin typo gets the ordinary `TYPE-E410` unresolved-binding
  diagnostic, no special carve-out. 197 tests total in this crate (up from
  189), 0 failed.
- `crates/cad-runtime` (6 new, in `interp.rs`): `box`/`cylinder` each
  create the expected `GeometryOp` node with correctly-converted canonical
  magnitudes; `cut(box(...), cylinder(...))` composes with the correct
  `lhs`/`rhs` dependency ordering (left-to-right argument evaluation order
  determines `GeomId` order); `transform` composes and its resulting
  `Transform::apply_point` matches the expected translation; an ordinary
  AICAD-defined function (`make`) calls a Safe CAD function (`box`) and
  returns its geometry value through plain `return`; ordinary `let`/`var`/
  arithmetic keep working correctly in a function that also calls two Safe
  CAD builtins (proving no interference between the two mechanisms). 89
  tests total (up from 83), 0 failed.
- `crates/cad-geometry-runtime` (1 new, in `dispatch.rs`): a full
  `.aicad` source -> parser -> binding -> type checker -> typed HIR ->
  `RuntimeBuiltin` dispatch -> `GeometryGraph` -> this crate's dispatcher
  -> real `OcctContext` -> OCCT pipeline, producing an exact valid B-rep
  with the expected closed-form volume (a box with a translated through-
  hole cylinder cut out). Not a substitute for `AICAD-063`'s own larger
  end-to-end gate — this task's own proof that the D18 mechanism reaches a
  real kernel call. 10 tests total (up from 9), 0 failed.

### Required-tests-list coverage (`project/OWNER_DECISIONS.md#D18`'s ruling)

1. Resolves through ordinary name binding — `cad-hir`.
2. Argument/return types checked through ordinary machinery — `cad-hir`.
3. `box(...)` creates the expected node — `cad-runtime`.
4. `cylinder(...)` creates the expected node — `cad-runtime`.
5. `cut(box(...), cylinder(...))` composes with correct ordering —
   `cad-runtime`.
6. `transform` composes normally — `cad-runtime`.
7. Invalid argument dimensions fail with stable diagnostics before
   dispatch — `cad-hir`.
8. Unknown standard-function names fail through ordinary binding
   diagnostics — `cad-hir`.
9. Runtime-backed functions cannot expose OCCT/native types into HIR —
   architectural: `crates/cad-hir/Cargo.toml` and `crates/cad-runtime/
   Cargo.toml` have no `cad-occt-bridge`/`native` dependency at all
   (verified by inspection of both files; `cad-runtime`'s only new
   dependencies this session are `cad-geometry-api` and `cad-kernel-api`,
   both pure/kernel-independent per their own module doc comments).
10. Runtime execution reaches the real OCCT dispatcher and produces valid
    exact B-rep evidence — `cad-geometry-runtime`.
11. **Not implemented as a literal synthetic non-geometry test builtin.**
    `BuiltinFnId` is deliberately closed (`DL-15`: "a closed
    compiler/runtime-owned mechanism"), and adding a synthetic variant
    would require either (a) permanently polluting the real, public
    Stage-2 catalogue with a meaningless test-only function name, or (b)
    fragile cross-crate `cfg(test)`/Cargo-feature-gating between `cad-hir`
    and `cad-runtime` (a `#[cfg(test)]` variant in `cad-hir` does not exist
    when `cad-hir` is compiled as an ordinary dependency of `cad-runtime`'s
    own test build, since `cfg(test)` only applies to the crate under test
    itself — this would need a dedicated Cargo feature threaded through
    both crates' `[dev-dependencies]`, adding permanent build-graph
    complexity for a single proof). Neither is a clean engineering
    trade-off. Instead, generality is evidenced architecturally: the eight
    real catalogue entries already have materially different shapes
    (0/1/2/3/4-argument arities; `Quantity`-only, `Geometry`-only, and
    mixed `Geometry`+`Quantity`+`List<Int>` signatures), and every part of
    the mechanism upstream of `Interpreter::dispatch_builtin`'s own
    `match id { ... }` (name seeding, binding, arity/type checking,
    `Interpreter::call`'s `BindingKind::Fn` dispatch, `run_fn_body`'s
    `FunctionImplementation` match, `enter_call`/`exit_call` budget
    accounting) contains no geometry-specific logic at all — the only
    geometry-aware code in the entire mechanism is that one match's own
    per-variant `GeometryOp` construction. Flagged here rather than
    silently claimed as satisfied.
12. Ordinary AICAD-defined functions can call Safe CAD functions and
    return their geometry values — `cad-runtime`.
13. Local lexical bindings/function calls still behave normally alongside
    runtime-backed functions — `cad-runtime`.

## Exact commands and results

```
$ cargo build -p cad-hir -p cad-runtime -p cad-geometry-api -p cad-geometry-runtime
Finished, clean.

$ cargo fmt --all -- --check
(clean, after one `cargo fmt --all` pass)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.76s
(clean, zero warnings)

$ cargo test --workspace
Every crate: test result: ok, 0 failed anywhere.
cad-hir: 197 (up from 189)
cad-runtime: 89 (up from 83)
cad-geometry-runtime: 10 (up from 9)
cad-geometry-api: 17 (unchanged from the prior session)
All other crates: unchanged from the AICAD-059 session's own baseline.
```

## Known limitations

- Requirement 11's literal synthetic-non-geometry-builtin test was not
  added — see the required-tests-list entry above for the full rationale.
- `transform` is translate-only (documented, deliberate Stage-2 narrowing
  — `docs/API/safe-cad-api.md`); a future task adding source-level
  rotation needs its own signature/name, not silently folded into
  `transform`.
- Geometry queries (`volume`, `is_valid`, ...) are not source-visible in
  Stage 2 (`DL-15` explicitly does not require this); a future task making
  them source-visible must treat "does letting source control flow depend
  on a kernel-evaluated result need a different execution model" as its
  own question, per `DL-15`'s own instruction.
- No wall-clock/geometry-op-shaped resource budget was added (unchanged
  limitation from the prior session — `AICAD-058`'s own module doc comment
  already flagged this as needing a real measurement hook that doesn't
  exist yet).

## Unresolved questions

None outstanding for this task. `project/OWNER_DECISIONS.md#D18` is
resolved (`project/DECISION_LOG.md#DL-15`).
