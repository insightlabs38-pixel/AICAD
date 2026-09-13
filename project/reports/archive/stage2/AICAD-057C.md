# AICAD-057C — Data-carrying enum variants, constructors, destructuring patterns, and match exhaustiveness

## Objective

Third step of the `D17`-mandated remediation sequence
(`project/OWNER_DECISIONS.md#D17`, `project/DECISION_LOG.md#DL-14`,
`project/reports/AICAD-057A.md`, `project/reports/AICAD-057B.md`): give
AICAD enums the three general variant shapes D17 specifies —

```
enum Example {
    Unit,
    Tuple(T1, T2),
    Record { x: T1, y: T2 },
}
```

— usable as constructor expressions, with corresponding tuple/record
destructuring patterns (normal lexical scope, variant-derived binding
types), and a genuine nominal-enum match-exhaustiveness check (D17: "the
compiler must diagnose non-exhaustive matches unless a wildcard or
otherwise exhaustive pattern is present"). Explicitly **not** this task's
scope, per the owner's own fixed task breakdown: call-site generic
instantiation/inference for a `Name<Args>` type *reference*
(`AICAD-057D`), and `Result<T,E>`/`Optional<T>` themselves (`AICAD-057E`).

## Base commit

`1888ed3` ("AICAD-057B: Implement generic parameter/type-application
syntax plus AST/HIR representation") — the tip of `origin/claude/
aicad-stage2-dev` at session start.

## Scope decisions (not escalated — applies the existing D17 ruling)

1. **Tuple-variant construction reuses ordinary call syntax.** `Ok(value)`
   already parses as `Expr::Call { callee: "Ok", args: [value] }` — no new
   AST node needed. Which callee names are ordinary functions, structs
   (`AICAD-053`), or (as of this task) enum variants is a name/type-
   resolution concern, not a grammar one, exactly like the existing
   struct-construction precedent.
2. **Record-variant construction gets its own new syntax, mirroring the
   pattern side exactly.** The owner's own D17 worked example uses
   identical brace shape both directions — `Record { x, y }` to
   destructure, and (by the same convention every Rust-like language
   uses) `Record { x: v, y: v }` to construct. Reusing `Point(x = 1, y =
   2)`-style call syntax for a *record* variant would create an
   unmotivated inconsistency with the pattern side this same task is
   introducing — so a new `Expr::RecordLiteral`/`Pattern::Record` pair was
   added, deliberately matching each other's shape.
3. **A standard "no record literal in condition position" parser
   restriction was needed, and is not new/speculative syntax.** Once a
   bare identifier can be followed by `{ field: expr }`, `if x { ... }`
   becomes genuinely ambiguous (is the trailing `{` the record literal or
   the `if`'s own block?). Every Rust-like language with this construction
   syntax carries the identical restriction (Rust's own "no struct
   literals in condition position"); `crates/cad-parser`'s `Parser::
   no_record_literal` implements the same technique — suppressed while
   parsing an `if`/`while` condition or a `match` scrutinee, reset again
   inside any nested unambiguous delimiter (`(...)`, `[...]`, or the
   record literal's own `{...}`).
4. **`VariantShape::Unit` stays distinct from `VariantShape::Tuple(vec![])`.**
   A zero-field tuple variant (`Empty()`) is a real constructor call with
   no arguments; a `Unit` variant is never call syntax at all, with or
   without arguments — matching Rust's own identical rule for unit-like
   enum variants (`None()` is not valid Rust either). Diagnosed as
   `UNIT_VARIANT_NOT_CALLABLE` (`TYPE-E448`) unconditionally when a `Unit`
   variant is reached through `Expr::Call`.
5. **No rest (`..`) pattern was added.** D17's own scope limit lists no
   such syntax; a record pattern must therefore name every field the
   variant declares — an uncovered field is `MISSING_VARIANT_FIELD`
   (`TYPE-E454`), the same code record-*construction* uses for a missing
   field, applied symmetrically to the pattern side.
6. **Payload-type resolution routes through `AICAD-057B`'s own
   `active_type_params`/`with_type_params` mechanism**, exactly as that
   task's own "Limitations / follow-up" section anticipated — a new
   `Checker::collect_enum_variant_shapes` pass mirrors `collect_struct_
   fields` verbatim, resolving each variant's payload types under
   `with_type_params(enum's own type_params, ...)` so a generic enum's
   `Tuple(T)`/`Record { x: T }` payload correctly resolves `T` to that
   enum's own declared parameter.

## Changes

- **`crates/cad-ast/src/item.rs`**: new `EnumVariant` enum (`Unit(Spanned<
  String>)` / `Tuple { name, fields: Vec<Type>, span }` / `Record { name,
  fields: Vec<Field>, span }`, plus `.name()`/`.span()` helpers);
  `Item::Enum::variants` changed from `Vec<Spanned<String>>` to
  `Vec<EnumVariant>`.
- **`crates/cad-ast/src/expr.rs`**: new `Expr::RecordLiteral { name,
  fields: Vec<(Spanned<String>, Expr)>, span }`; `Pattern` gained `Tuple {
  name, elems: Vec<Pattern>, span }` and `Record { name, fields:
  Vec<RecordPatternField>, span }`; new `RecordPatternField { name,
  pattern, span }` (shorthand `{ x, y }` desugars to `pattern:
  Pattern::Ident(x)` at parse time, so every later phase sees one uniform
  shape).
- **`crates/cad-ast/src/printer.rs`**: `print_enum_variant` (all three
  shapes); `print_pattern`'s `Tuple`/`Record` arms (record-pattern
  printing collapses back to shorthand when the field pattern is exactly
  `Ident(<same name>)`, matching what a human would actually write);
  `Expr::RecordLiteral` printing.
- **`crates/cad-parser/src/lib.rs`**: `parse_enum_variants` now recognizes
  all three shapes (`(...)`/`{...}`/bare name); new `parse_record_literal`;
  `parse_pattern`'s `Ident` branch extended with `parse_tuple_pattern`/
  `parse_record_pattern`; new `Parser::no_record_literal` field plus
  `with_no_record_literal` helper, applied around every `if`/`while`
  condition and `match`/`for` scrutinee/iterable, and reset inside
  `parse_call_args`/`parse_list_literal`/parenthesized grouping/the record
  literal's own body.
- **`specs/language/grammar.ebnf`**: new `enum_variant` production (the
  three shapes); the grammar's first-ever formal `pattern` production
  (previously only referenced, never defined) plus `tuple_pattern`/
  `record_pattern`/`record_pattern_field`; `record_literal`/
  `record_field_init` added to `or_expr`; header patch note per the file's
  own required process.
- **`crates/cad-compiler/src/binder.rs`**: `declare_item_name`'s `Item::
  Enum` arm uses `variant.name()`; `check_expr` gained an `Expr::
  RecordLiteral` arm (constructor name checked like any scope reference,
  field values checked); `bind_pattern` gained `Pattern::Tuple`/`Record`
  arms — `name` is *always* checked as an existing reference (never a
  fresh binding, unlike the genuinely ambiguous bare-`Ident` case), each
  sub-pattern bound recursively in the same arm scope.
- **`crates/cad-hir/src/hir.rs`**: `HirExpr::RecordLiteral { name, binding:
  Option<BindingId>, fields: Vec<HirRecordField>, span }`; `HirPattern`
  gained `Tuple`/`Record` (both carry `variant: Option<BindingId>` — `None`
  only when lowering could not resolve the name at all, unlike `HirPattern
  ::Variant`'s non-`Option` field, since `Name(...)`/`Name { ... }` syntax
  is never a legitimate fresh binding the way a bare `Ident` can be); new
  `HirRecordField`/`HirRecordPatternField`; `HirEnumVariant` gained
  `payload: HirVariantPayload` (`Unit`/`Tuple(Vec<HirTypeRef>)`/
  `Record(Vec<HirField>)`, purely syntactic — resolution is `cad_hir::
  typeck`'s job, mirroring `HirField::ty`'s own convention).
- **`crates/cad-hir/src/lower.rs`**: `declare_item`'s `Item::Enum` arm uses
  `variant.name()`; new free function `lower_variant_payload`; `lower_expr`
  gained an `Expr::RecordLiteral` arm (resolves the constructor name via
  the existing `resolve_or_diagnose`, exactly like `Expr::Call`'s callee);
  `lower_pattern` gained `Pattern::Tuple`/`Record` arms (same resolution,
  recursing into sub-patterns).
- **`crates/cad-hir/src/typeck.rs`**: new `VariantShape` enum (`Unit`/
  `Tuple(Vec<Option<CheckedType>>)`/`Record(Vec<FieldInfo>)`); `Checker`
  gained `variant_shapes: HashMap<BindingId, VariantShape>` and
  `enum_variants: HashMap<BindingId, Vec<BindingId>>` (full variant-id list
  per enum, for exhaustiveness); new `collect_enum_variant_shapes` pass
  (mirrors `collect_struct_fields`, routed through `with_type_params`);
  `check_call`'s `HirCallee::Fn` match gained a `BindingKind::EnumVariant`
  arm dispatching to new `check_variant_tuple_construction`/
  `check_variant_positional_args`; new `check_record_literal`; `check_expr`
  gained an `HirExpr::RecordLiteral` arm; `check_match`/`check_match_expr`
  now thread a `span` through (needed for the exhaustiveness diagnostic)
  and call new `check_match_exhaustiveness` after processing every arm;
  `bind_pattern` gained `HirPattern::Tuple`/`Record` arms, plus a shared
  `check_pattern_enum_match` helper factored out of the pre-existing
  `HirPattern::Variant` arm (identical enum-identity check, now reused by
  three pattern shapes instead of one).
- **`crates/cad-runtime/src/value.rs`**: `Value::EnumVariant(BindingId)`
  changed to `Value::EnumVariant { variant: BindingId, payload:
  VariantPayload }`; new `VariantPayload` enum (`Unit`/`Tuple(Vec<Value>)`/
  `Record(Vec<(String, Value)>)`).
- **`crates/cad-runtime/src/interp.rs`**: `HirExpr::Ident`'s enum-variant
  fast path now builds `payload: VariantPayload::Unit`; `call()` gained a
  `BindingKind::EnumVariant` arm (evaluates args in source order, builds
  `VariantPayload::Tuple`) — already type-checked (arity/shape) by
  `cad_hir::typeck`, so this crate does not re-verify shape, matching its
  own established "no scope-stack re-derivation" convention; `eval_expr`
  gained an `HirExpr::RecordLiteral` arm; `pattern_matches` gained `HirPattern
  ::Tuple`/`Record` arms (structural match plus recursive sub-pattern
  binding); `values_equal` extended to recurse into `EnumVariant` payloads
  (new `variant_payloads_equal` helper) and into `List` elements;
  `eval_comparison`'s `EnumVariant`/`EnumVariant` arm now delegates to
  `values_equal` for genuine structural equality (`Ok(1) == Ok(2)` is no
  longer trivially true merely because both are `Ok`) instead of the old
  tag-only `BindingId` comparison.

## New diagnostics (provisional `TYPE-Exxx`, pending `D10` — same convention as every prior batch)

| Code | Name | Where |
|---|---|---|
| `TYPE-E446` | `NON_EXHAUSTIVE_MATCH` | `check_match_exhaustiveness` |
| `TYPE-E447` | `VARIANT_ARITY_MISMATCH` | tuple-variant construction/pattern arity |
| `TYPE-E448` | `UNIT_VARIANT_NOT_CALLABLE` | a `Unit` variant reached via `Expr::Call` |
| `TYPE-E449` | `VARIANT_FIELD_TYPE_MISMATCH` | tuple/record field value type mismatch |
| `TYPE-E450` | `TUPLE_VARIANT_NAMED_ARGUMENT` | a named arg supplied to a tuple variant (no field names exist) |
| `TYPE-E451` | `RECORD_VARIANT_NEEDS_BRACES` | a record variant constructed with `(...)` |
| `TYPE-E452` | `UNKNOWN_VARIANT_FIELD` | unknown field name, construction or pattern |
| `TYPE-E453` | `DUPLICATE_VARIANT_FIELD` | duplicate field name, construction or pattern |
| `TYPE-E454` | `MISSING_VARIANT_FIELD` | uncovered field, construction or pattern (no rest pattern exists) |
| `TYPE-E455` | `RECORD_LITERAL_NOT_RECORD_VARIANT` | `Name { ... }` where `Name` is not a record-shaped variant |
| `TYPE-E456` | `PATTERN_SHAPE_MISMATCH` | a tuple/record pattern used against a variant of a different declared shape |

## Required tests

(from D17's own required-test list, the AICAD-057C-owned slice — generics/
`Result`/`Optional`-specific items belong to `AICAD-057D`/`E`/`F`, not
here)

- unit enum variant — `parses_unit_tuple_and_record_enum_variants_in_one_
  enum` (parser), pre-existing `parses_enum_decl_matching_paper_example`
  unaffected.
- tuple enum variant — same parser test; `tuple_variant_construction_
  with_correct_types_checks_cleanly` (typeck); `tuple_variant_construction_
  and_destructuring_round_trips_the_payload` (runtime).
- record enum variant — same parser test; `record_variant_construction_
  with_correct_fields_checks_cleanly` (typeck); `record_variant_
  construction_and_shorthand_destructuring_round_trips_the_payload`
  (runtime).
- payload construction — `parses_record_literal_construction_expression`/
  `parses_tuple_variant_constructor_call_expression` (parser); the typeck/
  runtime tests above.
- tuple destructuring — `parses_tuple_pattern_destructuring` (parser);
  `tuple_pattern_binds_its_elements` (binder); `tuple_pattern_
  destructuring_binding_has_correct_field_type` (typeck);
  `nested_tuple_variant_destructuring_reaches_the_inner_payload` (runtime).
- record destructuring — `parses_record_pattern_shorthand_and_explicit_
  fields` (parser); `record_pattern_shorthand_binds_its_fields` (binder);
  `record_pattern_shorthand_binding_has_correct_field_type` (typeck);
  `record_pattern_explicit_rename_binds_the_renamed_name` (runtime).
- payload binding has correct type — the tuple/record typeck tests above
  directly assert the bound `HirPattern::Binding`'s `checked.binding_types`
  entry equals the declared field type.
- non-exhaustive enum match -> diagnostic — `non_exhaustive_match_over_
  tuple_and_record_variants_is_reported`, `non_exhaustive_match_over_
  mixed_unit_tuple_record_variants_names_the_missing_ones` (typeck);
  contrasted with `exhaustive_match_covering_every_variant_has_no_
  diagnostic`/`non_exhaustive_match_with_wildcard_has_no_diagnostic`.
- evidence the machinery is general, not tied to any specific enum name —
  every test above uses enum names other than `Result`/`Optional`/`List`/
  `Range` (`R`, `Shape`, `Box`, `Message`, ...); `generic_enum_tuple_
  variant_destructures_cleanly` specifically exercises a generic (`Box<T>`)
  variant payload.

## Adversarial / negative tests

- unclosed tuple-variant paren, missing colon in a record-variant field,
  unclosed tuple-pattern paren — each a parser diagnostic, not a hang/panic
  (`unclosed_tuple_variant_paren_is_reported`, `record_variant_field_
  missing_colon_is_reported`, `unclosed_tuple_pattern_paren_is_reported`).
- tuple/record construction: wrong arity (too few/too many), field-type
  mismatch, calling a `Unit` variant with parens, constructing a record
  variant with parens instead of braces, missing/unknown/duplicate record
  field — each its own typeck test (see the diagnostics table above).
- tuple pattern against a record-shaped variant (`PATTERN_SHAPE_MISMATCH`)
  — proves the checker does not silently accept the wrong pattern shape
  for a variant's actual declared kind.
- a pattern naming an undefined variant (`tuple_pattern_against_undefined_
  variant_name_is_reported`) and a record-literal constructor naming an
  undefined name (`record_literal_constructor_name_is_checked_as_a_scope_
  reference`) — both ordinary `UNDEFINED_NAME`, not a crash.
- `if`/`while`/`match` condition-position record-literal ambiguity: proven
  resolved correctly both directions — `if cond { a; }` still parses `cond`
  as a bare identifier and `{ a; }` as the block
  (`if_condition_does_not_misparse_a_record_literal_as_its_own_block`,
  parser; `record_literal_does_not_misparse_a_following_if_block`,
  round-trip), while a record literal remains constructible in condition
  position once parenthesized or inside a nested call's own arguments
  (same round-trip test).
- structural (not merely nominal) equality: `Ok(1) == Ok(1)` true,
  `Ok(a) == Ok(b)` false for `a != b`, a record variant's fields compare
  equal regardless of construction order (`tuple_variants_with_equal_
  payloads_compare_equal`, `tuple_variants_with_different_payloads_
  compare_unequal`, `record_variants_with_equal_fields_compare_equal_
  regardless_of_construction_order`) — this is a real correctness fix this
  task's own payload addition made necessary (the pre-existing tag-only
  equality would otherwise have silently gone wrong for the very payloads
  being introduced).
- pre-existing regression test `match_variant_pattern_against_mismatched_
  scrutinee_enum_is_reported` (two enums sharing a variant *name*) now
  correctly reports *two* diagnostics instead of one
  (`TYPE-E434`+`TYPE-E446`) — a genuine, previously-undetected soundness
  gap `AICAD-057A`'s own audit flagged (finding #8: "today's `match`
  performs no coverage check ... at all"), now caught; the test was updated
  to assert both, with a comment explaining why this is correct, not a
  regression.

## Commands / results

- `cargo build --workspace --all-targets` — clean.
- `cargo test --workspace` — every crate `ok`, 682 tests passed, 0 failed
  (full native/OCCT Stage-1 suite included, unaffected). Per-crate deltas
  from `AICAD-057B`'s own last-known-good baseline: `cad-ast` (printer
  round-trip file) 15 -> 19 (+4); `cad-parser` 108 -> 119 (+11);
  `cad-compiler` 43 -> 49 (+6); `cad-hir` 123 -> 146 (+23: 24 new typeck
  tests, 1 existing test updated in place); `cad-runtime` 61 -> 69 (+8).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  clean (one `while_let_loop` lint fixed during development, in the
  tuple-variant field-list parser).
- `cargo fmt --all -- --check` — clean (`cargo fmt --all` applied once
  during this task; verified clean afterward).

## Known limitations

- `Name<Args>` type-reference resolution for a user-defined generic
  struct/enum still does not exist (`AICAD-057D`'s job, unchanged from
  `AICAD-057B`'s own documented limitation) — a parameter/`let` typed
  `Box<Int>` still resolves to `None`, so this task's own generic-payload
  test (`generic_enum_tuple_variant_destructures_cleanly`) deliberately
  avoids returning the payload value itself (which would hit the still-
  unresolved `CheckedType::TypeParam`-vs-concrete-type comparison
  `AICAD-057D` is meant to fix) and only exercises the destructuring shape
  this task actually owns.
- No `..` rest pattern exists (D17's own scope limit) — a record pattern
  must always name every declared field.
- `Value::EnumVariant`'s runtime construction path (`Interpreter::call`'s
  new `BindingKind::EnumVariant` arm) does not itself re-verify shape/arity
  — it trusts `cad_hir::typeck` already did, matching this crate's
  existing, already-documented convention (e.g. `check_call_args`'s own
  arity checking is never re-derived at the runtime layer either). A
  hand-built `HirExpr` that bypasses type-checking (as some existing tests
  deliberately do to exercise the evaluator alone) can still reach a
  `Unit`-shaped variant with a non-empty argument list; this constructs a
  `VariantPayload::Tuple` rather than panicking or silently coercing to
  `Unit`, matching this crate's "never crash on an unexpected shape"
  discipline.
- `Value::EnumVariant`'s new structural equality does not compare `List`
  payload elements against `Range` elements or vice versa (falls through
  to `_ => false`, same as before this task) — no required test needs
  that, and inventing more general structural equality across every
  `Value` shape combination is outside this task's own scope.

## Status

`AICAD-057C` is complete. Next: `AICAD-057D` (generic instantiation/
inference/type checking for the approved Stage-2 generic subset). Do not
begin `AICAD-057D` until this report and `project/TASKS.yaml`'s status
update are both committed.
