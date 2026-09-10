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
`else if`/`match` (expression and statement position, `AICAD-055`),
`while`/`loop` with `break`/`continue` (`AICAD-056`, threaded as two new
`Signal` variants exactly like `Signal::Return`), and (after `project/
OWNER_DECISIONS.md#D16`'s owner ruling on collection/iterator construction
syntax) `[e1, e2, ...]` list literals, `start..end`/`start..=end` range
expressions, and `for var in iterable { ... }` execution over a `List<T>`
or an auto-iterable `Range<Int>`/`Range<UInt>` (`iterable` evaluated
exactly once; each iteration draws down a minimal iteration-budget
placeholder for `AICAD-058`'s own scheduled resource-budget scope). `src/
value.rs`: `Value`/`NumberValue`/`RangeValue` — the runtime value
representation (canonical-unit dimensional magnitudes; numeric scalars
deliberately collapse to one runtime tag, see that module's own doc comment
for why — this is also why `for`-range iteration cannot independently
re-verify `Int`/`UInt`-only at run time the way `cad_hir::typeck` does at
compile time, see `Interpreter::exec_for`'s own doc comment). `src/
error.rs`: `RuntimeError` — every way execution can fail to produce a
value, converted to a `cad_diagnostics::Diagnostic` under RFC-0005's
`RUNTIME` family. 54 tests.

Not yet implemented here (each fails with a clean `RuntimeError::
Unsupported` diagnostic, never a panic — see `src/interp.rs`'s own module
doc comment "Scope: what this task executes, and what it does not"):
struct/enum value construction and field access (no runtime value
representation exists yet for either); method calls (no method/interface-
implementation declaration syntax exists anywhere in the language). Also
documented as known, narrow scope gaps (not blocking any completed task):
ambiguous derived-dimension arithmetic (`Force * Length`-shaped
Torque/Energy, Pressure/Stress) that relies on `cad_hir::typeck`'s own
`expected`-type-directed disambiguation — see `src/interp.rs`'s module doc
comment "Known limitation" for the full reasoning; and D16's own named
scope limit — no `Set<T>`/`Map<K,V>`, collection comprehensions,
user-defined iterator protocols, or dimensional-range stepping, none of
which D16 authorized.

Plan references: `docs/plan/01_SYSTEM_ARCHITECTURE.md` §2.2;
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §17;
`docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` (canonical-unit internal
representation); `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-04;
`project/OWNER_DECISIONS.md#D16`.
