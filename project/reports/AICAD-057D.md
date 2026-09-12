# AICAD-057D — Generic instantiation/inference/type checking for the approved Stage-2 generic subset

## Objective

Fourth step of the `D17`-mandated remediation sequence
(`project/OWNER_DECISIONS.md#D17`, `project/DECISION_LOG.md#DL-14`,
`project/reports/AICAD-057A.md`, `AICAD-057B.md`, `AICAD-057C.md`): close
the four gaps `AICAD-057B`'s and `AICAD-057C`'s own "Known limitations"
sections explicitly deferred here —

1. call-site type-parameter instantiation/inference for generic functions
   (`identity(5mm)` infers `T = Length`);
2. `Name<Args>` type-reference resolution for a user-defined generic
   struct/enum (`Pair<Length, Mass>`), with substitution propagated into
   field access, struct-literal/variant construction, and variant
   patterns;
3. a diagnostic (never an arbitrary selection) for an ambiguous generic
   call; and
4. a diagnostic for a wrong number of type arguments on a `Name<Args>`
   type reference.

Explicitly **not** this task's scope, per the owner's own fixed task
breakdown: `Result<T,E>`/`Optional<T>` themselves (`AICAD-057E`); no
call-site turbofish/explicit-type-argument syntax (the grammar has none,
and none was added); no interface/trait bounds, higher-kinded types, or
any other item on `D17`'s scope-limit list.

## Base commit

`91ff121` ("AICAD-057C: Implement data-carrying enum variants,
constructors, destructuring patterns, and match exhaustiveness") — the
tip of `origin/claude/aicad-stage2-dev` at session start.

## Scope decisions (not escalated — applies the existing D17 ruling)

1. **New `CheckedType::Instantiated { base: BindingId, args:
   Vec<CheckedType> }`** — `base` is the declaring struct/enum's own
   `BindingId` (the same identity `CheckedType::Struct`/`::Enum` already
   use); `args` are the resolved, already-substituted type arguments in
   declared order. Nominal (not structural) equality: two `Instantiated`s
   are compatible only when `base` matches and every argument pairwise
   agrees (`types_compatible`). This is the design named as an option in
   the task brief, chosen because it slots directly into the existing
   `CheckedType` enum shape and needs no new side table.
2. **`CheckedType` is no longer `Copy`** (only `Clone`) — the new
   `Instantiated` variant carries a `Vec<CheckedType>`, which cannot be
   `Copy`. This is a purely mechanical consequence: every call site that
   relied on an implicit copy now clones explicitly (`ParamSig`'s own
   `derive(Copy)` was dropped for the same reason). No existing test's
   assertions needed to change — `assert_eq!` borrows rather than moves,
   so every pre-existing `checked.binding_types[...]` comparison in the
   test suite kept compiling unchanged.
3. **Substitution and unification are free functions, not `Checker`
   methods** (`substitute_type`/`substitute_opt`/`unify_type_param`) —
   neither needs `self` (an instantiation's own `args` are carried inline
   on `CheckedType::Instantiated` itself, not looked up from a side
   table), so keeping them free functions avoids an unnecessary borrow of
   `self` at every call site, mirroring `types_compatible`'s/
   `value_types_compatible`'s own existing free-function shape.
4. **Generic-struct/enum construction and record-literal substitution is
   driven by the call's own contextual/expected type, not by
   unifying from constructor arguments.** For a generic **function**
   call, argument-driven inference is exactly what the task asks for
   (`identity(5mm)`). For a generic **struct/enum construction**
   (`Pair(5mm, 2kg)`, `Full(5mm)`, `Boxed { value: 5mm }`), the task's own
   wording only requires substitution "wherever that instantiated type's
   fields are later inspected (field access, struct-literal field-type
   checking, etc)" — i.e. driven by an already-known instantiated type
   from context (a `let`/`return`/parameter annotation), not by inventing
   a second, independent unification pass for constructor calls. This
   keeps the two mechanisms (generic-function call inference vs.
   generic-type-instantiation substitution) cleanly separated and matches
   the task's own explicit examples. See "Known limitations" for the
   resulting gap (a bare `let p = Pair(5mm, 2kg);` with no annotation at
   all still does not infer `T`/`U`).
5. **An unknown `Name<Args>` (no matching struct/enum, not `List`/
   `Range`) still resolves to `None` silently, exactly as before this
   task** — only a *known* struct/enum referenced with the *wrong arity*
   is a genuine diagnostic. `resolve_generic_type_application` is reached
   only after the dedicated `List`/`Range` arms fail to match, and itself
   returns `None` immediately (`self.type_names.get(name)?`) for any name
   not registered as a struct/enum — preserving `resolve_type_ref`'s own
   documented convention ("not yet a typeable one, not a wrong name") for
   every remaining unresolvable case (e.g. a plan-level sketch such as
   `Vector2<Length>` with no matching declaration).
6. **`List`/`Range` were *not* migrated onto the new general
   instantiation machinery.** `D17`'s "List/Range cleanup" note
   authorizes but does not require this migration before `AICAD-057F`.
   `List<T>`/`Range<T>` keep their own dedicated, non-generic
   `CheckedType::List`/`::Range` variants and dedicated `resolve_type_ref`
   arms (checked *before* the new general arm, so they never reach it) —
   migrating them onto `CheckedType::Instantiated` would require deciding
   what "`List`'s own declaring `BindingId`" even means for a built-in
   with no `HirItem::Struct`/`Enum` declaration anywhere in a program,
   which is exactly the kind of architectural question this task's own
   scope does not ask it to resolve. Documented here per the task's own
   "document what you did and did not migrate" instruction.
7. **`check_call`, `check_struct_construction`,
   `check_variant_tuple_construction`, and `check_record_literal` all
   gained an `expected: Option<CheckedType>` parameter** — previously
   `check_call` did not receive the call expression's own contextual type
   at all (`check_expr`'s `Call` arm discarded it). Threading it through
   is what makes both requirement #1 (generic function inference from
   return-type context) and requirement #2 (struct-literal/variant
   construction substitution) possible; every existing (non-generic) call
   site is unaffected, since a concrete, non-generic constructor/function
   never inspects `expected` at all.

## Changes by file

- **`crates/cad-hir/src/typeck.rs`** (the only file touched — `cad-ast`/
  `cad-parser`/`cad-compiler`/`cad-runtime` needed no changes; confirmed
  by direct inspection that `CheckedType` is private to this module and
  never reaches `cad-runtime`, so generics remain fully compile-time-only
  per `D17`):
  - New `CheckedType::Instantiated { base, args }` (see scope decision 1);
    `CheckedType` and `ParamSig` lost `derive(Copy)` (scope decision 2).
  - `types_compatible` gained an `Instantiated`/`Instantiated` arm
    (nominal: same `base`, pairwise-compatible `args`).
  - `describe` gained an `Instantiated` arm (`Name<Arg1, Arg2>` rendering
    for diagnostics).
  - New free functions `substitute_type`/`substitute_opt` (recursive
    `TypeParam` substitution, also recursing into a nested
    `Instantiated`'s own args) and `unify_type_param` (structural
    unification of a raw, possibly-`TypeParam`-carrying declared type
    against a concrete observed type, extending a `subst` map, reporting
    failure on a structural mismatch or an inconsistent re-binding).
  - New `Checker::type_params_of: HashMap<BindingId, Vec<BindingId>>`
    (a struct/enum's own declared type-parameter identity, in order) and
    `Checker::instantiation_subst` (builds the substitution map for one
    instantiation by zipping `type_params_of` against supplied args) —
    both populated/used alongside the pre-existing `type_names`/
    `active_type_params` machinery.
  - `register_type_names` now also populates `type_params_of` for every
    struct/enum.
  - `resolve_type_ref`'s `HirTypeRef::Generic` catch-all (previously
    unconditional `None`) now calls new `resolve_generic_type_application`
    — arity-checks against `type_params_of`, resolves each argument
    recursively, and returns `CheckedType::Instantiated`.
  - `FnSignature` gained `type_params: Vec<BindingId>` (empty for a
    non-generic function); `collect_signatures` populates it.
  - `check_call`/`check_expr`'s `Call`/`RecordLiteral` arms now thread
    `expected: Option<CheckedType>` through to `check_struct_construction`,
    `check_variant_tuple_construction`, `check_record_literal`, and (for a
    generic function) new `check_generic_call`.
  - New `Checker::check_generic_call` — the three-pass call-site
    inference algorithm for a generic function (match args to parameter
    slots; check each argument's own type and unify it against the raw
    declared parameter type, also unifying the raw return type against
    the call's own `expected` context; diagnose any still-unbound type
    parameter as ambiguous, otherwise re-compare every argument against
    its *substituted* declared type and return the substituted return
    type).
  - `check_struct_construction`/`check_variant_tuple_construction`/
    `check_record_literal` each gained the "if `expected` names a genuine
    instantiation of *this* struct/enum, substitute its field/payload
    types before checking, and return that same instantiation" logic
    (scope decision 4/7); each also updated its final return type to
    `CheckedType::Instantiated` in that case instead of the bare
    non-generic `Struct`/`Enum`.
  - `check_field_access` gained an `Instantiated`-wrapping-a-struct arm
    (substitutes the found field's type via `instantiation_subst`); an
    `Instantiated` wrapping an *enum* base correctly falls through to the
    existing "not a struct" diagnostic arm, unchanged.
  - `check_match_exhaustiveness`/`check_pattern_enum_match` both now
    accept a scrutinee typed as `CheckedType::Instantiated` (extracting
    `base` for enum-identity/exhaustiveness purposes exactly like a plain
    `CheckedType::Enum`).
  - `bind_pattern`'s `Tuple`/`Record` arms now build a substitution map
    from the scrutinee's own `Instantiated` args (when present) before
    binding each sub-pattern's declared field type — this is the
    "substitute the enum's own type parameters ... when checking a
    variant construction/pattern against an instantiated generic enum
    type" the task names explicitly.
  - Every other call site that read a `CheckedType`/`Option<CheckedType>`
    by value from a place more than once (a consequence of dropping
    `Copy`) now clones explicitly — `check_expected`, `check_block`'s
    `If` arm, `check_match`'s per-arm loop, `check_binary`'s comparison
    and `Add`/`Sub` arms, `check_list_literal`, `check_range_expr`,
    `unify_value_type`, `HirStmt::Assign`/`Return`, and the `HirItem::Fn`
    body's `current_fn_return` save/restore. None of these change any
    existing diagnostic's condition or wording — confirmed by the full,
    unchanged-assertion 146/146 pre-existing `cad-hir` test pass (see
    Commands/results).
  - One pre-existing test's own comment
    (`generic_enum_tuple_variant_destructures_cleanly`) updated in place
    to note that `Box<Int>` now genuinely resolves (this task's own
    change) rather than remaining `None` — the test's assertion itself
    (`diagnostics.is_empty()`) is unchanged and still passes.

## New diagnostics (provisional `TYPE-Exxx`, pending `D10` — same convention as every prior batch)

| Code | Name | Where |
|---|---|---|
| `TYPE-E457` | `TOO_FEW_TYPE_ARGUMENTS` | `Name<Args>` reference against a known struct/enum, too few arguments |
| `TYPE-E458` | `TOO_MANY_TYPE_ARGUMENTS` | `Name<Args>` reference against a known struct/enum, too many arguments |
| `TYPE-E459` | `AMBIGUOUS_GENERIC_CALL` | a generic function call whose type parameter(s) cannot be determined from arguments/context |

An inconsistent multi-occurrence type-parameter binding (`fn
pair_of<T>(a: T, b: T)` called with a `Length` then a `Mass`) deliberately
reuses the existing `TYPE-E418 ARGUMENT_TYPE_MISMATCH` rather than a new
code — once `T` is bound from the first argument, the second argument's
mismatch against the now-substituted parameter type is structurally
identical to an ordinary argument-type mismatch, and inventing a
generic-specific code for the same observable condition would be an
unmotivated special case.

## Required tests (from D17's remediation list, the AICAD-057D-owned slice)

All added to `crates/cad-hir/src/typeck.rs`'s existing `#[cfg(test)] mod
tests` (146 -> 167, +21), under a new "Generic instantiation/inference"
section:

- generic struct, one type parameter, referenced as a type —
  `generic_struct_with_one_type_parameter_referenced_as_a_type_resolves`.
- generic struct, two type parameters, referenced as a type, both fields
  substitute correctly —
  `generic_struct_with_two_type_parameters_referenced_as_a_type_resolves_both_fields`.
- generic enum referenced as a type, with variant construction *and*
  destructuring against the instantiated type, tuple and record shapes,
  enum named `Holder`/`Wrap` (not `Result`/`Optional`/`List`/`Range`) —
  `generic_enum_tuple_variant_construction_against_instantiated_type_checks_cleanly`,
  `generic_enum_tuple_variant_pattern_against_instantiated_type_yields_substituted_field_type`,
  `generic_enum_record_variant_construction_against_instantiated_type_checks_cleanly`,
  `generic_enum_record_variant_pattern_against_instantiated_type_yields_substituted_field_type`.
- generic function, successful inferred call (`identity(5mm)` infers `T =
  Length`) — `generic_function_call_infers_type_parameter_from_argument`.
- ambiguous generic call -> diagnostic —
  `ambiguous_generic_call_with_type_parameter_only_in_return_type_is_reported`
  (`TYPE-E459`), contrasted with
  `generic_function_call_infers_type_parameter_from_expected_return_context`
  (the same function, called where an expected return-type context makes
  it *not* ambiguous).
- wrong number of type arguments -> diagnostic, too few and too many as
  separate tests —
  `generic_struct_type_reference_with_too_few_type_arguments_is_reported`
  (`TYPE-E457`),
  `generic_struct_type_reference_with_too_many_type_arguments_is_reported`
  (`TYPE-E458`).
- inconsistent multi-occurrence type-parameter binding -> diagnostic, not
  silent coercion —
  `generic_function_called_with_inconsistent_argument_types_for_the_same_type_parameter_is_reported`
  (`TYPE-E418`, `fn pair_of<T>(a: T, b: T)` called with `5mm`/`2kg`).
- nested/nontrivial case —
  `nested_generic_type_reference_resolves_and_substitutes_recursively`
  (`Wrapper<Pair<Length, Mass>>`, substitution threaded through two
  levels of instantiation).

## Adversarial / negative tests

- `generic_struct_field_access_substitutes_the_declared_type_parameter_not_a_wildcard`,
  `generic_struct_two_type_parameters_second_field_substitutes_its_own_parameter_not_the_first`,
  `generic_enum_tuple_variant_pattern_binding_is_not_a_wildcard_type`,
  `generic_function_call_argument_inference_is_not_a_wildcard`,
  `nested_generic_type_reference_field_access_is_not_a_wildcard` — each
  deliberately declares the *wrong* expected/return type after a
  substitution and asserts exactly one `TYPE-E419 RETURN_TYPE_MISMATCH`.
  These exist because "no diagnostics" alone does not prove a
  substitution produced the *correct* concrete type rather than merely
  something unresolved (`None` silently propagates with no diagnostic in
  this module's own established convention) — asserting a *specific,
  intentionally-wrong* expected type turns "field/pattern access
  substituted correctly" into a falsifiable claim.
- `generic_enum_tuple_variant_construction_wrong_expected_type_is_reported`/
  `generic_enum_record_variant_construction_wrong_expected_type_is_reported`
  — the payload value's dimension genuinely disagrees with the
  instantiation's substituted field type (`TYPE-E449
  VARIANT_FIELD_TYPE_MISMATCH`), not silently accepted.
- `unknown_generic_type_name_still_resolves_to_none_without_a_diagnostic`
  — regression guard proving this task did not turn *every* unresolved
  `Name<Args>` into a diagnostic, only a known struct/enum with the wrong
  arity (`Vector2<Length>`, naming nothing declared, stays silently
  unresolved exactly as before this task).
- Full pre-existing `cad-ast`/`cad-parser`/`cad-compiler`/`cad-hir`/
  `cad-runtime` suites re-run unchanged (see Commands/results) — in
  particular every `AICAD-056` `List<T>`/`Range<T>` test, every
  `AICAD-053` non-generic struct/enum test, and every `AICAD-057B`/`C`
  generic-declaration/data-carrying-enum test, none of which needed any
  assertion changed.

## Commands / results

- `cargo fmt --all -- --check` — clean (`cargo fmt --all` applied once
  during this task for the new test block's own line wrapping; verified
  clean afterward).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — clean.
- `cargo build --workspace --all-targets` — clean.
- `cargo test --workspace` — every crate `ok`, 0 failed. Per-crate deltas
  from `AICAD-057C`'s own baseline: `cad-ast` 19 -> 19 (unchanged);
  `cad-parser` 119 -> 119 (unchanged); `cad-compiler` 49 -> 49
  (unchanged); `cad-hir` 146 -> 167 (+21, all new); `cad-runtime` 69 -> 69
  (unchanged — confirms no runtime-visible change was needed, matching
  the task brief's own expectation that generics stay compile-time-only).

## Known limitations

- **Generic struct/enum construction does not infer type arguments from
  its own constructor arguments alone** — only from the call's own
  contextual/expected type (scope decision 4). A bare `let p = Pair(5mm,
  2kg);` with no annotation still checks the construction's fields
  against the raw, unsubstituted `TypeParam` field types (reporting a
  spurious `STRUCT_FIELD_TYPE_MISMATCH`, the same honest interim
  behavior `AICAD-057B` already documented for generic function calls
  before this task). The task's own required-test list and D17's worked
  examples both only exercise instantiation via a type-reference/expected
  context, so this narrower scope was deliberately chosen over
  duplicating `check_generic_call`'s argument-unification machinery for
  constructors too; a future task could extend `check_struct_construction`/
  `check_variant_tuple_construction` to fall back to argument-driven
  inference (mirroring `check_generic_call`) when no `expected` context
  names the right instantiation, if a later gate finds this gap material.
- **`List<T>`/`Range<T>` were not migrated onto the new general
  `CheckedType::Instantiated` machinery** (scope decision 6) — `D17`'s
  "List/Range cleanup" explicitly does not require finishing this before
  `AICAD-057F`, and doing so would need to decide what a built-in
  collection's own "declaring `BindingId`" means with no corresponding
  `HirItem` in any program, which this task's own scope does not ask it
  to resolve.
- **A generic function's argument type is checked with no contextual
  hint during inference** (`check_generic_call`'s Pass 2 always calls
  `check_expr(expr, None)`) — an ambiguous-unit literal argument (e.g.
  `identity(5Pa)`, ambiguous between `Pressure`/`Stress`) cannot be
  resolved by the not-yet-known parameter type the way an ordinary
  (non-generic) call's declared parameter type already resolves such
  ambiguity. No required test exercises this combination; documented here
  as a known, narrow gap rather than silently left undiscovered.
- **No `..` rest pattern, no interface/trait bounds, no turbofish** — all
  unchanged from `D17`'s own scope limit and prior batches; nothing in
  this task touched any of them.

## Status

`AICAD-057D` is complete. Next: `AICAD-057E` (`Result<T,E>`/`Optional<T>`
via the ordinary generic-enum machinery this task's instantiation support
now makes usable). Do not begin `AICAD-057E` until this report and
`project/TASKS.yaml`'s status update are both committed.
