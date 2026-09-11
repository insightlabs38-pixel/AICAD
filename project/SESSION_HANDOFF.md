# Session Handoff

## Latest: `AICAD-057E` (`Result<T,E>`/`Optional<T>` via the ordinary prelude) COMPLETE. Resume at `AICAD-057F` next — do not skip ahead.

This session started from `ba7706a` ("AICAD-057D: Implement generic
instantiation/inference/type checking for the approved Stage-2 generic
subset"), the tip of `origin/claude/aicad-stage2-dev` at session start
(re-confirmed via `git fetch`/`git reset --hard`/`git log -3 --oneline`;
working tree was clean beforehand). `AICAD-057A`'s audit and the owner's
`D17` ruling (`project/DECISION_LOG.md#DL-14`) authorize a fixed
remediation sequence — `AICAD-057B` through `AICAD-057F` — before the
original `AICAD-057` ("recursion and Result/error propagation") may
resume. This session completed the fifth of those, `AICAD-057E`.

### What this session did

Defined `Result<T, E>` (`Ok(T)`/`Err(E)`) and `Optional<T>` (`Some(T)`/
`None`) as ordinary prelude generic enums, built from exactly the
enum/generic machinery `AICAD-057B`/`C`/`D` already gave every user
program, with no `Result`-specific compiler/runtime semantics beyond
ordinary prelude registration/loading — and proved both type-check and
*execute* correctly, including nested generics
(`Optional<Result<Int,E>>`) and explicit `Err` propagation through nested
function calls via ordinary `match` (no `?` operator). Full details, exact
scope boundary, and complete test list in
`project/reports/AICAD-057E.md`. Summary:

- New `crates/cad-hir/src/prelude.rs`: `PRELUDE_SOURCE` is literal,
  parsed AICAD source text (`enum Result<T, E> { Ok(T), Err(E), } enum
  Optional<T> { Some(T), None, }`) — not a hand-built AST/HIR tree, so
  `Result`/`Optional` are declarations a user could have typed themselves,
  never a compiler-only shape. `with_prelude(user_program: &Program) ->
  Program` parses this text via the ordinary `cad_parser::parse_program`
  and prepends its two `enum` items to the caller's own already-parsed
  program — the one hook every real full-pipeline caller inserts between
  parsing and lowering. It is opt-in per call site (never automatic inside
  `lower_program`/`check_program`), since several pre-existing tests
  across `cad-compiler`/`cad-hir`/`cad-runtime` already declare their own
  unrelated, non-generic `Ok`/`Err`/`Result`/`Some`/`None`-named test
  fixtures that must stay completely unaffected.
- `cad-hir` gained a new, real (non-dev) dependency on `cad-parser`
  (previously dev-only) — no cycle (`cad-parser` never depends on
  `cad-hir`).
- **No hook existed in `cad_compiler`'s module loader/binder to reuse, and
  none was needed**: direct inspection confirmed neither is part of the
  actual execution pipeline today (that pipeline is `cad_parser::
  parse_program` -> `cad_hir::lower_program` -> `cad_hir::typeck::
  check_program` -> `cad_runtime::Interpreter`, entirely bypassing
  `cad_compiler`). `cad-cli` (`AICAD-061`, not yet started) needs no
  wiring either — it is still the zero-dependency `AICAD-002`/`003`
  scaffolding placeholder; its own future implementation must call
  `cad_hir::prelude::with_prelude` directly, documented in `prelude.rs`'s
  own module doc comment for that task to find.
- **One narrow, general (not `Optional`-specific) gap in `AICAD-057D`'s
  own instantiation support had to be closed**, discovered by writing this
  task's own required `Optional<Length>` test first: `register_type_names`
  gives *every* enum variant (including `Unit`) the bare, non-instantiated
  `CheckedType::Enum` unconditionally, and `AICAD-057D`'s own "substitute
  from the call's own expected type" logic was added only to tuple-/
  record-variant *construction* (`check_variant_tuple_construction`/
  `check_record_literal`) — a `Unit` variant (`None`) is never called, so
  it never reached either function. Confirmed by hand-building the failing
  case first (`return None;` against a declared `-> Optional<Int>` gave
  `TYPE-E419`, "expected type Optional<Int>, found Optional") before
  writing any fix, per `AGENTS.md`'s evidence rule. The fix is in
  `check_expr`'s `HirExpr::Ident` arm (`crates/cad-hir/src/typeck.rs`):
  when a variant's registered type is the bare `CheckedType::Enum(base)`
  and the call's own `expected` type is a `CheckedType::Instantiated {
  base: eb, .. }` naming the *same* enum, adopt `expected` instead — never
  fabricating or inferring anything, only adopting an already-known,
  already-verified expected type (the same "trust the annotation" rule
  `AICAD-057D` already applies elsewhere). General, not `Optional`-
  specific: proved via a dedicated adversarial test using a user-defined
  generic enum named `Maybe` (not `Result`/`Optional`/`List`/`Range`).
  Neither `check_generic_call` nor `check_struct_construction` themselves
  were touched — this is a distinct, previously-unaddressed code path,
  necessary (not "beyond what's needed") for the prelude's own required
  test.
- No new diagnostic codes were added — the prelude's `Result`/`Optional`
  reuse every diagnostic `AICAD-057B`/`C`/`D` already built.
- No production code in `cad-runtime` changed at all — every
  `Value::EnumVariant`/`VariantPayload` this task's runtime tests produce
  is the exact same general shape `AICAD-057C` already built.
- **Grep evidence required by this task's own instructions**: `git diff`
  across every changed/added file, restricted to non-comment, non-test
  lines, for the string literals `"Ok"`/`"Err"`/`"Some"`/`"None"`/
  `"Result"`/`"Optional"` — zero matches. The only non-comment, non-test
  lines containing the bare words `Some`/`None` at all are Rust's own
  `Option::Some` pattern matches in the `HirExpr::Ident` fix, unrelated to
  AICAD's own prelude type. Full detail in the report.

Test deltas (all passing, 0 regressions): `cad-ast` 19 -> 19; `cad-parser`
119 -> 119; `cad-compiler` 49 -> 49; `cad-hir` 167 -> 181 (+14: 4 new
`prelude.rs` module tests, 10 new `typeck.rs` "AICAD-057E" tests);
`cad-runtime` 69 -> 75 (+6, all new "AICAD-057E" tests). Full `cargo test
--workspace` all crates `ok`, 0 failed; `cargo clippy --workspace
--all-targets --all-features -- -D warnings` clean (one
`clippy::doc_nested_refdefs` lint on the new `lib.rs` doc bullet fixed);
`cargo fmt --all -- --check` clean; `cargo build --workspace --all-targets`
clean.

## Current state / next action

- **Active stage**: Stage 2. Stage 1 closed (DL-11); D5 policy-shape closed
  (DL-12); D16 closed (DL-13); D17 closed (DL-14).
- **Current/next batch**: S2-09, extended by the `D17`-mandated remediation
  sequence. `AICAD-056` **COMPLETE**. `AICAD-057A` **COMPLETE**.
  `AICAD-057B` **COMPLETE**. `AICAD-057C` **COMPLETE**. `AICAD-057D`
  **COMPLETE**. `AICAD-057E` **COMPLETE** (this session, `status: done`).
  `AICAD-057F`: **not started**, `status: todo`, linear `depends_on` chain
  in `project/TASKS.yaml`. The original `AICAD-057` remains blocked on
  `AICAD-057F` (task-graph `depends_on`, not just prose).
- **THE NEXT INVOCATION MUST START `AICAD-057F` NEXT, IN ORDER** —
  "Adversarial integration pass proving generic/enum machinery is general,
  not `Result`-specific": per `project/TASKS.yaml`'s own acceptance
  criterion, exercise a user-defined generic enum whose name is not
  `Result`/`Optional`/`List`/`Range` through the same generic/pattern/
  exhaustiveness machinery end to end, confirm the full `OWNER_DECISIONS.md
  #D17`/`DECISION_LOG.md#DL-14` required-test list passes across
  `AICAD-057B`-`E` combined, and confirm no `?` syntax and no general
  compiler-intrinsic facility were added anywhere in that whole span. Note
  that `AICAD-057E` already contributed one piece of this evidence
  (`a_non_prelude_generic_enum_unit_variant_also_resolves_against_an_
  instantiated_expected_type`, `crates/cad-hir/src/typeck.rs`) — `057F`
  should treat that as a down payment, not assume it alone satisfies the
  task's own dedicated adversarial pass. Read `project/reports/
  AICAD-057E.md` first, especially "Known limitations" (struct
  construction is still not implemented by `cad-runtime`'s own
  interpreter — unrelated, pre-existing, out of scope; a user program that
  redeclares a prelude name is silently shadowed by the lowerer's own
  pre-existing "no duplicate-declaration diagnostic in this pass" design,
  not a new gap).
- **Exact recent state**: this session's own fresh runs (most recent
  first): `cargo test --workspace` all crates `ok`, 0 failed (per-crate
  breakdown above); `cargo build --workspace --all-targets` clean; `cargo
  clippy --workspace --all-targets --all-features -- -D warnings` clean;
  `cargo fmt --all -- --check` clean.
- **No open regressions.**
- **No escalations filed this session.** The one gap this session found in
  `AICAD-057D`'s own scope (`HirExpr::Ident`'s missing `Unit`-variant
  substitution) was judged to be within, not beyond, "what's needed to
  make the prelude enums usable exactly as any other user-defined generic
  enum already is" — see the report's own "Scope decisions" #2 and "The
  `HirExpr::Ident` fix" section for the full reasoning trail, including the
  hand-built failing-case evidence gathered *before* writing the fix.
- **Unresolved owner decisions**: unchanged from prior sessions — `D17`
  resolved (`DL-14`). Open/partial: D3, D5 (concrete tolerance constants
  only), D10, D11, D12, D15. This session added zero new provisional
  `TYPE-Exxx`/`RUNTIME-Exxx` codes (the prelude reuses every diagnostic
  `AICAD-057B`/`C`/`D` already built).
- **D5 status/evidence**: unchanged. This task's own determinism-relevant
  finding: `with_prelude`'s own item concatenation (`prelude items ++
  user items`) is a plain, deterministic `Vec::extend` in fixed order — no
  new ordering dependence introduced.
- **Pre-existing `TASKS.yaml` staleness** (unchanged, not this batch's
  scope): `AICAD-001` through `AICAD-037` still show `status: todo` despite
  being long complete.
- **Recommended next action**: start `AICAD-057F` — read `project/
  reports/AICAD-057E.md` first (its own "Known limitations" and "The
  `HirExpr::Ident` fix" sections document exactly what changed and why),
  plus `project/OWNER_DECISIONS.md#D17`/`project/DECISION_LOG.md#DL-14`
  for the exact required-test list `057F` must confirm end to end. Do not
  begin the original `AICAD-057`, `AICAD-058`, or the `STAGE2-C_EXECUTION`
  checkpoint before it.

## Environment

Unchanged from prior sessions (reconfirmed at session start): Rust 1.98.1
(auto-installed via `rustup`, matching `rust-toolchain.toml` — the
toolchain does not persist across sessions in this container), edition
2024. This session added **zero** new third-party dependencies (only a
new intra-workspace dependency edge, `cad-hir -> cad-parser`, promoted
from dev-only to a real dependency — no cycle). `cad-hir` (new
`prelude.rs` module, `lib.rs`, `typeck.rs`, `Cargo.toml`) and
`cad-runtime` (`interp.rs` tests only, no production code) were touched,
plus the usual `project/` bookkeeping files. No `specs/language/
grammar.ebnf` change was needed (no new syntax — the prelude uses exactly
the existing generic-enum/tuple-variant grammar `AICAD-057B`/`C` already
froze). No native/OCCT work was touched. `crates/cad-cli` was inspected
but not touched (still the zero-dependency scaffolding placeholder).

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
