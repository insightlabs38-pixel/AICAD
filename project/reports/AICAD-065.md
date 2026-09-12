# AICAD-065 — Implement first-class param declarations and derived expressions

## Result
PASS

## Objective
Make `param` declarations first-class in the Stage-3 parametric model:
explicit parameter identity, typed values, derived-expression
dependencies, deterministic evaluation, and edit/rebuild semantics
suitable for the upcoming feature DAG (`AICAD-066`/`AICAD-067`, Batch
S3-01) — without reimplementing Stage-2's already-existing `param`
parser/HIR/execution support. Second and final task of Stage-3 Batch
S3-00, per `project/TASKS.yaml`.

## Dependencies checked
`AICAD-064A` (D5 v1 comparison-profile calibration) — complete, commit
`1865855` on this branch. Not otherwise load-bearing for this task (no
geometry/comparison logic is involved in parameter identity/dependency/
evaluation), but required by the fixed Batch S3-00 order.

## Base / resulting commit
Base: `1865855` (this branch, `AICAD-064A`'s own commit). Resulting
commit: this task's own commit on `claude/aicad-stage3-dev`.

## Changes
- `crates/cad-runtime/src/params.rs` (new) — `ParamId` (a thin newtype
  over the param's existing `BindingId`, not a parallel identity
  allocation), `ParamDecl`/`ParamModel` (explicit dependency edges + a
  deterministic topological evaluation order via Kahn's algorithm, ties
  broken by declaration order), `ParamModelError` (a structured cyclic-
  dependency diagnostic, never an arbitrary evaluation order), and
  `ParamOverrides` (a plain `ParamId -> Value` edit map). 7 unit tests.
- `crates/cad-runtime/src/interp.rs` — `Interpreter::
  run_top_level_parametric`: evaluates top-level `let`/`const` in source
  order (identical to the existing `run_top_level`), then evaluates every
  top-level `param` via the model's dependency-ordered, override-aware
  schedule instead of naive source order. Refactored the shared per-item
  evaluate-and-store-to-globals logic (previously inlined only in
  `run_top_level`) into `Interpreter::eval_top_level_value`, reused by
  both methods — a behavior-preserving refactor, not a semantic change to
  `run_top_level` itself. 7 new tests exercising this method end to end
  (derived-param evaluation, override-driven rebuild, override-type-
  mismatch rejection, cyclic-dependency-to-diagnostic conversion,
  determinism across repeated rebuilds).
- `crates/cad-runtime/src/error.rs` — two new `RuntimeError` variants,
  `RUNTIME` family (matching every other structural-well-formedness
  variant already here, e.g. `NonExhaustiveMatch` — not a new diagnostic
  family): `CyclicParamDependency` (`RUNTIME-E124`) and
  `ParamOverrideTypeMismatch` (`RUNTIME-E125`).
- `crates/cad-runtime/src/value.rs` — `Value::operand_type()`: extracts a
  value's intrinsic `OperandType` (`Number`'s own field; `Scalar(Bool)`/
  `Scalar(String)` for `Bool`/`Str`; `None` for every other kind), reused
  by override-type validation.
- `crates/cad-runtime/src/lib.rs` — `pub mod params;` and re-exports.
- `crates/cad-runtime/README.md` — Stage-3 status section.

## Material decisions

### Where this lives: `cad-runtime`, not a new crate or `cad-compiler`
`docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-04 ("General runtime") owns
"deterministic standard operations," and this crate's own existing
`Interpreter::run_top_level` already evaluates top-level `let`/`const`/
`param` values — it is the existing, natural extension point, not a new
crate (`AGENTS.md`: "Keep agent-support infrastructure minimal and
project-specific" generalizes to "don't add a crate where an existing one
already owns the concept"). `crates/cad-feature-graph` (WP-06) is reserved
for `AICAD-066`/`AICAD-067`'s own feature-DAG node identity/dependency
model — deliberately not pulled forward here (`AGENTS.md`: "do not
implement later-task code because it will be needed soon"); this task
builds only the parameter layer the feature DAG will later sit on top of.

### Identity: reuse `BindingId`, don't mint a parallel one
A `param`'s `BindingId` (minted by `cad_hir::lower::lower_program`'s
deterministic counter) is already process-stable for identical source
(`DECISION_LOG.md#DL-12` Level-1 determinism). `ParamId` is a thin newtype
wrapping it rather than a second, independently-allocated identity scheme
— avoids exactly the kind of parallel-bookkeeping risk `AGENTS.md`'s
"Stable semantic references are preferred" non-negotiable warns about.

### Dependency detection: structural HIR walk, not a general expression-dependency graph
`ParamModel::build` walks each param's `default` expression collecting
every `HirExpr::Ident`/`HirCallee::Fn`/`HirExpr::RecordLiteral` binding
reference, exhaustively over every `HirExpr`/`HirStmt` variant (the match
statements have no wildcard arm, so a future HIR variant addition forces
this file to be updated rather than silently under-counting references —
matching this codebase's own established "no silent gaps" convention for
HIR consumers). A reference only becomes a dependency *edge* if it
resolves to another modeled top-level param (checked via the model's own
`index_by_id`, not by `BindingKind` alone) — a param's default referencing
a `let`/`const` or calling an ordinary function still evaluates correctly
through the ordinary interpreter, it just is not part of this
param-to-param dependency graph (explicitly scoped in the module's own
doc comment "Scope boundary" — this is `AICAD-065`'s stated scope, "derived
expressions" of the parametric model specifically, not a general top-level
value dependency graph for every `let`/`const`, which remains
`cad_runtime::interp`'s own separately-documented pre-existing gap).

### Determinism: Kahn's algorithm with declaration-order tie-breaking
Cycle detection uses a standard topological sort, but readiness queue
processing always considers indices in ascending (declaration) order —
never `HashMap`/`HashSet` iteration order — so identical source always
produces the identical evaluation schedule (`DECISION_LOG.md#DL-12` Level
1). Verified directly by a dedicated test (`evaluation_order_is_
deterministic_across_rebuilds`) that lowers the same source twice
independently and asserts identical schedules, plus an end-to-end test
(`rebuild_is_deterministic_across_repeated_runs_with_the_same_overrides`)
rebuilding a 3-param chain 5 times with the same override and asserting
bit-for-bit identical results every time.

### Cyclic dependency: a structured diagnostic, never an arbitrary order
A cycle (including a direct two-param cycle and a single param referencing
itself) is rejected with `ParamModelError`, convertible to
`RuntimeError::CyclicParamDependency` — never silently resolved by picking
an arbitrary partial order. This directly follows `AGENTS.md`'s "ambiguity
is an error, never an arbitrary selection," extended here from
semantic-reference resolution (its original context) to a dependency
graph, which is the same shape of problem.

### Edit/rebuild: `run_top_level_parametric` as a complete alternative, not an addition
Rather than mutating `run_top_level`'s own existing behavior (risking a
regression in the already-passing Stage-2 `stage2_end_to_end` proof and
every existing `run_top_level`-based test), this task adds a new,
parallel entry point. A caller wanting ordinary Stage-2 source-order
semantics keeps using `run_top_level`; a caller wanting first-class
parameter identity/dependency/override semantics uses
`run_top_level_parametric` instead. Wiring this into `cad-cli build` (a
real `--param name=value` flag) is explicitly left to a future task,
mirroring `project/DECISION_LOG.md#DL-15`'s own precedent of a mechanism-
introducing task leaving source/CLI wiring to a later one.

### Override type validation reuses the type checker's own resolution
`crate::params::value_matches_checked_type` compares an override's
intrinsic `OperandType` (`Value::operand_type`) against the param's own
already-resolved `cad_hir::typeck::CheckedType` (from a caller-supplied
`TypeCheckResult`, looked up by `BindingId` index — no re-derivation of
`HirTypeRef` resolution inside `cad-runtime`, consistent with
`crate::value`'s own module doc comment on why this crate never
re-derives type-checker context independently). Only `CheckedType::Value`
(an ordinary scalar/dimensional param — the only shape a `param` has ever
been able to declare in practice) is currently supported as an override
target; every other `CheckedType` shape compares unequal (a documented
scope boundary, not a silent accept). `type_check` is `Option` — a caller
with no type-checked program at all (e.g. a hand-built test) can still use
this method, simply skipping override-type validation.

## Verification
- `cargo fmt --all -- --check` → clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings, 27 crates.
- `cargo test -p cad-runtime` → 101 passed (83 pre-existing + 18 new: 11
  in `src/params.rs`, 7 in `src/interp.rs`), 0 failed.
- `cargo test --workspace` → 0 failures across every crate.
- `cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1` →
  3/3 (Stage-2 gate proof unaffected — `run_top_level` itself is
  behavior-unchanged, only refactored to share `eval_top_level_value`).

## Regressions/tests
None found. 18 new tests added; no existing test modified, weakened, or
removed. `run_top_level`'s own extracted-helper refactor is covered by
every pre-existing test that already exercised it (all still pass
unchanged).

## Findings / limitations
- Scope is top-level `param` items only, matching `run_top_level`'s own
  pre-existing scope — `part`-body params are not modeled (a future
  extension, not a regression: `part` bodies had no parametric-model
  support before this task either).
- Override-type validation supports only `CheckedType::Value` (ordinary
  scalar/dimensional params) — every `param` example in the codebase today
  is exactly this shape; a future struct/enum-typed `param` (if ever
  authorized) would need this extended.
- `cad-cli build` does not yet expose any way for a user to actually
  supply overrides (no `--param` flag) — this task establishes the
  mechanism only, per its own acceptance criteria; CLI wiring is explicit
  future work.

## Owner blockers
None.

## Next dependency
Batch S3-00 is now complete (`AICAD-064A` + `AICAD-065`). Per the fixed
batch order, the next batch is S3-01 (`AICAD-066`: "Create cad-feature-
graph node identity/dependency model"), which this task's `ParamModel`
was deliberately built to sit underneath without pulling any of its scope
forward.
