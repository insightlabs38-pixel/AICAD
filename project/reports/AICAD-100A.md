# AICAD-100A: Complete production semantic-reference integration disclosed by the Stage-4 gate packet

## Status

Done. Batch S4-09, following Batch S4-08 (`AICAD-100`).

## Objective

The owner's own review of the `AICAD-100` gate packet disclosed several
significant Stage-4 capabilities that existed in IR/test form but were
incomplete or unreachable through the real production path. The owner did
not want these deferred into Stage 5. This task's own brief: fix them now,
rerun the real Stage-4 hard gate, then stop.

## Base / resulting commit

Base: `AICAD-100` (`3078846`, `origin/claude/aicad-stage4-dev` HEAD at the
start of this task). This task's own commits: see `git log --oneline
3078846..HEAD` on `origin/claude/aicad-stage4-dev`.

## What was implemented

### 1. `D31` resolution: part-scoped feature discovery

Owner ruling implemented: **`part { ... }` is an abstraction/scope
boundary, not a feature-visibility barrier.** `cad_feature_graph::
FeatureGraph::build` now recurses into every `HirItem::Part` body (one
level, matching the grammar's own current single-level `part` nesting)
with AICAD-owned scoped identity (`FeatureNode::scope: Vec<String>`, the
enclosing part-name path), never OCCT/kernel identity. A part-nested
feature's own bare leaf name resolves collision-safely
(`crate::parametric_build::qualified_feature_name`/`resolve_scoped_name`,
`cad-cli`): a fully qualified dotted path always resolves unambiguously;
a bare name resolves only when unambiguous program-wide, failing closed
(never an arbitrary pick) on a genuine collision. Recorded as RESOLVED in
`project/OWNER_DECISIONS.md#D31` with a corresponding `project/
DECISION_LOG.md#DL-33` entry.

### 2. Explicit candidate scope for persistent references

`resolve_reference`'s `FeatureLineage`/`Ancestry` arms now always call
`.scoped_to(...)` — whole-session unscoped resolution is no longer the
implicit default for these strategies. The unscoped production API
(`ParametricBuildSession::candidates`) keeps its exact prior
whole-live-universe semantics unchanged (regression-tested); scoping never
geometrically deduplicates distinct candidates (regression-tested); an
invalid/missing required scope fails closed (`Broken(ScopeNotFound)`).

### 3. Remaining Stage-4 predicates

`Convex`/`Concave`/`Manifold`/`NonManifold`/`ConnectedTo`/`Contains`/
`Intersects` all now have real, kernel-neutral production semantics in
`crates/cad-query/src/eval.rs` (`evaluate_topology` matches every variant
exhaustively). `Candidate::root`/`Candidate::with_root` give a predicate
access to its enclosing shape for adjacency computation. `NearestTo`/
`FarthestFrom` are correctly routed through `resolve.rs`'s own
`filter_and_rank` ranking rewrite rather than evaluated as a per-candidate
boolean. New native bridge accessor `Shape::duplicate()` (re-registers the
same `TopoDS_Shape` under a new slot) gives predicates an independently-
owned handle where needed.

### 4. Production evidence sources for resolver strategies

Every `ConstructionStrategy` now has a real `ParametricBuildSession`
override: `resolve_export`/`resolve_structural_role`/
`resolve_user_confirmed`/`lookup_query` (`ExplicitExport`/
`StructuralRole`/`UserConfirmed`/`SemanticQuery`), plus `Ancestry`'s own
`resolve_reference` code path (`feature_scope_of` +
`resolve_query_with_cardinality`). At least one real `ParametricBuildSession`
test exists per strategy (`crates/cad-cli/tests/
stage4_evidence_sources.rs`, 9 tests) — `Ancestry`'s own positive path was
a real gap this task's own limitation sweep (§10 below) found and closed.
`GeometricFingerprint` remains unconditionally `Broken` (D7/`DL-8`,
unchanged).

### 5. Lineage coverage including Edge

`reference_replay::FeatureLineageIndex` is now keyed by `(FeatureAnchor,
EntityKind)` instead of `FeatureAnchor` alone, so a feature's own captured
`Face` and `Edge` lineage reports are two entirely separate
classifications. `enumerate_faces` generalized to `enumerate_prior_entities`;
`descended_from_closure` takes an explicit `kind` parameter. Covers
unchanged/generated/modified/split/merged/deleted states for both kinds.

### 6. Shell/Solid candidate enumeration

Native bridge additions: `aicad_occt_shape_shell_count`/`get_shell`,
`aicad_occt_shape_solid_count`/`get_solid` (native C++, `cad-occt-bridge`
FFI + safe wrapper). `reference_replay::candidates_of_kind` now covers all
six `EntityKind` variants.

### 7. `.aicad` source syntax for persistent references

RFC-0003 §7's reserved `query { ... }` surface is promoted into a real
grammar production — the smallest syntax completion consistent with that
surface and existing AICAD binding/value semantics, per that section's own
explicit authorization:

- **`cad-lexer`**: `query` reserved as a keyword.
- **`cad-ast`**: `Item::Query { name, entity_kind, scope, clauses, span }`
  — `query name : EntityKind in scope { clause* }`, an explicit `in scope`
  clause (never an implicit `body.faces`-shaped target) so entity kind and
  candidate scope are each their own token.
- **`cad-parser`**: production for the syntax above; clauses reuse the
  ordinary call-argument grammar (`name(args);`) verbatim — no new
  expression syntax (comparison operators, a `within` modifier, direction
  literals).
- **`cad-hir`**: `HirItem::Query`, `HirQueryClause`/`HirQueryArg` (`Name`
  | `Number`, including a negated-literal special case for direction
  components like `normal(0, 0, -1)`). Entity-kind *spelling* is validated
  here against the closed six-variant set (`TYPE-E460`); scope/clause
  *semantics* stay unresolved, carried structurally for `cad-cli` (which
  owns `cad_references::EntityKind` and the D31 scoped-name resolution) to
  interpret — matching `HirItem::Import`'s existing "not this task's job"
  precedent. Malformed clause arguments are `TYPE-E461`.
- **`cad-cli`** (`crate::query_lowering`, new module): interprets the
  closed clause-name vocabulary — topology (`generated_by`/`modified_by`/
  `descended_from`/`convex`/`concave`/`manifold`/`nonmanifold`), geometry
  surface kind (`planar`/`cylindrical`/`conical`/`spherical`/`toroidal`/
  `bspline`), geometry comparisons (`radius`/`length`, reusing
  `cad_units`'s real unit registry for canonical magnitudes — not `area`:
  RFC-0004's frozen unit set has no `Area`-dimensioned literal spelling
  yet, a pre-existing gap this task does not paper over), direction
  comparisons (`normal`/`axis`), ranking (`first`/`largest`/`smallest`),
  and cardinality (`unique`/`expect_count`) — into real `cad_query::Query`
  + `cad_references::AnyRef` values, always `DurabilityLevel::QueryStrong`
  (none of the supported predicates is geometry-fingerprint-based, so
  `QueryGeometric` would be dishonest). An unknown or malformed clause is
  a structured `REF-E103`/`REF-E104` diagnostic, never silently dropped.
  `ParametricBuildSession` lowers and registers every source-declared
  query at construction time, exposing the result via
  `source_references()`.
- **`cad refs check`** (`crate::refs_check`) now consumes
  `source_references()` in place of the previous hardcoded empty
  `Vec::new()` — a program that declares a persistent reference gets a
  genuinely non-empty, really-resolved health report.
- **Docs**: `specs/language/grammar.ebnf`'s `query_decl` production;
  `rfcs/0003-semantic-references.md` §7/§8 updated; stale "`query { ... }`
  is unimplemented" doc comments in `cad-query` (`lib.rs`, `health.rs`)
  corrected.

### 8. Real production queries against the frozen corpus

`stage4_resolver_execution.rs` now executes real production-path queries
for all seven public corpus cases (`01`/`02`/`04`/`06`/`07`/`08`/`09`),
using the corpus's own official intended query wherever the underlying
operation actually supplies lineage evidence (`01`/`02`/`07`/`09`, now
possible since D31 is resolved), or an honest position-tracking
alternative where it does not (`04`: `radial_pattern` is not one of the
five lineage-capable ops, so the case's own `Reasoning` section's real
alternative — instance 0's fixed position — is used rather than inventing
pattern-instance lineage; `07`: the same established `AICAD-099`
precedent). No lineage was synthesized from geometric coincidence; no
case's own expected target was edited to fit a measured result —
`case01`'s own real measured outcome (`Resolved(1)` in both variants,
diverging from `case.md`'s own untested pre-resolver prose prediction of
`explicit_ambiguity`) is recorded as an honest finding in that test's own
doc comment, never force-matched. Held-out cases (`03`/`05`/`10`) were
deliberately left untouched — `held_out/HELD_OUT_README.md`'s own
milestone-only discipline means a held-out case is touched at an
owner-designated checkpoint (already exercised once, at `AICAD-099A`), not
opportunistically whenever a new capability lands.

### 9. Limitation sweep

A systematic grep sweep (`NotYetSpecified`/`unimplemented!`/`todo!`/
default-`None` evidence hooks/placeholder branches/unsupported topology
kinds/test-only registries/proxy-only paths/`TODO`/`FIXME`) across
`cad-references`, `cad-query`, `cad-feature-graph`, `reference_replay`,
`ParametricBuildSession`, `query_lowering`, HIR/runtime reference
lowering, and kernel topology enumeration found the codebase's own
evidence machinery already exhaustive and honestly documented (every
`ResolverContext`/`EvaluationEvidence` method has a real production
override; `evaluate_topology`/`evaluate_geometry` match exhaustively with
no wildcard arm; all six `EntityKind` variants have real kernel-neutral
enumeration; D7's fail-closed fingerprint policy unchanged). Two real
findings, neither buried:

- `ConstructionStrategy::Ancestry`'s positive resolution path had zero
  test coverage anywhere in the workspace — fixed with a real
  `ParametricBuildSession` test (§4 above).
- `part`-in-`part` nesting is grammatically legal (the parser/AST/HIR
  place no depth limit on it) but silently inert: four independent call
  sites (the interpreter's own `eval_part_body`, `FeatureGraph::build`,
  `cad-cli`'s scoped-binding walkers, and this task's own
  `query_lowering`) each recurse exactly one level and silently stop. A
  general, pre-existing (since `AICAD-071`) Stage-2/3
  execution-completeness gap, not a Stage-4 semantic-reference defect (it
  would affect a program using no Stage-4 feature at all) — documented as
  a new non-decision item in `project/OWNER_DECISIONS.md` rather than
  fixed under this task's own narrower charter.

## Recommendation

**PASS** (reaffirmed — see `project/gates/stage-4-gate.md` for the full
reasoning, §10 specifically for this task's own before/after account).
This task does not, and per `AGENTS.md` cannot, approve Stage 4; only the
owner may record that decision in `project/DECISION_LOG.md`.

## Regressions

None. `cargo fmt`/`clippy`/`cargo test --workspace` clean at 1323 passed/0
failed/0 ignored (up from 1251 passed/1 ignored), every added test a
net-new positive proof, no existing assertion weakened. No frozen-corpus
fixture (public or held-out) was edited; held-out `MANIFEST.sha256`
checksums re-verify clean. `case01`'s own honest divergence from
`case.md`'s pre-resolver prose (§8 above) is documented, not force-matched
by editing either the fixture or the implementation.

## Verification

```
$ cargo fmt --all -- --check
(clean)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(zero warnings)

$ cargo test --workspace
1323 passed; 0 failed; 0 ignored, 0 measured, across 77 test binaries

$ python3 scripts/ci/semantic_ref_harness.py validate
{"cases": 10, "splits": {"held_out": 3, "public": 7}, "status": "ok"}

$ python3 scripts/ci/semantic_ref_harness.py self-test
{"status": "ok", "self_test": "silent-wrong gate exercised"}

$ python3 scripts/ci/stage4_task_audit.py --check
Stage-4 task metadata audit OK

$ (cd project/benchmarks/stage4_semantic_reference/held_out && sha256sum -c MANIFEST.sha256)
[all entries] OK

$ grep -rn "TopoDS\|occt::\|gp_Pnt\|BRepBuilder" crates/cad-query/src crates/cad-references/src
(no matches)
```

## Limitations

See `project/gates/stage-4-gate.md` §5/§10 for the full, disclosed list.
Genuinely remaining (by design, not oversight):

- §5.1: the unscoped `ParametricBuildSession::resolve` API can be
  spuriously ambiguous by design (every permanently-live top-level binding
  stays a candidate) — `Query::scoped_to` is the opt-in fix, unchanged
  since `AICAD-099A`.
- §5.5: fingerprint automatic recovery remains disabled (D7/`DL-8`);
  whether/when to enable it is an explicitly deferred future owner
  decision.
- §5.7 (new): `part`-in-`part` nesting is grammatically legal but silently
  inert — a pre-existing, non-blocking, general execution-completeness
  gap, not a Stage-4 semantic-reference defect.
- `area(...)` has no source-syntax mapping in `query_lowering` (no
  Area-dimensioned unit literal exists in the language at all yet — an
  existing RFC-0004 gap, not created by this task).
- Several plan-listed predicates (`adjacent_to`, `boundary`,
  `connected_to`, `contains`, `intersects`, `nearest_to`, `farthest_from`,
  `above`/`below`/`left`/`right`, `inside`, `within`, ranking
  `nearest`/`farthest`) have real Rust-level production semantics
  (`cad_query::eval`) but no `.aicad` source-syntax mapping yet — each
  needs either a nested query/reference argument or a `Point3`/`Frame3`
  literal shape this task's minimal clause grammar has no syntax for; a
  later authorized task may extend the vocabulary.

## Next dependency

Per `AGENTS.md`'s "FINAL STOP RULE": **STOP ROADMAP DEVELOPMENT.** No
future invocation may begin `AICAD-101`, finalize/activate a provisional
Stage-5 task queue as executable roadmap work, or otherwise expand into
Stage-5 scope without a separate, later, explicit owner approval recorded
in `project/DECISION_LOG.md`. Only the owner may review
`project/gates/stage-4-gate.md`, approve Stage 4, and authorize
`AICAD-101`+/Stage 5.
