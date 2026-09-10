# AICAD-050: Implement name binding/scopes/symbol table

## Objective

Implement name binding per `project/TASKS.yaml` AICAD-050 (Stage-2 batch
S2-05, second/final task) — `docs/plan/02_LANGUAGE_AND_COMPILER.md` §17
phase 3 ("Name binding"), immediately after phase 2
(`AICAD-044`'s module-loader). Completes Batch S2-05 (`049 -> 050`); per
the campaign brief, `AICAD-051` ("typed HIR and AST-to-HIR lowering") must
not start in this same invocation.

## Base commit

`332a6c9` (this session's own `AICAD-049` commit).

## Plan references read

`docs/plan/02_LANGUAGE_AND_COMPILER.md` §17 (the 17-phase pipeline list,
confirming binding is phase 3, right after module/import resolution and
before type/dimensional checking); `crates/cad-ast/src/item.rs`'s own doc
comments (`Stmt::Assign`: "Whether `name` actually refers to a `var` (not
a `let`) is a binding-phase (`AICAD-050`) concern, not the parser's";
`Item::Import`: "Resolving `names` against the target module's actual
exported symbols is name-binding's job (`AICAD-050`+)"); `crates/cad-ast/
src/expr.rs`'s `Pattern` doc comment ("disambiguating [a bound name vs. an
enum-variant-shaped name] is a binding-phase concern, not the parser's");
`project/reports/AICAD-044.md`'s "Known limitations" (explicitly defers
"symbol-level resolution" to "`AICAD-050`+"); `crates/cad-compiler/src/
loader.rs` (read in full for its established `Diagnostic`-construction
and depth-first/deterministic-traversal conventions, reused here).

## Implementation

- `crates/cad-compiler/src/binder.rs` (new): the whole task.
  - `SymbolKind` (`Let`/`Var`/`Const`/`Param`/`Fn`/`Struct`/`Enum`/
    `EnumVariant{enum_name}`/`Part`/`Import`/`ForLoopVar`/`MatchBinding`)
    and `Symbol { name, kind, span }`.
  - `bind_program(program, file, source) -> BindResult`: a two-pass,
    depth-first walk. At every nesting level (module, and each `part`'s
    own item list), pass 1 declares every sibling item's name into the
    current scope *before* pass 2 checks any of their bodies — this is
    what makes forward references and mutual recursion between sibling
    functions/parts work regardless of source order.
  - Full expression/statement traversal: `Expr::Ident`/`Call` callees are
    resolved against the active scope chain (`UNDEFINED_NAME` if not
    found); `Expr::MethodCall`'s `method` and `Expr::Field`'s `field` are
    walked past (into their `receiver`) but never themselves checked as
    scope names (see "Decisions" #3); every block-introducing construct
    (`fn` body, `if`/`while`/`loop`/`for` bodies, `match` arms, block
    expressions) pushes/pops its own lexical scope.
  - `Stmt::Assign` resolves its target and requires `SymbolKind::Var`
    (`ASSIGN_TO_IMMUTABLE` otherwise, `UNDEFINED_NAME` if the target
    doesn't resolve at all — checked as two distinct outcomes, see tests).
  - `Pattern::Ident` in a `match` arm resolves against the active scope
    first: if it names an already-declared `EnumVariant`, the arm matches
    that variant (no new binding); otherwise the identifier becomes a
    fresh `MatchBinding` scoped to just that arm (see "Decisions" #1).
  - Diagnostics: `TYPE-E401` (`UNDEFINED_NAME`), `TYPE-E402`
    (`DUPLICATE_BINDING`, message names the original declaration's
    line/column), `TYPE-E403` (`ASSIGN_TO_IMMUTABLE`) — see "Decisions"
    #4 for the family choice.
- `crates/cad-compiler/src/lib.rs`: adds `pub mod binder;` and extends the
  module doc comment.
- `crates/cad-compiler/README.md`: adds a "Status" section for both
  `AICAD-044` and this task.

No third-party dependency added; no new intra-workspace dependency added
(`binder` uses only `cad-ast`/`cad-diagnostics`, both already
dependencies; `cad-parser` — also already a dependency — is used only by
this module's own tests, to parse test fixtures the same way
`crate::loader` does).

## Decisions made and why

1. **Enum variants are declared as ordinary scoped symbols
   (`SymbolKind::EnumVariant`), not tracked in a separate global table.**
   `cad_ast::expr::Pattern`'s own doc comment assigns this exact
   disambiguation to name-binding. Reusing the same scope-stack mechanism
   (rather than a second, parallel lookup structure) means variant
   visibility automatically follows ordinary lexical scoping — a variant
   declared inside a `part` is invisible outside it, exactly like any
   other name declared there — with no special-casing, and is exercised
   by `part_scoped_enum_variant_is_not_visible_outside_the_part`.
2. **Only *selective* imports (`import ...::{Name}`) bind a name; whole-
   module imports (`import path;`) bind nothing.** `Item::Import`'s doc
   comment says selective-import resolution is binding's job "AICAD-050+"
   — the "+" (not just "="), together with `AICAD-044.md`'s own "Known
   limitations" wording, signals this may legitimately extend past this
   one task. No frozen grammar/RFC material anywhere shows how a
   whole-module import's members would later be referenced (no `module.
   member` qualified-access syntax exists), so inventing binding
   semantics for it now would be exactly the speculative syntax
   `AGENTS.md`'s "No speculative future work" warns against; a selective
   import's names, by contrast, are directly evidenced ("a selective
   import of just those symbols") and safe to bind. This task does not
   attempt to verify a selective import's names against the target
   file's actual exports — see "Known limitations".
3. **`Expr::MethodCall`'s `method` and `Expr::Field`'s `field` are never
   checked as scope-bound names.** Both are resolved against the
   receiver's *type* (its available methods/fields), not against a
   lexical scope — the type checker's job (`AICAD-052`+), not name
   binding's. Binding still walks into `receiver`/`args` (any identifiers
   *there* must resolve), just not `method`/`field` themselves;
   `method_name_is_never_checked_as_a_scope_name`/
   `field_name_is_never_checked_as_a_scope_name` pin this.
4. **Diagnostics use the `TYPE` family (`TYPE-E401`/`402`/`403`), not a
   new family.** `cad_diagnostics::DIAGNOSTIC_FAMILIES` is RFC-0005 §2's
   already-frozen taxonomy and has no dedicated "name binding" bucket;
   adding one would be changing already-frozen diagnostic schema, an
   `AGENTS.md` escalation trigger this task has no reason to hit. Name
   binding is squarely part of RFC-0005's broader semantic-analysis
   phase, and `TYPE` is the closest existing bucket for exactly that
   (`IMPORT`, by contrast, is `crate::loader`'s own narrower module-
   resolution family, not a fit for value-name/mutability errors). Codes
   401-403 were unused by any other crate at the time of writing (checked
   via `grep -rn "TYPE-" crates/ docs/ rfcs/ specs/ project/`); like every
   `cad-diagnostics` code, these remain provisional pending D10.
5. **Cross-module binding (wiring this into `crate::loader::LoadResult`)
   is explicitly not attempted.** `crate::loader::Module` retains only
   `{path, program}` — no raw source text — so building accurate
   line/column diagnostics for a loaded (non-entry) module would need
   either re-reading its file from disk a second time or reworking
   `AICAD-044`'s already-completed `Module` struct to retain `source`.
   Given `AICAD-044.md`'s own note that symbol-level resolution is
   "AICAD-050+" work (not necessarily complete *at* 050), and that this
   task's stated acceptance criteria says nothing about multi-file
   resolution, extending a previous task's completed public struct was
   judged out of this task's minimal scope; `bind_program` instead takes
   one already-parsed `Program` plus its own `file`/`source`, mirroring
   `cad_parser::parse_program`'s own per-file shape exactly. Flagged as a
   known limitation/likely follow-up, not silently dropped.
6. **No escalation triggered.** This task adds no new public syntax
   (grammar is unchanged), makes no typed-unit/dimension changes, and its
   one real design call (enum-variant-vs-binding disambiguation) was
   already assigned to this exact task by `cad_ast::expr::Pattern`'s own
   doc comment, not invented here; no existing gate/test was weakened.

## Files changed

- Added: `crates/cad-compiler/src/binder.rs`.
- Modified: `crates/cad-compiler/src/lib.rs`, `crates/cad-compiler/README.md`.
- Modified: `project/TASKS.yaml` (AICAD-050 `status: done`).
- Added: `project/reports/AICAD-050.md` (this report).

## Verification (exact commands/results)

```
$ cargo test -p cad-compiler
running 43 tests
test binder::tests::* (33 tests) ... ok
test loader::tests::* (10 tests) ... ok
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo fmt --all -- --check
(exit code 0, no output)

$ cargo clippy -p cad-compiler --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(zero warnings)

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo test --workspace
(61 test binaries; every one `test result: ok`; exit code 0)
```

## Tests/regressions

33 new tests in `binder.rs` (43 total in `cad-compiler`, combined with
`AICAD-044`'s 10 `loader` tests): forward reference/mutual recursion
between sibling top-level and `part`-nested functions; undefined-name
detection for a bare identifier and a call callee; confirmation that
method/field names are never checked as scope names; duplicate-binding
detection for top-level items, function parameters, same-block `let`s,
and enum variants; `let`-shadowing a parameter and shadowing across a
nested (`if`-body) scope, both allowed; the full `Stmt::Assign` mutability
matrix (`var` allowed, `let`/`param` rejected as `ASSIGN_TO_IMMUTABLE`,
an actually-undefined target reported as `UNDEFINED_NAME` instead);
`for`/`while`/`loop`/`if`/`if`-`else`-`if`-chain scoping (each branch's
own bindings do not leak to sibling branches or past the construct); the
full enum-variant-vs-fresh-binding `match`-pattern matrix (a
variant-shaped identifier matches without introducing a binding, a
non-variant identifier becomes a fresh per-arm binding, that binding does
not leak outside its arm, and it may shadow an outer name); a `part`-
scoped enum variant being invisible outside the `part`; selective-import
name usability vs. whole-module-import binding nothing; a `part` body
seeing enclosing module-scope names; and diagnostic shape (source span
attached, duplicate-binding message names the original declaration's
line). No regressions found or introduced — `loader.rs`'s own 10
pre-existing tests are untouched and still pass.

## Known limitations

- Not wired to `crate::loader::LoadResult` — binds one already-parsed
  program at a time (see "Decisions" #5); a selective import's names are
  bound into scope but never checked against the target file's actual
  top-level items.
- No type-name (`cad_ast::item::Type`) resolution — a distinct namespace
  needing `cad-types`/`cad-units`, left for the type checker (`AICAD-052`).
- No struct-field duplicate-name checking — `project/TASKS.yaml` assigns
  "structs/enums field and variant typing" to `AICAD-053` explicitly.
- No detection of circular top-level `const`/`let` value dependencies
  (`const A = B; const B = A;` binds fine — a compile-time-evaluation
  concern, `docs/plan/02` §17 phase 6, not phase 3's).
- The `TYPE-E4##` codes this module raises are provisional, like every
  other `cad-diagnostics` code, pending D10.

## Unresolved questions

None requiring further owner escalation. Batch S2-05 (`AICAD-049` ->
`AICAD-050`) is now complete. Per the campaign's fixed batch order,
`AICAD-051` ("Create typed HIR and AST-to-HIR lowering") is Batch S2-06's
sole task and must not begin in this same invocation; `project/
SESSION_HANDOFF.md` records this explicitly for the next invocation.
