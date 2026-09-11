# Session Handoff

## Latest: `AICAD-057C` (data-carrying enum variants, constructors, destructuring patterns, match exhaustiveness) COMPLETE. Resume at `AICAD-057D` next — do not skip ahead.

This session started from `1888ed3` ("AICAD-057B: Implement generic
parameter/type-application syntax plus AST/HIR representation"), the tip
of `origin/claude/aicad-stage2-dev` at session start. `AICAD-057A`'s audit
and the owner's `D17` ruling (`project/DECISION_LOG.md#DL-14`) authorize a
fixed remediation sequence — `AICAD-057B` through `AICAD-057F` — before the
original `AICAD-057` ("recursion and Result/error propagation") may
resume. This session completed the third of those, `AICAD-057C`.

### What this session did

Implemented the three general enum-variant shapes D17 specifies end to
end — `enum Example { Unit, Tuple(T1, T2), Record { x: T1, y: T2 } }` —
usable as constructor expressions (`Ok(value)` reuses ordinary call
syntax; `Point { x: 1mm, y: 2mm }` gets new brace-literal syntax mirroring
the pattern side), corresponding tuple/record destructuring patterns with
normal lexical scope and variant-derived binding types, and a genuine
nominal-enum match-exhaustiveness check (a real, previously-undetected
soundness gap `AICAD-057A`'s own audit found — finding #8: "today's
`match` performs no coverage check ... at all"). Full details, the exact
scope boundary, and the complete test list in `project/reports/
AICAD-057C.md`. Summary:

- `cad-ast`: `EnumVariant` enum (`Unit`/`Tuple`/`Record`) replaces the old
  `Item::Enum::variants: Vec<Spanned<String>>`; new `Expr::RecordLiteral`;
  `Pattern` gained `Tuple`/`Record` (plus `RecordPatternField`, which
  desugars shorthand `{ x, y }` to `Pattern::Ident` at parse time).
- `cad-parser`: `parse_enum_variants` handles all three shapes;
  `parse_record_literal`; `parse_tuple_pattern`/`parse_record_pattern`; new
  `Parser::no_record_literal` restriction (the standard "no record literal
  in `if`/`while`/`match` condition/scrutinee position" technique every
  Rust-like language with this syntax needs — reset inside any nested
  unambiguous delimiter).
- `specs/language/grammar.ebnf`: new `enum_variant` production; the
  grammar's first-ever formal `pattern` production (previously only
  referenced, never defined) plus `tuple_pattern`/`record_pattern`;
  `record_literal` added to `or_expr`.
- `cad-compiler` (binder): `Expr::RecordLiteral`/`Pattern::Tuple`/`Record`
  handled — a `Name(...)`/`Name { ... }` pattern's `name` is always
  checked as an existing reference (never a fresh binding, unlike the
  genuinely ambiguous bare-`Ident` case).
- `cad-hir`: `HirExpr::RecordLiteral`; `HirPattern::Tuple`/`Record`
  (`variant: Option<BindingId>`, unlike `HirPattern::Variant`'s
  non-`Option` field — see the report for why); `HirEnumVariant` gained a
  `payload: HirVariantPayload` field.
- `cad-hir` (typeck): new `VariantShape`/`Checker::variant_shapes`/
  `enum_variants`; new `collect_enum_variant_shapes` pass (routes payload-
  type resolution through `AICAD-057B`'s own `active_type_params`/
  `with_type_params`, exactly as that task's own follow-up note
  anticipated); `check_variant_tuple_construction`/`check_record_literal`;
  `check_match_exhaustiveness`; `bind_pattern` extended with real
  structural/type checking for the new pattern shapes. 11 new provisional
  `TYPE-Exxx` diagnostic codes (`446`-`456`, full table in the report).
- `cad-runtime`: `Value::EnumVariant` gained a `payload: VariantPayload`
  field (`Unit`/`Tuple`/`Record`); `call()`/`eval_expr` construct variants;
  `pattern_matches` destructures them; `values_equal` now recurses into
  variant payloads (and `List` elements) — a genuine correctness fix
  (`Ok(1) == Ok(2)` was trivially `true` under the old tag-only equality
  before payloads existed to make that wrong).

Test deltas (all passing, 0 regressions — one pre-existing test correctly
gained a second diagnostic, see report): `cad-ast` (printer round-trip)
15 -> 19; `cad-parser` 108 -> 119; `cad-compiler` 43 -> 49; `cad-hir`
123 -> 146; `cad-runtime` 61 -> 69. Full `cargo test --workspace` 682
passed, 0 failed; `cargo clippy --workspace --all-targets --all-features
-- -D warnings` clean; `cargo fmt --all -- --check` clean.

## Current state / next action

- **Active stage**: Stage 2. Stage 1 closed (DL-11); D5 policy-shape closed
  (DL-12); D16 closed (DL-13); D17 closed (DL-14).
- **Current/next batch**: S2-09, extended by the `D17`-mandated remediation
  sequence. `AICAD-056` **COMPLETE**. `AICAD-057A` **COMPLETE**.
  `AICAD-057B` **COMPLETE**. `AICAD-057C` **COMPLETE** (this session,
  `status: done`). `AICAD-057D` through `AICAD-057F`: **not started**,
  `status: todo`, linear `depends_on` chain in `project/TASKS.yaml`. The
  original `AICAD-057` remains blocked on `AICAD-057F` (task-graph
  `depends_on`, not just prose).
- **THE NEXT INVOCATION MUST START `AICAD-057D` NEXT, IN ORDER** —
  "Implement generic instantiation/inference/type checking for the
  approved Stage-2 generic subset": call-site type-parameter instantiation
  from argument types (`identity(5mm)` binding `T = Length`), a
  `Name<Args>` type *reference* resolving against a user-defined generic
  struct/enum (`Pair<Length, Mass>` as a declared type — currently `None`,
  no diagnostic, per both `AICAD-057B`'s and this session's own documented
  limitation), a diagnostic (not an arbitrary selection) for an ambiguous
  generic call, and a diagnostic for a wrong number of explicit type
  arguments. Read `project/reports/AICAD-057B.md` and `AICAD-057C.md`
  ("Known limitations") before starting — both already document exactly
  which generic-instantiation gaps remain and why deferring them was
  correct for their own tasks. `AICAD-057C`'s own `variant_shapes`
  machinery (payload types keyed by `BindingId`, resolved once via
  `with_type_params`) is the natural place a substitution step would plug
  in when checking a tuple/record variant construction whose enum is
  generic — worth reading before designing `AICAD-057D`'s own approach,
  not necessarily reusing it verbatim.
- **Exact recent state**: this session's own fresh runs (most recent
  first): `cargo test --workspace` 682 passed, 0 failed (per-crate
  breakdown above); `cargo build --workspace --all-targets` clean; `cargo
  clippy --workspace --all-targets --all-features -- -D warnings` clean;
  `cargo fmt --all -- --check` clean.
- **No open regressions.**
- **Unresolved owner decisions**: unchanged from prior sessions — `D17`
  resolved (`DL-14`). Open/partial: D3, D5 (concrete tolerance constants
  only), D10, D11, D12, D15. This session added 11 new provisional
  `TYPE-Exxx` codes (`446`-`456`, `cad-hir`'s `typeck.rs`), still
  provisional pending D10, same convention as every prior batch.
- **D5 status/evidence**: unchanged. This task's own determinism-relevant
  finding: `Checker::collect_enum_variant_shapes` (like `collect_struct_
  fields` before it) is a plain, deterministic source-order iteration over
  `program.items`, populated once before any body/value is checked — no
  new ordering dependence introduced.
- **Pre-existing `TASKS.yaml` staleness** (unchanged, not this batch's
  scope): `AICAD-001` through `AICAD-037` still show `status: todo` despite
  being long complete.
- **Recommended next action**: start `AICAD-057D` — read `project/
  reports/AICAD-057B.md` and `AICAD-057C.md` first (both documented
  their own generics-instantiation limitations precisely for this
  handoff), plus `project/OWNER_DECISIONS.md#D17`/`project/
  DECISION_LOG.md#DL-14` for the exact authorized instantiation/inference
  subset (unambiguous inference from argument/expected types; explicit
  type arguments if the grammar requires them; a diagnostic — never a
  silent/arbitrary choice — for ambiguity or a wrong arity). Do not begin
  `AICAD-057E`/`F`, the original `AICAD-057`, or `AICAD-058` before it.

## Environment

Unchanged from prior sessions (reconfirmed at session start): Rust 1.98.1
(auto-installed via `rustup`, matching `rust-toolchain.toml` — the
toolchain does not persist across sessions in this container), edition
2024. This session added **zero** new third-party dependencies. Crates
touched: `cad-ast`, `cad-parser`, `cad-compiler`, `cad-hir`, `cad-runtime`,
plus `specs/language/grammar.ebnf` and the usual `project/` bookkeeping
files. No native/OCCT work was touched.

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
