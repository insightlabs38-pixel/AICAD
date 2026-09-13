# AICAD-057B — Generic parameter/type-application syntax plus AST/HIR representation

## Objective

First step of the `D17`-mandated remediation sequence
(`project/OWNER_DECISIONS.md#D17`, `project/DECISION_LOG.md#DL-14`,
`project/reports/AICAD-057A.md`): give `fn`/`struct`/`enum` declarations an
ordinary generic type-parameter list (`struct Pair<T, U> { ... }`, `enum
Optional<T> { ... }`, `fn identity<T>(...)`) and represent it through the
AST and HIR layers, with the minimum type-checker support needed for a
generic declaration's own field/parameter/return types (and body) to
resolve `T`-style names without spurious `UNKNOWN_TYPE_NAME` diagnostics.
Explicitly **not** this task's scope (per the owner's own fixed task
breakdown): data-carrying enum variants/destructuring/exhaustiveness
(`AICAD-057C`), call-site generic instantiation/inference/arity checking
(`AICAD-057D`), and `Result<T,E>`/`Optional<T>` themselves (`AICAD-057E`).

## Scope decision (not escalated — applies the existing D17 ruling)

A `Name<Args>` type *reference* already parsed/lowered generically before
this task (`cad_ast::item::Type::Generic`, `cad_hir::types::HirTypeRef::
Generic` — used today only by `List<T>`/`Range<T>`, `D16`). The genuine gap
was entirely on the **declaration** side: no `struct`/`enum`/`fn` could
declare its own type parameters at all. This task closes exactly that gap
and, deliberately, no more:

- A generic declaration's own type-parameter names resolve as valid
  (opaque) types *within that declaration's own signature and body* (a new
  `CheckedType::TypeParam(BindingId)`), so declaring `struct Pair<T, U> {
  first: T; second: U; }` or `fn identity<T>(value: T) -> T { let y: T =
  value; return y; }` type-checks cleanly today.
- A `Name<Args>` type *reference* to a user-defined generic struct/enum
  (e.g. writing `Pair<Length, Mass>` somewhere) is **not** resolved by this
  task — it falls through the pre-existing `HirTypeRef::Generic => None`
  catch-all exactly as it did before (no diagnostic — "not yet a typeable
  one, not a wrong name"), same as every other not-yet-handled generic
  reference. Resolving it (arity checking, type-argument substitution) is
  `AICAD-057D`'s explicit job ("generic instantiation/inference/type
  checking"), not this one's — collapsing the two would blur exactly the
  boundary the owner's own fixed task list draws.
- Calling a generic function before `AICAD-057D` lands is expected to
  report a type mismatch (the call's concrete argument type is not
  `types_compatible` with the unsubstituted `CheckedType::TypeParam`) —
  this is a correct, honest interim state, not a bug: nothing in this
  task's own scope authorizes inventing substitution/unification.

## Changes

- `crates/cad-ast/src/item.rs`: `Item::Fn`/`Item::Struct`/`Item::Enum` each
  gained `type_params: Vec<Spanned<String>>` (empty for an ordinary
  declaration).
- `crates/cad-ast/src/printer.rs`: prints `<T, U>` after the declared name
  when non-empty (`print_type_params`).
- `crates/cad-parser/src/lib.rs`: `parse_type_params()` — an optional `"<"
  identifier { "," identifier } [","] ">"` list, positioned right after the
  declared name for all three declaration forms. No lexer change needed
  (`<`/`>`/`,` already tokenize; declaration position is never ambiguous
  with a comparison expression).
- `specs/language/grammar.ebnf`: `fn_decl` amended, new `type_params`
  production (also documented as applying, in the same shape, to the
  still-only-informally-specified `struct_decl`/`enum_decl`), header patch
  note per the file's own required process.
- `crates/cad-hir/src/ids.rs`: new `BindingKind::TypeParam` — explicitly
  documented as a *type*-namespace kind, unlike every other `BindingKind`.
- `crates/cad-hir/src/hir.rs`: new `HirTypeParam { binding, name, span }`;
  `type_params: Vec<HirTypeParam>` added to `HirItem::Fn`/`Struct`/`Enum`.
- `crates/cad-hir/src/lower.rs`: `Lowerer::mint_type_params` mints a fresh
  `BindingId` per declared type parameter and diagnoses a duplicate name
  within one declaration's own list (`TYPE-E445 DUPLICATE_TYPE_PARAMETER`)
  — deliberately **not** using `mint`/`self.scopes` (the ordinary
  value-lookup chain), since a type parameter is never a value-level name
  (`crate::binder`'s own module doc comment already assigns all type-name
  resolution to the type checker). `DeclaredItem` gained a `Generic`
  variant (`Fn`/`Struct`) and `Enum` gained a `type_params` field, so
  declaration and lowering stay a two-pass, forward-reference-safe
  operation exactly like every other item kind.
- `crates/cad-hir/src/typeck.rs`: new `CheckedType::TypeParam(BindingId)`
  (compatible only with itself, by `BindingId`); `Checker::
  active_type_params: HashMap<String, BindingId>` and `Checker::
  with_type_params` scope it for exactly the duration of resolving one
  generic declaration's own fields (`collect_struct_fields`), signature
  (`collect_signatures`), and body (`check_item`'s `Fn` arm, so a `let y: T
  = ...;` local inside a generic function's own body also resolves `T`).
  `resolve_type_ref`'s `Named` arm checks `active_type_params` before
  primitives/dimensions/struct-enum names (no observed effect on any
  non-generic program — no primitive/dimension/struct/enum is plausibly
  named `T`/`U` in practice).

## Decisions

1. **Type parameters never enter the value-level scope chain.** Confirmed
   by a dedicated adversarial test
   (`type_parameter_names_are_never_inserted_into_the_value_scope`):
   `fn identity<T>(value: T) -> T { T; return value; }` reports exactly one
   `TYPE-E410` (undefined value identifier `T`) — the type parameter `T`
   and a hypothetical value named `T` are completely independent
   namespaces, matching `crate::binder`'s own existing type/value
   namespace split.
2. **`with_type_params` is a save/restore, not an unconditional clear.**
   Generic declarations never nest in AICAD today (no nested `fn`
   declarations exist at all), so this has no observable effect now, but
   keeps the mechanism correct without new invariants if a later task ever
   changes that.
3. **Declaration-level generics only — reference-side resolution deferred
   to `AICAD-057D`.** See "Scope decision" above.

## Required tests (from `D17`'s remediation list, declaration-level slice)

- generic struct with one type parameter —
  `parses_generic_struct_with_one_type_parameter` (parser),
  `generic_struct_type_params_mint_bindings_with_type_param_kind`
  (lowering), `generic_struct_with_one_type_parameter_type_checks_cleanly`
  (typeck).
- generic struct with two type parameters —
  `parses_generic_struct_with_two_type_parameters`,
  `generic_struct_with_two_type_parameters_type_checks_cleanly`.
- generic enum — `parses_generic_enum`, `generic_enum_type_params_are_
  lowered`, `generic_enum_type_checks_cleanly`.
- generic function — `parses_generic_function`, `generic_fn_type_params_
  are_lowered`,
  `generic_function_with_matching_param_and_return_type_parameter_type_
  checks_cleanly`.
- wrong number of type arguments / ambiguous generic call / successful
  inferred call: **not** this task's tests — `AICAD-057D`'s (call-site
  instantiation does not exist yet).

Additional coverage this task added on its own initiative: round-trip
printing of all four generic-declaration shapes
(`round_trips_generic_declarations`); ordinary non-generic declarations
still parse/lower with an empty `type_params` list (regression guard,
parser + lowering); a trailing comma in a type-parameter list
(`generic_type_parameter_list_allows_a_trailing_comma`); malformed
type-parameter lists — missing `>`, a non-identifier token — each produce
a diagnostic rather than hanging or panicking
(`reports_missing_closing_angle_bracket_on_type_param_list`,
`reports_non_identifier_in_type_param_list`); a duplicate type-parameter
name is reported once and both occurrences still get their own binding
(`duplicate_type_parameter_name_is_reported`,
`...is_reported_by_type_checking_too`); two different generic declarations
that happen to both name a parameter `T` use independent, non-equal
`BindingId`s (`two_different_generic_declarations_use_independent_type_
parameters`); a generic function whose two type parameters are genuinely
different is still correctly rejected when a `U`-typed value is returned
where `T` is expected
(`mismatched_generic_function_type_parameters_are_reported`) — proof that
`CheckedType::TypeParam` performs real per-declaration identity, not a
universal wildcard; a generic function body can reference its own type
parameter in a `let` annotation
(`generic_function_body_can_reference_its_own_type_parameter_in_a_let_
annotation`).

## Commands / results

- `cargo build --workspace --all-targets` — clean.
- `cargo test --workspace` — every crate `ok`, 0 failed (full native/OCCT
  Stage-1 suite included, unaffected). Per-crate deltas from `AICAD-057A`'s
  own last-known-good baseline: `cad-ast` 14 -> 15 (+1, round-trip);
  `cad-parser` 100 -> 108 (+8); `cad-hir` 109 -> 123 (+14: 6 in
  `lower.rs`, 8 in `typeck.rs`).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  clean.
- `cargo fmt --all -- --check` — clean (two files needed `cargo fmt --all`
  applied once during this task; verified clean afterward).

## Limitations / follow-up

- `Name<Args>` type-reference resolution for a user-defined generic
  struct/enum (e.g. using `Pair<Length, Mass>` as a variable's declared
  type) remains unresolved (`None`, no diagnostic) until `AICAD-057D`.
- Calling a generic function produces a type mismatch today (the
  unsubstituted `CheckedType::TypeParam` never structurally matches a
  concrete argument type) — expected and correct until `AICAD-057D` adds
  instantiation/inference.
- Enum variant payload types cannot reference an enum's own type
  parameters yet, since variant payloads themselves do not exist until
  `AICAD-057C` — `HirItem::Enum::type_params` is fully wired end-to-end
  (parsed, lowered, minted) but has no consumer inside the enum body yet;
  `AICAD-057C` is expected to route payload-type resolution through the
  same `Checker::active_type_params`/`with_type_params` mechanism this
  task built, not reinvent one.

## Status

`AICAD-057B` is complete. Next: `AICAD-057C` (general data-carrying enum
variants, constructors, destructuring patterns, and nominal-enum match
exhaustiveness).
