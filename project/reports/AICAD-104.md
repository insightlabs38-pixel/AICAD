# AICAD-104: Prove the complete source query vocabulary through the production path

## Status

Done. Batch S5-00, fourth task (depends on `AICAD-103`) — the last of the
four originally-queued S5-00 tasks. Together with `AICAD-101`, `AICAD-102`,
`AICAD-103`, and the owner-requested `AICAD-104A`, this closes batch
`S5-00`.

## Objective

Prove, through the real production `ParametricBuildSession`/resolver path
(never direct Rust construction of a `Query`/`Candidate`/
`ResolutionOutcome`), that the complete source-visible query vocabulary
(`AICAD-100A`'s original subset plus `AICAD-103`'s completion) actually
works end to end, covering every required case category: positive,
no-match, ambiguous, invalid-scope, wrong-cardinality, nested-reference,
and spatial-value.

## Base / resulting commit

Base: `8528685` (`claude/aicad-stage5-dev`, the `AICAD-103` correction).
This task's commits are the remainder of `git log 8528685..HEAD` on that
branch.

## A correction found and fixed during this task

While building the production-path fixture, deeper reading of
`cad_query::resolve::filter_and_rank` found that `SpatialPredicate::
NearestTo`/`FarthestFrom` are rewritten into the equivalent
`RankingDirective` *before* evaluation, giving them real, working
semantics through `resolve_query`/`resolve_reference` — contradicting
`AICAD-103`'s own original decision to treat `nearest_to`/`farthest_from`
as unrecognized clause names. Fixed in a dedicated commit (`AICAD-103
correction: nearest_to/farthest_from are real, supported clauses`,
recorded in `project/reports/AICAD-103.md`) before this task's own new
work: both are now real, lowered, tested clause spellings, and this
task's own `spatial_value_nearest_to_and_farthest_from_resolve_through_
the_ranking_rewrite` test proves it holds through the actual production
resolver, not only at the lowering-unit-test level.

## Fixture

One `.aicad` program (`crates/cad-cli/tests/stage5_query_production_path.rs`'s
`SOURCE`): `part Wall { let base = box(10mm, 20mm, 5mm); part Door { let
hinge = box(1mm, 1mm, 1mm); } }` (exercising `AICAD-101`'s unbounded part
nesting two levels deep) plus ten `query { ... }` declarations, each
chosen against `base`'s exact known face-area geometry (two `200mm²`, two
`100mm²`, two `50mm²` faces) — never an assumption about kernel face
ordering or normal direction, only area, cardinality, and position, which
are geometrically unambiguous for an axis-aligned box at a known origin.

## Case coverage (all via `ParametricBuildSession::new` + `resolve_reference`/`resolve`)

| Case | Query | Result |
|---|---|---|
| Positive | `whole : Solid in Wall.base { unique(); }` | `Resolved(1)` |
| No-match | `none_such : Face in Wall.base { area(gt, 999999mm2); unique(); }` | `Broken(NoMatch)` |
| Ambiguous | `many_faces : Face in Wall.base { area(gt, 40mm2); unique(); }` | `Ambiguous(6)` (every face exceeds 40mm²) |
| Invalid-scope | `bad_scope : Face in NoSuchFeature { planar(); }` | `Broken(ScopeNotFound(FeatureAnchor::named("NoSuchFeature")))` |
| Wrong-cardinality (too few) | `too_few : Face in Wall.base { area(gt, 150mm2); expect_count(3); }` | `Broken(TooFew { expected: 3, found: 2 })` |
| Wrong-cardinality (too many) | `too_many : Face in Wall.base { area(gt, 150mm2); expect_count(1); }` | `Ambiguous(2)` |
| Nested-reference (scope) | `hinge_ref : Solid in Wall.Door.hinge { unique(); }` — two `part` levels deep | `Resolved(1)` |
| Nested-reference (clause arg) | `near_hinge_ref : Solid in Wall.base { within(1m, Wall.Door.hinge); unique(); }` | `Resolved(1)` |
| Spatial-value (`Point3` literal) | `contains_pt : Solid in Wall.base { contains(1mm, 1mm, 1mm); unique(); }` | `Resolved(1)` |
| Spatial-value (ranking rewrite) | `nearest_pt`/`farthest_pt : Solid in Wall.base { nearest_to(...)/farthest_from(...); unique(); }` | `Resolved(1)` each |

`expect_count`'s own cardinality is only honored by `ParametricBuildSession::
resolve` (`cad_query::resolve_query`, which uses the query's own stated
cardinality) — `resolve_reference` always forces `CardinalityExpectation::
Unique` for a single stable reference regardless of a `SemanticQuery`-backed
query's own cardinality (its own doc comment says so explicitly), so the two
wrong-cardinality cases go through `resolve` via `ParametricBuildSession::
lookup_query` (a public `ResolverContext` trait method) instead.

No case picks an arbitrary candidate: `Ambiguous` always reports the full
candidate set, `Broken` always names the exact structured reason
(`NoMatch`/`TooFew`/`ScopeNotFound`), matching D7's fail-closed contract.

## Frozen corpus / harness status

- `cargo test -p cad-cli --test stage4_resolver_execution -- --test-threads=1`:
  11/11 passing, unchanged.
- `python3 scripts/ci/semantic_ref_harness.py validate`: `{"cases": 10,
  "splits": {"held_out": 3, "public": 7}, "status": "ok"}`.
- `python3 scripts/ci/semantic_ref_harness.py self-test`: `{"status": "ok",
  "self_test": "silent-wrong gate exercised"}`.
- Automatic geometry-fingerprint recovery remains disabled — not touched by
  this task.

## Verification

```
cargo fmt --all -- --check                                                     # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings           # clean
cargo test -p cad-cli --test stage4_resolver_execution -- --test-threads=1     # 11/11
python3 scripts/ci/semantic_ref_harness.py validate                            # ok
python3 scripts/ci/semantic_ref_harness.py self-test                           # ok
cargo test --workspace                                                        # 81 test-result blocks, all ok, 0 failed
```

## Source-visible query limitations remaining after this batch

- A literal nested `query { ... }`-shaped clause argument
  (`AdjacencyTarget::Query`) has no `.aicad` syntax — every predicate
  needing "a ref/query" is reachable only via a named reference
  (`AICAD-103`'s own report).
- `adjacent_to`/`connected_to`/`intersects`'s reference target, and
  `within`/`nearest`/`farthest`/`nearest_to`/`farthest_from`'s named-target
  form, always take the *querying* query's own `EntityKind` (except
  `inside(...)`, always `Solid`) — there is no syntax to name a different
  entity kind for the target side.
- `ParamModel`'s own `param` coverage (`AICAD-104A`) does not extend to
  `.aicad` source syntax that would let a query clause argument reference
  a `param` by name and have it participate in dependency/dirty tracking
  as part of the *query* itself (a query clause argument is a structural
  literal, not an expression) — out of scope for this batch, unaffected.
- `cad_query::eval::compare_magnitude` still does not independently
  re-validate a `Magnitude`'s own dimension against the predicate it is
  attached to (`AICAD-102`'s original finding, carried forward
  unaddressed — the real source-reachable risk remains nil since every
  `Magnitude` this module constructs is correctly dimensioned by
  construction).

## Batch S5-00 status

Complete: `AICAD-101`, `AICAD-102`, `AICAD-103` (plus its correction),
`AICAD-104`, and the owner-requested `AICAD-104A` are all done, committed,
and pushed to `claude/aicad-stage5-dev`. Per explicit owner instruction,
Stage-5 implementation does not proceed to batch `S5-01` in this
invocation.
