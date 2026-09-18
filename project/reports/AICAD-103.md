# AICAD-103: Complete .aicad lowering for the remaining Stage-4 query vocabulary

## Status

Done. Batch S5-00, third task (depends on `AICAD-101` and `AICAD-102`).

**Correction (recorded during `AICAD-104`):** this report originally
excluded `nearest_to`/`farthest_from` as clause spellings, reasoning that
`cad_query::eval::evaluate_spatial` rejects `SpatialPredicate::NearestTo`/
`FarthestFrom` outright (`EvalError::NotYetSpecified`). Deeper
investigation while building `AICAD-104`'s production-path proof found
that `cad_query::resolve::filter_and_rank` — the actual path every real
query executes through — rewrites that exact clause shape into the
equivalent `RankingDirective::Nearest`/`Farthest` *before* evaluation
(`cad_query::eval`'s own module doc comment says so explicitly: "Every
real query executed through `cad_query::resolve_query`/`resolve_reference`
... therefore gives `nearest_to`/`farthest_from` real, already-tested
comparative semantics ... never a placeholder"). The original exclusion
was a mistake, not a considered scope boundary. Fixed in place, in the
same commit as this correction note: `nearest_to`/`farthest_from` are now
real, supported, tested clause spellings (see `crates/cad-cli/src/
query_lowering.rs`'s updated module doc comment and the replaced test
`nearest_to_and_farthest_from_lower_to_the_real_spatial_predicate`). The
"Deliberately not implemented" section below (a literal nested
`query { ... }` clause argument) is unaffected and remains accurate.

## Objective

`AICAD-100A` gave a first, deliberately minimal subset of `docs/plan/
06_REFERENCES_QUERIES_FEATURE_DAG.md` §6's query-clause vocabulary a real
`.aicad` source form (`generated_by`, `radius`, `normal`, `largest`, ...).
This task completes the remaining predicates that already have real
`cad_query::eval` production semantics: `area`, `boundary`, `adjacent_to`,
`connected_to`, `intersects`, `contains`, `inside`, `within`, `above`/
`below`/`left`/`right`, and the `nearest`/`farthest` ranking directives.

## Base / resulting commit

Base: `8db0bc2` (`claude/aicad-stage5-dev`, `AICAD-102`). This task's
commits are the remainder of `git log 8db0bc2..HEAD` on that branch.

## Key finding: no grammar change needed

`cad_ast::item::Item::Query`'s own doc comment already states the design
intent: "no ... `within` modifier, direction literals ... is introduced; a
clause needing one of those is written with plain numeric/identifier
arguments instead." Investigation confirmed every remaining predicate is
in fact expressible that way:

- a `Point3` argument (`contains`, `within`'s point form, a `Frame3`
  origin) as three trailing `Length`-unit numbers;
- a `Frame3` argument (`above`/`below`/`left`/`right`) as twelve numbers
  (a `Length` origin triple, then three unitless axis triples);
- a reference target (`adjacent_to`/`connected_to`/`intersects`/
  `inside`/`within`'s ref form/`nearest`/`farthest`'s ref form) as a bare
  or dotted name — `cad_hir::lower`'s existing `Expr::Field` handling
  already produces a qualified `HirQueryArg::Name("Wall.hinge")` with no
  change needed, giving "nested reference argument" support for free.

**No `cad-ast`/`cad-parser`/`cad-hir` code changed at all.** Every
addition is new match arms and argument-parsing helper functions inside
`crates/cad-cli/src/query_lowering.rs` alone.

## `nearest_to`/`farthest_from`: real semantics via the resolver rewrite

`cad_query::predicate::SpatialPredicate::NearestTo`/`FarthestFrom` are
genuinely comparative across a candidate *set*, so a *direct* call to
`cad_query::eval::evaluate_spatial` on either reports `EvalError::
NotYetSpecified`. But `cad_query::resolve::filter_and_rank` — the real
path `resolve_query`/`resolve_reference` (every production query
execution in this workspace) goes through — rewrites this exact clause
shape into the equivalent `RankingDirective::Nearest`/`Farthest` *before*
evaluation ever runs, giving it real, already-tested comparative
semantics in production (see the correction note at the top of this
report). This module lowers `nearest_to(target)`/`farthest_from(target)`
directly to `SpatialPredicate::NearestTo`/`FarthestFrom`, and separately
`nearest(target)`/`farthest(target)` to the equivalent
`RankingDirective::Nearest`/`Farthest` — plain synonyms, both real,
matching the plan's own two named spellings ("spatial predicates:
`nearest_to(point|ref)`" and "ranking: `nearest(target)`") for what turns
out to be the same underlying mechanism.

## What was implemented

New `.aicad` clause spellings, all in `crates/cad-cli/src/query_lowering.rs`:

- `area(cmp, magnitude)` → `GeometryPredicate::Area` (trivial, reuses
  `magnitude_comparison`, now that `AICAD-102` added `Area` unit literals).
- `boundary(outer|inner)` → `TopologyPredicate::Boundary`.
- `adjacent_to(name)`, `connected_to(name)`, `intersects(name)` →
  `TopologyPredicate::{AdjacentTo,ConnectedTo,Intersects}(AdjacencyTarget::Ref(...))`.
- `contains(x, y, z)` → `TopologyPredicate::Contains(Point3)`.
- `inside(name)` → `SpatialPredicate::Inside(AnyRef)` — the reference's
  entity kind is always `Solid` ("a volume" is unambiguous), not the
  querying query's own kind.
- `within(distance, name)` / `within(distance, x, y, z)` →
  `SpatialPredicate::Within(Magnitude, SpatialTarget)`.
- `above(...)`/`below(...)`/`left(...)`/`right(...)` (12 numeric args) →
  `SpatialPredicate::RelativeTo(RelativeDirection, Frame3)`.
- `nearest(name|x,y,z)`, `farthest(name|x,y,z)` →
  `RankingDirective::{Nearest,Farthest}(SpatialTarget)`.
- `nearest_to(name|x,y,z)`, `farthest_from(name|x,y,z)` →
  `SpatialPredicate::{NearestTo,FarthestFrom}(SpatialTarget)` (real
  semantics via the resolver's ranking rewrite — see above).

Every named-reference argument resolves via `ConstructionStrategy::
StructuralRole` — the same collision-safe bare/dotted-name lookup
(`D31`'s `resolve_scoped_name`) every other named-binding evidence source
in this codebase already uses; a reference's own entity kind is the
querying query's kind (mirroring `descended_from`'s own pre-existing
identical simplifying assumption), except `inside(...)`'s always-`Solid`
convention above.

**Deliberately not implemented:** a literal nested `query { ... }`-shaped
clause argument (`AdjacencyTarget::Query`). Every plan-listed predicate
needing "a ref/query" is reachable via a named reference instead, so this
task's minimal-extension charter does not require it; representing an
inline nested query as a clause argument would be a real grammar change
(embedding a recursive clause list inside an argument position), out of
proportion to what the remaining vocabulary actually needs.

## Tests added

`crates/cad-cli/src/query_lowering.rs` (`#[cfg(test)]`): one focused test
per new clause (positive path plus a malformed/negative case for
`boundary`, `contains`, and `within`'s wrong-arity form), a dotted-nested-
reference test (`adjacent_to(Wall.hinge)`), and two required conformance
tests:

- `source_vocabulary_conformance_table_covers_every_supported_clause` — a
  table of every supported clause spelling (38 entries spanning all six
  entity kinds where relevant), each asserted to lower with zero
  diagnostics and exactly one produced clause.
- `intentionally_unsupported_clause_spellings_are_named_explicitly` — the
  complementary negative table: a literal nested `query { ... }` argument
  (does not even parse as a valid clause argument), asserted to remain
  unsupported rather than silently accepted.

## Verification

```
cargo fmt --all -- --check                                                       # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings             # clean
cargo test -p cad-lexer -p cad-parser -p cad-hir -p cad-query -p cad-references -p cad-cli
                                                                                    # all green (cad-cli: 98 tests)
cargo test --workspace                                                           # 80 test-result blocks, all ok, 0 failed
python3 scripts/ci/semantic_ref_harness.py validate                              # {"cases": 10, "splits": {"held_out": 3, "public": 7}, "status": "ok"}
python3 scripts/ci/semantic_ref_harness.py self-test                             # {"status": "ok", "self_test": "silent-wrong gate exercised"}
```

## Limitations / follow-ups (for AICAD-104)

- A literal nested `query { ... }` clause argument remains unsupported —
  see above.
- The `AdjacencyTarget`/`SpatialTarget`/`inside` reference-target entity
  kind defaults (querying query's own kind, or always `Solid` for
  `inside`) are documented simplifying assumptions, not a general
  entity-kind-selector syntax — no such syntax exists in this minimal
  grammar.
- `AICAD-102`'s own deferred finding (`cad_query::eval::compare_magnitude`
  does not independently re-validate a `Magnitude`'s dimension against
  the predicate it is attached to) remains unaddressed: every `Magnitude`
  this module constructs is correctly dimensioned by construction
  (`parse_magnitude`/`length_magnitude_arg` resolve strictly through the
  unit registry), so the real source-reachable risk is nil, but the
  defensive check itself was not added, to keep this task's own diff
  scoped to lowering, not `cad-query`'s evaluation internals.
- `AICAD-104` is responsible for proving this vocabulary through the real
  `ParametricBuildSession`/resolver production path (this task's own
  tests exercise `lower_hir_queries` directly, not end-to-end resolution).
