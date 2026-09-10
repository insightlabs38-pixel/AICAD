# AICAD-042 — Implement declarations: let/const/param/fn/struct/enum/part

## Objective
Implement the six declaration ("item") forms this task is scoped to, plus
the non-control-flow statement/block infrastructure they need for a
working `fn` body, per `project/TASKS.yaml` AICAD-042 (Stage-2 batch
S2-02, second task, depends on AICAD-041).

## Base commit
`be878df` (AICAD-041, this session).

## Plan references read
Same batch-wide set as `AICAD-041`'s report (`AGENTS.md`, `CLAUDE.md`,
`project/CURRENT_STAGE.md`, `project/DECISION_LOG.md` DL-1/DL-2,
`project/OWNER_DECISIONS.md`, `specs/language/grammar.ebnf`,
`rfcs/0001-language-principles.md`, `rfcs/0004-units-type-system.md`,
`project/SESSION_HANDOFF.md`), plus this task's own `plan_references`:
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §4 (declaration forms) and §6
(function example), `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §9-10
(collections/generics, structs/enums), `docs/plan/22_REPOSITORY_WORK_PACKAGES.md`
§3 (WP-02). Re-read `examples/assemblies/stage0_paper_example.aicad` and
its own header (explicitly "NOT compilable source") to confirm which of
its constructs are and aren't evidence for this task — see "Decisions"
below.

## Grammar gaps this task closes
`specs/language/grammar.ebnf`'s `item` production names `let_decl` /
`const_decl` / `param_decl` / `struct_decl` / `enum_decl` / `part_decl` (and
five others out of this task's scope) but never defines any of them — only
`fn_decl` (via `params`/`param`) already has a real production. It also
references `type` from `let_stmt`/`var_stmt`/`param` without ever defining
it. This task defines exactly the productions needed for its six forms
(see the doc comments in `crates/cad-ast/src/{ty,item}.rs` for the full
EBNF added and the specific evidence each shape is drawn from — worked
examples in `docs/plan/02`/`03`, plus the enum-variant pattern-matching
shapes already used in `docs/plan/02` §9). This is the same category of
gap `AICAD-041` closed for `binary_expr`/`literal`, not new architecture.

`type = identifier , [ "<" , type , { "," , type } , ">" ] ;` — covers
every type spelling evidenced for Stage-2 declaration syntax (`Length`,
`Optional<Length>`, `List<Point2>`, `Map<K,V>`); deliberately excludes
qualified/path types, const-generic array types, and bounded generic
parameters (`fn mount<T: MotorMount>`), none of which are needed by this
task's six forms and none of which have grammar evidence anywhere.

## Implementation

### `crates/cad-ast`
- `src/ty.rs`: `Type { name: Ident, args: Vec<Type>, span }`.
- `src/item.rs`: `Item` (`Let`/`Const`/`Param`/`Fn`/`Struct`/`Enum`/`Part`)
  with an `Item::span()` helper; `LetDecl`/`ConstDecl`/`ParamDecl`;
  `Param` (fn-parameter shape, distinct type from `ParamDecl` even though
  both share `name`/`ty`/`default`/`span`); `FnDecl`; `FieldDecl`;
  `StructDecl`; `EnumVariantKind` (`Unit`/`Tuple(Vec<Type>)`/
  `Struct(Vec<FieldDecl>)`); `EnumVariant`; `EnumDecl`; `PartMember`
  (the same six-minus-`Part` kinds); `PartDecl`.
- `src/stmt.rs`: `Block` (grammar `block`, no trailing value — used by
  `fn_decl`'s body); `Stmt` (`Let`/`Var`/`Assign`/`Expr` only — this
  task's non-control-flow subset) with a `Stmt::span()` helper;
  `LetStmt`/`VarStmt`/`AssignStmt`/`ExprStmt`.
- `src/lib.rs`: wires in the three new modules and re-exports their types.

### `crates/cad-parser`
- `src/ty.rs`: `parse_type` (recursive for generic arguments; closing a
  doubly-nested generic like `List<Optional<Length>>` needs no special
  handling since the lexer has no `>>` token — it's just two consecutive
  `Gt` tokens, each consumed by its own `expect(Gt)` call) and
  `parse_optional_type_annotation`.
- `src/item.rs`: `parse_item` (dispatches on the leading keyword to one of
  the six forms), `parse_part_member` (same dispatch minus `part`),
  `parse_let_decl`/`parse_const_decl`/`parse_param_decl`/`parse_fn_param`/
  `parse_fn_params`/`parse_fn_decl`/`parse_field_decl`/
  `parse_struct_decl`/`parse_enum_variant`/`parse_enum_decl`/
  `parse_part_decl`.
- `src/stmt.rs`: `parse_block`, `parse_statement` (dispatches `let`/`var`
  keywords, else delegates to `parse_assign_or_expr_stmt`),
  `parse_let_stmt`/`parse_var_stmt`/`parse_assign_or_expr_stmt` (the
  assign-vs-expr-statement disambiguation reuses `AICAD-041`'s exact
  two-token-lookahead technique: bare `Ident` immediately followed by a
  bare `Eq`, never `EqEq`).
- `src/lib.rs`: three new public entry points (`parse_item`, `parse_block`,
  `parse_statement`), each requiring full token consumption like
  `parse_expr` already does; `at_keyword`/`expect_keyword` cursor helpers
  added to the shared `Parser` core. No new diagnostic codes were needed —
  every malformed-declaration case is already covered by the existing
  `PARSE-E005 UNEXPECTED_TOKEN` / `PARSE-E006 UNEXPECTED_EOF`.
- Cross-module method visibility: several `AICAD-041` helper methods
  (`parse_expression`, `parse_ident`) needed to go from private-to-their-
  defining-module to `pub(crate)` so the new sibling modules (`ty`/`item`/
  `stmt`) can call them — Rust's default item visibility is "the defining
  module and its descendants," and these new modules are siblings of
  `expr`, not descendants of it. This is a visibility-only change; no
  behavior of `AICAD-041`'s code changed (its own 35 tests are unaffected
  and still pass).

## Decisions made and why
1. **`part_decl`'s body is restricted to `part_member`** — the same six
   item kinds this task defines (`let`/`const`/`param`/`fn`/`struct`/
   `enum`), explicitly excluding nested `part`, and excluding the
   `constraint`/`expose`/`sketch`/`query`/`test`/`unsafe geometry`/
   `metadata`/loop-and-conditional-directly-in-body constructs
   `examples/assemblies/stage0_paper_example.aicad` shows. That file's own
   header states it is "NOT compilable source," and several of the
   constructs it uses are explicitly flagged there as depending on the
   still-open `OWNER_DECISIONS.md` D3 (sketch entity/object model) or on
   later-stage concepts with no grammar of their own — copying them now
   would be inventing syntax rather than filling an evidenced gap, and
   loops/conditionals are literally `AICAD-043`'s own next task. No open
   `OWNER_DECISIONS.md` item was touched.
2. **Enum struct-variant fields reuse `field_decl`'s semicolon-terminated
   shape** (`identifier ":" type ";"`), not a second, comma-separated
   field-list syntax. `docs/plan/02_LANGUAGE_AND_COMPILER.md` §9's own
   `Cylinder { radius, axis }` is a *pattern* (no types), not a
   declaration, so it's evidence for `AICAD-043`'s pattern grammar, not
   for this task's variant-declaration syntax; picking one consistent
   field-list convention across `struct_decl` and enum struct-variants
   avoids introducing an unevidenced second syntax.
3. **`Param` (fn parameter) and `ParamDecl` (top-level `param` item) are
   distinct types** even though both carry identical `name`/`ty`/
   `default`/`span` fields — the grammar's own `param` production (used
   inside `fn_decl`'s parameter list, no `;`) and the top-level `param
   ...;` item are syntactically different productions that happen to
   share a shape; keeping them as separate AST types avoids a later task
   having to guess whether a shared type's semantics apply identically in
   both positions.
4. **No new diagnostic codes.** Every malformed-declaration case
   encountered while writing this task's adversarial tests (missing `=`,
   missing type, missing semicolon, missing comma, unmatched
   brace/paren, wrong leading keyword) is already exactly
   `UNEXPECTED_TOKEN`/`UNEXPECTED_EOF` — `AICAD-041`'s two generic codes
   were general enough that no declaration-specific code category emerged.
5. **Method visibility widened to `pub(crate)` where needed for cross-
   module calls** (see Implementation above) — a mechanical consequence
   of splitting the parser into per-construct modules, not a change to
   any parsing behavior.

No escalation condition was triggered: this only fills named-but-undefined
grammar productions with the reading already licensed by DL-1 plus
directly-evidenced worked examples, introduces no kernel-type exposure,
and does not touch any open `OWNER_DECISIONS.md` item (D3/D10/D11/D12/D15
remain exactly as they were — in particular, D3 was read and explicitly
*not* touched by restricting `part_decl`'s body, per Decision 1).

## Files changed
- Added: `crates/cad-ast/src/ty.rs`, `crates/cad-ast/src/item.rs`,
  `crates/cad-ast/src/stmt.rs`, `crates/cad-parser/src/ty.rs`,
  `crates/cad-parser/src/item.rs`, `crates/cad-parser/src/stmt.rs`,
  `project/reports/AICAD-042.md`.
- Modified: `crates/cad-ast/src/lib.rs` (wires in the three new modules),
  `crates/cad-parser/src/lib.rs` (three new module declarations, three new
  public entry points, `at_keyword`/`expect_keyword` helpers),
  `crates/cad-parser/src/expr.rs` (`parse_ident` widened to `pub(crate)`),
  `project/TASKS.yaml` (AICAD-042 `status: todo` -> `done`).

## Verification (exact commands/results)
```
$ cargo test -p cad-ast -p cad-lexer -p cad-diagnostics -p cad-parser
cad-ast:          running 7 tests  ... ok. 7 passed; 0 failed
cad-diagnostics:  running 20 tests ... ok. 20 passed; 0 failed
                  running 10 tests ... ok. 10 passed; 0 failed (schema_conformance.rs)
cad-lexer:        running 27 tests ... ok. 27 passed; 0 failed
cad-parser:       running 76 tests ... ok. 76 passed; 0 failed (35 from
                  AICAD-041 unchanged + 41 new this task)

$ cargo fmt --all -- --check
(exit code 0, no output)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(zero warnings)

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo test --workspace
(every crate: ok, 0 failed; pre-existing suites — cad-ast 7,
 cad-diagnostics 20+10, cad-lexer 27, cad-kernel-api 23, cad-occt-bridge
 84+9+6+3 — all still passing; cad-parser's 76 include this task's 41 new
 tests)
```

## Tests added (41 new, across `crates/cad-parser/src/{item,stmt}.rs`)
Positive (`item.rs`, 20): `let`/`const` with and without a type
annotation; `param` with and without a default; `fn` with params/return
type, `pure fn`, zero-param/zero-return-type `fn`, a parameter default;
a plain generic annotation and a doubly-nested one (`List<Optional<Length>>`,
exercising the no-`>>`-token case); a struct with fields and an empty
struct; a unit-only enum, a trailing-comma enum, a tuple-variant enum, a
struct-variant enum, an empty enum; a part with one of each of its six
member kinds, and an empty part.

Positive (`stmt.rs`, 8): `let`/`var` (with a type), `assign` (RFC-0001 §5's
own `body = body.cut(hole);` rebind example), `expr_stmt` (the paired
"`body.cut(hole);` does NOT rebind" case), the `Eq`-vs-`EqEq` lookahead
regression for statements, a multi-statement block, an empty block, and a
cross-check that a `fn` body block round-trips identically through
`parse_item` and the standalone `parse_block` entry point.

Adversarial/negative (13): a `param` missing its required type; a struct
field missing its semicolon; a part body containing a bare `for` (rejected
cleanly, not silently misparsed, since control flow doesn't exist yet);
an unrelated leading keyword (`if`) rejected as not-a-declaration; a `let`
missing `=`; unterminated `fn` parameter list and struct body (both
`UNEXPECTED_EOF`); an enum variant list missing its comma; trailing
garbage after a complete item; a statement missing its semicolon; an
assignment with no value; an unclosed block; `var` with no binding name.

## Known limitations / follow-up
- `Stmt`/`ExprKind` still lack every control-flow variant
  (`If`/`For`/`While`/`Loop`/`Match`/`Return`/`Break`/`Continue`) — by
  design, `AICAD-043` adds them next in this same batch.
- `interface_decl`/`assembly_decl`/`requirement_decl`/`test_decl`/
  `import_decl` remain entirely unparsed (not even a stub) — none has an
  assigned Stage-2 task yet except `import_decl` (`AICAD-044`, next
  batch, explicitly out of scope for this session).
- No error recovery (carried over from `AICAD-041`) — every entry point
  still stops at the first diagnostic.
- Generic type arguments are parsed but never checked for arity/kind
  against any registry — that's a type-checking-phase concern
  (`AICAD-047`-`049`+), not this parser's job.
