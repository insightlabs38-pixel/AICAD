# AICAD-080: Define stable VertexRef/EdgeRef/WireRef/FaceRef/ShellRef/SolidRef semantic-recipe representations

## Status

Done. First task of Batch S4-00 (the first Stage-4 implementation batch).

## Objective

Implement the Rust-level **representation** of a Stage-4 stable reference —
"a reproducible semantic recipe plus lineage context"
(`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §2) — for the six
persistent topology classes, per `rfcs/0003-semantic-references.md`
(accepted Stage-0 design contract) and this task's own `project/TASKS.yaml`
acceptance list. No resolution algorithm, lineage capture, raw-handle
epoch, or health report — those are `AICAD-085` onward.

## Base / resulting commit

- Base: `f587251` (`origin/main` HEAD — the Stage-3→Stage-4 transition
  merge, `#12`), synchronized into this session's working branch at the
  start of this invocation.
- This task's commit: see `git log` (`AICAD-080` commit, `crates/
  cad-references`).

## What was implemented

`crates/cad-references` (previously an `AICAD-002` placeholder stub):

- **`EntityKind`** (`src/entity.rs`) — the six closed topology classes
  (`Vertex`/`Edge`/`Wire`/`Face`/`Shell`/`Solid`), with a stable
  machine-readable name.
- **`FeatureAnchor`** (`src/feature.rs`) — names a lineage anchor by
  stable *source-level binding name* (`Named(String)`) or `this`
  (`CurrentFeature`), never by `cad_feature_graph::FeatureId` (confirmed,
  by direct inspection, to be per-`FeatureGraph`-build SSA numbering, not
  reusable cross-build identity) or any raw topology index.
- **`DurabilityLevel`** (`src/durability.rs`) — the plan §11 five-level
  scale (`Raw < QueryGeometric < QueryStrong < Lineage < Explicit`),
  totally ordered for future reporting (`AICAD-095`).
- **`FingerprintEvidence`** (`src/fingerprint.rs`) — plain, untyped
  position/normal/area/radius evidence for the fingerprint strategy,
  deliberately not `cad_units`-typed (the plan's own "approximate...
  fallback discriminator" framing).
- **`ConstructionStrategy`** (`src/recipe.rs`) — all seven plan §3
  strategies (`ExplicitExport`, `FeatureLineage`, `SemanticQuery`,
  `StructuralRole`, `Ancestry`, `GeometricFingerprint`, `UserConfirmed`),
  each with a `durability()` that is a fixed function of the strategy
  shape (never an independently-settable field a caller could set too
  high). `ConstructionStrategy::semantic_query` is a smart constructor
  that rejects `Explicit`/`Lineage`/`Raw` durability for a query-backed
  strategy (`InvalidSemanticQueryDurability`), so a query can never
  masquerade as feature-lineage- or export-level evidence — this task's
  own type-level enforcement of the D7/`DL-8` fail-closed policy, ahead
  of `AICAD-091`'s later reporting-layer version of the same guarantee.
- **`VertexRef`/`EdgeRef`/`WireRef`/`FaceRef`/`ShellRef`/`SolidRef`**
  (`src/refs.rs`) — six distinct newtypes over one shared
  `ReferenceRecipe`, each constructible only via its own `from_strategy`,
  which fixes `entity_kind` correctly (no public way to build a recipe
  whose kind disagrees with its wrapper type). Plus `AnyRef`, a sum type
  over the six, for contexts needing "some reference, kind not yet fixed"
  (used by `ConstructionStrategy::Ancestry` and, in `AICAD-081`, by
  `cad-query`'s topology predicates).
- **Deterministic canonical serialization** (`src/serialize.rs`) — reuses
  the workspace's existing dependency-free `cad_diagnostics::json::Json`
  canonical writer (no serde or other third-party (de)serialization crate
  exists anywhere in this workspace; introducing one was not necessary and
  would have been a bigger, unrequested dependency decision). A golden
  fixed-string test locks the exact canonical shape.

## Design decisions

1. **Six distinct newtypes over one shared recipe, not six independent
   structs.** Satisfies "structurally distinct where the approved model
   requires it" (a `FaceRef` cannot be passed where an `EdgeRef` is
   expected — proven by a compile-time-property test) while sharing one
   `ConstructionStrategy`/durability implementation, avoiding six-way
   duplication `AGENTS.md`'s "smallest correct solution" guidance weighs
   against.
2. **`cad-references` depends on nothing but `cad-diagnostics`(for
   `Json`); it does not depend on `cad-query`.** `AICAD-081`'s own
   `depends_on: AICAD-080` means `cad-query` needs `cad-references`'
   vocabulary (`FeatureAnchor`, `AnyRef`) for its predicates — the
   opposite direction would be a dependency cycle, since `cad-query`
   needs those types too. Consequently `ConstructionStrategy::
   SemanticQuery` stores a `QueryHandle` (a stable name) rather than a
   literal `cad_query::Query` — see `src/recipe.rs`'s own doc comment.
   This is an ordinary architecture choice within this task's approved
   scope, not a `D7`/`D8` escalation: it changes nothing about resolution
   policy or kernel independence, only which crate physically owns the
   query AST.
3. **`StructuralRole` is an open `String` tag, not a closed enum.** The
   plan's own examples ("outer boundary," "top mounting surface," "bore
   axis") are illustrative, not a frozen vocabulary; freezing one now
   would be new language-adjacent semantics this task's `escalate_if`
   list does not authorize.
4. **No read-side JSON parser for `ReferenceRecipe`.** Only
   `to_json`/`to_canonical_string` are implemented (write side). No
   current consumer needs to reload a persisted recipe; a future task
   that does can add a complementary `from_json`, following the
   `cad_diagnostics::json::Json::parse` precedent this task already
   reuses for canonical writing.

## Tests / verification

- `cargo fmt --all -- --check` → clean (whole workspace).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings (whole workspace).
- `cargo test -p cad-references` → 18/18 passed, including: every
  strategy's durability mapping; `semantic_query`'s accept/reject
  boundary (positive: `QueryStrong`/`QueryGeometric`; negative:
  `Explicit`/`Lineage`/`Raw` all rejected); each of the six wrapper types
  fixes the correct `EntityKind`; a compile-time-property test that
  `FaceRef`/`EdgeRef` are not interchangeable; canonical-serialization
  determinism (repeated-call byte-identity, a golden fixed string, and
  structurally-different recipes serializing differently).
- `cargo test --workspace` → all crates green (1,088 total test-result
  lines, 0 failed), no regressions in any existing crate.
- `python3 scripts/ci/semantic_ref_harness.py validate` →
  `{"cases": 10, "splits": {"held_out": 3, "public": 7}, "status": "ok"}`
  (frozen `AICAD-079A` corpus untouched).
- `python3 scripts/ci/stage4_task_audit.py --check` → `Stage-4 task
  metadata audit OK`.

## Limitations

- No resolution algorithm exists (by design — `AICAD-088` onward). This
  task cannot itself be evaluated against `tests/semantic_refs/`'s
  outcome-class contract or the `project/benchmarks/
  stage4_semantic_reference/` corpus; it only defines the vocabulary a
  future resolver will consume.
- `FeatureAnchor::Named` assumes a single flat source-level binding
  namespace. Whether a future nested/anonymous-feature naming scheme
  needs a richer anchor (e.g. a dotted path) is left to whichever later
  task (`AICAD-085`+, lineage capture) first needs one.
- `ConstructionStrategy::UserConfirmed`'s `confirmation_note` is a free
  string, not a structured audit record (who confirmed, when, against
  which candidate set) — this task defines the recipe shape recording
  that a confirmation occurred, not a confirmation workflow.

## Regressions

None. All pre-existing tests unaffected.

## Next dependency

`AICAD-081` (query AST/IR), which `depends_on: AICAD-080` — implemented
in this same invocation, see `project/reports/AICAD-081.md`.
