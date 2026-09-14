# cad-references

WP-07 (Semantic references / queries). **Gate: this work package must
reach benchmark quality (Stage 4, the topological-naming hard gate) before
broad high-level feature expansion.**

`AICAD-080` implements the stable reference **representation**: `VertexRef`/
`EdgeRef`/`WireRef`/`FaceRef`/`ShellRef`/`SolidRef`, each a distinct type
wrapping a `ReferenceRecipe` (one of the plan's seven construction
strategies, plus the durability level it implies). No resolution algorithm,
lineage capture, raw-handle epoch, or health report is implemented yet —
those are `AICAD-085` onward. See `src/lib.rs`'s own module doc comment for
the exact scope boundary and `project/reports/AICAD-080.md` for this task's
evidence.

Resolution precedence and geometry-fingerprint fallback policy remain
partially open owner decisions — see `project/OWNER_DECISIONS.md` D7
(`project/DECISION_LOG.md#DL-8`) and D8 (`project/DECISION_LOG.md#DL-9`).
This crate's `ConstructionStrategy::GeometricFingerprint` and
`ConstructionStrategy::semantic_query`'s durability restriction already
enforce D7's fail-closed policy at the type level (fingerprint/weak-query
evidence can never be constructed with `explicit`/`lineage` durability).

Plan references: `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md`;
`docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-07;
`rfcs/0003-semantic-references.md`.
