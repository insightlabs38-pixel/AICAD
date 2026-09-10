# AICAD-039 — Create cad-ast and cad-lexer with token/span model

## Objective
Implement `crates/cad-ast` (the shared source-span model) and
`crates/cad-lexer` (tokenization of AICAD source into spanned tokens),
per `project/TASKS.yaml` AICAD-039 (Stage-2 batch S2-01, second task).

## Base commit
`57a31d5` (AICAD-038, this session).

## Plan references read
`rfcs/0001-language-principles.md` §4/§7 (surface syntax freeze, grammar
sketch); `specs/language/grammar.ebnf`; `docs/plan/02_LANGUAGE_AND_COMPILER.md`
§3 (lexical elements); `rfcs/0004-units-type-system.md` §10 (tolerance
comparison operators).

## Implementation

### `crates/cad-ast`
- `Span` (half-open `u32` byte-offset range), `Spanned<T>`, `LineColumn`,
  and `LineIndex` (byte offset -> 1-based line/column, matching
  `cad-diagnostics`/RFC-0005 §3's position shape exactly).
- **Scope decision:** the actual AST node types (`Expr`/`Stmt`/`Item`/...)
  are *not* added yet — see "Decisions" below.

### `crates/cad-lexer`
- `token.rs`: `Keyword` (24 reserved words — exactly the set already
  appearing in `specs/language/grammar.ebnf`), `TokenKind` (identifiers,
  keywords, bool literals, raw number text, strings/raw strings, doc
  comments, and punctuation/operators), `Token` (`kind` + `Span`).
- `lib.rs`: a hand-written scanner (`tokenize(source, file) -> (Vec<Token>,
  Vec<Diagnostic>)`) producing `cad_diagnostics::Diagnostic`s (never bare
  strings) for lexical errors: `PARSE-E001` unexpected character,
  `PARSE-E002` unterminated string, `PARSE-E003` unterminated block
  comment, `PARSE-E004` invalid escape sequence (recovers and continues
  rather than aborting the whole lex).
- Comments: `//` line and `/* */` block comments are trivia (never
  tokens, non-nesting block comments); `///` doc comments are retained as
  `TokenKind::DocComment` tokens (attaching them to a following item is
  parser scope, not lexer scope).
- Strings: `"..."` with escapes (`\\ \" \n \r \t \0`); `r"..."` raw
  strings with zero escape processing.
- Numbers: raw digit/decimal-point/scientific-exponent text only, no unit
  suffix — `AICAD-040`'s own scope.
- Operators: maximal-munch multi-char (`-> => :: == != <= >= && || ~=`)
  plus single-char (`{ } ( ) [ ] ; : , . @ = < > + - * / !`).

## Decisions made and why

1. **`cad-ast` gets only the span model in this task, not AST node
   types.** `AGENTS.md`'s "No speculative future work" warns against
   public APIs/abstractions owned by a later task being invented ahead of
   time. The real node shapes (`Expr`, `Stmt`, declaration items, etc.)
   are exactly what the parser tasks (`AICAD-041` expressions,
   `AICAD-042` declarations, `AICAD-043` control flow) are scoped to
   define — guessing their representation now risks committing to a shape
   the parser then has to work around. `Span`/`Spanned<T>`/`LineIndex`
   are different: both the lexer (this task) and the parser (next batch)
   need "a range of source text" as a primitive, so building it once,
   shared, is the smallest correct foundation rather than duplicated
   private scaffolding in each crate.
2. **Keyword set is exactly what `specs/language/grammar.ebnf` already
   reserves** — 13 declaration keywords + 11 control-flow keywords + `pure`,
   24 total. Words used only in `docs/plan/` prose examples but never
   formalized into the grammar artifact (`expose`, `query`, `unsafe`,
   `comptime`, `yield`, `drawing`, `simulation`, `configuration`) are
   deliberately **not** reserved yet. Reserving a word makes it
   permanently unavailable as an identifier everywhere; a later task that
   actually adds grammar for one of them should reserve it in that same
   change, with the grammar update as evidence, rather than this task
   guessing which prose examples will become real syntax and in what
   form.
3. **`true`/`false` resolve to `TokenKind::BoolLiteral`, not a `Keyword`
   variant** — they are literals of RFC-0004 §2's `Bool` primitive type,
   not declaration/control-flow syntax, so representing them as their own
   token kind (as most lexers do for boolean/null literals) is more
   accurate than folding them into `Keyword`.
4. **Operator set is restricted to what has textual evidence** in
   `docs/plan/`/`rfcs/`: arithmetic (`+ - * /`), comparison (`== != < <=
   > >=`), logical (`&& || !`), assignment (`=`), member/path (`. :: ->
   =>`), and the tolerance-comparison operator `~=` RFC-0004 §10
   explicitly names. `%`, bitwise/shift, and range operators appear
   nowhere in the plan bundle or RFCs, so they are omitted rather than
   guessed — extending the token set when `AICAD-041` actually freezes
   expression precedence is a small additive change made with real
   evidence in hand, not a redesign.
5. **Line/column counting uses Unicode scalar values (`char`s), not
   UTF-16 code units.** Simpler and well-defined; if LSP integration
   (`crates/cad-lsp`, Stage 9) later needs UTF-16 columns, that conversion
   belongs there (documented in `cad-ast`'s own doc comment), not baked
   into the shared `LineIndex`.
6. Unicode identifiers are accepted (`is_alphabetic`/`is_alphanumeric`)
   per `docs/plan/02` §3 ("Unicode identifiers optionally allowed"), but
   this task does **not** implement the "normalized" half of that
   sentence (no NFC/NFKC normalization) — the normalization algorithm
   isn't specified anywhere in the plan bundle, and guessing one would be
   inventing behavior. Disclosed as a known limitation, not silently
   dropped.

No escalation condition was triggered: this only implements lexical
scanning already licensed by RFC-0001/RFC-0004 and the frozen grammar
artifact; it introduces no new public syntax, no kernel-type exposure, and
touches no open architecture alternative.

## Files changed
- Added: `crates/cad-lexer/src/token.rs`,
  `project/reports/AICAD-039.md`.
- Modified: `crates/cad-ast/src/lib.rs` (placeholder -> span model),
  `crates/cad-ast/README.md`, `crates/cad-lexer/src/lib.rs` (placeholder
  -> lexer), `crates/cad-lexer/Cargo.toml` (adds `cad-ast`/
  `cad-diagnostics` path dependencies), `project/TASKS.yaml`.

## Verification (exact commands/results)
```
$ cargo test -p cad-ast -p cad-lexer -p cad-diagnostics
cad-ast:          running 7 tests  ... ok. 7 passed; 0 failed
cad-lexer:        running 21 tests ... ok. 21 passed; 0 failed
cad-diagnostics:  running 20 tests ... ok. 20 passed; 0 failed
                  running 10 tests ... ok. 10 passed; 0 failed  (schema_conformance.rs)

$ cargo fmt --all -- --check
(exit code 0, no output)

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(zero warnings)

$ cargo test --workspace
(all crates: ok, 0 failed; the 48 tests above are the only non-zero suites
 so far — every other crate remains an unimplemented placeholder)
```

## Regressions found and fixed during this task
1. `cad-ast::LineIndex`: an early version of the multibyte-character test
   itself had a wrong expected byte length (test bug, not implementation
   bug) — corrected the test's own arithmetic (`"café\nx"` is 7 bytes, not
   6) rather than changing the implementation, which was already correct.
2. **Genuine lexer bug**: the raw-string prefix check (`c == 'r' &&
   peek_at(1) == '"'`) was ordered *after* the generic identifier-start
   branch in the main scan loop, so `'r'` always matched as an identifier
   first and raw strings were never recognized — `r"C:\no\escapes"` was
   mis-lexed as `Ident("r")` followed by an ordinary escaped string, which
   then raised a spurious `PARSE-E004` on `\e`. Root-caused and fixed by
   moving the raw-string check before the identifier-start check (order
   matters: `r"` must be tried first, and only a bare `r` not followed by
   `"` falls through to identifier scanning). Added a permanent regression
   test (`identifier_named_r_is_not_mistaken_for_a_raw_string_prefix`)
   confirming both a variable literally named `r` and an identifier
   merely starting with `r` (`radius`) still lex correctly, plus the
   original raw-string test, which now passes.

## Known limitations / follow-up
- No AST node types yet (by design — see Decisions §1); `AICAD-041`
  through `AICAD-045` add them alongside the parser productions that
  produce them.
- No Unicode identifier normalization (Decisions §6).
- Numeric literals carry no unit-suffix/value semantics yet — `AICAD-040`.
- The lexer does not yet fuse a number immediately followed by an
  identifier (e.g. `5mm`) into one literal; today that lexes as two
  adjacent tokens (`Number("5")`, `Ident("mm")`) with no space between
  their spans. This is intentional interim behavior for this task and is
  exactly what `AICAD-040` changes.
