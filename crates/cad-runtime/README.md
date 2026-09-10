# cad-runtime

WP-04 (General runtime). Function calls, scopes, control flow, collections,
iterators/generators, recursion, pure-function cache, capability/resource
accounting, deterministic standard operations.

## Status (`AICAD-054`)

Implemented: a tree-walking evaluator over `cad_hir::HirProgram` (`src/
interp.rs`: `Interpreter`) — ordinary function calls with lexical parameter/
local (`let`/`var`/`=`-reassignment) binding, literal/identifier/unary/
binary-arithmetic/binary-comparison expression evaluation, block
expressions (including a `return` correctly unwinding through arbitrary
expression nesting), and top-level `let`/`const`/`param` globals. `src/
value.rs`: `Value`/`NumberValue` — the runtime value representation
(canonical-unit dimensional magnitudes; numeric scalars deliberately
collapse to one runtime tag, see that module's own doc comment for why).
`src/error.rs`: `RuntimeError` — every way execution can fail to produce a
value, converted to a `cad_diagnostics::Diagnostic` under RFC-0005's
`RUNTIME` family. 24 tests.

Not yet implemented here (each fails with a clean `RuntimeError::
Unsupported` diagnostic, never a panic — see `src/interp.rs`'s own module
doc comment "Scope: what this task executes, and what it does not"):
`if`/`match` expression/statement execution (`AICAD-055`); `for`/`while`/
`loop`/`break`/`continue` execution (`AICAD-056`); struct/enum value
construction and field access (no runtime value representation exists yet
for either); method calls (no method/interface-implementation declaration
syntax exists anywhere in the language). Also documented as a known,
narrow scope gap (not blocking this task): ambiguous derived-dimension
arithmetic (`Force * Length`-shaped Torque/Energy, Pressure/Stress) that
relies on `cad_hir::typeck`'s own `expected`-type-directed disambiguation —
see `src/interp.rs`'s module doc comment "Known limitation" for the full
reasoning.

Plan references: `docs/plan/01_SYSTEM_ARCHITECTURE.md` §2.2;
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §17;
`docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` (canonical-unit internal
representation); `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-04.
