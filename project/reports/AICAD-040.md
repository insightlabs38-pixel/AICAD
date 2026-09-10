# AICAD-040 — Implement numeric literals with engineering-unit suffix tokenization

## Objective
Extend `cad-lexer`'s numeric-literal scanning to recognize an
immediately-adjacent engineering-unit suffix (`5mm`, `12.4MPa`, `30deg`),
per `project/TASKS.yaml` AICAD-040 (Stage-2 batch S2-01, third and final
task).

## Base commit
`2b3c6fe` (AICAD-039, this session).

## Plan references read
`rfcs/0004-units-type-system.md` §4 (initial unit set: length, angle,
mass, force, pressure/stress, affine temperature) and §10 (numerical
precision policy); `docs/plan/02_LANGUAGE_AND_COMPILER.md` §3 (`5mm`,
`12.4MPa`, `30deg` worked examples).

## Implementation
- `TokenKind::Number(String)` (tuple variant, raw text only) became
  `TokenKind::Number { text: String, unit: Option<String> }`. All
  `AICAD-039` call sites/tests updated accordingly (no behavior change for
  the `unit: None` case).
- `Lexer::scan_number` now, after scanning the numeric text exactly as
  before, checks whether the very next character (zero intervening
  whitespace) starts an identifier; if so it consumes an identifier-shaped
  run as the unit suffix and fuses it into the same token.
- The unit suffix is **not validated** against RFC-0004 §4's list or any
  other registry — see Decisions below.

## Decisions made and why

1. **Suffix recognition, not suffix validation.** RFC-0004 §4 explicitly
   states "the standard library may expand this set without a grammar
   change." If the lexer hard-coded and enforced exactly the current
   initial unit list, adding a new unit later would require a lexer
   change despite the RFC's own promise that it wouldn't. Instead the
   lexer treats *any* identifier-shaped run immediately following a
   number's digits as a candidate unit suffix and fuses it structurally;
   whether a given suffix names a real, registered unit is deferred to
   the unit registry (`AICAD-048`, `crates/cad-units`), which is where
   RFC-0004 §4's actual table belongs as executable data. This is
   consistent with `AGENTS.md`'s "smallest correct change" and avoids
   building (and then having to keep in sync) two copies of the unit
   list.
2. **Adjacency is exact: zero whitespace.** `5mm` fuses into one token;
   `5 mm` does not (two tokens: a bare number, then a separate
   identifier). This matches ordinary suffixed-literal conventions (e.g.
   Rust's `5u32` vs. `5 u32`) and gives an unambiguous rule with no
   lookahead beyond "is the very next character part of an identifier."
3. **Found and resolved a real spelling collision: `in`.** RFC-0004 §4
   lists `in` (inches) as a length unit, but `specs/language/grammar.ebnf`
   already reserves `in` as the for-loop keyword (`for x in y`). Verified
   this is not an actual ambiguity: the keyword `in` only ever appears as
   a standalone, whitespace-delimited word, while the unit suffix `in`
   only exists via the number-scanner's own suffix-consumption path,
   which never revisits `token::lookup_reserved_word`. `5in` therefore
   always fuses to a unit literal, and `for x in y` is completely
   unaffected — verified by a dedicated test
   (`inches_unit_suffix_does_not_collide_with_the_in_keyword`) exercising
   both directions in the same test. Documented in both `token.rs`'s
   `lookup_reserved_word` doc comment and here so a future task
   (`AICAD-041`+) doesn't "fix" an ambiguity that doesn't actually exist.
4. **No numeric value parsing (still raw text).** Converting `"12.4"` to
   an actual `f64`/typed quantity and canonicalizing the unit is
   type-checker/HIR territory (`AICAD-046`-`AICAD-052`), consistent with
   RFC-0004 §5's canonical-representation rules — the lexer's job is
   recognizing lexical shape, not performing unit canonicalization.

No escalation condition was triggered: this extends lexical scanning
strictly within RFC-0004's own frozen unit-literal syntax; it validates
nothing that would require a type-system ruling, and the one genuine
ambiguity found (`in`) was resolved by evidence (how the two spellings can
actually appear in valid source), not by an arbitrary choice.

## Files changed
- Modified: `crates/cad-lexer/src/token.rs` (`TokenKind::Number` shape;
  `lookup_reserved_word` doc note on the `in` collision),
  `crates/cad-lexer/src/lib.rs` (`scan_number` suffix scanning; updated
  `AICAD-039` tests to the new `Number` shape; new AICAD-040 tests),
  `crates/cad-lexer/README.md`, `project/TASKS.yaml`.
- Added: `project/reports/AICAD-040.md`.

## Verification (exact commands/results)
```
$ cargo test -p cad-lexer
running 27 tests ... test result: ok. 27 passed; 0 failed

$ cargo test -p cad-ast -p cad-lexer -p cad-diagnostics
cad-ast:          7 passed
cad-lexer:        27 passed
cad-diagnostics:  20 passed + 10 passed (schema_conformance.rs)

$ cargo fmt --all -- --check
(exit code 0, no output)

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(zero warnings)
```

New tests added this task: the exact three `docs/plan/02` §3 worked
examples (`5mm`, `12.4MPa`, `30deg`); every one of RFC-0004 §4's 26
initial units individually; a negative test proving whitespace prevents
fusion; scientific notation fusing with a unit (`1.5e-3kg`); an unknown
suffix still fusing without a spurious diagnostic; and the `in`
keyword/unit-suffix non-collision (both directions).

## Regressions found and fixed during this task
None new — the `in` finding in Decisions §3 was caught by design analysis
before writing the fusing logic (considered and tested up front), not
discovered as a failing test afterward.

## Known limitations / follow-up
- Still no numeric value parsing/canonicalization (Decisions §4) —
  `AICAD-046`-`AICAD-052`.
- Still no validation that a given suffix is a real, registered unit —
  intentionally deferred to `AICAD-048`.
- `@material(...)`-style attribute syntax remains only a bare `@` token;
  full attribute-argument parsing is `AICAD-041`/`AICAD-042` scope.

## Batch S2-01 status
This completes Batch S2-01 (`AICAD-038` -> `AICAD-039` -> `AICAD-040`),
the first Stage-2 batch. Per the fixed batch order, the next invocation
begins Batch S2-02 (`AICAD-041` expression parser and precedence) only
after this batch's work is on the canonical branch.
