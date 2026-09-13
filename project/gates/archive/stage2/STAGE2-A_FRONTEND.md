# Stage-2 Batch checkpoint — Front-end (AICAD-038..045)

Prepared after `AICAD-045`, per the active scheduled-task brief's batch
checkpoint requirement ("After AICAD-045 create:
`project/gates/STAGE2-A_FRONTEND.md`"). This is a **batch checkpoint**
gating Batch S2-04 (`AICAD-046` onward), not the Stage-2 owner gate
packet (that is `AICAD-064`'s job, per `project/gates/README.md`'s
format for `stage-<n>-gate.md`). Per `AGENTS.md` ("Stage gates"),
preparing evidence and a recommendation is within this agent's role;
this checkpoint does not itself constitute owner approval of anything —
Stage 2 as a whole still requires the owner-recorded decision `AICAD-064`
will seek, per `project/CURRENT_STAGE.md`.

## 1. Exact git revision at checkpoint time

Prepared on branch `branch/wonderful-thompson-19031x`, HEAD `6f00639`
(the `AICAD-045` commit), immediately before this checkpoint's own
additional adversarial tests (§6) and this document land. Batch S2-01
through S2-03 span commits from `1eaac7c` (Stage-1 approval + D5
policy, advance to Stage 2) through `6f00639`, following `cbf769a`
(PR #9 merge — owner-approved Stage-1 main commit). Working tree was
clean at the start of this checkpoint's own verification run.

## 2. Batch scope and task reports

| Task | Title | Report |
|---|---|---|
| AICAD-038 | Create cad-diagnostics crate and JSON-schema conformance tests | `project/reports/AICAD-038.md` |
| AICAD-039 | Create cad-ast and cad-lexer with token/span model | `project/reports/AICAD-039.md` |
| AICAD-040 | Implement numeric literals with engineering-unit suffix tokenization | `project/reports/AICAD-040.md` |
| AICAD-041 | Implement expression parser and precedence | `project/reports/AICAD-041.md` |
| AICAD-042 | Implement declarations: let/const/param/fn/struct/enum/part | `project/reports/AICAD-042.md` |
| AICAD-043 | Implement control-flow syntax: if/for/while/match/return | `project/reports/AICAD-043.md` |
| AICAD-044 | Implement module/import syntax and loader skeleton | `project/reports/AICAD-044.md` |
| AICAD-045 | Implement minimal formatter / AST pretty-printer | `project/reports/AICAD-045.md` |

Crates in scope: `cad-diagnostics`, `cad-ast`, `cad-lexer`,
`cad-parser`, `cad-compiler` (its `loader` module only — the rest of
that crate stays an empty placeholder pending later tasks).

## 3. Checklist (per the scheduled-task brief's Batch S2-03/Stage-2A checkpoint items)

- **Grammar/RFC agreement.** `specs/language/grammar.ebnf` freezes
  `let_stmt`/`var_stmt`/`assign_stmt`/`expr_stmt`/`if_stmt`/`for_stmt`/
  `while_stmt`/`loop_stmt`/`match_stmt`/`return_stmt`/`break_stmt`/
  `continue_stmt`, `fn_decl`, `expression`/`block_expr`/`if_expr`/
  `match_expr`, `call_expr`/`method_call_expr`/`args`/`named_arg` — every
  one is implemented exactly as specified (`cad-parser`'s own doc
  comments cite the exact grammar production per construct).
  `struct_decl`/`enum_decl`/`part_decl`/`import_decl` are referenced by
  the grammar's `item` production but never defined there (confirmed by
  reading the whole file, recorded in `AICAD-044`'s report) — those five
  forms were operationalized from the Stage-0 paper example
  (`struct`/`enum`/`part`) and `docs/plan/02_LANGUAGE_AND_COMPILER.md`
  §10's own worked examples (`import`), per each task's own
  decision-record, never invented independent of evidence.
  `interface_decl`/`assembly_decl`/`requirement_decl`/`test_decl` remain
  correctly out of scope (unreserved keywords, no task names them yet —
  `cad-ast/src/item.rs`'s own module doc comment tracks this explicitly).
  No accidental deviation from DL-1 (braces/semicolons, no significant
  indentation, no ASI) or DL-2 (method syntax is sugar, never in-place
  mutation — `Expr::MethodCall` is preserved as a distinct AST node,
  desugaring deferred to `AICAD-051` HIR lowering, never done early).
  **PASS.**
- **Parser precedence.** `AICAD-041`'s frozen precedence-climbing table
  (`or -> and -> equality/tolerance -> relational -> additive ->
  multiplicative -> unary -> postfix -> primary`) is exercised by
  `multiplication_binds_tighter_than_addition`,
  `same_precedence_is_left_associative`,
  `comparison_binds_looser_than_additive`,
  `unary_binds_tighter_than_binary`, and
  `parentheses_override_precedence` (`cad-parser`'s `tests` module).
  `AICAD-045`'s printer independently re-validates this precedence
  reasoning from the opposite direction (proving no parentheses ever
  need to be *synthesized* beyond an explicit `Expr::Paren` node — see
  `printer.rs`'s own "Precedence note" and `AICAD-045.md` decision 3) —
  two independent lines of evidence for the same invariant. **PASS.**
- **Span fidelity.** Every AST node carries its own `Span`
  (`cad-ast/src/lib.rs`); exact-span assertions exist at every layer —
  `cad-lexer` (3 exact `Span::new` assertions against hand-computed
  byte offsets), `cad-ast` (`LineIndex` byte-offset-to-line/column, 4
  assertions), `cad-parser` (5 exact-span assertions, e.g.
  `parses_let_with_type_annotation`'s `Type::Named` span). `AICAD-044`
  added no new span-model changes (confirmed against `cad-ast/src/lib.rs`'s
  own doc comment on `Span`, which explicitly anticipated this task:
  "when module resolution (`AICAD-044`) needs to distinguish spans
  across files, the natural extension is a separate `SourceId` carried
  alongside a `Span`, not a change to `Span` itself" — this task
  followed that guidance exactly: `cad-compiler::loader::Module` tracks
  a file `path` at the loader level, `Span` itself stayed untouched).
  **PASS.**
- **Diagnostics.** Every diagnostic raised by this batch is a real
  `cad_diagnostics::Diagnostic` (never a bare string), per RFC-0005 §3/§5
  — verified structurally (the type system enforces this: `Parser::error`
  and `cad-compiler::loader`'s `diagnostic`/`diagnostic_without_source`
  helpers are the only diagnostic-construction paths in this batch's own
  code, both going through `Diagnostic::new`). Two diagnostic families
  are in use: `PARSE` (lexer/parser syntax errors, `E001`-`E012`, one new
  code this batch: `E012` "malformed relative-import prefix") and the
  pre-reserved `IMPORT` family (loader resolution errors: `E001`
  unreadable file, `E002` cyclic import, `I001` unresolved package
  import — the first task to actually populate this family, confirming
  RFC-0005's own family taxonomy anticipated exactly this need).
  `specs/schemas/diagnostic.schema.json` conformance is re-validated by
  `cad-diagnostics`' own 10 `schema_conformance` tests (unchanged by
  this batch — no new field/shape was needed). **PASS.**
- **Declaration/control-flow coverage.** `Item`:
  `Let`/`Const`/`Param`/`Fn`/`Struct`/`Enum`/`Part`/`Import`, all
  implemented and round-trip-formatted. `Stmt`:
  `Let`/`Var`/`Assign`/`Expr`/`If`/`For`/`While`/`Loop`/`Match`/`Return`/
  `Break`/`Continue`, all implemented and round-trip-formatted.
  `Expr`: `Literal`/`Ident`/`Unary`/`Binary`/`Paren`/`Call`/
  `MethodCall`/`Field`/`Block`/`If`/`Match`, all implemented and
  round-trip-formatted. Every one of these variants has at least one
  positive parser test (`decl_tests`/`control_flow_tests`/`import_tests`
  modules) and is exercised by the printer's own round-trip suite
  (`crates/cad-ast/tests/printer_round_trip.rs`), confirmed by direct
  cross-check against each enum's own variant list while writing this
  checkpoint. **PASS.**
- **Module-loader behavior.** `cad-compiler::loader::load_entry`
  resolves a `Relative`-path import graph depth-first in source order,
  deduplicates diamond imports (loads a shared dependency exactly once —
  `deduplicates_a_diamond_import_instead_of_reparsing`), detects both a
  two-file cycle and a single-file self-cycle without hanging
  (`detects_a_two_file_cyclic_import`, `detects_a_self_import_cycle`),
  resolves a straight 5-file-deep chain correctly (added this checkpoint,
  §6), attributes an unresolvable import to the *importing* file's own
  `import` statement (not the unreachable target), and records
  `Package`-path imports as informational/unresolved rather than either
  erroring on valid-but-not-yet-resolvable syntax or fabricating package
  resolution ahead of a package system that does not exist yet.
  10 tests total, all passing. **PASS.**
- **Parser/formatter round-trip or structural-equivalence tests.**
  `crates/cad-ast/tests/printer_round_trip.rs`'s 14 tests prove printing
  is idempotent and always reparses without new diagnostics, for every
  implemented construct individually and one program combining all of
  them; a separate golden-output test locks in the chosen canonical
  style for six representative constructs. See `AICAD-045.md` decision 2
  for why this property (not span-insensitive `Program` equality) is the
  meaningful one to test, and why a custom span-stripping equality was
  not worth writing. **PASS.**
- **Adversarial parsing tests.** Pre-existing: 19 negative/recovery
  tests in `cad-parser` (malformed numeric/unit literals, unmatched
  delimiters, invalid declarations, malformed control flow, unexpected
  tokens, and two dedicated "does not hang" regression guards from
  `AICAD-042`/`AICAD-043`) plus 5 in `cad-lexer` (invalid escape
  sequences, unterminated strings, etc.) — none touched or weakened by
  this batch. **Added this checkpoint** (§6, run fresh before signing
  off, not merely re-cited): a combined malformed-`import`-plus-garbage
  parser regression guard, confirming this batch's new
  `parse_import_path` error path participates correctly in the parser's
  existing "force progress" recovery guarantee rather than introducing
  its own hang risk; and a 5-deep straight-line relative-import chain in
  the loader, distinct from the existing diamond/cycle cases. **PASS.**

## 4. Fresh verification run at checkpoint time

```
$ git status --short
(empty)

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0)

$ cargo build --workspace --all-targets
(exit 0)

$ cargo test -p cad-ast -p cad-lexer -p cad-diagnostics -p cad-parser -p cad-compiler
test result: ok. 7 passed (cad-ast lib)
test result: ok. 14 passed (cad-ast printer_round_trip)
test result: ok. 10 passed (cad-compiler loader::tests, incl. this checkpoint's new deep-chain test)
test result: ok. 20 passed (cad-diagnostics lib)
test result: ok. 10 passed (cad-diagnostics schema_conformance)
test result: ok. 27 passed (cad-lexer)
test result: ok. 89 passed (cad-parser, incl. this checkpoint's new adversarial-import test)
Total: 177 passed, 0 failed.

$ cargo test --workspace
(every crate's suite passes, including the full native/OCCT Stage-1
Rust suite: cad-occt-bridge 84 unit + 9 adversarial_sweep + 6
concurrency_probe + 3 stage1_bracket, cad-kernel-api 23 — all
unaffected by this batch, included here only because `--workspace`
runs everything)
```

Environment: same as every prior Stage-1/Stage-2 session (Rust 1.98.1,
edition 2024, per `rust-toolchain.toml`) — reconfirmed, not assumed.

## 5. Cross-check against D5/DL-12 (determinism)

This batch is the first to introduce a `HashMap` in a code path whose
output feeds a canonical result (`cad-compiler::loader`'s
`index_by_path: HashMap<PathBuf, usize>`). Re-examined specifically for
this checkpoint: the map is used only for point lookups (`get`/`insert`),
never iterated to produce `LoadResult::modules` (a plain `Vec` built in
depth-first source-traversal order) or `LoadResult::diagnostics` (pushed
in traversal order) — so its randomized bucket order cannot leak into
either observable output. Cyclic-import detection uses an explicit
`on_stack: Vec<PathBuf>` walked with `.contains()`, not a hash-based set,
sidestepping the question entirely for that check. `loader.rs`'s own
module doc comment records this reasoning permanently, not just here.
No other Stage-2 front-end code introduces unordered iteration,
concurrency, or filesystem-enumeration dependence. **No D5 Level-1
violation found.**

## 6. Adversarial tests added during this checkpoint

Two tests added and verified passing (both with a bounded runtime,
confirming no hang) before this document was finalized — not carried
over from an earlier task's report, but new evidence gathered
specifically for this gate:

- `cad-parser::import_tests::does_not_hang_on_malformed_imports_adjacent_to_garbage`
  — `"import . ; import @@@ ; import ./ ; import ;"` (four different
  malformed-import shapes back-to-back) terminates with diagnostics, no
  hang.
- `cad-compiler::loader::tests::resolves_a_five_deep_relative_import_chain`
  — a straight `a -> b -> c -> d -> e` chain (no diamond, no cycle)
  resolves every module in order.

## 7. Known limitations (carried forward, not blocking Batch S2-04)

- No comment/doc-comment attachment to any AST node yet (`AICAD-045`'s
  own known limitation) — `///` doc comments are lexed but not consumed
  by the parser or printer. Not named by any task through `AICAD-045`;
  revisit only if a future task's scope actually requires it.
- `Package`-path imports remain unresolved by design (no package system
  exists before Stage 5+, per `docs/plan/12_PACKAGES_PLUGINS_EXTENSIONS.md`
  and `project/TASKS.yaml`) — `IMPORT-I001` is the correct, honest
  signal for this, not a gap to close now.
- The D5 concrete numeric tolerance constants (`DECISION_LOG.md#DL-12`)
  remain undetermined — unchanged since Batch S2-01, still correctly
  scoped to a future `cad-validation`/execution-determinism task, not
  this front-end batch.
- `project/TASKS.yaml`'s pre-`AICAD-038` staleness (`status: todo` on
  long-complete Stage-0/Stage-1 tasks) remains unfixed — out of every
  Stage-2 batch's own scope, noted again for the next session per prior
  batches' own handoffs.

## 8. Recommendation

**PASS — Batch S2-04 (`AICAD-046` onward) may begin.** All seven
checklist items in §3 are met, with two of them (round-trip tests,
adversarial parsing tests) directly and freshly exercised as part of
this checkpoint rather than only inherited from individual task
reports. No `project/OWNER_DECISIONS.md` item was touched or newly
required by any task in this batch, and no accidental deviation from
RFC-0001/RFC-0004/DL-1/DL-2 was found. This recommendation does not
itself constitute Stage-2 owner approval — Stage 2 as a whole still
requires the owner-recorded decision `AICAD-064` will seek, per
`project/CURRENT_STAGE.md`.
