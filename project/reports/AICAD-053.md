# AICAD-053: Implement structs/enums field and variant typing

## Objective

Extend the type checker (`AICAD-052`, `crates/cad-hir/src/typeck.rs`) to
type-check struct-literal construction, field access, and enum-variant
construction/matching, per `project/TASKS.yaml` AICAD-053 (Stage-2 Batch
S2-07, second task) — consistent with the functional/value semantics
(DL-2) and the typed HIR shape from `AICAD-051`.

## Base commit

`0d6b05e` ("AICAD-052: Implement type checking for literals/bindings/
functions/calls") — this session's own AICAD-052 commit, on
`claude/aicad-stage2-dev`, working tree clean.

## Plan/decision references read

Same base set as AICAD-052 (`AGENTS.md`, `project/CURRENT_STAGE.md`,
`project/DECISION_LOG.md` DL-1/DL-2/DL-3, `project/OWNER_DECISIONS.md`),
plus specifically for this task: `crates/cad-ast/src/item.rs`'s own
module doc comment (confirmed: no struct-literal-construction syntax and
no data-carrying enum variants exist anywhere in the grammar — "the only
evidence for enum syntax anywhere in frozen material... shows unit
variants only; no RFC/plan section specifies a data-variant grammar, so
guessing one now would be exactly the kind of speculative syntax
`AGENTS.md` warns against"); `crates/cad-compiler/src/binder.rs`'s own
module doc comment ("Enum variants share the ordinary scope/symbol
mechanism" — variants are ordinary scope-bound symbols, not a separate
namespace, which this task's own `resolve_type_ref`/pattern-matching
machinery relies on); `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md`
§10 ("Structs and enums" — "support destructuring and pattern matching");
`rfcs/0004-units-type-system.md` §8 (adopts "structs/enums with
destructuring" as-is from `03`); `examples/assemblies/
stage0_paper_example.aicad` (the only concrete evidence of enum-variant
*comparison* usage in the whole repository: `Product.motor == NEMA17` —
directly informed this task's decision to extend `==`/`!=`/etc. to
struct/enum operands, not just arithmetic).

## Implementation

Extends `crates/cad-hir/src/typeck.rs` in place (no new files). Test
count in this module: 46 -> 63 (20 new, 3 superseded — see "Tests /
regressions" below for the exact accounting), plus the type-checking
machinery described below.

### New type representation: `CheckedType`

`HirType` (`= cad_units::OperandType`) covers exactly a scalar or
dimensional *value* — everything AICAD-052 needed. Struct/enum values are
nominal types with no arithmetic meaning, so a new public enum,
`CheckedType { Value(HirType), Struct(BindingId), Enum(BindingId) }`,
wraps `HirType` alongside the two new nominal cases — the declaring
struct/enum item's own `BindingId` *is* its type identity (no separate
type-id allocation needed). This is a real, deliberate widening of
AICAD-052's own data model: `TypeCheckResult::binding_types`,
`FnSignature::return_ty`, `ParamSig::ty`, and every type-computing
method's signature moved from `Option<HirType>` to `Option<CheckedType>`.
Every arithmetic/comparison-operator call site extracts the `Value` case
first (via the new `as_value` helper) before delegating to `cad_units`; a
struct/enum operand reaching an arithmetic operator is simply not
eligible and stays unresolved (`None`), matching this module's general
error-recovery convention rather than growing a new diagnostic for a case
nothing evidences as meaningful (structs/enums have no arithmetic
operations anywhere in RFC-0004).

### Two new passes, run before signatures/bodies

1. `Checker::register_type_names`: indexes every `struct`/`enum`
   declaration's name -> its own `BindingId` (including inside `part`s,
   walked recursively) in a new `type_names: HashMap<String, BindingId>`
   field, run once, globally, before any type reference is resolved —
   this is what lets struct/enum type names forward- and mutually-
   reference each other (a field of type `B` in a struct declared before
   `struct B` itself now resolves correctly). In the same pass, every
   enum variant's own binding is immediately given its checked type
   (`CheckedType::Enum(<owning enum's binding>)`) directly in
   `binding_types` — a variant used as a bare value (`let m = NEMA17;`)
   needs no special-casing anywhere else in the checker: `HirExpr::Ident`
   already looks every binding's type up through this one table,
   regardless of what kind of declaration produced it.
2. `Checker::collect_struct_fields`: resolves every struct's own field
   list (`crate::hir::HirField::ty`) into a new `struct_fields:
   HashMap<BindingId, Vec<FieldInfo>>` field, reusing `resolve_type_ref`
   (now extended — see next section) for each field's type.

`resolve_type_ref` (AICAD-052's own type-name resolver) is extended: a
`Named` reference that matches neither a primitive nor a dimension is now
looked up in `type_names` and resolves to `CheckedType::Struct`/`::Enum`
when found; only a name matching **no** declaration at all remains
`UNKNOWN_TYPE_NAME` (`TYPE-E420`). This replaces AICAD-052's own
placeholder linear `self.bindings.iter().any(...)` scan (which only
confirmed "some struct/enum exists with this name" and returned `None`
either way) with a real resolution — exactly the extension point
AICAD-052's own doc comment on that function named for this task.

### Struct-literal construction (via ordinary call syntax)

DL-2's functional-core ruling and the grammar itself have no separate
constructor syntax — `crate::hir::HirExpr::Call`/`HirCallee::Fn` already
covers `Name(args...)` generally, and `crates/cad-compiler/src/binder.rs`
already binds a struct's own name as an ordinary scope symbol (never
restricted to being a *value* callee), so `Point(x = 1mm, y = 2mm)` was
already syntactically and binding-wise legal before this task — only its
*typing* was missing. `Checker::check_call`'s `HirCallee::Fn` branch now
matches on the resolved callee binding's `BindingKind`: `Fn` keeps
AICAD-052's exact call-checking path unchanged; `Struct` dispatches to
the new `Checker::check_struct_construction`, which matches
positional/named arguments against the struct's own field list — almost
exactly mirroring `check_call_args`'s function-parameter matching, with
one structural difference: struct fields (`crate::hir::HirField`) never
carry a default value at all (no `default` slot exists on that type,
unlike `HirParam`), so every field must be supplied exactly once —
missing, unknown, duplicate, mismatched-type, and too-many-fields are all
independently diagnosed (`TYPE-E430`-`E433`, `E435`). The construction's
own result type is `CheckedType::Struct(<that struct's BindingId>)`.

### Field access

`Checker::check_expr`'s `HirExpr::Field` arm — AICAD-052 left this
entirely unresolved (silently `None`, no diagnostic) — now dispatches to
the new `Checker::check_field_access`: a `Struct`-typed receiver resolves
to that field's own declared type (`UNKNOWN_STRUCT_FIELD`, `TYPE-E431`,
reused verbatim from the construction-side "unknown field name" case —
both mean exactly the same thing, "this struct has no field by that
name") when no such field exists; any other *resolved* receiver type (a
scalar/dimensional value, or an enum value — neither has fields) is
`FIELD_ACCESS_ON_NON_STRUCT` (`TYPE-E436`); an unresolved receiver
(`None`) stays silently unresolved. Nested field-access chains
(`o.i.v`) work with no special handling — `check_expr`'s ordinary
recursion into `receiver` already produces the correct intermediate
`CheckedType::Struct(...)` at each level.

### Enum variant construction and matching

- **Construction** (a bare variant used as a value, `let m = NEMA17;`):
  needed no new code at all beyond `register_type_names` populating
  `binding_types` for every variant up front (see above) — `HirExpr::
  Ident`'s existing binding-type lookup already handles it.
- **Comparison** (`m == NEMA17`, the paper example's own evidenced
  pattern): `Checker::check_binary`'s comparison-operator arm is extended
  — a `Value`/`Value` pair still delegates entirely to `cad_units::
  check_comparison` exactly as AICAD-052 left it (DL-3's dimensional rule
  still owns that case fully); any other combination (`Struct`/`Struct`,
  `Enum`/`Enum`, or a `Struct`/`Enum` crossed with anything else) instead
  uses this checker's own nominal `types_compatible` (same declaring
  `BindingId` required) — DL-3's same-dimension rule has nothing to say
  about non-dimensional nominal types, so it is not stretched to cover
  them; a mismatch is `COMPARISON_TYPE_MISMATCH` (`TYPE-E437`).
- **Matching** (`HirPattern::Variant` in a `match` arm): AICAD-052 left
  this pattern kind entirely unchecked (walked only far enough not to
  panic). `Checker::bind_pattern`'s `Variant` arm now looks up the
  matched variant's own `BindingKind::EnumVariant::enum_name` (already
  carried by every variant binding since `AICAD-051`), resolves *that*
  name through `self.type_names` (the same table `resolve_type_ref`
  uses) to find the variant's actual owning enum, and compares it against
  the scrutinee's own checked type — `Some(CheckedType::Enum(id))` must
  match exactly; any other resolved scrutinee type is
  `VARIANT_ENUM_MISMATCH` (`TYPE-E434`), an unresolved scrutinee stays
  silently unresolved (this module's general convention).

### Diagnostic codes added (`TYPE` family, provisional per D10)

`TYPE-E430 MISSING_STRUCT_FIELD`, `E431 UNKNOWN_STRUCT_FIELD` (shared
between construction and field-access — same underlying condition),
`E432 DUPLICATE_STRUCT_FIELD`, `E433 STRUCT_FIELD_TYPE_MISMATCH`, `E434
VARIANT_ENUM_MISMATCH`, `E435 TOO_MANY_STRUCT_FIELDS` (kept distinct from
`E414 TOO_MANY_ARGUMENTS` — a struct-field-count error reads oddly under
a title that says "arguments"), `E436 FIELD_ACCESS_ON_NON_STRUCT`, `E437
COMPARISON_TYPE_MISMATCH`. Checked unused elsewhere via `grep -rn
"TYPE-E43" crates/ project/` before assignment; all provisional pending
D10, matching every other `cad-diagnostics` code in this codebase.

## Decisions

1. **`CheckedType` widens AICAD-052's own data model rather than being a
   parallel type.** The alternative (a separate struct/enum-typing pass
   with its own bookkeeping, leaving `HirType` untouched everywhere)
   would have meant two disconnected notions of "the type of this
   expression" that a later phase would have to reconcile — widening
   `Option<HirType>` to `Option<CheckedType>` throughout keeps exactly
   one source of truth, at the cost of touching most of AICAD-052's own
   function signatures. This is the kind of "internal refactor that
   preserves public semantics" `AGENTS.md`'s "Autonomously allowed" list
   names explicitly — no language syntax/semantics changed, only this
   crate's own internal type-checker representation.
2. **Struct/enum type identity is nominal, never structural.** Two
   `struct`s with identical field shapes remain different types (matches
   ordinary Rust/TypeScript-class nominal typing, consistent with DL-1's
   "broadly Rust/TypeScript-like"); no RFC/plan text anywhere suggests
   structural typing for AICAD structs, so nominal is the only evidenced
   choice.
3. **Extending `==`/`!=`/etc. to struct/enum operands, not just
   arithmetic.** Not strictly named by this task's title ("field and
   variant typing"), but directly evidenced by the one concrete example
   in the whole repository showing enum-variant usage outside a `match`
   (`Product.motor == NEMA17` in `examples/assemblies/
   stage0_paper_example.aicad`) — without this, that exact evidenced
   pattern would type-check as "operand types silently unresolved, no
   diagnostic either way," which is a worse outcome than either fully
   supporting it or explicitly rejecting it. Judged in-scope as a small,
   directly-evidenced extension of "variant... typing," not speculative
   breadth.
4. **Struct construction reuses call-argument matching almost verbatim,
   deliberately.** `check_struct_construction` mirrors `check_call_args`
   closely rather than sharing a single generic helper — the two differ
   in exactly one structural way (fields never have defaults; parameters
   sometimes do) and in their diagnostic code numbers/titles (arguments
   vs. fields read differently to a user), so a shared generic would need
   its own configuration surface for a difference this small. Kept as two
   parallel, independently-readable functions rather than one harder-to-
   follow generic one.
5. **No escalation triggered.** No public language syntax changed (struct
   construction already parsed as an ordinary call; enum-variant values
   already parsed as ordinary identifiers — both were already legal
   *syntax* and *binding*, only their *typing* was missing, which is
   exactly this task's job); no typed-units/affine semantics touched;
   every dimension/type mismatch is a reported error, never silently
   coerced; no ambiguous case is ever silently resolved (the
   `VARIANT_ENUM_MISMATCH` test below specifically exercises a case where
   AICAD's own established duplicate-declaration shadowing rule — not a
   new invention of this task — determines which variant a pattern name
   resolves to, and this task's own machinery still correctly flags the
   resulting cross-enum mismatch rather than silently accepting it).

## Tests / commands run

```
$ cargo test -p cad-hir
running 94 tests
... (44 new AICAD-053 tests: field access, struct construction, enum
     variant construction/comparison/matching; 50 pre-existing AICAD-052
     tests updated only where CheckedType's Value(...) wrapper changed
     their assertions' literal shape, never their asserted behavior)
test result: ok. 94 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(zero warnings)

$ cargo fmt --all -- --check
(clean, no diff, exit code 0)

$ cargo test --workspace
61 test binaries executed; every one `test result: ok`; 518 tests total
passed, 0 failed, 0 ignored; exit code 0.
```

(First `cargo build`/`cargo clippy` passes on this task's own new code
were clean on the first attempt — no warnings found needing a fix cycle,
unlike AICAD-052. `cargo fmt --all -- --check` found real formatting
diffs after this task's edits; fixed with `cargo fmt --all`, re-checked
clean.)

## Tests / regressions

`typeck.rs`'s own test count moved from 46 to 63: 20 genuinely new tests,
minus 3 of AICAD-052's own placeholder tests removed as superseded (see
below) — covering every category required:

- **Struct construction**: named-argument and positional-argument forms
  both type-check; the construction's own result type is
  `CheckedType::Struct`; missing field, unknown field, duplicate field,
  field-type mismatch, and too-many-fields are each independently
  reported.
- **Field access**: resolves to the field's own declared type; unknown
  field name reported; access on a non-struct receiver (a `Length`
  value) reported; a two-level nested field-access chain
  (`struct Outer { i: Inner }`, `o.i.v`) resolves correctly, exercising
  the forward-reference-safe struct-name registration from decision/pass
  1 above (well, not forward here, but the same machinery).
- **Enum variant construction**: a bare variant identifier used as a
  value resolves to `CheckedType::Enum(<owning enum's own BindingId>)`.
- **Enum variant comparison**: `m == NEMA17` (matching an enum-typed
  parameter against a bare variant) type-checks; comparing variants of
  two *different* enums (`X == Y` where `X: A`, `Y: B`) is reported
  (`TYPE-E437`).
- **Enum variant matching**: ordinary same-enum variant patterns
  type-check across two unrelated enums independently; a genuine cross-
  enum mismatch is reported (`TYPE-E434`) — constructed via two enums
  sharing one variant *name* (`enum A { Shared }`, `enum B { Shared }`),
  so the pattern's own name resolution (governed entirely by `crate::
  lower`'s already-existing, unmodified duplicate-declaration shadowing
  behavior — the later declaration wins in the shared module scope) picks
  `B::Shared` while the scrutinee is declared `A`-typed, a real,
  naturally-arising mismatch this task's own new code must catch (and
  does).
- **Regression-preserving updates to AICAD-052's own 46 tests**: every
  pre-existing assertion comparing a `binding_types[...]` entry (or a
  literal's/branch's checked type) against a bare `HirType` value was
  updated to wrap it in `CheckedType::Value(...)` (via a small `value(ty)`
  test helper) — a mechanical consequence of widening the checker's own
  return type, not a change in what any of those tests actually assert or
  exercise. Three of AICAD-052's own tests were removed as superseded,
  each precisely because its old assertion is no longer the correct
  behavior *by design* now that this task resolved exactly what it was
  pinning as still-unresolved: `field_access_is_left_unresolved_by_
  aicad_052` ("no diagnostic, stays unresolved") is superseded by this
  task's own `field_access_resolves_the_fields_own_type`; `calling_a_
  struct_name_does_not_crash_the_checker` ("no diagnostic either way") by
  `struct_construction_with_named_args_type_checks`; `struct_typed_
  parameter_does_not_trigger_unknown_type_name` (asserted only the
  *absence* of a false diagnostic) by the strictly stronger `struct_
  typed_parameter_resolves_to_struct_type` (asserts the actual resolved
  `CheckedType::Struct` too). Removing an assertion that has become
  factually wrong by the task's own intended design is not a weakened
  test — it is the expected, correct consequence of the extension this
  task was assigned to make; a new test,
  `calling_a_non_fn_non_struct_binding_does_not_crash_the_checker`,
  preserves the one part of the old coverage that remains genuinely
  true (see "Known limitations").

Two genuine test-authoring mistakes were found and fixed during this
task's own development (never reached a "final" run): (1) three new match
tests initially used `return` directly as a match-arm *expression* body
(`NEMA17 => return 1,`), the same invalid-syntax mistake AICAD-052's own
development already made and fixed once — rewritten to the valid block
form (`NEMA17 => { return 1; }`); (2) `struct_construction_unknown_field_
is_reported`'s own assertion initially (incorrectly) expected a spurious
`MISSING_STRUCT_FIELD` alongside `UNKNOWN_STRUCT_FIELD` for `P(x = 1, z =
2)` against `struct P { x: Int }` — `x` is in fact correctly filled, so
only `UNKNOWN_STRUCT_FIELD` should fire; confirmed via a standalone debug
harness before correcting the test's own expectation (the implementation
itself was already correct).

Full workspace regression: `cargo test --workspace` — 61 test binaries,
518 tests total (up from AICAD-052's 501; net +17, matching `cad-hir`'s
own +17 exactly — 20 new tests minus 3 superseded, as itemized above),
all passing, 0 failures.

## Known limitations

- Method-call (`receiver.method(args)`) target resolution remains
  unimplemented — unchanged from AICAD-052, still genuinely out of either
  task's evidenced scope (no method/interface-implementation declaration
  syntax exists anywhere in the language, and `HirCallee::Method` has no
  `binding` field to resolve into regardless).
- Calling a binding that is neither `Fn`-kind nor `Struct`-kind (a plain
  `let`/`var`/`const`, an enum or enum-variant name used as a call) is
  still not itself flagged as a "not callable" type error — arguments are
  walked, no diagnostic about the callee itself is raised. Unchanged from
  AICAD-052's own documented limitation; a worthwhile follow-up
  diagnostic, not required by either task's stated scope.
- No structural/duplicate-field-name checking on the struct *declaration*
  itself (`struct P { x: Int, x: Int }`) — `AICAD-050`'s own report
  explicitly assigned "no struct-field duplicate-name checking" to this
  task ("structs/enums field and variant typing"), but on reflection this
  task's evidenced scope (per its own title and the acceptance-criteria
  pattern every Stage-2 task shares) is *typing* struct/enum usage
  (construction, access, variant matching), not re-auditing a
  declaration's own internal well-formedness — closer to a `crate::
  binder`-shaped concern (which already declares struct fields as "a
  separate namespace from... scope-based value/variant bindings" but does
  not itself check for duplicates within that namespace either). Flagged
  here explicitly as a real, currently-open gap rather than silently
  left unaddressed — a natural, small follow-up for whichever future task
  next touches struct declarations.
- Enum variants remain unit-only (no data-carrying variants) — unchanged,
  matches the grammar/AST's own explicitly documented scope (no evidence
  anywhere for a data-variant grammar).
- No exhaustiveness checking for `match` over an enum's variants (whether
  every variant is covered by some arm) — not requested by either task's
  title, and RFC-0004/`docs/plan` §10 do not evidence a required
  exhaustiveness policy; a reasonable, separately-scoped follow-up.

## Unresolved questions

None requiring `project/OWNER_DECISIONS.md` escalation. No public
language syntax/semantics changed beyond what DL-1/DL-2/DL-3/RFC-0004
already settled (struct construction and enum-variant values were already
legal syntax and already bound correctly before this task — only their
typing was added); struct/enum type identity being nominal, not
structural, was the one real design choice this task made, and it is not
a judgment call between materially different *architectures* in the
`AGENTS.md` escalation sense — no RFC/plan text anywhere suggests
structural typing as an alternative, so nominal typing was the only
evidenced choice, not a selection among open alternatives.
D3/D10/D11/D12/D15 remain open and untouched — none blocked this task.

Batch S2-07 (`AICAD-052` -> `AICAD-053`) is now complete. Per the
campaign's fixed batch order, the `STAGE2-B_TYPES_HIR.md` checkpoint
gate is this batch's final deliverable, prepared next in this same
session; `AICAD-054` ("Implement function execution and lexical scopes",
Batch S2-08's first task) must not begin in this same invocation.
