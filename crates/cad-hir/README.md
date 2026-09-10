# cad-hir

Typed HIR and AST -> HIR lowering, per the compiler IR-layer strategy in
`docs/plan/01_SYSTEM_ARCHITECTURE.md` §5 (Source AST -> Typed HIR ->
Engineering HIR -> Feature IR -> Geometry IR -> Kernel call graph -> B-rep).

## Status (`AICAD-051`)

Implemented (the typed-HIR data model and AST -> HIR lowering *skeleton*
— see `src/lower.rs`'s own module doc comment "Scope boundary" for the
full accounting of what is deliberately deferred to `AICAD-052`):

- `src/ids.rs`: `BindingId`/`BindingKind`/`Binding` — explicit lexical
  binding identity (`AGENTS.md` HIR invariant), minted fresh per
  declaration by lowering itself (not consumed from `cad_compiler::
  binder`, which cannot be a dependency here — see `src/lower.rs`'s
  module doc comment "Relationship to `cad_compiler::binder`").
- `src/types.rs`: `HirType` (reuses `cad_units::OperandType`) and
  `HirTypeRef` (an unresolved syntactic type reference).
- `src/hir.rs`: the typed HIR node types (`HirProgram`/`HirItem`/
  `HirStmt`/`HirExpr`/...) — see its own module doc comment for why these
  are distinct from, not aliases of, `cad_ast`'s node types, and how
  control flow gets unambiguous value semantics.
- `src/lower.rs`: `lower_program`/`LowerResult` — the lowering pass
  itself. Desugars `cad_ast::Expr::MethodCall` into ordinary functional
  call shape (`AGENTS.md` HIR invariant); resolves a unit-suffixed
  numeric literal's `HirType` when `cad_units::lookup_any` finds exactly
  one matching dimension (e.g. `5mm` -> `Length`), leaving it unresolved
  for an unknown or ambiguous suffix (`5Pa`, matching both `Pressure` and
  `Stress`) rather than guessing, per `AGENTS.md`'s "ambiguity is an
  error, never an arbitrary selection". 31 tests.

## Status (`AICAD-052`)

`src/typeck.rs`: `check_program`/`TypeCheckResult` — the type checker.
Fills in `lower.rs`'s own "Scope boundary": numeric-literal-type
(`Int`/`Float`) defaulting, and full type checking for `let`/`const`/
`param`/`var` bindings, function declarations, function calls (arity,
parameter types, return type), and literal expressions. Every dimensional
arithmetic/comparison/negation rule is delegated to `cad_units`
(`check_binary_arithmetic`/`check_comparison`/`check_unary_neg`), never
re-derived; an ambiguous or unknown unit-suffixed literal can now resolve
via a surrounding expected type (an annotation, parameter type, or
function return type) when that context names a matching dimension,
still reported rather than guessed at otherwise. See `src/typeck.rs`'s
own module doc comment for the full design (why no scope stack is needed,
the two-pass signature-then-body structure, and exactly where the
052/053 boundary falls). 44 tests.

Not yet implemented here: struct/enum field and variant typing (struct-
literal construction via call syntax, field access, enum-variant
construction/matching — `AICAD-053`); method-call (`receiver.method(...)`)
target resolution (no method/interface-implementation declaration syntax
exists anywhere in the language yet, and `HirCallee::Method` has no
`binding` field to resolve into); Engineering HIR (the next IR layer
down, `docs/plan/01_SYSTEM_ARCHITECTURE.md` §5).

Plan references: `docs/plan/01_SYSTEM_ARCHITECTURE.md` §5;
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §17 (phases 4 and 7);
`docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §15;
`rfcs/0004-units-type-system.md` §8-9.
