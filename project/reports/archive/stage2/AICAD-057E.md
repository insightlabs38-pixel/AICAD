# AICAD-057E — Result<T,E>/Optional<T> via the ordinary prelude

## Objective

Fifth step of the `D17`-mandated remediation sequence
(`project/OWNER_DECISIONS.md#D17`, `project/DECISION_LOG.md#DL-14`,
`project/reports/AICAD-057A.md`, `AICAD-057B.md`, `AICAD-057C.md`,
`AICAD-057D.md`): define `Result<T, E>` (`Ok(T)`/`Err(E)`) and
`Optional<T>` (`Some(T)`/`None`) as **ordinary prelude generic enums**,
built from exactly the enum/generic declaration machinery `AICAD-057B`/
`C`/`D` already gave every user program, with "no `Result`-specific
compiler/runtime semantics beyond ordinary prelude registration/loading"
(`D17`'s own wording), and prove both type-check and *execute* correctly,
including nested generics (`Optional<Result<Int, E>>`) and explicit `Err`
propagation through nested function calls via ordinary `match` (no `?`
operator). Explicitly **not** this task's scope, per the owner's own fixed
task breakdown: `AICAD-057F`'s own adversarial generality pass (though
this task's own tests already include one non-Result/Optional generic
enum as direct evidence — see "Adversarial / negative tests" below); any
extension to `check_generic_call`'s or `check_struct_construction`'s own
inference algorithm beyond what was strictly needed to make the prelude
enums usable (one narrow, symmetric gap was found and closed — see "Scope
decisions" #2 below, not an extension of either of those two functions).

## Base commit

`ba7706a` ("AICAD-057D: Implement generic instantiation/inference/type
checking for the approved Stage-2 generic subset") — confirmed via
`git fetch origin claude/aicad-stage2-dev && git reset --hard
origin/claude/aicad-stage2-dev` and `git log -3 --oneline` before any work
began; working tree was clean beforehand (`git status --porcelain` empty).

## Scope decisions (not escalated — applies the existing D17 ruling)

1. **The prelude is literal, parsed AICAD source text — not hand-built
   AST/HIR.** New `crates/cad-hir/src/prelude.rs`'s `PRELUDE_SOURCE`
   constant is exactly:
   ```
   enum Result<T, E> {
       Ok(T),
       Err(E),
   }

   enum Optional<T> {
       Some(T),
       None,
   }
   ```
   `with_prelude(user_program: &Program) -> Program` parses this text with
   the ordinary `cad_parser::parse_program` every `.aicad` file goes
   through and prepends its two `enum` items to the caller's own already-
   parsed program. This is the literal reading of D17's "ordinary prelude
   registration/loading": constructing `Item::Enum`/`HirItem::Enum` values
   directly in Rust would still use the same node *shapes*, but would
   produce a `Result`/`Optional` declaration no user could ever type
   themselves — exactly the kind of `Result`-specific machinery D17 rules
   out. Sourcing it as text closes that gap completely.
2. **One narrow, general (not `Result`/`Optional`-specific) gap in
   `AICAD-057D`'s own instantiation support had to be closed for this
   task's own required `Optional<Length>` test to be writable at all** —
   see "The `HirExpr::Ident` fix" below for the full account. This is not
   an extension of `check_generic_call`'s or `check_struct_construction`'s
   own algorithms (neither function was touched); it is a third,
   previously-unaddressed code path (`check_expr`'s `HirExpr::Ident` arm)
   that those two functions' own `AICAD-057D` substitution logic never
   reached. It was necessary, not "beyond what's needed" (the task's own
   explicit bar), confirmed by writing the failing case first (see below)
   before touching any code.
3. **Prelude injection is opt-in per call site, never automatic inside
   `lower_program`/`check_program` themselves.** Those two phase functions
   are unchanged. `with_prelude` is one extra step a caller inserts
   between `cad_parser::parse_program` and `lower_program`. This was
   necessary, not merely convenient: several pre-existing tests across
   `cad-compiler`/`cad-hir`/`cad-runtime` already declare their own
   unrelated, non-generic `Ok`/`Err`/`Result`/`Some`/`None`-named test
   fixtures (e.g. `crates/cad-compiler/src/binder.rs`'s `enum Result {
   Ok(Int), Err(Int) }`, `cad-hir/src/typeck.rs`'s `enum R { Ok(Int),
   Err(Int) }`, `cad-runtime/src/interp.rs`'s `enum R { Ok(Length),
   Err(Length) }`/`enum Opt { Some(Length), None }`) — automatically
   injecting the prelude into every existing test's own pipeline would
   have silently changed their meaning (a same-named, differently-shaped
   declaration colliding with the prelude's own). Making the prelude
   opt-in keeps every one of those pre-existing tests completely
   untouched while still proving the prelude works end to end via new,
   dedicated tests (`check_with_prelude`/`compiled_with_prelude` helpers,
   used only by the new AICAD-057E test sections).
4. **No hook existed in `cad_compiler`'s module loader (`AICAD-044`,
   `crates/cad-compiler/src/loader.rs`) to reuse, and none was needed.**
   Direct inspection: `loader.rs`'s `LoadResult` keeps each file-based
   module's own separately-parsed `Program` distinct (never merges
   multiple files' items into one `Program`); `cad_compiler::binder`
   binds one already-parsed `Program` at a time and is not part of the
   actual execution pipeline at all — `AICAD-057D`'s own report already
   found "`cad-ast`/`cad-parser`/`cad-compiler`/`cad-runtime` needed no
   changes" for generics, and this task confirms why: the real, currently-
   exercised full pipeline (used by every `cad-hir`/`cad-runtime` full-
   program test) is `cad_parser::parse_program` -> `cad_hir::lower_program`
   -> `cad_hir::typeck::check_program` -> `cad_runtime::Interpreter`,
   entirely bypassing `cad_compiler::binder`/`loader`. `cad-hir` has no
   single "compile a whole program" production entry point of its own
   either — `lower_program`/`check_program` are separate phase functions
   every caller wires together itself. Given that, the smallest faithful
   hook is a reusable `cad_hir::prelude::with_prelude` function inserted
   at the one real seam every such caller already has (between parsing
   and lowering), not a new abstraction layered on top of the two
   existing phase functions.
5. **`cad-hir` gained a new, real (non-dev) dependency on `cad-parser`.**
   `with_prelude` must parse `PRELUDE_SOURCE`, and `cad-hir` previously
   only had `cad-parser` as a *dev*-dependency (used by its own test
   helpers). Promoting it to an ordinary dependency creates no cycle
   (`cad-parser` depends only on `cad-ast`/`cad-lexer`/`cad-diagnostics`,
   never on `cad-hir`) and is the natural direction (a lowering crate
   depending on the parser that produces its input), unlike a hypothetical
   `cad-hir -> cad-compiler` dependency, which `crate::lower`'s own
   pre-existing module doc comment already documents as a genuine
   workspace cycle to avoid.
6. **`cad-cli` needs no wiring — it does not exist yet as a real driver.**
   Direct inspection: `crates/cad-cli/src/lib.rs` is still the
   `AICAD-002`/`003` scaffolding placeholder ("No implementation yet"),
   and its `Cargo.toml` has zero dependencies (not even on `cad-parser`/
   `cad-hir`). `AICAD-061` (the eventual `cad build` command) is `status:
   todo`, unscheduled before this task. There is nothing to wire the
   prelude into yet; `AICAD-061`'s own future implementation must call
   `cad_hir::prelude::with_prelude` as part of compiling a real user
   program — documented here so that task does not have to rediscover it.

## The `HirExpr::Ident` fix (why it was needed, and why it is general)

Writing this task's own required `Optional<Length>` (`Some`/`None`) test
first surfaced a genuine, previously-undetected gap: `register_type_names`
gives **every** enum variant — `Unit`, `Tuple`, or `Record` alike — the
bare, non-instantiated `CheckedType::Enum(<owning enum's binding>)` as its
type, unconditionally, at declaration time (this predates `AICAD-057D`,
from `AICAD-057C`). `AICAD-057D`'s own "substitute from the call's own
expected/contextual type" logic was added only to
`check_variant_tuple_construction` (`Ok(value)`) and `check_record_literal`
(`Boxed { value }`) — both reached only when a variant is actually
*called*. A `Unit` variant (`None`) is never called, so it never flows
through either function, and `HirExpr::Ident`'s own type lookup (`binding
.and_then(|b| self.binding_types[b.index()].clone())`) never consulted the
expected/contextual type at all. Concretely, before this fix:

```
enum Optional<T> { Some(T), None }
fn f() -> Optional<Int> { return None; }
```

reported `TYPE-E419 RETURN_TYPE_MISMATCH` ("expected type Optional<Int>,
found Optional") — confirmed by hand-building this exact case in a
throwaway harness before writing any fix, per `AGENTS.md`'s evidence rule.
This is **not** `Optional`-specific: it would misfire identically for any
generic enum's own `Unit` variant (`enum Maybe<T> { Found(T), Empty }`,
`return Empty;` against `-> Maybe<Length>`). Since this task's own required
test list explicitly needs `Optional<Length>`'s `None` to type-check and
execute, and the gap blocks it for *every* generic enum equally (not one
this task invented a special case for), this falls within — not beyond —
"what's needed to make the prelude enums usable exactly as any other
user-defined generic enum already is," the explicit bar the task text sets
for touching `AICAD-057D`'s own territory. The fix, in `check_expr`'s
`HirExpr::Ident` arm (`crates/cad-hir/src/typeck.rs`): when a variant's own
registered type is the bare `CheckedType::Enum(base)` and the call's own
`expected` type is a `CheckedType::Instantiated { base: eb, .. }` naming
the *same* enum (`base == eb`), adopt `expected` instead of the bare
`Enum`. It never fabricates or infers anything — it only adopts an
already-known, already-verified expected type, exactly the same "trust the
annotation" rule `AICAD-057D` already applies to tuple/record variant
construction. An unrelated or wrong expected type is completely unaffected
and still reported exactly as before (`optional_none_against_
instantiated_expected_type_is_not_a_wildcard` proves this). Generality
evidence: `a_non_prelude_generic_enum_unit_variant_also_resolves_against_
an_instantiated_expected_type` (`typeck.rs`) exercises the identical fix
via `enum Maybe<T> { Found(T), Empty }`, a name with no relationship to
`Result`/`Optional`/`List`/`Range`.

## Changes by file

- **`crates/cad-hir/src/prelude.rs`** (new): `PRELUDE_SOURCE` (the fixed
  AICAD source text above) and `with_prelude(user_program: &Program) ->
  Program` (parses the prelude, prepends its items to the caller's own
  program). Module doc comment documents the design rationale and exactly
  where this hooks into the pipeline (see "Scope decisions" #4). Four of
  its own tests (see "Required tests" below) prove the prelude alone
  parses/lowers/type-checks cleanly, that `with_prelude` prepends the
  right items in the right order, and that a combined program using
  `Result`/`Ok`/`Err` with zero declarations of its own lowers and type-
  checks cleanly end to end.
- **`crates/cad-hir/Cargo.toml`**: `cad-parser` moved from
  `[dev-dependencies]` to `[dependencies]` (needed by `prelude.rs`'s
  production code, not just tests).
- **`crates/cad-hir/src/lib.rs`**: `pub mod prelude;` plus a doc-comment
  bullet describing it (matching the existing per-module bullet-list
  style); `pub use prelude::{PRELUDE_SOURCE, with_prelude};`.
- **`crates/cad-hir/src/typeck.rs`**:
  - The `HirExpr::Ident` fix described above (the only production-code
    behavior change in this task — see its own inline doc comment at the
    call site for the full rationale, mirrored from this report).
  - New `check_with_prelude(source)` test helper (parallel to the
    existing `check(source)`, but calls `crate::prelude::with_prelude`
    before lowering) — used only by the new AICAD-057E test section.
  - New "AICAD-057E: Result<T,E>/Optional<T> via the ordinary prelude"
    test section (10 tests) — see "Required tests"/"Adversarial" below.
- **`crates/cad-runtime/src/interp.rs`**:
  - New `compiled_with_prelude(source)` test helper (parallel to the
    existing `compiled(source)`) — used only by the new AICAD-057E test
    section.
  - New "AICAD-057E: Result<T,E>/Optional<T> via the ordinary prelude"
    test section (6 tests) — see "Required tests"/"Adversarial" below.
  - **No production code in this crate changed at all** — confirmed by
    `git diff crates/cad-runtime/src/interp.rs` touching only the
    `#[cfg(test)] mod tests` block. Every `Value::EnumVariant`/
    `VariantPayload` this task's own runtime tests produce is the exact
    same general shape `AICAD-057C` already built; `Interpreter::call`'s
    `BindingKind::EnumVariant` arm needed no change to construct `Ok`/
    `Err`/`Some`/`None` values, since it already treats any enum variant
    identically regardless of name.

## New diagnostics

None. No new diagnostic code was added or needed — the prelude's `Result`/
`Optional` reuse every diagnostic `AICAD-057B`/`C`/`D` already built
(`TYPE-E446` `NON_EXHAUSTIVE_MATCH`, `TYPE-E449`
`VARIANT_FIELD_TYPE_MISMATCH`, `TYPE-E419` `RETURN_TYPE_MISMATCH`, etc.),
confirmed by this task's own adversarial tests below.

## Required tests (from D17's remediation list, the AICAD-057E-owned slice)

All in `crates/cad-hir/src/typeck.rs`'s "AICAD-057E" section (compile-time,
167 -> 177, +10) and `crates/cad-runtime/src/interp.rs`'s "AICAD-057E"
section (runtime, 69 -> 75, +6), plus 4 new tests in the new
`crates/cad-hir/src/prelude.rs` module itself (167 -> 181 total for
`cad-hir` across both additions):

- `Result<Int, String>` construct/match — `result_int_string_construct_
  and_match_type_checks_cleanly` (typeck), `result_int_string_construct_
  and_match_executes_correctly` (runtime, both `Ok`/`Err` call paths
  asserted).
- `Result<Length, SomeErrorType>` (a user-defined error type, not a
  primitive) — `result_length_with_user_defined_error_type_type_checks_
  cleanly` (typeck, uses a **struct** `SensorFault { code: Int }`),
  `result_length_with_user_defined_error_type_executes_correctly`
  (runtime, uses a user-defined **enum** `SensorFault` instead — struct
  *construction* is not yet implemented by this crate's own interpreter
  at all, an unrelated, already-documented pre-existing gap; see "Known
  limitations"). The task's own required-test wording explicitly allows
  either ("a user-defined error enum/struct, not a primitive").
- `Optional<Length>` `Some`/`None` — `optional_length_some_and_none_type_
  checks_cleanly` (typeck), `optional_length_some_and_none_execute_
  correctly` (runtime, both paths asserted with real `Length` magnitudes).
- explicit successful `Result` match, value flows correctly —
  `successful_result_match_flows_the_ok_value_correctly` (typeck and,
  separately, runtime — the runtime version asserts the exact numeric
  magnitude that reaches the caller).
- explicit `Err` propagation through nested function calls —
  `err_propagation_through_nested_function_calls_type_checks_cleanly`
  (typeck) and `err_propagates_through_nested_function_calls_to_the_top`
  (runtime): a helper (`inner`) returns `Result<Length, String>`; the
  caller (`outer`) matches it and does `Err(e) => { return Err(e); }` —
  ordinary `match`, no `?` operator anywhere; the runtime test asserts the
  *exact* propagated `Err` payload string (`"boom"`) survives two full
  function-call/match hops unchanged, plus the complementary successful-
  path assertion in the same test.
- nested generic types, `Optional<Result<Int, E>>` — `nested_optional_of_
  result_type_checks_cleanly` (typeck) and `nested_optional_of_result_
  constructs_and_matches_correctly` (runtime, a single `match` destructures
  both levels at once: `Some(Ok(v))`/`Some(Err(e))`/`None`, all three call
  combinations asserted).
- generality evidence (this task's own, not `AICAD-057F`'s broader pass) —
  `a_non_prelude_generic_enum_unit_variant_also_resolves_against_an_
  instantiated_expected_type` (typeck): a user-defined generic enum named
  `Maybe` (not `Result`/`Optional`/`List`/`Range`) exercises the exact
  `HirExpr::Ident` fix this task added, proving it is not `Optional`-
  specific.
- prelude-module tests (`crates/cad-hir/src/prelude.rs`):
  `prelude_source_parses_lowers_and_type_checks_cleanly_alone` (the
  prelude's own two declarations, with no user code at all, are clean at
  every phase); `with_prelude_prepends_the_two_prelude_enums_before_user_
  items`; `with_prelude_leaves_an_empty_user_program_with_just_the_
  prelude`; `combined_program_with_ordinary_user_code_lowers_and_type_
  checks_cleanly` (a user function using `Result`/`Ok`/`Err` with no
  declaration of its own lowers and type-checks cleanly end to end).

## Adversarial / negative tests

- `result_err_payload_wrong_type_against_user_defined_error_type_is_
  reported` — `Err(1)` against a declared `SensorFault` error type is a
  genuine `TYPE-E449 VARIANT_FIELD_TYPE_MISMATCH`, not silently accepted
  just because it is the `Err` arm of a `Result`.
- `optional_none_against_instantiated_expected_type_is_not_a_wildcard` —
  the `HirExpr::Ident` fix's own substitution is genuine, not a universal
  wildcard: a `Some`-bound `Length` value returned where `Mass` is wrongly
  declared is a real `TYPE-E419` mismatch.
- `non_exhaustive_match_over_result_is_reported` — omitting the `Err` arm
  over the prelude's own `Result<T, E>` is `TYPE-E446
  NON_EXHAUSTIVE_MATCH`, proving the prelude enum gets exactly the same
  exhaustiveness check as any user-defined enum, not an exemption.
- `a_non_prelude_generic_enum_unit_variant_also_resolves_against_an_
  instantiated_expected_type` — see "generality evidence" above.
- **Grep evidence the machinery contains no `Result`/`Optional`-specific
  compiler logic** (this task's own required check, per its own
  instructions): `git diff` across every changed/added file, restricted to
  non-comment, non-`#[cfg(test)]` lines, for the string literals `"Ok"`/
  `"Err"`/`"Some"`/`"None"`/`"Result"`/`"Optional"` — zero matches. The
  only non-comment, non-test lines anywhere in the diff that contain the
  bare words `Some`/`None` at all are Rust's own `Option::Some`/pattern
  matches in the `HirExpr::Ident` fix (`Some(CheckedType::Enum(base))`,
  `Some(CheckedType::Instantiated { .. })`) — ordinary `Option<T>` pattern
  matching, unrelated to AICAD's own `Optional<T>` prelude type. Every
  other occurrence of these five names in the diff is either inside a doc
  comment/inline comment explaining the design, inside `PRELUDE_SOURCE`
  itself (unquoted AICAD source identifiers, not Rust string literals —
  `enum Result<T, E> { Ok(T), Err(E), }`), or inside a `#[cfg(test)] mod
  tests` block (AICAD source strings exercising the prelude). Commands run:
  `git diff -- crates/cad-hir/src/typeck.rs crates/cad-runtime/src/
  interp.rs crates/cad-hir/src/lib.rs | grep -n "^+" | grep -iE '"Ok"|
  "Err"|"Some"|"None"|"Result"|"Optional"'` (empty); a second, broader pass
  without the quote requirement, manually reviewed line by line (see
  above).

## Commands / results

- `git fetch origin claude/aicad-stage2-dev && git reset --hard origin/
  claude/aicad-stage2-dev` — confirmed `ba7706a` as tip, working tree
  clean beforehand.
- `cargo fmt --all -- --check` — clean (`cargo fmt --all` applied once
  during this task; verified clean afterward).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — clean (one `clippy::doc_nested_refdefs` lint on the new `lib.rs`
  bullet fixed by adding an empty link-reference marker, `[`prelude`][]:`,
  matching clippy's own suggested fix).
- `cargo build --workspace --all-targets` — clean.
- `cargo test --workspace` — every crate `ok`, 0 failed. Per-crate deltas
  from `AICAD-057D`'s own last-known-good baseline (re-verified fresh at
  session start via `git log`/a clean reset, not trusted second-hand):
  `cad-ast` 19 -> 19 (unchanged); `cad-parser` 119 -> 119 (unchanged);
  `cad-compiler` 49 -> 49 (unchanged); `cad-hir` 167 -> 181 (+14: 4 new
  `prelude.rs` module tests + 10 new `typeck.rs` "AICAD-057E" tests);
  `cad-runtime` 69 -> 75 (+6, all new "AICAD-057E" tests). Every other
  crate in the workspace (`cad-diagnostics`, `cad-lexer`, `cad-types`,
  `cad-units`, `cad-kernel-api`, `cad-occt-bridge`, and every still-
  unimplemented placeholder crate) unaffected, confirmed by the full
  `cargo test --workspace` run's own per-crate `test result: ok` lines.

## Known limitations

- **Struct construction is not yet implemented by `cad-runtime`'s own
  interpreter** (`crates/cad-runtime/src/interp.rs`'s pre-existing
  `struct_construction_is_not_yet_supported` test, confirmed still
  present and passing, unrelated to and out of scope for this task) — the
  runtime test for a non-primitive `E` therefore uses a user-defined enum
  (`SensorFault`) rather than the struct the typeck test uses for the same
  scenario. Both are explicitly authorized by this task's own required-
  test wording ("a user-defined error enum/struct, not a primitive").
  Fixing struct-construction execution is a separate, larger, unscoped
  task, not part of `D17`'s `AICAD-057B`-`F` remediation sequence.
- **A user program that itself redeclares one of the five prelude names
  (`Result`/`Optional`/`Ok`/`Err`/`Some`/`None`) at the top level is not
  specially rejected or protected by `with_prelude`** — this is not a new
  gap this task introduced; it is `crate::lower`'s own already-documented,
  pre-existing "no duplicate-declaration diagnostic in this pass" design
  (`crate::binder`'s `DUPLICATE_BINDING` already owns that check in the
  parts of the pipeline that use it), simply inherited unchanged by
  merging the prelude's items into the same flat declare-then-lower pass.
  A later declaration of the same name in the *lowerer's* own internal
  scope map silently overwrites an earlier one (no diagnostic from
  `cad-hir`); adding a special case for this one specific condition would
  itself be exactly the kind of `Result`-specific machinery `D17`
  prohibits, so none was added. If a future task wires `cad_compiler::
  binder` into the same real execution pipeline (it currently is not —
  see "Scope decisions" #4), that binder would already catch such a
  collision as an ordinary `DUPLICATE_BINDING`, with no prelude-specific
  code needed there either.
- **`with_prelude`'s combined program is checked/lowered against the
  caller-supplied `file`/`source` string, not a synthetic combined one**
  — a diagnostic that happened to be raised against one of the prelude's
  own two (fixed, tested, expected-clean) declarations would report a
  clamped, cosmetically-wrong line/column position (`cad_ast::LineIndex::
  line_column`'s own documented offset-clamping, which never panics).
  Since the prelude source is fixed and covered by its own
  `prelude_source_parses_lowers_and_type_checks_cleanly_alone` test
  (zero diagnostics, always), this is a purely cosmetic, extremely narrow
  edge case (only reachable via the redeclaration scenario above, which
  itself produces no diagnostic in the current pipeline) — not a
  correctness or soundness issue, and not something a real span-remapping
  mechanism was built for here, since no required test needs it and doing
  so would be unrelated scope expansion.
- **`cad_compiler`'s own binder/loader were not touched or extended to
  know about the prelude** — see "Scope decisions" #4: neither is part of
  the actual execution pipeline today, and no required test needed them.
  `cad-cli`'s future real driver (`AICAD-061`, not yet started) must call
  `cad_hir::prelude::with_prelude` directly, as documented in `prelude.rs`'s
  own module doc comment.
- No `?` operator or other new propagation syntax was added or considered
  — unchanged from `D17`'s own scope limit; every propagation test in this
  report uses ordinary `match`.

## Status

`AICAD-057E` is complete. Next: `AICAD-057F` (adversarial integration pass
proving the generic/data-carrying-enum machinery is general, not
`Result`-specific — this task's own tests already contribute one piece of
that evidence, `a_non_prelude_generic_enum_unit_variant_also_resolves_
against_an_instantiated_expected_type`, but `AICAD-057F` owns the full,
dedicated adversarial pass per `D17`'s own required-test list). Do not
begin `AICAD-057F`, the original `AICAD-057`, or `AICAD-058` before this
report and `project/TASKS.yaml`'s status update are both committed.
