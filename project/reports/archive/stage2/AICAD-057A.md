# AICAD-057A — Stage-2 coverage audit

## Objective

`AICAD-056` (`D16`) and `AICAD-057` (`D17`) both hit an owner-escalation
trigger for the same underlying reason: `project/TASKS.yaml`'s Stage-2
task list (`AICAD-038`-`AICAD-064`) was never checked end-to-end against
the *complete* normative Stage-2 language surface `docs/plan/` assumes.
The owner's `D17` ruling directed a one-time coverage audit — compare the
Stage-2 gate's actual requirements against what `AICAD-038`-`064` cover,
the frontend/HIR/runtime as implemented, and approved RFCs/owner
decisions — and classify every gap, so the remediation this ruling
authorizes (`AICAD-057B`-`AICAD-057F`) is exactly what the Stage-2 gate
needs and nothing wider. This report is that audit. It does not implement
anything; it only classifies gaps and amends `project/TASKS.yaml` with the
remediation tasks the owner's ruling already names plus any others the
audit found actually required.

## Method

1. Read the Stage-2 exit gate and "Build" list
   (`docs/plan/15_IMPLEMENTATION_ROADMAP.md` "Stage 2 — Core language/
   compiler/runtime") and the concrete Stage-2 task acceptance criteria
   (`project/TASKS.yaml` `AICAD-038`-`064`, `project/CURRENT_STAGE.md`).
2. Read the long-term general-language surface `docs/plan/` describes
   (`02_LANGUAGE_AND_COMPILER.md` §4-14, `03_TYPE_SYSTEM_UNITS_CONTROL_
   FLOW.md` §9-20, `21_FEATURE_INVENTORY.md` section A) — this is *not*
   itself a Stage-2 requirement list; it is the superset the audit must
   filter against the actual Stage-2 gate rather than adopt wholesale
   (`AGENTS.md`: "Do not silently widen Stage 2 simply because a feature
   appears somewhere in the long-term language plan," restated verbatim by
   the owner's ruling).
3. For each feature the owner named, inspect the current implementation
   directly (not from memory/prior reports) — `cad-lexer`'s keyword table,
   `cad-ast`'s `Item`/`Pattern`/`Type` shapes, `cad-parser`'s declaration
   parsing, `cad-hir`'s `HirTypeRef`/typeck, `cad-runtime`'s `Value` — to
   determine actual current status, not assumed status.
4. Classify each into `REQUIRED-BEFORE-STAGE2-GATE`, `REQUIRED-LATER`,
   `OPTIONAL/FUTURE`, or `ALREADY-SATISFIED`, using the Stage-2 exit gate
   ("Ordinary programs can construct geometry and compute parameterized
   patterns without special interpreter shortcuts") and `AICAD-063`'s
   concrete acceptance ("AICAD source using typed units and ordinary
   control flow compiles through HIR/Geometry IR to valid exact bracket
   and verified STEP") as the actual bar — not the long-term plan surface.
5. Amend `project/TASKS.yaml` with only the tasks the classification
   requires.

## Findings

| # | Capability | Stage-2 gate evidence | Current implementation (direct inspection) | Classification | Task |
|---|---|---|---|---|---|
| 1 | Generic type-parameter syntax on `struct`/`enum`/`fn` declarations | `D17` ruling authorizes it explicitly for `Result<T,E>`/`Optional<T>`, which `AICAD-057` (inside the fixed S2-09 batch, before the Stage-2 gate) needs | `Type::Generic { name, args }` already parses `Name<Args>` at **type-reference** sites (`crates/cad-ast/src/expr.rs` line 55 area; used today only by `List<T>`/`Range<T>` in `cad_hir::typeck::resolve_type_ref`). No **declaration**-site type-parameter list exists anywhere: `Item::Fn`/`Item::Struct`/`Item::Enum` (`crates/cad-ast/src/item.rs`) have no `type_params` field, and `cad-parser`'s struct/enum/fn-declaration parsing never looks for a `<...>` list after the name. | REQUIRED-BEFORE-STAGE2-GATE | `AICAD-057B` |
| 2 | Generic function syntax (`fn identity<T>(...)`) | Same as #1 — `D17` | Same evidence as #1: no type-parameter list parsing on `fn` declarations. | REQUIRED-BEFORE-STAGE2-GATE | `AICAD-057B` |
| 3 | Type application (`Name<T,U>`) | Same as #1 | **Partially satisfied** at the type-*reference* level (see #1); not satisfied at the declaration level. `HirTypeRef::Generic` already exists in `cad-hir` (`crates/cad-hir/src/types.rs`) and is lowered from `Type::Generic` (`crates/cad-hir/src/lower.rs` `lower_type`), so the AST/HIR shape for type application is reusable, not new — `AICAD-057B` extends an existing shape rather than inventing one. | REQUIRED-BEFORE-STAGE2-GATE (declaration side only) | `AICAD-057B` |
| 4 | Generic instantiation/inference at call sites | `D17` ruling, needed for `Result`/`Optional` construction and for any user-defined generic function `AICAD-057`'s own required tests exercise | No user-defined generic function exists to instantiate; `cad_hir::typeck`'s existing call-checking (`AICAD-052`) only ever checks concrete (non-generic) parameter/return types. | REQUIRED-BEFORE-STAGE2-GATE | `AICAD-057D` |
| 5 | Data-carrying enum variants (`Tuple(T1,T2)`, `Record{x,y}`) | `D17` ruling; `AICAD-057`'s own scope | `Parser::parse_enum_variants` (`crates/cad-parser/src/lib.rs` line 1198) returns `Vec<Spanned<String>>` — bare names only. `Item::Enum::variants: Vec<Spanned<String>>` (`crates/cad-ast/src/item.rs`) confirms unit-variants-only at the AST level too. `Value::EnumVariant(BindingId)` (`crates/cad-runtime/src/value.rs`) carries no payload slot. | REQUIRED-BEFORE-STAGE2-GATE | `AICAD-057C` |
| 6 | Enum variants as constructors/expressions | `D17` ruling | Not implemented — no variant currently takes constructor arguments to construct (only bare-name reference via `HirPattern::Variant`/lookup, `crates/cad-hir/src/lower.rs` line ~767). | REQUIRED-BEFORE-STAGE2-GATE | `AICAD-057C` |
| 7 | Destructuring patterns (tuple/record) | `D17` ruling | `crates/cad-ast/src/expr.rs`'s `Pattern` enum (line 325) has exactly three variants: `Wildcard`, `Literal`, `Ident`. No tuple- or record-shaped pattern exists anywhere in `cad-ast`/`cad-hir` (`HirPattern` mirrors the same three: `Wildcard`/`Literal`/`Variant`/`Binding`, `crates/cad-hir/src/hir.rs`). | REQUIRED-BEFORE-STAGE2-GATE | `AICAD-057C` |
| 8 | Nominal-enum match exhaustiveness | `D17` ruling states it explicitly ("the compiler must diagnose non-exhaustive matches unless a wildcard or otherwise exhaustive pattern is present") | Confirmed absent: `Checker::check_match`/`check_match_expr` (`crates/cad-hir/src/typeck.rs` lines 1572-1596) only unifies arm result types; there is no coverage check over an enum's variant set anywhere in the checker. This is a genuine, previously-undetected Stage-2 language-soundness gap, not merely a `Result`-construction blocker — an enum `match` today can omit variants silently. | REQUIRED-BEFORE-STAGE2-GATE | `AICAD-057C` |
| 9 | `Optional<T>` | `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §9/§14 ("do not use implicit null... `Optional<FaceRef>` must be handled explicitly"); `D17` ruling names it as a required remediation test (`Optional<Length>`, `Optional<Result<Int,E>>`) | Not implemented; no `Optional` name is special-cased or reserved anywhere in `cad-hir`/`cad-runtime`. | REQUIRED-BEFORE-STAGE2-GATE | `AICAD-057E` |
| 10 | `Result<T,E>` | Original `AICAD-057` title/scope; `D17` ruling | Not implemented — see `project/reports/AICAD-057.md`/`OWNER_DECISIONS.md#D17` for the original finding. | REQUIRED-BEFORE-STAGE2-GATE | `AICAD-057E` |
| 11 | `List<T>`/`Range<Int>`/`Range<UInt>` + `for`-loop iteration | Stage-2 "Build" list ("collections/iterators"); `AICAD-056` | **ALREADY-SATISFIED** — `AICAD-056`/`D16`, `project/reports/AICAD-056.md`, 61/61 `cad-runtime` tests. | ALREADY-SATISFIED | none |
| 12 | `Set<T>`/`Map<K,V>`/collection comprehensions/user-defined iterator protocols/dimensional-range stepping | `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §9 lists them in the long-term collections set | `D16` explicitly considered and rejected authorizing these ("Explicitly not authorized by this decision", `DECISION_LOG.md#DL-13`). No Stage-2 task (`AICAD-038`-`064`) or the Stage-2 exit gate/`AICAD-063` acceptance criterion references `Set`/`Map`/comprehensions. Nothing in the concrete Stage-2 gate ("construct geometry and compute parameterized patterns") requires them — `List<T>`/`Range<T>` already cover every existing example's iteration need. | REQUIRED-LATER (no new task — re-affirms `D16`'s own scope limit; do not silently widen) | none |
| 13 | Closures (`|e| e.direction ~= +Z`) | `docs/plan/02_LANGUAGE_AND_COMPILER.md` §6, `03_TYPE_SYSTEM...` §17 — used for query composition (`body.edges().filter(...)`) | Not implemented anywhere (no closure/lambda expression syntax in `cad-ast`). No `AICAD-038`-`064` task or Stage-2 exit-gate/`AICAD-063` wording requires them — every closure example in the plan docs is a geometry-*query* composition pattern (`WP-07` semantic references/queries, `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` §8), which is explicitly Stage 4+ scope (`docs/plan/15_IMPLEMENTATION_ROADMAP.md` Stage 4 "query system"), not Stage 2. | OPTIONAL/FUTURE (no new task) | none |
| 14 | Generators/`yield` | `docs/plan/02_LANGUAGE_AND_COMPILER.md` §6/§7, `03_TYPE_SYSTEM...` §18 (`bolt_circle` example) | Not implemented; `yield` is explicitly **not** a reserved keyword yet (`crates/cad-lexer/src/token.rs`'s own module doc comment lists it among words "used only in prose examples... deliberately not reserved yet"). The `bolt_circle` example is high-level pattern generation (`docs/plan/04_HIGH_LEVEL_MODELING_API.md`-era feature), not required by the Stage-2 exit gate or any `AICAD-038`-`064` task. | OPTIONAL/FUTURE (no new task) | none |
| 15 | Interfaces/traits, generic bounds (`T: MotorMount`) | `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §11/§12 | `interface` **is** already a reserved keyword (`Keyword::Interface`, `crates/cad-lexer/src/token.rs` — reserved because it appears in the frozen `specs/language/grammar.ebnf` sketch) but has zero parser/HIR/typeck support (confirmed: no `parse_interface`/`Item::Interface` exists; `cad_hir::typeck.rs` line ~1257 explicitly notes "No method/trait/geometry-API declaration syntax exists"). `D17`'s own text defers interface bounds "unless the Stage-2 coverage audit demonstrates they are required by the Stage-2 gate." No Stage-2 task or the `AICAD-063` acceptance criterion requires interfaces/bounds; `MotorMount`-style interfaces are a Stage-6-era assemblies/mechanical-interface concept in the roadmap, not Stage 2. | OPTIONAL/FUTURE (no new task; `AICAD-057B`'s generic-parameter design must stay extensible to a future bound list per the owner's own instruction, but that is a design constraint on `057B`, not a separate task) | none |
| 16 | `comptime` compile-time evaluation | `docs/plan/02_LANGUAGE_AND_COMPILER.md` §11 | Not implemented; not reserved as a keyword (same module doc comment as #14). No Stage-2 task references it; the Stage-2 exit gate is about ordinary runtime evaluation of parameterized geometry, which does not need compile-time evaluation to be met. | OPTIONAL/FUTURE (no new task) | none |
| 17 | Macros/`@derive`/typed metaprogramming | `docs/plan/02_LANGUAGE_AND_COMPILER.md` §12, `21_FEATURE_INVENTORY.md` "Typed/hygienic metaprogramming later" (the inventory's own wording already flags this as later-stage) | Not implemented; not reserved. No Stage-2 task references it. | OPTIONAL/FUTURE (no new task) | none |
| 18 | Attributes/annotations (`@param(...)`, `@rationale(...)`) | `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §22/§23 | Not implemented; no `@...` syntax exists in `cad-lexer`/`cad-parser`. No `AICAD-038`-`064` task requires it, and no example under `examples/` uses `@param`/`@derive`/`@rationale` today (checked directly — zero matches), so nothing currently blocks on it. | OPTIONAL/FUTURE (no new task) | none |
| 19 | Nested generics (`Optional<Result<Int,E>>`) as a standalone capability | `D17` ruling names it as a required test | Not a separate feature — it falls out of #1-#4/#9-#10 (ordinary nested type application over a working generic-enum system) once `AICAD-057B`/`D`/`E` land; the required test lives in `AICAD-057E`/`F`'s test list, not a new task. | REQUIRED-BEFORE-STAGE2-GATE (covered by existing remediation tasks) | `AICAD-057E`/`AICAD-057F` |
| 20 | Adversarial proof the new machinery is general (not `Result`-specific) | `D17` ruling requires it explicitly ("a user-defined generic enum whose name is NOT Result, Optional, List, or Range") | N/A — new requirement, not previously tracked anywhere | REQUIRED-BEFORE-STAGE2-GATE | `AICAD-057F` |
| 21 | Resource-budget accounting (distinct from the engine recursion ceiling) | Stage-2 "Build" list ("resource budgets"); already an existing task | `AICAD-058`, `status: todo` — already tracked, untouched by this audit. `D17`'s "Recursion policy" clarification (see `DECISION_LOG.md#DL-14`) is guidance for `AICAD-058`, not a new task. | ALREADY-SATISFIED (already an existing, correctly-scoped task) | none (guidance folded into `AICAD-058`'s existing scope) |

No other general-language capability referenced by the Stage-2 exit gate,
`AICAD-063`'s concrete acceptance criterion, or any `AICAD-038`-`064` task
was found missing from `project/TASKS.yaml`. In particular: modules/
imports (`AICAD-044`, done), structs/enums field typing (`AICAD-053`,
done — unit-variant enums and non-generic structs specifically), name
binding (`AICAD-050`, done), the deterministic evaluator (`AICAD-054`/
`055`/`056`, done), the formatter (`AICAD-045`, done), and the Tree-sitter
grammar (`AICAD-062`, todo, already scheduled) all already have tasks
matching their Stage-2 gate scope.

## Decision

No new owner decision was made by this audit — it applies the `D17`
ruling already recorded (`OWNER_DECISIONS.md#D17`, `DECISION_LOG.md#DL-14`)
rather than inventing scope. Findings #12-#18 are explicitly **not**
promoted to Stage-2 remediation tasks, per the owner's own instruction not
to widen Stage 2 to match the long-term plan; each is recorded above with
its concrete evidence so a later stage's own audit does not have to
re-derive it from scratch.

## `project/TASKS.yaml` amendment

Inserted six new tasks between `AICAD-057` and `AICAD-058`, in the fixed
order the owner's ruling specifies, each `depends_on` the previous one so
the existing dependency-driven work loop cannot execute them out of order:

- `AICAD-057A` (this task) — `status: done`.
- `AICAD-057B` — generic parameter/type-application syntax + AST/HIR
  representation. `status: todo`.
- `AICAD-057C` — general data-carrying enum variants, constructors,
  destructuring patterns, nominal-enum match exhaustiveness. `status:
  todo`.
- `AICAD-057D` — generic instantiation/inference/type checking for the
  approved Stage-2 generic subset. `status: todo`.
- `AICAD-057E` — `Result<T,E>`/`Optional<T>` via the ordinary generic-enum
  machinery. `status: todo`.
- `AICAD-057F` — adversarial integration pass proving the machinery is
  general, not `Result`-specific. `status: todo`.

The original `AICAD-057` (`status: todo`, unchanged) now `depends_on:
[AICAD-057F]` instead of `[AICAD-056]`, formally encoding "only after
`AICAD-057A` through `AICAD-057F` are complete may the original `AICAD-057`
resume." `AICAD-058`'s `depends_on` is unchanged (`[AICAD-057]`), so the
existing chain `AICAD-058 -> AICAD-059 -> ... -> AICAD-064` is undisturbed.

## Commands / results

- `grep -n "fn parse_enum_variants" -A 15 crates/cad-parser/src/lib.rs` —
  confirmed bare-name-only enum variant parsing (finding #5).
- `grep -n "enum Pattern" -A 15 crates/cad-ast/src/expr.rs` — confirmed
  three-variant `Pattern` enum, no tuple/record shape (finding #7).
- `sed -n '1572,1660p' crates/cad-hir/src/typeck.rs` — confirmed
  `check_match`/`bind_pattern` perform no exhaustiveness check (finding
  #8).
- `grep -n "Struct\|Enum" crates/cad-runtime/src/value.rs` — confirmed no
  `Value::Struct`, `Value::EnumVariant(BindingId)` carries no payload
  (findings #5/#6).
- `grep -rniE "closure|generator|yield|interface|trait" crates/cad-ast
  crates/cad-parser crates/cad-hir crates/cad-lexer --include=*.rs` —
  confirmed `interface` reserved-but-unimplemented, no closure/generator/
  yield support anywhere (findings #13/#14/#15).
- `grep -rniE "Set<|Map<|comprehension" crates/ --include=*.rs` and
  `grep -rln "@param|@derive|@rationale" examples/` — confirmed no
  `Set`/`Map`/comprehension implementation and no example currently uses
  attribute syntax (findings #12/#18).
- No `cargo` build/test/clippy/fmt commands were run — this task changes
  no Rust source, only planning documents (`project/TASKS.yaml`,
  `OWNER_DECISIONS.md`, `DECISION_LOG.md`, this report,
  `SESSION_HANDOFF.md`), so the Rust-workspace `required_checks` do not
  apply (consistent with every prior planning-only task in this project).

## Limitations / follow-up

- This audit classifies gaps against the *Stage-2* gate only. Findings
  #12-#18 are not closed questions forever — each names the later stage
  the roadmap already assigns it to (Stage 3 for closures/`comptime`,
  Stage 4 for query-composition closures specifically, Stage 6 for
  interfaces-as-mechanical-interfaces), so a future stage's own coverage
  audit should re-check them against that stage's gate rather than assume
  this report settled them permanently.
- `AICAD-057B`'s generic-parameter design should keep an explicit extension
  point for a future bound list (`T: SomeInterface`) per `D17`'s own
  "generic foundation must be designed so such bounds can be added later
  without replacing the generic type model" — this is a design constraint
  recorded here for `AICAD-057B` to honor, not a separate task.

## Status

`AICAD-057A` is complete. All required checks for a planning-only task
(this report, `TASKS.yaml`/`OWNER_DECISIONS.md`/`DECISION_LOG.md`
consistency) are satisfied. Next: `AICAD-057B`.
