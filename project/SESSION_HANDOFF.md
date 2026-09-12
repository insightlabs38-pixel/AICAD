# Session Handoff

## Latest: Batch S2-12 COMPLETE (`AICAD-062`, its own single-task batch).

This invocation implemented `tree-sitter-aicad/`, the Tree-sitter grammar
for the syntax subset `crates/cad-parser` already implements (per
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §18-19's "must share a conformance
test corpus so syntax never diverges" and the campaign brief's "must
match the already-approved AICAD syntax... must not become a second
independent language specification").

### What this invocation did

Full detail in `project/reports/AICAD-062.md`. Summary:

- `tree-sitter-aicad/grammar.js`: declarations, statements, the full
  expression precedence chain (range/`||`/`&&`/equality/relational/
  additive/multiplicative/unary/postfix/primary), patterns, both import
  path forms — matching `crates/cad-parser`'s *actual* implemented subset
  (not the fuller aspirational `specs/language/grammar.ebnf` sketch, which
  still lists `interface`/`assembly`/`requirement`/`test` declarations no
  parser implements yet).
- Two of `crates/cad-parser`'s context-sensitive restrictions
  (`no_record_literal` in `if`/`while`/`for`/`match` conditions; a bare
  `if`/`match` never becoming a `block_expr`'s trailing value) are
  reproduced despite Tree-sitter being a single context-free grammar — see
  `grammar.js`'s own module doc comment and the report for exactly how
  (a parallel expression hierarchy for the first; `prec.dynamic` +
  declared GLR conflicts on `block`/`block_expr` and `match_stmt`/
  `match_expr` for the second — a parallel hierarchy was tried first for
  the second restriction too but produced an unresolvable reduce-reduce
  conflict, documented in the report for the next person who considers
  that approach).
- A genuine Tree-sitter gotcha was found and worked around:
  `alias(seq(...multiple fields...), Name)` does not alias the whole seq
  as one node in this Tree-sitter version (0.27.0) — it fragments the
  alias across each field instead. Fixed by giving every field-bearing
  production its own separate `_..._impl` rule and aliasing only a
  reference to that rule, never an inline `seq(...)`. Documented in
  `grammar.js` itself so it isn't rediscovered.
- `tree-sitter-aicad/test/corpus/` (49 cases, 100% passing): positive
  coverage for every construct above, plus adversarial/negative cases
  (missing semicolon, unmatched delimiters, malformed struct field/import,
  non-chainable range, `if` without `else` in expression position) with
  their exact `(ERROR ...)`/`(MISSING ...)` trees obtained via
  `tree-sitter test --show-fields`, not guessed.
- `tree-sitter-aicad/queries/highlights.scm`: basic highlighting query,
  validated with `tree-sitter query`.
- **The actual "shared conformance corpus" link**: new
  `tests/parser/corpus/{positive,negative}/*.aicad` fixture files plus new
  `crates/cad-parser/tests/shared_corpus.rs`, which runs
  `cad_parser::parse_program` over every one of those same files
  (positive => zero diagnostics; negative => at least one). This is a
  concrete cross-parser check, not a restructuring of `cad-parser`'s
  existing inline unit tests.
- **Found via that same shared corpus**: a real, pre-existing
  `cad-parser`/`cad-lexer` gap — a `///` doc comment immediately before a
  top-level declaration produces a spurious `EXPECTED_ITEM` parse error
  (`Parser::parse_item`'s `match` has no `TokenKind::DocComment` arm).
  **Not fixed in this invocation** (out of `AICAD-062`'s scope — a defect
  in an earlier task's parser dispatch, not the grammar this task owns).
  The shared positive fixture works around it by simply not placing a doc
  comment directly before a declaration. Flagged as a follow-up for
  whichever task next touches `cad-parser`'s item dispatch; see
  `tree-sitter-aicad/README.md`'s own "Known limitation" section and the
  report's "Follow-up bugs".

**Escalations filed this invocation:** none.

## Current state / next action

- **Active stage**: Stage 2. D5/D16/D17/D18 all resolved (`DL-12`/`DL-13`/
  `DL-14`/`DL-15`), unchanged this invocation.
- **Batch S2-12 is COMPLETE**: `AICAD-062` done (its own single-task
  batch, per the fixed order).
- **THE NEXT INVOCATION MUST START BATCH S2-13** (`AICAD-063`, "Implement
  the complete end-to-end: `.aicad` source -> lexer/parser -> binding ->
  units/type checking -> typed HIR -> ordinary execution/control flow ->
  Geometry IR -> Stage-1 kernel API -> exact bracket -> valid STEP" — its
  own single-task batch). Do not start `AICAD-064` in that same
  invocation. Perform the complete Stage-2 integration proof before
  starting `AICAD-064`.
- **Exact recent state**: `tree-sitter generate` clean (no warnings/
  conflicts); `tree-sitter test` 49/49 passing; `cargo fmt --all --
  --check`/`cargo clippy --workspace --all-targets --all-features -- -D
  warnings`/`cargo test --workspace` all clean, 0 failures anywhere. New
  this invocation: `cad-parser` +2 tests (the new `shared_corpus.rs`
  integration test file — its unit-test count is otherwise unchanged).
  All other crates unchanged.
- **No open regressions.**
- **Unresolved owner decisions**: D3, D5 (concrete tolerance constants
  only), D10, D11, D12, D15 — all unchanged, pre-existing; nothing new
  added this invocation.
- **Recommended next action**: read `project/TASKS.yaml`'s `AICAD-063`
  entry, then the plan references it names, then design the end-to-end
  proof program (parameters, engineering units, derived expressions,
  functions, ordinary control flow, geometry operations) and its
  verification (B-rep validity, bounds, dimensions, volume, center of
  mass, solid count, topology sanity, STEP round-trip) per the campaign
  brief's "END-TO-END STAGE-2 PROOF" section. No demo-specific interpreter
  shortcuts are allowed; the program must flow through the ordinary
  language/HIR/Geometry-IR/kernel-API mechanisms already built by
  `AICAD-054`-`AICAD-061`.

## Environment

Unchanged from the `AICAD-061` session's own record, plus: `tree-sitter`
CLI 0.27.0 and Node v22.22.2 are available in this environment (used to
`generate`/`test` `tree-sitter-aicad/`; not a new Cargo workspace
dependency — `tree-sitter-aicad` is intentionally not a member of the
root `Cargo.toml` workspace, matching `docs/plan/
22_REPOSITORY_WORK_PACKAGES.md`'s repository layout, which places it as a
sibling of `crates/`, not inside it). Rust 1.98.1, edition 2024,
unchanged. Zero new third-party Cargo dependencies; `crates/cad-parser`'s
new `tests/shared_corpus.rs` uses only `std::fs`/`std::path` plus the
crate's own public API.

## Git identity

Unchanged from every prior session: global git config remains `Claude
<noreply@anthropic.com>` with `core.hooksPath` pointed at the repo's
identity-enforcing hooks, not modified by this invocation. Commits set
`GIT_AUTHOR_NAME`/`GIT_AUTHOR_EMAIL`/`GIT_COMMITTER_NAME`/
`GIT_COMMITTER_EMAIL` to `insightlabs38-pixel`/`insightlabs38@gmail.com` as
process-local environment variables for the `git commit` invocation only.
No hook bypassed; `--no-verify` never used.
