# AICAD-042: Implement declarations: let/const/param/fn/struct/enum/part

## Objective

Implement the `item` alternatives named by this task's own title —
`let_decl`/`const_decl`/`param_decl`/`fn_decl`/`struct_decl`/`enum_decl`/
`part_decl` — plus enough statement-level parsing (`let_stmt`/`var_stmt`/
`assign_stmt`/`expr_stmt`) for `fn`/`part` bodies to be usable at all, per
`project/TASKS.yaml`'s `AICAD-042` entry and Batch S2-02's task order
(`041 -> 042 -> 043`).

## Base commit

`e85f2ad` (this session's own `AICAD-041` commit, on
`branch/tender-hypatia-w0huqo`, based on `origin/main`'s `cbf769a`).

## Files changed

- `crates/cad-ast/src/item.rs` (new): `Type`, `Field`, `FnParam`, `Stmt`,
  `Block`, `Item`, `Program` AST node types.
- `crates/cad-ast/src/lib.rs`: wire up the new `item` module.
- `crates/cad-parser/src/lib.rs`: adds `parse_type`, `parse_binding_tail`
  (shared by `let`/`var`/`const`), `parse_block`, `parse_stmt`,
  `parse_fn_params`, `parse_struct_fields`, `parse_enum_variants`,
  `parse_item`, `parse_program`, the `parse_program()` free-function
  convenience wrapper, and 24 new tests (`decl_tests` module). Renamed
  `peek_is_named_arg_start` to `peek_next_is_bare_eq` (unchanged logic,
  now shared by both named-call-argument and `assign_stmt` disambiguation
  — see decision 4).

No third-party dependency added; `Cargo.lock` unaffected by this task
(cad-parser's dependency set already settled in `AICAD-041`).

## Material implementation decisions

1. **Scope boundary, restated precisely**: excluded from this task —
   `interface_decl`/`assembly_decl`/`requirement_decl`/`test_decl`/
   `import_decl` (not named by this task's title; several use keywords
   `cad-lexer` has deliberately not reserved yet — `configuration`,
   `component`, `assembly`, `instance`, `mate`, `joint`, `requirement`,
   `test`, `constraint`, `expose`, `query`, `unsafe`); every control-flow
   statement/expression form (`AICAD-043`'s own title). Documented in
   `cad_ast::item`'s module doc comment.
2. **`Type` is purely syntactic and deliberately narrow**: a bare name
   (`Length`) or a name with comma-separated generic arguments
   (`Vector2<Length>`, `List<Point2>` — both drawn verbatim from
   `examples/assemblies/stage0_paper_example.aicad`). No array/tuple/
   function-type syntax — nothing evidences a need for them yet, and the
   grammar sketch never spells out a `type` production at all (only used
   informally in examples), so this implements exactly the two shapes
   with concrete evidence and nothing broader.
3. **Enum variants are unit-only** (`enum MotorSize { NEMA17, NEMA23 }` —
   the paper example's own, only piece of enum-syntax evidence). No
   tuple/struct data-carrying variant syntax is implemented — no RFC/plan
   section specifies one, and guessing a shape now would be exactly the
   speculative syntax `AGENTS.md` warns against. A later task adding data
   variants (plausibly `AICAD-053`, "structs/enums field and variant
   typing") is a small additive grammar change, not a redesign of this
   one.
4. **`assign_stmt` vs. `expr_stmt` disambiguation reuses `AICAD-041`'s
   named-argument lookahead trick.** Both `named_arg = identifier "="
   expression` (inside call args) and `assign_stmt = identifier "="
   expression ";"` (at statement position) need exactly the same one-
   token-of-lookahead check — "is the token after this identifier a bare
   `Eq`, not `EqEq`?" — so the existing helper
   (`peek_is_named_arg_start`) was renamed to the more general
   `peek_next_is_bare_eq` and reused rather than duplicated. Regression
   tests: `distinguishes_assign_stmt_from_expr_stmt_starting_with_an_identifier`,
   `does_not_confuse_equality_comparison_with_assignment`.
5. **`assign_stmt`'s target stays a bare identifier**, exactly as
   `assign_stmt = identifier "=" expression ";"` specifies — no
   field-assignment target (`a.b = x;`). Consistent with DL-2's
   functional-core ruling: nothing in DL-2 or the grammar describes
   mutating a field through assignment, so this is not a gap to fill (see
   `AICAD-041`'s own `Expr::Field` gap-fill, which *was* evidenced —
   assignment-to-a-field is not).
6. **`param_decl`'s type annotation is mandatory**, unlike `let`/`const`
   (where it is optional, matching `let_stmt`/`var_stmt`'s own `[":"
   type]`). The only concrete evidence
   (`param width: Length = 80mm;`) always carries a type, and a part's
   configurable-input surface is exactly where an explicit type is
   load-bearing rather than ergonomic sugar. Regression test:
   `reports_malformed_param_decl_missing_type`.
7. **Recovery is "always make progress."** Every loop that parses a
   sequence (block statements, struct fields, enum variants, `part`/
   top-level items) records `self.pos` before attempting one element and
   force-advances past the current token if parsing that element
   consumed nothing — so a malformed input can never hang the parser in
   an infinite loop. Regression test:
   `does_not_hang_on_a_sequence_of_unrecognized_tokens` (a run of tokens
   that start no valid item at all still terminates, with one diagnostic
   per bad token). An unrecognized top-level/item-position token is
   reported (new code `PARSE-E010`) and skipped, letting parsing continue
   with whatever follows — `reports_unrecognized_top_level_token_and_recovers`
   confirms a well-formed declaration after a bad token is still found.
8. **Diagnostic codes**: adds `PARSE-E010` ("expected a declaration") to
   the `PARSE-E00N` family `cad-lexer`/`AICAD-041` started. No other new
   code was needed — missing colons/braces/parens reuse `E006`
   ("expected token X, found Y"), missing names reuse `E007`.

## Exact commands and results

```
$ cargo test -p cad-ast -p cad-lexer -p cad-diagnostics -p cad-parser
test result: ok. 7 passed (cad-ast)
test result: ok. 20 passed (cad-diagnostics lib)
test result: ok. 10 passed (cad-diagnostics schema_conformance)
test result: ok. 27 passed (cad-lexer, unchanged)
test result: ok. 50 passed (cad-parser: 26 AICAD-041 + 24 new AICAD-042)

$ cargo build --workspace --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s)   (clean)

$ cargo clippy -p cad-ast -p cad-parser --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s)   (zero warnings, after fixing
one `while_let_loop` lint in the new `parse_fn_params` — restructured to
`while let Some(name) = self.expect_ident(...) { ... }`, matching the
pattern `AICAD-041`'s `parse_binary_level` already used for the same lint)

$ cargo fmt --all -- --check
(no output — clean)
```

Native/OCCT Stage-1 suite was not re-run — no native/kernel code touched.

## Tests / regressions

24 new tests in `crates/cad-parser/src/lib.rs`'s new `decl_tests` module:

- **Positive, matching frozen example material verbatim**:
  `param width: Length = 80mm;` (paper example), `enum MotorSize { NEMA17,
  NEMA23 }` (paper example), generic type annotations
  (`Vector2<Length>`).
- **Positive, structural**: `let`/`const` at item scope (with and without
  a type annotation), `param` with/without a default, `fn` with params/
  return type/zero params/a parameter default, `pure fn`, `struct` (with
  fields, with a trailing comma, and empty), a cut-down `part` body using
  only this task's item vocabulary, all four statement kinds inside one
  block, a whole program mixing multiple item kinds.
- **Disambiguation** (decision 4): assign vs. expression statement
  starting with an identifier; equality comparison not mistaken for
  assignment.
- **Adversarial/negative**: an unrecognized top-level token recovers and
  still finds the next well-formed declaration; a missing semicolon after
  `let` recovers and still finds a subsequent declaration; a `param`
  missing its mandatory type is reported; an unclosed `struct` is
  reported and does not hang; an empty program produces zero items and
  zero diagnostics; a run of tokens that start no valid item never hangs
  the parser (exact diagnostic count checked).

No pre-existing test (`AICAD-041`'s 26, or any other crate's) was modified
or weakened. One real bug in test authorship — not implementation — was
caught before commit: an early draft of `parses_pure_fn_decl` and
`parses_part_decl_with_nested_items` used `return` inside a function body,
which is `AICAD-043`'s own keyword and not yet parseable by this task;
both were caught by `program_ok`'s own "no diagnostics" assertion failing
in exactly the way the scope boundary in decision 1 predicts, and rewritten
to stay within this task's actual vocabulary.

## Known limitations

- No control-flow statement/expression exists yet (`if`/`for`/`while`/
  `loop`/`match`/`return`/`break`/`continue`, `block_expr`/`if_expr`/
  `match_expr`) — `AICAD-043`.
- `Block` never accepts a trailing value-producing expression yet (that
  is `block_expr`, `AICAD-043`'s own extension of this same brace-
  delimited shape).
- No name resolution, type checking, or DL-2 functional desugaring of
  anything parsed here — purely syntactic, per this crate's ongoing
  contract.
- Enum variants and `Type` are both deliberately narrower than the full
  grammar surface a mature language would eventually need (see decisions
  2 and 3) — both are documented, additive extension points for later
  tasks, not gaps discovered by accident.

## Unresolved questions

None requiring owner escalation. No `OWNER_DECISIONS.md` entry added.
