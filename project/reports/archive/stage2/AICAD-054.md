# AICAD-054: Implement function execution and lexical scopes

## Objective

Batch S2-08's first task. Populate `crates/cad-runtime` (previously an
empty WP-04 placeholder) with a tree-walking evaluator over `cad_hir::
HirProgram` — the first Stage-2 component to actually *run* a program
rather than only analyze it. Per the fixed batch order, this task's scope
is exactly "function execution and lexical scopes": ordinary function
calls, parameter/local binding, and expression evaluation excluding
conditionals/loops (`AICAD-055`/`AICAD-056`'s own scheduled scope).

## Base commit

`0c01e58` ("Update SESSION_HANDOFF.md: Stage-2 Batch S2-07 complete") on
`origin/claude/aicad-stage2-dev` — this session started from that branch's
newest head, working tree clean, per the campaign brief's continuity rules.

## Files changed

- `crates/cad-runtime/Cargo.toml`: added dependencies (`cad-ast`,
  `cad-diagnostics`, `cad-hir`, `cad-types`, `cad-units`) and one
  dev-dependency (`cad-parser`, for test fixtures — mirrors `cad-hir`'s
  own dev-dependency exactly).
- `crates/cad-runtime/src/lib.rs` (new): crate-level module doc/re-exports.
- `crates/cad-runtime/src/value.rs` (new): `Value`/`NumberValue` — the
  runtime value representation.
- `crates/cad-runtime/src/error.rs` (new): `RuntimeError` — every
  execution failure mode, converted to a `cad_diagnostics::Diagnostic`
  under RFC-0005's `RUNTIME` family (or the wrapped `cad_units` error's
  own `UNIT-Exxx` code, for dimensional-arithmetic failures).
- `crates/cad-runtime/src/interp.rs` (new): `Interpreter` — the evaluator.
  24 tests.
- `crates/cad-runtime/README.md`: updated from the AICAD-002/003
  placeholder text to describe this task's actual scope.
- `project/TASKS.yaml`: `AICAD-054` status `todo` -> `done`.

## Material implementation decisions

1. **No separate "Engineering HIR."** `docs/plan/01_SYSTEM_ARCHITECTURE.md`
   §5's longer-run architecture sketch names an "Engineering HIR" layer
   between typed HIR and Feature IR/Geometry IR, but no Stage-2 batch
   schedules building one, and `docs/plan/02_LANGUAGE_AND_COMPILER.md`
   §17's own compiler-phase list has no phase for it either (phase 7 is
   "Lower to typed HIR", phase 12 is "Lower high-level features to
   Geometry IR" — execution sits between them with nothing else named).
   `Interpreter` therefore consumes `cad_hir::HirProgram` directly. Not an
   architecture decision this task is positioned to make unilaterally in
   either direction — flagging it here (not `OWNER_DECISIONS.md`, since
   nothing about this task's own implementation depends on resolving it,
   and no non-negotiable/escalation trigger applies) for whoever picks up
   `AICAD-059` (Geometry IR) to have the context if it becomes relevant.

2. **Numeric scalars collapse to one runtime tag (`crate::value`'s own
   module doc comment).** The type checker distinguishes `Int`/`UInt`/
   `Float`/`Decimal`; this evaluator does not, tagging every non-
   dimensional number `OperandType::Scalar(PrimitiveType::Float)`
   regardless of source shape. This is safe, not merely convenient:
   `cad_ast::BinaryOp` has no operator that could ever observe the
   distinction (no `%`, no bitwise/shift op, no truncating-division form —
   only `+ - * /`, which compute identical `f64` results either way).
   Re-deriving the type checker's own `expected`-type-directed defaulting
   here would risk the opposite of safety: an independently-guessed `Int`
   tag could spuriously fail `cad_units::check_binary_arithmetic`'s
   `ScalarTypeMismatch` against a sibling value the type checker had
   already (correctly, using context this evaluator lacks) resolved as
   `Float` — rejecting a program that type-checked cleanly. See decision 3
   for the analogous, *not* fully resolved gap this leaves for dimensional
   values specifically.

3. **Known, documented gap: ambiguous derived-dimension arithmetic.**
   `cad_hir::typeck::check_binary` threads an `expected` target dimension
   (from a `let`/parameter/return-type annotation) into `cad_units::
   check_binary_arithmetic` specifically to disambiguate a `*`/`/` whose
   result vector matches more than one named `Dimension` (Pressure/Stress,
   Torque/Energy). This evaluator calls the same function with
   `expected: None` (`Interpreter::eval_arith`) — threading the same
   `expected` context through would require re-implementing `cad_hir::
   typeck`'s own `HirTypeRef` resolution independently in this crate,
   which is exactly the divergence risk decision 2 above already flags,
   generalized: a second independent resolver for the same context can
   only ever match or diverge from the first, never safely replace it.
   A source program relying on annotation-driven disambiguation for such
   an expression type-checks successfully but fails at runtime with
   `UNIT-E109` (`AmbiguousDerivedDimension`). Narrow in practice (six unit
   spellings — `Pa`/`kPa`/`MPa`/`GPa`/`psi`/`ksi` — across two dimension
   pairs) and not evidenced as needed by this task's own acceptance (no
   test program needs it), but a real, not-yet-closed gap for whichever
   later task next extends this evaluator's context. Recorded here rather
   than `OWNER_DECISIONS.md` because closing it doesn't require selecting
   among materially different architectures — it needs the same
   `expected`-threading mechanism decision 2 already describes, just
   applied to one more call site once a `HirTypeRef`-resolution path is
   available to this crate (e.g. if `TypeCheckResult` is ever extended
   with a per-expression-node type map — not attempted here, since
   `cad_hir::typeck`'s public output shape is a different, already-
   checkpointed batch's (`S2-07`) surface, and widening it was not this
   task's assigned scope).

4. **`call_by_name`/direct-`Value` injection is a convenience API, not a
   type-checking bypass.** `Interpreter::call_by_name`/`call_by_values`
   bind caller-supplied `Value`s straight to parameter slots with no
   cross-check against the callee's *declared* parameter types. This is
   intentional (the type checker's job is done once, statically, before
   execution — see `src/interp.rs`'s module doc comment "This evaluator
   trusts, but verifies") and is exactly what lets this task's own test
   suite exercise `eval_arith`/`eval_comparison`'s defensive dimensional-
   mismatch paths (conditions no *source* program that type-checks could
   ever reach) without hand-building a malformed HIR tree.

5. **Finding, not fixed:** `cad_units::check_comparison` requires a
   *numeric* scalar unconditionally, even for `==`/`!=`/`~=` — meaning
   `cad_hir::typeck` (already checkpointed at `S2-07`) currently rejects
   `String == String`/`Bool == Bool` as a `UNIT-E101` diagnostic
   (`NonNumericOperand`) at compile time. Confirmed empirically: a test
   source string `"fn f(a: String, b: String) -> Bool { return a == b; }"`
   fails this task's own `compiled()` type-checking fixture with exactly
   that diagnostic. Out of scope to fix here (a different, already-passed
   batch's own component; fixing it is a `cad_units`/`cad_hir::typeck`
   change with its own required-checks/regression-suite obligations this
   task was not assigned). This evaluator's own `Value::Bool`/`Value::Str`
   comparison-dispatch branches (`Interpreter::eval_comparison`) are
   implemented and tested directly (bypassing `compiled()`, since no
   currently-compilable source program can reach them) so the runtime
   itself is ready the moment that type-checker gap closes. Recorded here
   as an open finding for the owner/next relevant task, not silently
   worked around and not escalated (fixing a pre-existing scope gap in a
   different completed task is not one of `AGENTS.md`'s listed escalation
   triggers).

6. **Struct/enum value construction has no owning task yet.** `AICAD-053`
   fully type-checks struct-literal construction (via ordinary call
   syntax) and enum-variant values/equality, but no Stage-2 batch task
   title mentions giving either a *runtime* value representation
   (`AICAD-055`/`056`/`057`/`058`'s own titles are conditionals, loops,
   recursion/`Result`, and resource budgets respectively). This task
   leaves both unimplemented (`RuntimeError::NotCallable`/`Unsupported`
   for a struct constructor call or field access) rather than
   speculatively building a data-value model no task has asked for yet
   (`AGENTS.md`: "Small private scaffolding strictly necessary for the
   current task is allowed. Public APIs, semantics, ... owned by a later
   task must wait for that task."). Flagged here (not escalated — this is
   a scope-sequencing observation, not one of the listed escalation
   triggers) since it affects whether `AICAD-063`'s end-to-end proof
   program can use custom struct/enum types, or must stay within
   primitive/dimensional values and functions.

## Design (see `crates/cad-runtime/src/interp.rs`'s own module doc comment
for the full account)

- One flat `HashMap<BindingId, Value>` per function call ("`Frame`"), no
  scope-stack push/pop — mirrors `cad_hir::typeck`'s own identical "no
  scope stack needed" design, for the identical reason (lowering already
  scoped every `BindingId` correctly).
- A `Signal` enum (`Return(Value) | Error(RuntimeError)`) as the `Err` case
  of every evaluation method's `Result`, so a `return` nested arbitrarily
  deep inside expressions (including inside a block-expression's trailing
  position) propagates for free via `?` without a separate signaling
  channel. Verified directly by `nested_return_inside_block_expression_
  short_circuits`.
- Dimensional literal magnitudes are stored in their dimension's canonical
  unit (`docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md`'s own "Internally
  normalize quantities to canonical units" guidance), reusing `HirExpr::
  Literal::ty` (already resolved by lowering whenever the unit symbol is
  unambiguous) rather than re-deriving it, and `cad_units::{lookup,
  to_canonical_absolute}` for the actual conversion. `*`/`/` need no
  conversion factor at all beyond raw magnitude arithmetic, since every
  canonical unit in the registry is SI-coherent (verified for `Area`:
  `length_times_length_derives_area`).
- Every arithmetic/comparison/negation operation still calls the same
  `cad_units::{check_binary_arithmetic, check_comparison, check_unary_neg}`
  the type checker uses, reusing its own computed *result* `OperandType`
  rather than re-deriving dimensional-result rules independently (DRY;
  also what correctly produces `Absolute`/`Delta` affine-kind results for
  `+`/`-` without this crate re-implementing RFC-0004 §7's own table).
- Division by zero is rejected unconditionally (scalar or dimensional)
  rather than propagating IEEE-754 `inf`/`NaN` — a reversible interpreter-
  level judgment call (not evidenced by any RFC), not a public
  language-semantics decision, so not escalated.
- `~=` (`ApproxEq`) behaves identically to `==` — matching `cad_hir::
  typeck::check_binary`'s own identical treatment (RFC-0004 §6's
  `Tolerance<T>` is not implemented anywhere in the compiler stack yet,
  per `cad_units::arithmetic`'s own documented scope boundary), not an
  independent invention.
- Every out-of-scope HIR shape (`if`/`match` expressions and statements,
  `for`/`while`/`loop`/`break`/`continue`, field access, method calls) is
  matched exhaustively and returns `RuntimeError::Unsupported` — never a
  panic, verified directly by `unsupported_if_statement_is_a_clean_error`.

## Exact commands and results

```
cargo build -p cad-runtime
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.46s

cargo clippy -p cad-runtime --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.21s   (clean, after fixing
    two `clippy::result_large_err` and one `clippy::useless_conversion` finding from the first pass)

cargo fmt --all -- --check
    (clean, after running `cargo fmt --all` once to apply rustfmt's own struct-variant formatting)

cargo test -p cad-runtime
    running 24 tests
    test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.30s

cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.14s   (clean)

cargo test --workspace
    every test binary: test result: ok, 0 failed (includes the full native/OCCT Stage-1 suite and
    every Stage-2 front-end suite from prior batches, all unaffected by this task)
```

## Tests (24, in `crates/cad-runtime/src/interp.rs`)

Function execution/scoping: `calls_a_function_with_arguments`,
`lexical_scope_let_and_reassignment`,
`calling_sibling_function_declared_later_in_source`,
`nested_return_inside_block_expression_short_circuits`,
`block_with_no_trailing_expression_is_unit`,
`missing_return_on_declared_return_type_is_an_error`. Parameters:
`default_parameter_value_used_when_argument_omitted`,
`missing_required_argument_is_an_error`,
`too_many_arguments_is_an_error`, `named_argument_at_a_real_call_site`.
Dimensional arithmetic: `length_literal_addition_uses_canonical_metres`,
`length_times_length_derives_area`,
`dimensional_argument_passed_directly_as_a_value`,
`mixed_dimension_addition_is_a_dimensional_arithmetic_error`,
`division_by_zero_is_an_error`, `unary_negation`. Comparisons:
`numeric_comparisons`, `string_equality` (direct-dispatch, see decision 5),
`approx_eq_behaves_like_eq_until_tolerance_semantics_exist`. Globals:
`top_level_const_is_visible_inside_a_function`,
`top_level_const_referenced_before_run_top_level_is_unbound`. Scope
boundaries fail cleanly: `unsupported_if_statement_is_a_clean_error`,
`calling_a_non_function_binding_is_not_callable`,
`struct_construction_is_not_yet_supported`.

## Known limitations

- Ambiguous derived-dimension arithmetic (decision 3) and the pre-existing
  `String`/`Bool` equality type-checker gap (decision 5, not this task's
  to fix) — both above.
- No genuine self/mutual recursion is exercised by this task's own test
  suite: any conditional needed to terminate recursion is itself
  `Unsupported` until `AICAD-055`, so a real recursion test is deferred to
  wherever `AICAD-057` ("recursion and Result/error propagation") lands;
  the call-dispatch machinery itself (a function calling a distinct,
  later-declared sibling) is tested
  (`calling_sibling_function_declared_later_in_source`).
- No resource/recursion-depth bound exists yet (`AICAD-058`'s own
  scheduled scope) — a sufficiently deep call chain would still overflow
  the host Rust stack. Not attempted here per the fixed batch order.
- `part` items are indexed for `fn` lookup only (so a function nested
  inside a `part` can still be called by this crate's own APIs); `part`
  *instantiation* semantics do not exist yet and are out of this task's
  scope.
- No original source/display unit is preserved on a runtime `Value`
  (`crate::value`'s own module doc comment) — only the canonical
  magnitude. Diagnostics built from a runtime value (none exist yet that
  need to) would need this later.

## Unresolved questions

- Whether/where "Engineering HIR" (decision 1) is meant to land in the
  fixed Stage-2 batch schedule, or whether the schedule's own execution/
  Geometry-IR split (`AICAD-054`-`058` then `AICAD-059`) already
  supersedes that longer-run architecture note for Stage 2's purposes.
  Not blocking any completed or remaining Stage-2 task as currently
  scheduled; noted for whoever scopes `AICAD-059`.
- Which future task is meant to extend this evaluator's `expected`-type
  context (decision 3) and/or give struct/enum values a runtime
  representation (decision 6) — neither is named in the fixed S2-08
  through S2-14 batch list as currently written.
