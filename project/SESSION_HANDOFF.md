# Session Handoff

## Latest: `AICAD-057D` (generic instantiation/inference/type checking for the approved Stage-2 generic subset) COMPLETE. Resume at `AICAD-057E` next — do not skip ahead.

This session started from `91ff121` ("AICAD-057C: Implement data-carrying
enum variants, constructors, destructuring patterns, and match
exhaustiveness"), the tip of `origin/claude/aicad-stage2-dev` at session
start. `AICAD-057A`'s audit and the owner's `D17` ruling
(`project/DECISION_LOG.md#DL-14`) authorize a fixed remediation sequence —
`AICAD-057B` through `AICAD-057F` — before the original `AICAD-057`
("recursion and Result/error propagation") may resume. This session
completed the fourth of those, `AICAD-057D`.

### What this session did

Closed the four gaps `AICAD-057B`'s and `AICAD-057C`'s own "Known
limitations" sections explicitly deferred to this task: call-site
type-parameter instantiation/inference for generic functions
(`identity(5mm)` infers `T = Length`); `Name<Args>` type-reference
resolution for a user-defined generic struct/enum (`Pair<Length, Mass>`,
previously `None` with no diagnostic), with substitution propagated into
field access, struct-literal/variant construction, and variant patterns;
a diagnostic (never an arbitrary selection) for an ambiguous generic
call; and a diagnostic for a wrong number of type arguments on a
`Name<Args>` reference. Full details, exact scope boundary, and complete
test list in `project/reports/AICAD-057D.md`. Summary:

- `cad-hir` (typeck) was the **only** crate touched — `cad-ast`/
  `cad-parser`/`cad-compiler`/`cad-runtime` needed no changes at all
  (`CheckedType` is private to this one module and never reaches
  `cad-runtime`, confirming generics stay fully compile-time-only per
  `D17`).
- New `CheckedType::Instantiated { base: BindingId, args:
  Vec<CheckedType> }` — a genuine instantiation of a user-defined generic
  struct/enum, nominal equality (same `base`, pairwise-compatible `args`).
  `CheckedType`/`ParamSig` lost `derive(Copy)` as a mechanical consequence
  (the new variant carries a `Vec`); every call site that relied on an
  implicit copy now clones explicitly — no existing diagnostic's
  condition or wording changed (confirmed by the full, unchanged-assertion
  146/146 pre-existing `cad-hir` test pass).
- New free functions `substitute_type`/`substitute_opt` (recursive
  `TypeParam` substitution) and `unify_type_param` (structural
  unification against a raw, possibly-generic declared type, extending a
  binding map, failing on a structural mismatch or an inconsistent
  re-binding).
- New `Checker::type_params_of`/`Checker::instantiation_subst`; new
  `Checker::resolve_generic_type_application` (arity-checks a `Name<Args>`
  reference against a known struct/enum, resolves each argument, returns
  `CheckedType::Instantiated`) — the previously-unconditional
  `HirTypeRef::Generic => None` catch-all in `resolve_type_ref` now calls
  it (`List`/`Range` keep their own dedicated arms, checked first,
  unmigrated — see the report's "Known limitations" for why).
- `FnSignature` gained `type_params: Vec<BindingId>`; new
  `Checker::check_generic_call` — a three-pass call-site inference
  algorithm (match args to slots; check each argument's own type and
  unify it against the raw declared parameter/return type, including the
  call's own contextual/expected type; diagnose an unresolved parameter
  as ambiguous, otherwise re-compare every argument against its
  substituted type and return the substituted return type).
- `check_call`/`check_expr`'s `Call`/`RecordLiteral` arms now thread the
  call's own contextual/expected type through to
  `check_struct_construction`/`check_variant_tuple_construction`/
  `check_record_literal`/`check_generic_call` — this is what lets a
  generic struct-literal/variant construction substitute correctly when
  an enclosing `let`/`return`/parameter annotation already names the
  right instantiation (not from the constructor's own arguments alone —
  a documented, deliberately narrower scope than generic-function
  inference; see the report).
- `check_field_access`, `check_match_exhaustiveness`,
  `check_pattern_enum_match`, and `bind_pattern`'s `Tuple`/`Record` arms
  all now handle a scrutinee/receiver typed as `CheckedType::Instantiated`
  (substituting field/payload types, or extracting `base` for
  enum-identity/exhaustiveness purposes, exactly like the corresponding
  plain `Struct`/`Enum` case already did).
- 3 new provisional `TYPE-Exxx` diagnostic codes (`457`-`459`:
  `TOO_FEW_TYPE_ARGUMENTS`, `TOO_MANY_TYPE_ARGUMENTS`,
  `AMBIGUOUS_GENERIC_CALL`); an inconsistent multi-occurrence
  type-parameter binding (`fn pair_of<T>(a: T, b: T)` called with a
  `Length` then a `Mass`) deliberately reuses the existing `TYPE-E418
  ARGUMENT_TYPE_MISMATCH` rather than a new code (same observable
  condition once substitution is applied).

Test deltas (all passing, 0 regressions): `cad-ast` 19 -> 19; `cad-parser`
119 -> 119; `cad-compiler` 49 -> 49; `cad-hir` 146 -> 167 (+21, all new);
`cad-runtime` 69 -> 69 (unchanged, confirming no runtime-visible change
was needed). Full `cargo test --workspace` all crates `ok`, 0 failed;
`cargo clippy --workspace --all-targets --all-features -- -D warnings`
clean; `cargo fmt --all -- --check` clean; `cargo build --workspace
--all-targets` clean.

## Current state / next action

- **Active stage**: Stage 2. Stage 1 closed (DL-11); D5 policy-shape closed
  (DL-12); D16 closed (DL-13); D17 closed (DL-14).
- **Current/next batch**: S2-09, extended by the `D17`-mandated remediation
  sequence. `AICAD-056` **COMPLETE**. `AICAD-057A` **COMPLETE**.
  `AICAD-057B` **COMPLETE**. `AICAD-057C` **COMPLETE**. `AICAD-057D`
  **COMPLETE** (this session, `status: done`). `AICAD-057E`/`AICAD-057F`:
  **not started**, `status: todo`, linear `depends_on` chain in
  `project/TASKS.yaml`. The original `AICAD-057` remains blocked on
  `AICAD-057F` (task-graph `depends_on`, not just prose).
- **THE NEXT INVOCATION MUST START `AICAD-057E` NEXT, IN ORDER** —
  "Define and execute `Result<T,E>` and `Optional<T>` using the ordinary
  generic enum machinery": these become ordinary prelude enums built from
  the now-complete generic/data-carrying-enum machinery
  (`AICAD-057B`/`C`/`D`) — `enum Result<T, E> { Ok(T), Err(E) }`, `enum
  Optional<T> { Some(T), None }` — with **no** `Result`-specific compiler
  semantics beyond ordinary prelude registration/loading (`D17`'s own
  explicit requirement). Read `project/reports/AICAD-057D.md` first,
  especially "Known limitations" (generic struct/enum *construction*
  without a matching `expected` context does not infer type arguments
  from its own arguments alone — `Result`/`Optional`'s own prelude
  registration and this task's required tests should be designed with
  that boundary in mind, e.g. `let r: Result<Int, String> = Ok(1);` works
  today via the `expected`-driven substitution path this session built;
  a bare `let r = Ok(1);` with no annotation does not yet infer `E`). No
  `?` operator or other new propagation syntax is authorized — propagate
  with ordinary `match`.
- **Exact recent state**: this session's own fresh runs (most recent
  first): `cargo test --workspace` all crates `ok`, 0 failed (per-crate
  breakdown above); `cargo build --workspace --all-targets` clean; `cargo
  clippy --workspace --all-targets --all-features -- -D warnings` clean;
  `cargo fmt --all -- --check` clean.
- **No open regressions.**
- **Unresolved owner decisions**: unchanged from prior sessions — `D17`
  resolved (`DL-14`). Open/partial: D3, D5 (concrete tolerance constants
  only), D10, D11, D12, D15. This session added 3 new provisional
  `TYPE-Exxx` codes (`457`-`459`, `cad-hir`'s `typeck.rs`), still
  provisional pending D10, same convention as every prior batch.
- **D5 status/evidence**: unchanged. This task's own determinism-relevant
  finding: `Checker::type_params_of`/`instantiation_subst` are populated/
  computed via plain, deterministic source-order iteration and structural
  zipping — no new ordering dependence introduced.
- **Pre-existing `TASKS.yaml` staleness** (unchanged, not this batch's
  scope): `AICAD-001` through `AICAD-037` still show `status: todo` despite
  being long complete.
- **Recommended next action**: start `AICAD-057E` — read `project/
  reports/AICAD-057D.md` first (its own "Known limitations" section
  documents exactly which generic-construction-inference gap remains and
  why deferring it was correct for this task), plus `project/
  OWNER_DECISIONS.md#D17`/`project/DECISION_LOG.md#DL-14` for the exact
  authorized `Result`/`Optional` scope (ordinary prelude enums, no
  `Result`-specific compiler semantics, no `?` operator). Do not begin
  `AICAD-057F`, the original `AICAD-057`, or `AICAD-058` before it.

## Environment

Unchanged from prior sessions (reconfirmed at session start): Rust 1.98.1
(auto-installed via `rustup`, matching `rust-toolchain.toml` — the
toolchain does not persist across sessions in this container), edition
2024. This session added **zero** new third-party dependencies. Only
`cad-hir` (plus the usual `project/` bookkeeping files) was touched — no
`specs/language/grammar.ebnf` change was needed (no new syntax; `Name<Args>`
already parsed at type-reference positions). No native/OCCT work was
touched.

## Git identity

This container's global git config is `Claude <noreply@anthropic.com>`
(with `user.useConfigOnly=true` and `core.hooksPath` pointed at the repo's
identity-enforcing hooks) — matching every prior session's own finding, and
**not modified** by this session (per `CLAUDE.md`/`AGENTS.md`: "NEVER
update the git config"). This session's commit sets `GIT_AUTHOR_NAME`/
`GIT_AUTHOR_EMAIL`/`GIT_COMMITTER_NAME`/`GIT_COMMITTER_EMAIL` to
`insightlabs38-pixel`/`insightlabs38@gmail.com` as process-local environment
variables for the `git commit` invocation only — the repo's
`prepare-commit-msg` hook passed without needing any config change. No
hook was bypassed or modified; `core.hooksPath` was left untouched;
`--no-verify` was never used.
