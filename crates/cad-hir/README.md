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

Not yet implemented here: general type checking/inference (numeric-
literal-type defaulting, propagating types through arbitrary expressions,
annotation/signature checking — `AICAD-052`); method/field member
resolution against a receiver's type (needs `AICAD-052`'s type checker);
struct-field namespace/duplicate checking (`AICAD-053`); Engineering HIR
(the next IR layer down, `docs/plan/01_SYSTEM_ARCHITECTURE.md` §5).

Plan references: `docs/plan/01_SYSTEM_ARCHITECTURE.md` §5;
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §17 (phase 7);
`docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §15;
`rfcs/0004-units-type-system.md` §8-9.
