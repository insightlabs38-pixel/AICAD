# AICAD-057F — Adversarial integration pass proving generic/enum machinery is general, not `Result`-specific

## Objective

Sixth and last step of the `D17`-mandated remediation sequence
(`project/OWNER_DECISIONS.md#D17`, `project/DECISION_LOG.md#DL-14`,
`project/reports/AICAD-057A.md` through `AICAD-057E.md`). Unlike
`AICAD-057B`-`E`, this task builds no new compiler machinery. It is an
**audit + adversarial-gap-filling** task: (1) build a coverage matrix
mapping every item on `D17`'s own "REQUIRED REMEDIATION TESTS (minimum)"
list to the specific test(s) that actually cover it today, re-verified
against the live test suite rather than trusted from prior reports'
prose; (2) close any genuine gap found, using only the existing
`057B`-`E` machinery; (3) add a dedicated, comprehensive "generality
proof" scenario built around one user-defined generic enum whose name is
not `Result`/`Optional`/`List`/`Range`; (4) re-confirm, with fresh grep
evidence across the full `057B..057F` diff range, that no `?`/propagation
syntax and no `Result`/`Optional`-specific compiler-intrinsic logic exist
anywhere.

## Base commit

`fd84357` ("AICAD-057E: Define and execute Result<T,E>/Optional<T> via
the ordinary generic-enum prelude") — confirmed via `git fetch origin
claude/aicad-stage2-dev && git reset --hard origin/claude/aicad-stage2-dev`
and `git log -3 --oneline` before any work began; working tree was clean
beforehand (`git status --porcelain` empty, checked before the reset per
this task's own instructions).

## Method

For every claim in `project/reports/AICAD-057B.md` through
`AICAD-057E.md`'s own "Required tests"/"Adversarial" sections, the named
test function was located directly in the current source
(`grep -n "fn <name>"` against `crates/cad-ast`, `crates/cad-parser`,
`crates/cad-compiler/src/binder.rs`, `crates/cad-hir/src/lower.rs`,
`crates/cad-hir/src/typeck.rs`, `crates/cad-hir/src/prelude.rs`,
`crates/cad-runtime/src/interp.rs`) and, where its exact assertion mattered
for the matrix, read in full — not merely assumed present because a prior
report said so. Every test named in this report's matrix was confirmed to
exist, at its claimed location, asserting what the claiming report says,
with **zero renames/removals found** across the whole `057B`-`E` span.

## Coverage matrix (D17's required-test list -> covering tests)

| D17 required-test item | Crate | Test(s) | What it asserts |
|---|---|---|---|
| generic struct, one type parameter | `cad-parser` | `parses_generic_struct_with_one_type_parameter` | declaration-side `<T>` parses |
| | `cad-hir` (lower) | `generic_struct_type_params_mint_bindings_with_type_param_kind` | mints a `BindingKind::TypeParam` binding |
| | `cad-hir` (typeck) | `generic_struct_with_one_type_parameter_type_checks_cleanly`, `generic_struct_with_one_type_parameter_referenced_as_a_type_resolves`, `generic_struct_field_access_substitutes_the_declared_type_parameter_not_a_wildcard` | declares cleanly; `Box<Length>` reference resolves; field access substitutes genuinely (adversarial) |
| | `cad-hir` (typeck, this task) | `generic_struct_construction_against_instantiated_expected_type_checks_cleanly`, `generic_struct_construction_wrong_expected_type_is_reported` | **gap closed** — construction itself (not just field access) against an instantiated expected type; adversarial wrong-type case |
| generic struct, two type parameters | `cad-parser` | `parses_generic_struct_with_two_type_parameters` | `<T, U>` parses |
| | `cad-hir` (typeck) | `generic_struct_with_two_type_parameters_type_checks_cleanly`, `generic_struct_with_two_type_parameters_referenced_as_a_type_resolves_both_fields`, `generic_struct_two_type_parameters_second_field_substitutes_its_own_parameter_not_the_first` | both fields resolve/substitute independently |
| generic enum | `cad-parser` | `parses_generic_enum` | `enum Optional<T>`-shaped declaration parses |
| | `cad-hir` (lower/typeck) | `generic_enum_type_params_are_lowered`, `generic_enum_type_checks_cleanly` | binding minted; declares cleanly |
| generic function | `cad-parser` | `parses_generic_function` | `fn identity<T>(...)` parses |
| | `cad-hir` (lower/typeck) | `generic_fn_type_params_are_lowered`, `generic_function_with_matching_param_and_return_type_parameter_type_checks_cleanly`, `mismatched_generic_function_type_parameters_are_reported` | binding minted; body type-checks; adversarial mismatch caught |
| successful inferred generic call | `cad-hir` (typeck) | `generic_function_call_infers_type_parameter_from_argument`, `generic_function_call_infers_type_parameter_from_expected_return_context`, `generic_function_call_argument_inference_is_not_a_wildcard` | `identity(5mm)` infers `T=Length`; return-context inference; adversarial non-wildcard proof |
| ambiguous generic call -> diagnostic | `cad-hir` (typeck) | `ambiguous_generic_call_with_type_parameter_only_in_return_type_is_reported` | `TYPE-E459 AMBIGUOUS_GENERIC_CALL` |
| wrong number of type arguments -> diagnostic | `cad-hir` (typeck) | `generic_struct_type_reference_with_too_few_type_arguments_is_reported` (`TYPE-E457`), `generic_struct_type_reference_with_too_many_type_arguments_is_reported` (`TYPE-E458`) | both directions diagnosed |
| unit enum variant | `cad-parser` | `parses_unit_tuple_and_record_enum_variants_in_one_enum` | all three shapes in one decl, incl. unit |
| tuple enum variant | `cad-parser` / `cad-hir` (typeck) / `cad-runtime` | same parser test; `tuple_variant_construction_with_correct_types_checks_cleanly`; `tuple_variant_construction_and_destructuring_round_trips_the_payload` | parses, type-checks, executes |
| record enum variant | same three crates | same parser test; `record_variant_construction_with_correct_fields_checks_cleanly`; `record_variant_construction_and_shorthand_destructuring_round_trips_the_payload` | parses, type-checks, executes |
| payload construction | `cad-parser` | `parses_record_literal_construction_expression`, `parses_tuple_variant_constructor_call_expression` | both construction syntaxes parse |
| tuple destructuring | `cad-parser` / `cad-compiler` / `cad-hir` (typeck) / `cad-runtime` | `parses_tuple_pattern_destructuring`; `tuple_pattern_binds_its_elements`; `tuple_pattern_destructuring_binding_has_correct_field_type`; `nested_tuple_variant_destructuring_reaches_the_inner_payload` | parses, binds, types, executes (incl. nesting) |
| record destructuring | same four crates | `parses_record_pattern_shorthand_and_explicit_fields`; `record_pattern_shorthand_binds_its_fields`; `record_pattern_shorthand_binding_has_correct_field_type`; `record_pattern_explicit_rename_binds_the_renamed_name` | parses, binds, types, executes (shorthand + rename) |
| payload binding has correct type | `cad-hir` (typeck) | `tuple_pattern_destructuring_binding_has_correct_field_type`, `record_pattern_shorthand_binding_has_correct_field_type`, and (generic case) `generic_enum_tuple_variant_pattern_against_instantiated_type_yields_substituted_field_type`, `generic_enum_record_variant_pattern_against_instantiated_type_yields_substituted_field_type` | bound variable's `checked.binding_types` entry equals the declared (substituted, for the generic case) field type |
| non-exhaustive enum match -> diagnostic | `cad-hir` (typeck) | `non_exhaustive_match_over_tuple_and_record_variants_is_reported`, `non_exhaustive_match_over_mixed_unit_tuple_record_variants_names_the_missing_ones`, contrasted with `exhaustive_match_covering_every_variant_has_no_diagnostic`/`non_exhaustive_match_with_wildcard_has_no_diagnostic` | `TYPE-E446 NON_EXHAUSTIVE_MATCH`; wildcard/full coverage correctly silent |
| `Result<Int,String>` | `cad-hir` (typeck) / `cad-runtime` | `result_int_string_construct_and_match_type_checks_cleanly`; `result_int_string_construct_and_match_executes_correctly` | both `Ok`/`Err` paths type-check and execute |
| `Result<Length,SomeErrorType>` | `cad-hir` (typeck) / `cad-runtime` | `result_length_with_user_defined_error_type_type_checks_cleanly` (struct error type); `result_length_with_user_defined_error_type_executes_correctly` (enum error type — struct construction is a pre-existing, unrelated runtime gap, see Known limitations); `result_err_payload_wrong_type_against_user_defined_error_type_is_reported` (adversarial) | user-defined (non-primitive) error type works both phases |
| `Optional<Length>` | `cad-hir` (typeck) / `cad-runtime` | `optional_length_some_and_none_type_checks_cleanly`; `optional_length_some_and_none_execute_correctly`; `optional_none_against_instantiated_expected_type_is_not_a_wildcard` (adversarial) | `Some`/`None` both phases, substitution genuine |
| explicit successful `Result` match | `cad-hir` (typeck) / `cad-runtime` | `successful_result_match_flows_the_ok_value_correctly` (both crates, same name) | `Ok` arm's bound value reaches the caller unchanged |
| explicit `Err` propagation through nested function calls | `cad-hir` (typeck) / `cad-runtime` | `err_propagation_through_nested_function_calls_type_checks_cleanly`; `err_propagates_through_nested_function_calls_to_the_top` | ordinary `match`/`return Err(e)`, no `?`; exact payload survives two call hops |
| nested generic types, `Optional<Result<Int,E>>` | `cad-hir` (typeck) / `cad-runtime` | `nested_optional_of_result_type_checks_cleanly`; `nested_optional_of_result_constructs_and_matches_correctly` | single nested `match` (`Some(Ok(v))`/`Some(Err(e))`/`None`), all three combinations executed |
| user-defined generic enum not `Result`/`Optional`/`List`/`Range` (generality) | `cad-hir` (typeck) / `cad-runtime`, **this task's dedicated scenario** | see "Generality-proof scenario" below | see below — the task's own namesake requirement |
| No `?` syntax | — | grep evidence, see below | confirmed absent |
| No general compiler-intrinsic mechanism | — | grep evidence, see below | confirmed absent |

**Verdict: every item on D17's required-test list is covered.** One
genuine, narrow coverage gap was found (generic **struct construction**
itself — as opposed to field access on an already-typed value — had no
dedicated test, even though `AICAD-057D`'s own `check_struct_construction`
already implements it) and was closed with two new tests; both passed on
first run with **zero production-code changes**, confirming this was a
test-suite gap, not an implementation bug. No other gap was found in the
`057B`-`E`-owned slice of the list; every other claim in those four
reports' own "Required tests"/"Adversarial" sections was re-verified
directly against current source and found accurate (no renamed/removed
test broke any prior claim).

## Generality-proof scenario (this task's own namesake requirement)

A single, independent, comprehensive scenario built around one new
generic enum, **`Either<L, R>`**, never used by any prior `057B`-`E` test
fixture (`Result`, `Optional`, `Holder<T>`, `Wrap<T>`, `Maybe<T>`, `Box<T>`
were each already used by an earlier task's own tests — this task
deliberately did not reuse any of them):

```
enum Either<L, R> { Left(L), Right { value: R } }
```

This declares **two** type parameters (`L`, `R` — `Result`/`Optional`
between them only ever get exercised with one enum having two params,
`Result`, and this task duplicates that shape independently), a
**tuple**-shaped variant (`Left(L)`) and a **record**-shaped variant
(`Right { value: R }`) in the same declaration. Six new tests, added to
`crates/cad-hir/src/typeck.rs`'s and `crates/cad-runtime/src/interp.rs`'s
own new "AICAD-057F" sections:

- `either_two_type_parameter_generic_enum_tuple_and_record_construction_and_destructuring_type_check_cleanly`
  (typeck) — constructs `Left(x)` and `Right { value: s }`, destructures
  both via `match`, both fully substituted against `Either<Length,
  String>`.
- `either_generic_enum_payload_bindings_are_not_wildcards` (typeck,
  adversarial) — a `Left`-bound value returned where the *other* type
  parameter's type is declared is a genuine `TYPE-E419` mismatch, not
  silently accepted.
- `either_nested_inside_itself_two_levels_type_checks_and_destructures_cleanly`
  (typeck) — `Either<Either<Int, Bool>, String>`, this enum nested inside
  itself (the exact nesting shape D17's own text suggests), one branch
  nesting through the **tuple** variant (`Left(Left(1))`) and the other
  through the **record** variant (`Left(Right { value: true })`), both
  destructured by one nested `match`.
- `either_non_exhaustive_match_missing_right_arm_is_reported` (typeck) —
  omitting the `Right` arm is `TYPE-E446 NON_EXHAUSTIVE_MATCH`.
- `either_tuple_and_record_variant_construction_and_destructuring_execute_correctly`
  (runtime) — the same `Either<Length, String>` program actually
  **executed**: `make_left`/`make_right`/`unwrap_left_or_zero`/
  `unwrap_right_or_empty` produce the correct `Length` magnitude and
  `String` value respectively; a `Right` value passed to
  `unwrap_left_or_zero` correctly falls through to its `Right` arm (proves
  runtime dispatch is on the actual constructed variant, not a fixed
  branch).
- `either_nested_inside_itself_executes_correctly` (runtime) — the nested
  `Either<Either<Int, Bool>, String>` program executed end to end, both
  branches (`inner_left = true` -> `1`, `inner_left = false` -> `0`)
  producing the correct distinct final `Int` value.

This exercises, in one coherent group: a two-type-parameter generic enum
declaration; both variant shapes; construction of each; destructuring of
each; a nested case (enum nested inside itself, via both variant shapes);
a non-exhaustive-match diagnostic; and full runtime execution producing a
correct final value — at least as rich as what `Result`/`Optional`
themselves receive across `057C`-`E` combined, using zero new production
code (every one of these six tests passed on its first run against the
existing `057B`-`E` machinery, confirming no `Either`-shaped case was
secretly unsupported).

## `?`/propagation-syntax and compiler-intrinsic absence (grep evidence)

**No `?` operator or other propagation syntax anywhere:**

```
grep -n "Question\|'\?'\|\"?\"\|propagat" specs/language/grammar.ebnf
grep -n "Question\|QuestionMark" crates/cad-lexer/src/*.rs
grep -n "Question\|postfix.*\?|try_expr\|TryExpr" crates/cad-parser/src/lib.rs
```

Zero matches for any token, grammar production, or parser handling of `?`
as an operator, anywhere in the lexer, parser, or grammar spec (the only
`grammar.ebnf` hits are unrelated prose using the English word
"propagat-" inside header patch-note comments describing this exact
remediation sequence's own history).

**No `Result`/`Optional`-specific compiler-intrinsic logic across the full
`057B..057F` range.** Base commit for the diff range confirmed via
`git log`: `1888ed3` is `AICAD-057B`'s own commit (the first commit of the
remediation sequence that touches production code), so the range is
`1888ed3~1..HEAD` (`HEAD` includes this task's own additions):

```
git diff 1888ed3~1..HEAD -- crates/ specs/ \
  | grep -n '^+' | grep -v '^+++' \
  | grep -E '"Ok"|"Err"|"Some"|"None"|"Result"|"Optional"'
```

Result: exactly **two** matches, both `assert_eq!(callee.node, "Ok")` /
`assert_eq!(name.node, "Ok")` inside `crates/cad-parser/src/lib.rs`'s own
`#[cfg(test)] mod tests` block (`057B`'s
`parses_tuple_variant_constructor_call_expression`/
`parses_tuple_pattern_destructuring` tests, asserting the parsed AST of an
*ordinary* `Ok(...)` call/pattern — proving the grammar treats `Ok` as an
unprivileged identifier, not evidence of privileged handling). No other
non-comment, non-test line in the entire six-commit range contains any of
these six string literals. This task's own new tests add **zero** new
matches to this grep (confirmed by re-running it after adding the
`Either`/struct-construction tests — the count is unchanged at 2,
identical to the count `AICAD-057E`'s own report already recorded for the
narrower `057B..057E` range).

## Known limitations (carried forward, none newly introduced)

- **Struct construction is not yet implemented by `cad-runtime`'s own
  interpreter** (`struct_construction_is_not_yet_supported`, confirmed
  still present and passing) — pre-existing, unrelated to `D17`'s generic/
  enum scope (it predates `AICAD-057B` entirely, from `AICAD-053`), and
  not on `D17`'s required-test list. This is why `Result<Length,
  SomeErrorType>`'s runtime test uses a user-defined *enum* error type
  while its typeck test uses a *struct* one — both are explicitly
  authorized by `D17`'s own "enum/struct" wording and by `057E`'s report.
  Not fixed here — fixing it is a separate, larger, unscoped task with no
  connection to the generic/enum remediation this task audits.
- **Generic struct/enum construction still does not infer type arguments
  from its own constructor arguments alone** (only from an already-known
  expected/contextual type) — `AICAD-057D`'s own documented limitation,
  unchanged; no required test needs constructor-argument-driven inference,
  and this task's own new struct-construction tests use exactly the
  expected-type-context path the implementation actually supports.
- **`List<T>`/`Range<T>` remain on their own dedicated, non-generic
  `CheckedType` variants**, not migrated onto `CheckedType::Instantiated`
  — `D17`'s "List/Range cleanup" explicitly authorizes but does not
  require this before `AICAD-057F`; this task did not perform that
  migration (out of scope — this task audits and closes required-test
  gaps, it does not perform optional internal refactors `D17` left
  discretionary).
- No `..` rest pattern, no interface/trait bounds, no turbofish/explicit
  type-argument call syntax — unchanged from every prior task's own scope
  limit; nothing in this task touched any of them.

## Escalations filed

**None.** Every item on `D17`'s required-test list was found genuinely
satisfiable — and, after this task's own two new tests, actually
satisfied — using only the existing `057B`-`E` machinery. The one gap
found (generic struct construction's own missing test) closed with zero
production-code changes, confirming it was a documentation/test-coverage
gap, not an implementation shortfall requiring escalation-list treatment.

## Changes by file

- `crates/cad-hir/src/typeck.rs`: new "AICAD-057F: adversarial generality
  proof" test section (4 tests: the `Either<L, R>` construction/
  destructuring, adversarial non-wildcard, nested-inside-itself, and
  non-exhaustive-match cases) plus 2 new tests closing the generic-
  struct-construction coverage gap (`generic_struct_construction_
  against_instantiated_expected_type_checks_cleanly`,
  `generic_struct_construction_wrong_expected_type_is_reported`). No
  production code changed — confirmed by `git diff --stat` touching only
  this file and `interp.rs`, both entirely inside their own `#[cfg(test)]
  mod tests` blocks.
- `crates/cad-runtime/src/interp.rs`: new "AICAD-057F: adversarial
  generality proof, runtime execution" test section (2 tests: the same
  `Either<L, R>` scenario's tuple/record construction+destructuring and
  its nested-inside-itself case, both actually executed via
  `Interpreter::call_by_name`). No production code changed.

## Commands / results

- `git fetch origin claude/aicad-stage2-dev && git reset --hard origin/
  claude/aicad-stage2-dev` — confirmed `fd84357` as tip, working tree
  clean beforehand (`git status --porcelain` empty, checked first).
- `cargo fmt --all -- --check` — clean, no changes needed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — clean.
- `cargo build --workspace --all-targets` — clean.
- `cargo test --workspace` — every crate `ok`, 0 failed. Per-crate deltas
  from `AICAD-057E`'s own last-known-good baseline (re-verified fresh at
  session start via a clean `git reset`, not trusted second-hand):
  `cad-ast` 19 -> 19 (unchanged, incl. 19 printer round-trip tests);
  `cad-parser` 119 -> 119 (unchanged); `cad-compiler` 49 -> 49 (unchanged);
  `cad-hir` 181 -> 187 (+6: 4 `Either` generality tests + 2 generic-
  struct-construction gap-closing tests, all in `typeck.rs`);
  `cad-runtime` 75 -> 77 (+2, both new `Either` runtime-execution tests).
  Every other crate in the workspace unaffected (full per-crate `test
  result: ok` breakdown captured in this session's own terminal output;
  `cad-occt-bridge` 84, `cad-units` 75, `cad-lexer` 29, `cad-diagnostics`
  20 + 10 schema-conformance, `cad-types` 14, `cad-kernel-api` 23,
  `cad-ast` printer round-trip 19 + 7 lib tests, all unchanged from
  before this task).

## Final verdict

**The AICAD-057B..057F remediation sequence required by D17/DL-14 is
complete; original AICAD-057 may now resume.** Every item on `D17`'s own
required-test list is covered by a specific, re-verified, currently
passing test; the one genuine coverage gap found (generic struct
construction) is closed with two new tests and zero production-code
changes; the dedicated adversarial generality scenario (`Either<L, R>`,
two type parameters, both variant shapes, nested-inside-itself, non-
exhaustive diagnostic, full runtime execution) is at least as rich as
`Result`/`Optional`'s own coverage and passed on first run against the
unmodified `057B`-`E` implementation; fresh grep evidence across the full
`1888ed3~1..HEAD` (`057B`-`F`) diff range confirms zero `?`/propagation
syntax and zero `Result`/`Optional`-specific compiler-intrinsic logic
anywhere. No escalations were required.

## Status

`AICAD-057F` is complete. Next: the original `AICAD-057` ("Implement
recursion and Result/error propagation") — see
`project/SESSION_HANDOFF.md` for the precise, narrowed scope that task now
has (recursion is already implemented per `project/reports/AICAD-057.md`;
`Result<T,E>`/`Optional<T>` and ordinary `match`-based propagation are now
available and proven end-to-end via `project/reports/AICAD-057E.md`'s and
this report's own tests).
