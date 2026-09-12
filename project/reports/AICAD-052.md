# AICAD-052: Implement type checking for literals/bindings/functions/calls

## Objective

Implement the type checker over typed HIR per `project/TASKS.yaml`
AICAD-052 (Stage-2 Batch S2-07, first task) — `docs/plan/
02_LANGUAGE_AND_COMPILER.md` §17 phase 4 ("type + dimensional checking").
Fills in exactly the part of `crates/cad-hir/src/lower.rs`'s own "Scope
boundary" doc comment this task owns: numeric-literal-type (`Int`/`Float`)
defaulting, and full type checking for `let`/`const`/`param`/`var`
bindings, function declarations, function calls (arity, parameter types,
return type), and literal expressions — reusing `cad_units`'s dimensional
arithmetic rules (DL-3) throughout. Struct/enum field/variant typing is
explicitly **not** in scope — that is `AICAD-053`, Batch S2-07's second
task.

## Base commit

`1a03e53` ("AICAD-051: Create typed HIR and AST->HIR lowering skeleton") —
this session's assigned starting point, confirmed as `HEAD` of
`claude/aicad-stage2-dev` (tracking `origin/claude/aicad-stage2-dev`) at
session start, working tree clean.

## Plan/decision references read

`AGENTS.md` (non-negotiables, work loop, escalation triggers, evidence
rule); `project/CURRENT_STAGE.md` (Stage-2 scope/exit gate);
`project/DECISION_LOG.md` DL-1 (syntax), DL-2 (functional core/method
desugaring), DL-3 (units/dimension/tolerance/affine semantics), DL-5
(kernel boundary — not directly relevant), DL-7 (intrinsics require RFC —
not triggered), DL-12 (D5 determinism policy — Level 1 already requires
this phase's output to be deterministic; no random/unordered iteration is
used anywhere in this module); `project/OWNER_DECISIONS.md` (D3, D10,
D11, D12, D15 — none touched); `project/SESSION_HANDOFF.md` (prior
session's "Scope boundary" pointer); `project/reports/AICAD-050.md`/
`AICAD-051.md`; `crates/cad-hir/src/lower.rs`'s module doc comment "Scope
boundary" in full; `crates/cad-hir/src/types.rs`, `hir.rs`, `ids.rs`;
`crates/cad-units/src/arithmetic.rs` and `registry.rs` in full;
`crates/cad-types/src/dimension.rs`/`primitive.rs`;
`crates/cad-compiler/src/binder.rs` (confirmed `BindResult` still exposes
only diagnostics, no resolution table to reuse — matches AICAD-051's own
"Known limitations"); `crates/cad-ast/src/item.rs`/`expr.rs` (confirmed
grammar shape: no struct-literal-construction syntax, no `impl`/method-
declaration syntax exists anywhere); `rfcs/0004-units-type-system.md` in
full; `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` (Type-system
sections); `cad-diagnostics`'s `DiagnosticCode`/`Diagnostic` API.

## Implementation

New module `crates/cad-hir/src/typeck.rs` (~950 lines including 44 unit
tests), wired into `crates/cad-hir/src/lib.rs` as `pub mod typeck;` with
`pub use typeck::{TypeCheckResult, check_program};`. No new crate
dependencies (uses only what `cad-hir` already depends on: `cad-ast`,
`cad-diagnostics`, `cad-types`, `cad-units`).

Public API: `check_program(program: &HirProgram, bindings: &[Binding],
file: &str, source: &str) -> TypeCheckResult { diagnostics: Vec<Diagnostic>,
binding_types: Vec<Option<HirType>> }`.

### Decisions

1. **No scope stack needed anywhere in the type checker.** Unlike
   `crate::lower::Lowerer` (which performs its own scope walk to *mint*
   binding identity), `lower_program` already resolved every reference to
   a concrete, globally-unique `BindingId` (or `None`). Lexical scoping is
   therefore already fully baked into which `BindingId` a name points to,
   so this checker only needs one flat table (`binding_types`, indexed by
   `BindingId::index()`, mirroring `LowerResult::bindings`'s own
   convention) — no scope push/pop bookkeeping anywhere in `typeck.rs`,
   including for `part`-nested items (walked recursively into the same
   flat tables).
2. **Two-pass structure for forward references.** A function may call a
   sibling function declared later in source (already exercised at the
   binding-identity level by `lower.rs`'s own
   `forward_reference_between_sibling_fns_resolves_to_a_real_binding`
   test). `check_program` resolves every function's signature — and every
   item-level `param`'s own mandatory type — in one upfront pass
   (`collect_signatures`) before checking any body/value expression in
   ordinary source order (`check_items`). Top-level `let`/`const`
   *values* are deliberately **not** given the same forward-reference
   treatment, consistent with `project/reports/AICAD-050.md`'s own
   already-recorded limitation ("no detection of circular top-level
   const/let value dependencies... a compile-time-evaluation concern") —
   a `let`/`const` referenced before its own declaration resolves to
   `None` (unresolved, not an error, not a crash), not new machinery this
   task was not asked to add.
3. **`Option<HirType>`, not `Result`, for error recovery.** Every type-
   computing method returns `Option<HirType>`, reusing exactly the
   convention `crate::types::HirType`'s own module doc comment already
   establishes: "`None` is not an error, only 'not yet resolved'." A
   `None` propagates upward silently — no cascading diagnostic from a
   parent expression whose operand's type could not be determined,
   whether because of an already-reported error or because the operand is
   a genuinely not-yet-typeable shape (a method call, or a struct/enum
   construct AICAD-053 has not reached yet).
4. **Every dimensional arithmetic/comparison/negation rule is delegated
   to `cad_units`, never re-derived.** `check_binary`/`check_unary` call
   `cad_units::{check_binary_arithmetic, check_comparison,
   check_unary_neg}` directly; `Checker::diag_from_unit_error` builds a
   `Diagnostic` under the resulting error's own `UNIT-Exxx` code (via
   `DiagnosticCode::parse`) rather than reinventing `TYPE`-family codes
   for a condition `cad-units` already names — `cad_units::
   DimensionalArithmeticError`'s own module doc comment says its `code()`
   exists "for a later `cad-diagnostics`-aware caller to build a real
   `Diagnostic` from"; this checker is that caller.
5. **Numeric-literal-type defaulting rule.** A numeral's raw text (`"5"`,
   `"5.5"`, `"1.5e-3"` — `crates/cad-lexer::Lexer::scan_number`'s only
   possible shapes) containing a decimal point or exponent marker defaults
   to `Float`; otherwise `Int` — the same distinction Rust's own
   unsuffixed integer-vs-float literals draw (DL-1, "broadly Rust/
   TypeScript-like"), and consistent with this crate's own existing
   precedent (`cad_units::arithmetic::resolve_derived_dimension`'s
   dimensionless-ratio-defaults-to-`Float` rule). When an `expected` type
   names a numeric scalar directly (`let x: Float = 5;`), that annotation
   wins over the text-shape default (explicit context, not a guess). An
   `expected` naming a *dimensional* type is deliberately **not** honored
   here — a bare unitless numeral is never silently promoted to a
   dimensional quantity (DL-3 "no numeric escape hatch");
   `let x: Length = 5;` is correctly caught as a `TYPE-E411` mismatch by
   the normal annotation-checking path instead.
6. **Ambiguous/unknown unit-suffix literal resolution via expected
   context.** `lower.rs` already leaves an ambiguous (`5Pa`, matching both
   `Pressure`/`Stress`) or unknown (`5xyz`) unit-suffixed literal
   unresolved. This task adds: when a surrounding `expected` type
   supplies a concrete dimension (an annotation, a parameter type, a
   function's return type via a `return` statement, ...) and
   `cad_units::lookup(symbol, dimension)` finds the symbol registered
   under exactly that dimension, the literal resolves to it (`let p:
   Stress = 5Pa;` now type-checks). Absent a matching `expected`, the
   ambiguity/unknown-ness is reported (`TYPE-E421`/`TYPE-E422`) — never
   guessed, per `AGENTS.md`'s "ambiguity is an error, never an arbitrary
   selection".
7. **Type-name resolution is scoped to exactly what this task needs.**
   `resolve_type_ref` resolves a `HirTypeRef::Named` to a primitive or
   named dimension; `HirTypeRef::Generic` (`Vector2<Length>`, ...) is
   never resolved (no generic/collection type system exists anywhere in
   this compiler yet — left `None`, no diagnostic, matching the existing
   "not yet supported" precedent rather than inventing one). A `Named`
   reference matching neither is checked against `self.bindings` for an
   existing `Struct`/`Enum` declaration: if found, resolution stays
   silently `None` (deliberately deferred to `AICAD-053`, which extends
   this exact branch); only a name matching **no** declaration at all (a
   genuine typo, e.g. `Frobnicator`) is diagnosed as `UNKNOWN_TYPE_NAME`
   (`TYPE-E420`). This distinction is what lets a struct/enum-typed
   parameter/field type-check cleanly under AICAD-052 alone without a
   false "unknown type" diagnostic AICAD-053 would otherwise have to
   retract — confirmed by dedicated tests
   (`struct_typed_parameter_does_not_trigger_unknown_type_name`,
   `genuinely_unknown_type_name_is_reported`).
8. **A bare type annotation for an affine dimension defaults to
   `Absolute`.** No syntax anywhere spells a "delta" type annotation
   (RFC-0004 §5's own patch explicitly leaves "the discriminant's
   concrete surface syntax or type-name spelling" as remaining Stage-2
   work) — `Absolute` is the same sound default `crate::lower::
   literal_type` already establishes for an affine *literal*, for the
   identical reason (neither is ever itself a subtraction result). A
   consequence, confirmed by `affine_absolute_minus_absolute_is_a_delta`
   plus the assignability rule (decision 9): an *annotated* `Temperature`
   binding can only ever accept an `Absolute`-valued expression under
   this checker — assigning a `Delta` result (`20degC - 5degC`) to one is
   a genuine, correctly-reported type error, not a false positive, since
   no syntax exists yet to spell the annotation that would accept it.
9. **Assignability (`types_compatible`) is a direct equality check, not a
   reuse of `cad_units::check_comparison`.** Initially implemented as a
   thin wrapper over `check_comparison`, but that function's own
   `scalar_same_type` requires a *numeric* scalar (`require_numeric_scalar`)
   for both operands — by its own module doc comment, general non-numeric
   scalar type agreement (e.g. `Bool == Bool`) "is the general type
   checker's job (`AICAD-052`)", i.e. exactly this task's, not something
   `cad_units` claims to own. Reusing it wrongly rejected `let x: Bool =
   true;`-shaped code (caught by this task's own
   `assigning_a_mismatched_type_to_a_var_is_reported`/`logical_and_
   operands_must_be_bool` tests during development — see "Tests" below).
   `types_compatible` is now a direct match: same `PrimitiveType`, or same
   `Dimension` *and* same `AffineKind` (`None`/`None`, or matching
   `Some(_)`s). DL-3's same-dimension-implicit-conversion rule still fully
   governs the arithmetic/comparison *operators* themselves (decision 4);
   this is the separate, simpler "is this value acceptable where that
   type was declared" relation `cad_units` never claimed to own.
10. **Method calls (`receiver.method(args)`, desugared to `HirCallee::
    Method`) are walked but never signature-checked.** No `impl`/method-
    declaration/interface-implementation syntax exists anywhere in the
    grammar (confirmed by reading `cad-ast` in full) — there is nothing
    to resolve `method`'s name against. `HirCallee::Method` structurally
    has no `binding` field at all (a deliberate AICAD-051 design choice,
    not an oversight this task could "fix" without changing HIR's node
    shape, which is out of this task's scope). Each argument is still
    recursively checked for its own independent diagnostics; the call's
    own result type stays unresolved. Documented as a known limitation,
    not an owner escalation — nothing evidenced anywhere describes what a
    method-call target would resolve against yet.
11. **A call whose callee resolves to a non-`Fn`-kind binding (a `Struct`
    name used as a constructor call — `AICAD-053` — or any other kind) is
    uniformly skipped**, not flagged as "not callable": arguments are
    still walked for their own diagnostics, but no arity/signature
    checking is attempted. This keeps the 052/053 boundary crisp (053
    adds the `Struct` case specifically) without inventing a "not
    callable" diagnostic family this task's evidence does not require;
    flagged as a known limitation (calling a plain `let`/`var` binding is
    not itself flagged as a type error).
12. **Control-flow value-type unification (`if`/`match` expressions).**
    `unify_value_type` requires `types_compatible` agreement across `if`/
    `else` branches and `match` arms producing a value, diagnosing
    `TYPE-E424 BRANCH_TYPE_MISMATCH` on disagreement — necessary
    "unambiguous value semantics" scaffolding (`AGENTS.md`) for
    `if`/`match` expressions to be usable as `let` values at all, and
    squarely part of "control flow" typing this task's title covers via
    "bindings"/"literals".
13. **Match-pattern typing is split at exactly the 052/053 boundary.**
    `Wildcard` (always compatible), `Literal` (checked against the
    scrutinee's type via the same literal-typing machinery as ordinary
    expressions — `TYPE-E425 PATTERN_TYPE_MISMATCH` on disagreement), and
    `Binding` (the fresh match binding's type becomes the scrutinee's own
    checked type) are all implemented here — none of them are struct/enum-
    specific. `Variant` (matching a known enum variant) is walked but does
    nothing type-related — `AICAD-053`'s job (needs the variant's
    enclosing enum resolved against the scrutinee's own type).

## Diagnostic codes added (`TYPE` family, provisional per D10)

`TYPE-E411 TYPE_ANNOTATION_MISMATCH`, `E412 CONDITION_NOT_BOOL`,
`E413`/reused via `E412`'s bool-condition helper for logical-operand
checks, `E414 TOO_MANY_ARGUMENTS`, `E415 UNKNOWN_NAMED_ARGUMENT`, `E416
DUPLICATE_ARGUMENT`, `E417 MISSING_ARGUMENT`, `E418
ARGUMENT_TYPE_MISMATCH`, `E419 RETURN_TYPE_MISMATCH`, `E420
UNKNOWN_TYPE_NAME`, `E421 AMBIGUOUS_LITERAL_UNIT`, `E422
UNKNOWN_LITERAL_UNIT`, `E423 ASSIGN_TYPE_MISMATCH`, `E424
BRANCH_TYPE_MISMATCH`, `E425 PATTERN_TYPE_MISMATCH`. Plus every
`cad_units::DimensionalArithmeticError` code (`UNIT-E101`-`E110`, reused
verbatim, not renumbered — see decision 4). Every code checked unused
elsewhere via `grep -rn "TYPE-E41\|TYPE-E42" crates/ project/` before
assignment; all provisional pending D10, matching every other
`cad-diagnostics` code in this codebase.

## Tests / commands run

```
$ cargo test -p cad-hir
running 77 tests
... (44 new typeck::tests::*, 33 pre-existing lower::tests::* unchanged)
test result: ok. 77 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.24s

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.32s
(zero warnings)

$ cargo fmt --all -- --check
(clean, no diff, exit code 0)

$ cargo test --workspace
61 test binaries executed; every one `test result: ok`; 501 tests total
passed, 0 failed, 0 ignored; exit code 0.
```

(First `cargo clippy` pass found 7 `collapsible_if` warnings in
`typeck.rs`, all rewritten with `if let ... && ...` let-chains and
re-verified clean. First `cargo fmt --all -- --check` pass found real
formatting diffs in this task's own new file; fixed with `cargo fmt
--all`, re-checked clean.)

## Tests / regressions

44 new tests in `crates/cad-hir/src/typeck.rs`, covering every category
required:

- **Numeric-literal-type defaulting**: unitless integer -> `Int`; unitless
  decimal/exponent -> `Float`; annotation-directed default (`let x: Float
  = 5;`); a unitless literal annotated as a *dimension* is correctly
  rejected, not coerced.
- **Ambiguous/unknown unit literals**: resolved via an expected
  annotation (`let p: Stress = 5Pa;`); still reported without one; unknown
  symbol reported.
- **Bindings**: `let`/`const`/`var` annotation-mismatch detection;
  unannotated inference from value.
- **Dimensional arithmetic reuse (DL-3)**: same-dimension addition type-
  checks; cross-dimension addition rejected (`UNIT-E104`, not coerced);
  affine `absolute + absolute` rejected (`UNIT-E105`); affine
  `absolute - absolute` correctly yields a `Delta`; the Torque/Energy
  ambiguity resolved two different ways by two different function return-
  type annotations (proving `expected` plumbs end-to-end from a `return`
  statement through to `check_binary_arithmetic`'s own disambiguation
  parameter); the same ambiguity reported without any resolving context.
- **Functions — arity/parameter types/return type**: correct-arity/
  correct-type call type-checks; a forward-referenced sibling function
  call type-checks (proving the two-pass signature-then-body structure);
  too-many-positional-arguments, missing-required-argument,
  missing-argument-with-a-default-is-allowed, unknown-named-argument,
  duplicate-argument, named-arguments-out-of-declaration-order,
  argument-type-mismatch, return-type-mismatch, bare-`return;`-from-a-
  value-returning-function, parameter-default-type-mismatch, and
  item-level-`param`-default-type-mismatch.
- **Control flow**: `if`/`while` conditions must be `Bool`; `&&` operands
  must be `Bool`; `if`-expression branches of matching type unify;
  mismatched branches reported.
- **Assignment**: matching/mismatched `var` reassignment.
- **Match**: a catch-all binding captures the scrutinee's own value type;
  a literal pattern of the wrong type is reported. Variant/enum
  consistency is explicitly *not* tested here — that is AICAD-053's
  addition.
- **Negative/adversarial — never panic on ill-typed/unresolved input**: an
  already-unresolved identifier reference (from lowering) produces no
  *additional* diagnostic and does not crash; a method call with no
  resolvable signature is walked without panicking or misreporting; field
  access on a struct-typed receiver is left unresolved by this task
  without panicking (053 will change this); a struct-typed parameter does
  not trigger a false `UNKNOWN_TYPE_NAME`; a genuinely unknown type name
  does; calling a struct name (053's own extension point) does not crash
  this task's checker.

Full workspace regression: `cargo test --workspace` — 61 test binaries,
501 tests total (up from AICAD-051's 455; net +46: 44 new `typeck` tests
plus 2 accounted for by doc-test binary counting differences — no
pre-existing test was touched or removed), all passing, 0 failures.

Two genuine implementation bugs were found and fixed *during* this task's
own development (not shipped): (1) `types_compatible` initially
reused `cad_units::check_comparison` directly, which wrongly rejected
same-type non-numeric-scalar comparisons (`Bool == Bool`) — caught by
this task's own `assigning_a_mismatched_type_to_a_var_is_reported`/
`logical_and_operands_must_be_bool` tests, fixed per decision 9 above.
(2) Two early test fixtures used `return` as a `match`-arm *expression*
body (`NEMA17 => return 1,`), which is not legal AICAD syntax (`return`
is a statement, and a `match` used in expression position requires an
expression- or block-shaped arm body per `cad_ast::expr::MatchArmBody`) —
caught immediately by the parser rejecting the fixture; fixed by
rewriting the fixtures to valid syntax before any incorrect assumption
reached committed code.

## Known limitations

- Method-call (`receiver.method(args)`) target resolution is unimplemented
  and, as currently HIR-shaped (no `binding` field on `HirCallee::
  Method`), cannot be implemented without a HIR node-shape change — no
  method/interface-implementation declaration syntax exists anywhere in
  the language yet for it to resolve against regardless. Not this task's
  or AICAD-053's evidenced scope.
- Calling a non-`Fn`, non-`Struct` binding (e.g. a plain `let`) is not
  itself flagged as a "not callable" type error — uniformly skipped
  alongside the (legitimate) `Struct`-constructor-call case AICAD-053
  will add. A worthwhile follow-up diagnostic, not required by either
  task's stated scope.
- `for`-loop variables never receive a resolved type — no
  collection/iterator type system exists anywhere in this compiler yet
  (`cad_units`/`cad_types` cover only primitives and named dimensions).
- Top-level `let`/`const` values are not forward-reference-checked
  (decision 2) — unchanged, already-documented limitation inherited from
  `AICAD-050`, not newly introduced here.
- `HirTypeRef::Generic` (`Vector2<Length>`, `List<Point2>`, ...) is never
  resolved to any type — no generic/collection type system exists yet
  anywhere in `cad-units`/`cad-types`.
- The concrete affine "delta" type-annotation spelling remains
  unspecified by RFC-0004 itself (explicitly deferred there as "Stage-2
  work") — this task's default-to-`Absolute` policy for a bare annotation
  is a sound, minimal, documented choice (decision 8), not a resolution
  of that open RFC question.

## Unresolved questions

None requiring `project/OWNER_DECISIONS.md` escalation. No public
language syntax/semantics changed beyond what DL-1/DL-2/DL-3/RFC-0004
already settled; no typed-units/affine semantics changed (only
*implemented*, per already-frozen rules); no dimension was ever silently
coerced (every cross-dimension/cross-scalar mismatch is a reported type
error); no ambiguous case was ever silently resolved (unit-literal
ambiguity and derived-dimension ambiguity both require an explicit
`expected`-type match, never a guess); no existing test/gate/check was
weakened; no compiler intrinsic was added. D3/D10/D11/D12/D15 remain open
and untouched — none blocked this task.
