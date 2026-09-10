# cad-runtime

WP-04 (General runtime). Function calls, scopes, control flow, collections,
iterators/generators, recursion, pure-function cache, capability/resource
accounting, deterministic standard operations.

## Status (`AICAD-054`-`AICAD-056`)

Implemented: a tree-walking evaluator over `cad_hir::HirProgram` (`src/
interp.rs`: `Interpreter`) — ordinary function calls with lexical parameter/
local (`let`/`var`/`=`-reassignment) binding, literal/identifier/unary/
binary-arithmetic/binary-comparison expression evaluation, block
expressions (including a `return` correctly unwinding through arbitrary
expression nesting), top-level `let`/`const`/`param` globals, `if`/`else`/
`else if`/`match` (expression and statement position, `AICAD-055`), and
`while`/`loop` with `break`/`continue` (`AICAD-056`, threaded as two new
`Signal` variants exactly like `Signal::Return`). `src/value.rs`: `Value`/
`NumberValue` — the runtime value representation (canonical-unit
dimensional magnitudes; numeric scalars deliberately collapse to one
runtime tag, see that module's own doc comment for why). `src/error.rs`:
`RuntimeError` — every way execution can fail to produce a value, converted
to a `cad_diagnostics::Diagnostic` under RFC-0005's `RUNTIME` family. 42
tests.

Not yet implemented here (each fails with a clean `RuntimeError::
Unsupported` diagnostic, never a panic — see `src/interp.rs`'s own module
doc comment "Scope: what this task executes, and what it does not"):
`for`-loop execution — blocked on `project/OWNER_DECISIONS.md#D16` (no
collection/iterator value can be constructed from any `.aicad` source
program today: the frozen grammar has no array/list-literal or
range-operator syntax, and no compiler-intrinsic-function mechanism exists
either — giving `for` a real meaning needs one or the other, both
`AGENTS.md` owner-escalation triggers); struct/enum value construction and
field access (no runtime value representation exists yet for either);
method calls (no method/interface-implementation declaration syntax exists
anywhere in the language). Also documented as a known, narrow scope gap
(not blocking any completed task): ambiguous derived-dimension arithmetic
(`Force * Length`-shaped Torque/Energy, Pressure/Stress) that relies on
`cad_hir::typeck`'s own `expected`-type-directed disambiguation — see
`src/interp.rs`'s module doc comment "Known limitation" for the full
reasoning.

Plan references: `docs/plan/01_SYSTEM_ARCHITECTURE.md` §2.2;
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §17;
`docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` (canonical-unit internal
representation); `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-04.
