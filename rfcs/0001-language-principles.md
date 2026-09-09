# RFC-0001: Language Principles

- Status: Draft (Stage 0)
- Stage-0 build items covered (`docs/plan/15_IMPLEMENTATION_ROADMAP.md`
  Stage 0): "language design principles", "grammar sketch".
- Owner rulings incorporated: DL-1 (surface syntax), DL-2 (mutation
  semantics), DL-4 (file extension/branding), DL-7 (compiler-intrinsic RFC
  requirement). See `project/DECISION_LOG.md`.

## 1. Summary

AICAD is a typed, compiled, Turing-complete general-purpose programming
language and execution environment for mechanical engineering
(`docs/plan/00_PRINCIPLES_AND_SCOPE.md` §1-2). This RFC freezes the
language's foundational, cross-cutting rules: what kind of thing AICAD is,
its non-negotiable invariants, its canonical surface-syntax family, its
mutation/value model, its source-file identity, and the process for adding
anything to the compiler core that could instead be a library.

Nothing in this RFC introduces new semantics beyond what `docs/plan/`
already specifies plus the owner rulings in `project/DECISION_LOG.md`; it
is a freeze, not new design.

## 2. Non-negotiable invariants (adopted verbatim)

The following, from `docs/plan/00_PRINCIPLES_AND_SCOPE.md` §3 and already
restated in `AGENTS.md`, are frozen as-is by this RFC and are binding on
every subsequent RFC, task, and package:

1. The language is Turing-complete by design.
2. High-level and low-level CAD live in one language — no separate
   "advanced language."
3. Low-level geometry is nearly kernel-complete; advanced users must not
   hit a high-level abstraction wall.
4. Raw topology access exists, is explicitly ephemeral/unsafe, and is
   epoch-bound (formalized in RFC-0002).
5. Units are in the type system, not plain untyped floats (formalized in
   RFC-0004).
6. Exact B-rep is the primary compiled geometry; meshes are views/export
   targets, never canonical design truth.
7. Source is canonical design intent; cached B-rep never replaces source
   semantics.
8. Semantic references are preferred to entity indices; ambiguity is an
   error, never an arbitrary selection (formalized in RFC-0003).
9. Determinism is the default (same source + lockfile + compiler/kernel
   versions -> equivalent geometry/validation results). The exact
   cross-platform equivalence tolerance is intentionally **not** frozen by
   this RFC — see `project/OWNER_DECISIONS.md` D5 (open).
10. Engineering assertions (requirements/tests/contracts) are executable
    and live beside design source.
11. GUI and source are two projections over one model; the GUI must not
    create irreducible hidden state.
12. Extensions are first-class.
13. The platform must remain usable by general-purpose coding models, not
    require a CAD-specialized model.

Plus the STEP-interoperability rule (`00` §9: AICAD does not compete with
STEP at the same layer) and the kernel-independence contract (`01` §8,
formalized in RFC-0002).

## 3. Product identity and file conventions (DL-4)

- Product/language name: **AICAD**.
- Canonical source file extension: **`.aicad`**.
- Canonical project manifest file name: **`aicad.toml`**.
- If a single-file packaged/archive bundle format is later needed (per
  `docs/plan/09_FILE_FORMAT_INTERCHANGE_RECONSTRUCTION.md` §2), it uses a
  distinct extension, **`.aicadpkg`**, never `.aicad` — a source file and a
  packaged bundle must always be distinguishable by extension alone.
- `CAD-IR` may continue to be used as an internal compiler/IR name
  (`docs/plan/README.md`'s "Working names") but is never product branding.

This resolves `project/OWNER_DECISIONS.md` D14 and the extension
disagreement identified in `project/reports/ORIENTATION_PASS.md` §6
(contradiction 2) between `docs/plan/02_LANGUAGE_AND_COMPILER.md` §2 and
`docs/plan/09_FILE_FORMAT_INTERCHANGE_RECONSTRUCTION.md` §1.

## 4. Canonical surface syntax family (DL-1)

- Blocks are delimited by braces (`{ }`).
- Statements are terminated by explicit semicolons (`;`).
- The language is broadly Rust/TypeScript-like in surface appearance
  (typed bindings, `fn`, `struct`, `enum`, `match`, `for`/`while`/`loop`,
  `if`/`else`) without attempting source compatibility with either
  language — AICAD keywords and grammar productions are defined on their
  own terms in `specs/language/grammar.ebnf`, not derived by reference to
  Rust's or TypeScript's grammars.
- Indentation is formatting only; it carries no syntactic meaning. A
  correctly-brace-and-semicolon-delimited program is valid AICAD
  regardless of its indentation, and `cargo fmt`-style reformatting must
  never change program meaning.
- Automatic semicolon insertion (ASI) is **not** part of the initial
  language. A missing required semicolon is a parse error, not a
  contextually inferred statement boundary. This removes a category of
  parser/formatter/codegen ambiguity that matters especially for AI-authored
  source (`docs/plan/00_PRINCIPLES_AND_SCOPE.md` §4.2, §10).

See §7 for a grammar sketch illustrating these rules concretely.

## 5. Mutation semantics: functional core, method syntax as sugar (DL-2)

- AICAD's semantic core is **functional and value-oriented**. Modeling/
  geometry operations consume semantic values and produce new semantic
  values; they do not mutate their inputs in place.
- Method/builder syntax (`body.cut(hole)`) is ergonomic sugar over an
  ordinary functional call (`cut(body, hole)`). It never implies in-place
  mutation of the receiver.
- Consequently, `body.cut(hole);` as a standalone statement does **not**
  rebind `body` and does not mutate it; its result, if not bound, is
  discarded like any other unused expression result. A workflow that
  intends to update `body` must rebind explicitly:

  ```aicad
  var body = base;
  body = body.cut(hole);       // explicit rebind — required
  ```

  or use chained/builder syntax whose desugared HIR is still the
  functional rebind above:

  ```aicad
  var body = base
      .cut(hole_a)
      .cut(hole_b);            // desugars to nested functional calls,
                                // then one rebind of `body`
  ```

- `crates/cad-hir` and the Geometry IR (`crates/cad-geometry-api`) expose
  only the functional/SSA form. Builder/method surface syntax is fully
  desugared by the parser/HIR-lowering phase (`docs/plan/02_LANGUAGE_AND_COMPILER.md`
  §17 phases 7-8) and never reaches those layers as a distinct concept.
- This keeps the feature DAG (RFC-0003; `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md`
  §9) exact: every feature node's inputs and outputs are ordinary values,
  never aliased in-place mutations that would need separate ownership
  tracking.

## 6. Compiler intrinsics require an RFC (DL-7)

- Adding a new compiler intrinsic (any construct implemented in the
  compiler/runtime itself rather than as ordinary AICAD source, a standard
  package, or an existing kernel API operation exposed through existing
  language mechanisms) requires its own RFC.
- That RFC must demonstrate the capability cannot reasonably be
  implemented as:
  1. ordinary AICAD source;
  2. a standard package (`std.*`, per `docs/plan/12_PACKAGES_PLUGINS_EXTENSIONS.md`
     §16);
  3. a kernel API operation exposed through existing language mechanisms.
- Each intrinsic's RFC must document: semantics, type rules, deterministic
  behavior, IR lowering, and specifically why a library solution is
  inadequate.
- No intrinsic may be introduced solely as an implementation convenience.
- This operationalizes the qualitative test in
  `docs/plan/00_PRINCIPLES_AND_SCOPE.md` §10 ("could this be a library
  instead of a compiler intrinsic? If yes, prefer the library") and the
  `AGENTS.md` non-negotiable/escalation trigger of the same shape.

## 7. Grammar sketch

This is a Stage-0 sketch, not the formal grammar (`specs/language/grammar.ebnf`
grows into the authoritative artifact starting at Stage 2, task AICAD-039+;
see `docs/plan/02_LANGUAGE_AND_COMPILER.md` §17-19). It exists to make §4-5
concrete and to seed `specs/language/grammar.ebnf`.

**Patch (independent Stage-0 review, `project/reports/reviews/STAGE0-INDEPENDENT-REVIEW.md`):**
the grammar sketch as originally drafted defined `if`/`match` only as
*statements* (`if_stmt`, `match_stmt`), with no `if`/`match` alternative
under `expression`, and left `block_expr` referenced but undefined. This
silently conflicted with existing precedent already used elsewhere in the
frozen material — `docs/plan/07_ASSEMBLIES_KINEMATICS_CONFIGURATIONS.md`
§10's own canonical example (`let wall = match Product.material { Plastic
=> 3mm, Aluminum => 2mm, };`) and the Stage-0 paper example's `let wall =
if Product.motor == NEMA17 { 3mm } else { 4mm };`
(`examples/assemblies/stage0_paper_example.aicad`) both require `if`/`match`
to appear in expression position, which the original grammar sketch never
actually granted. The `if_expr`/`match_expr`/`block_expr` productions added
below close that gap. This is a documentation/completeness fix, not a new
architecture decision: it follows the "broadly Rust-like" framing already
approved by DL-1 (Rust's `if`/`match` are expressions; this only makes that
explicit) and does not choose among any open `project/OWNER_DECISIONS.md`
item.

```ebnf
(* AICAD grammar sketch — RFC-0001, Stage 0. Informal; not exhaustive. *)

program        = { item } ;
item           = let_decl | const_decl | param_decl
               | fn_decl | struct_decl | enum_decl | interface_decl
               | part_decl | assembly_decl | requirement_decl | test_decl
               | import_decl ;

block          = "{" , { statement } , "}" ;
statement      = let_stmt | var_stmt | assign_stmt | expr_stmt
               | if_stmt | for_stmt | while_stmt | loop_stmt
               | match_stmt | return_stmt | break_stmt | continue_stmt ;

let_stmt       = "let" , identifier , [ ":" , type ] , "=" , expression , ";" ;
var_stmt       = "var" , identifier , [ ":" , type ] , "=" , expression , ";" ;
assign_stmt    = identifier , "=" , expression , ";" ;   (* explicit rebind, DL-2 *)
expr_stmt      = expression , ";" ;                       (* result discarded if unused *)

if_stmt        = "if" , expression , block , [ "else" , ( block | if_stmt ) ] ;
for_stmt       = "for" , identifier , "in" , expression , block ;
while_stmt     = "while" , expression , block ;
loop_stmt      = "loop" , block ;
match_stmt     = "match" , expression , "{" , { match_arm } , "}" ;
match_arm      = pattern , "=>" , ( expression , "," | block ) ;
return_stmt    = "return" , [ expression ] , ";" ;
break_stmt     = "break" , ";" ;
continue_stmt  = "continue" , ";" ;

fn_decl        = [ "pure" ] , "fn" , identifier , "(" , [ params ] , ")" ,
                 [ "->" , type ] , block ;
params         = param , { "," , param } ;
param          = identifier , ":" , type , [ "=" , expression ] ;

expression     = call_expr | method_call_expr | binary_expr | literal
               | identifier | "(" , expression , ")" | block_expr
               | if_expr | match_expr ;
block_expr     = "{" , { statement } , [ expression ] , "}" ;
                 (* an optional trailing, non-semicolon-terminated
                    expression is the block's value; a block with no
                    trailing expression has no value and may only be used
                    where a value is not required *)
if_expr        = "if" , expression , block_expr ,
                 "else" , ( block_expr | if_expr ) ;
                 (* the `else` arm is REQUIRED in expression position so
                    both arms produce a value of a unifiable type; an
                    `if` with no `else` remains valid only as if_stmt
                    (statement position, no value) *)
match_expr     = "match" , expression , "{" , { match_arm } , "}" ;
                 (* same arm shape as match_stmt (§ above); used in
                    expression position when every arm yields a value of a
                    unifiable type *)
call_expr      = identifier , "(" , [ args ] , ")" ;                (* cut(body, hole) *)
method_call_expr
               = expression , "." , identifier , "(" , [ args ] , ")" ; (* body.cut(hole) — sugar, DL-2 *)
args           = ( expression | named_arg ) , { "," , ( expression | named_arg ) } ;
named_arg      = identifier , "=" , expression ;

(* No indentation-sensitive productions exist anywhere in this grammar (§4).
   No production infers a statement terminator (§4: no ASI) — a missing ";"
   is always a parse error. *)
```

## 8. Alternatives considered

- **Python-style significant indentation** — rejected by DL-1; complicates
  deeply nested geometry/query blocks and machine-generated diffs, and
  general-purpose coding models have far more brace/semicolon-language
  training exposure than indentation-only-language exposure among
  domain-specific languages.
- **Two independently meaningful mutation styles** (functional calls and
  true in-place mutation both fully supported with distinct semantics) —
  rejected by DL-2; would require the feature DAG to track aliasing and
  in-place effects, undermining the exact/replayable incremental-build
  model in `docs/plan/01_SYSTEM_ARCHITECTURE.md` §7.
- **No RFC gate on compiler intrinsics** (informal guideline only) —
  rejected by DL-7; provides no enforcement as the standard library grows
  through Stage 3+.
- **Overloading `.aicad` for both source and bundle** — rejected by DL-4;
  reintroduces the exact ambiguity flagged in
  `docs/plan/02_LANGUAGE_AND_COMPILER.md` §2.

## 9. Open questions (intentionally not resolved here)

- `project/OWNER_DECISIONS.md` D5 (canonical-state/determinism-equivalence
  contract): this RFC states determinism as a goal (§2 rule 9) but does not
  define the cross-platform comparison tolerance. Needed before Stage 2's
  deterministic evaluator matures.
- `project/OWNER_DECISIONS.md` D3 (sketch entity/object binding model) is
  out of scope for this RFC — it belongs to the high-level modeling design
  work in Stage 3, not to core language principles.

## 10. Impact

- `specs/language/grammar.ebnf` is seeded from §7 (this task).
- `crates/cad-lexer`, `crates/cad-parser`, `crates/cad-ast`: implement §4
  (no ASI, brace/semicolon delimiting) starting Stage 2 (AICAD-039+).
- `crates/cad-hir`: implement the method-call desugaring in §5 (AICAD-051).
- `crates/cad-artifact`, `crates/cad-cli`: use `.aicad`/`aicad.toml`/
  `.aicadpkg` naming (§3) once artifact/CLI work begins (Stage 2 CLI
  baseline, Stage 8 artifact hardening).
- Every future RFC proposing a compiler intrinsic must follow §6's process.
