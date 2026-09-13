# RFC-0003: Semantic References

- Status: Draft (Stage 0)
- Stage-0 build item covered (`docs/plan/15_IMPLEMENTATION_ROADMAP.md`
  Stage 0): "feature/reference semantics".
- Owner rulings incorporated: DL-8 (reference-resolution fail-closed
  policy, partial), DL-9 (kernel-independent semantic graph, directional).
  See `project/DECISION_LOG.md`.
- Depends on: RFC-0002 (kernel handles are epoch-local and own no durable
  identity; this RFC defines what does).

## 1. Summary

This is the single most consequential RFC in the plan. `docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md`
Risk A names persistent topological naming as "the largest foundational
CAD risk," and `docs/plan/15_IMPLEMENTATION_ROADMAP.md` makes Stage 4 — the
stage whose sole purpose is proving this RFC's contract holds — a **hard
release gate**. Nothing here may be weakened without triggering the
AGENTS.md escalation trigger "semantic-reference ambiguity would require
arbitrary fallback."

## 2. Reference classes (frozen, from `06` §2)

- **Stable semantic references** (`VertexRef`, `EdgeRef`, `WireRef`,
  `FaceRef`, `ShellRef`, `SolidRef`, `AxisRef`, `FeatureRef`,
  `ComponentRef`) do not mean "store a kernel pointer forever." They mean
  "store a reproducible semantic recipe plus lineage context for resolving
  the intended entity after regeneration."
- **Raw handles** (`Vertex*`, `Edge*`, `Face*`, ...) represent current
  topology in the current geometry epoch only (RFC-0002 §4). They are
  never a substitute for a stable reference.

## 3. Reference construction strategies (frozen, from `06` §3)

A stable reference derives from one or more of: (1) feature lineage, (2)
explicit export, (3) semantic query, (4) structural role, (5) ancestry,
(6) geometric fingerprint (as an **explicit, source-authored**
discriminator — see §6's distinction from *automatic* fingerprint
fallback), (7) user confirmation when ambiguity cannot otherwise be
solved.

## 4. Explicit semantic exports and query model (frozen, from `06` §4-6)

Features export named semantic outputs via `expose { ... }` blocks
resolved by query criteria (`06` §4). Queries are **criteria objects**, not
materialized lists, so they can be re-evaluated after regeneration (`06`
§5). The geometry/topology/spatial predicate families and
ranking/disambiguation operators in `06` §6 (`planar`, `generated_by(...)`,
`adjacent_to(...)`, `largest(area)`, `unique()`, `expect_count(n)`, etc.)
are adopted as the baseline predicate vocabulary; RFC-0004 governs the
unit-bearing comparison syntax (`~= within`) these predicates use.

## 5. Feature DAG and topology lineage (frozen, from `06` §8-10)

- Every topology-changing operation reports lineage
  (`old entity -> unchanged/new/modified/split/merged/deleted -> new
  entities`), stored **at the feature node**, not only inside backend-native
  structures (this is the layer that consumes the per-operation lineage
  evidence RFC-0002 §3 requires the kernel adapter to expose).
- The feature DAG need not be linear; each node records id, source span,
  kind, parameters, input feature/semantic refs, input external assets,
  geometry/semantic outputs, lineage data, validation result, cache key,
  and provenance (`06` §9).
- Incremental invalidation on parameter change: find dependents, mark
  dirty, preserve unaffected cache, rebuild the dirty subgraph, **replay
  semantic-reference resolution**, compare old/new lineage, rerun only
  affected requirements/tests (`06` §10). This is the same contract
  RFC-0002 §5 already binds `cad-feature-graph` to; RFC-0003 adds that
  reference replay specifically is part of it.

## 6. Resolution policy: fail-closed (DL-8 / D7, resolved for Stage 4)

Reference resolution has exactly three possible outcomes — never a fourth:

```text
Resolved(entity)              — exactly one entity matched
Ambiguous(candidates, evidence) — more than one entity matched
Broken(reason)                — zero entities matched, or resolution
                                 could not be attempted (e.g. stale lineage)
```

**The resolver must never select an arbitrary "best" candidate to force a
`Resolved` outcome.** This is the direct implementation of
`docs/plan/00_PRINCIPLES_AND_SCOPE.md` §3 rule 8 and
`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §7 ("ambiguity must be
explicit").

### 6.1 Geometry-fingerprint fallback is disabled as an *automatic* recovery path

A critical distinction, because `06` §3 item 6 and `06` §11's
`query_geometric` durability tier both mention geometric-fingerprint-style
criteria and could otherwise be read as endorsing automatic use:

- **Explicit, source-authored geometric queries remain fully supported and
  unrestricted.** If an author writes
  `query body.faces { cylindrical; radius ~= 11mm within 0.01mm; }`, that
  is an ordinary query (durability tier `query_geometric`, §7 below) and
  resolves through the same three-outcome contract as any other query. The
  program asked for exactly this discriminator; using it is not "silent."
- **What is disabled** is the *resolver* itself automatically retrying an
  otherwise-`Ambiguous` or `Broken` resolution by inventing a
  geometry-fingerprint comparison the source program never asked for, in
  order to still produce a `Resolved` outcome. That automatic-recovery
  behavior — the specific mechanism task AICAD-092 names
  ("geometry-fingerprint fallback") — does not exist in the first Stage-4
  implementation.
- Fingerprinting may still be used, in the first implementation, for:
  **diagnostics** (showing candidates a user could disambiguate with),
  **ranking** candidates presented to a human/agent for manual
  confirmation (strategy 7, §3), and **experiments/benchmarking**
  (`benchmarks/topology-naming/`) — never to silently change a resolution
  outcome on its own.
- **Automatic fingerprint-based recovery can only be enabled later by a
  separate, future owner-approved policy**, supported by benchmark
  evidence (from `benchmarks/topology-naming/`, AICAD-096-099) showing an
  acceptably negligible silent-wrong-resolution rate. This RFC does not
  pre-approve that future policy or its threshold; both remain open.

## 7. Reference durability levels (frozen, from `06` §11-12)

| Level | Meaning |
|---|---|
| `explicit` | Feature exported the entity by semantic name. |
| `lineage` | Resolved through tracked creation/modification history. |
| `query_strong` | Unique result from robust semantic/topological criteria. |
| `query_geometric` | Uses author-written geometry-fingerprint/spatial criteria (§6.1 — explicit use, not automatic fallback). |
| `raw` | Current topology only; not persistent. |

`cad refs check` (`06` §12) reports the distribution of these levels across
a project plus ambiguous/broken counts, and is required Stage-4 tooling
(AICAD-095).

## 8. Kernel-independent semantic graph is authoritative (DL-9 / D8, directional)

- **AICAD owns a kernel-independent semantic graph** that is authoritative
  for language-level identity, features, dependencies, and durable
  references (`cad-references`). This is not an OCAF wrapper.
- **OCAF may be prototyped and used internally** as an OCCT-side
  persistence, labeling, or lineage aid, entirely inside
  `cad-occt-bridge`/`native/occt_bridge` — never surfaced through
  `cad-kernel-api` (RFC-0002 §3).
- **AICAD's public semantics and any serialized semantic identity must
  never depend on OCAF.** A future non-OCCT kernel backend must be able to
  implement `cad-kernel-api`'s lineage-evidence contract (RFC-0002 §3)
  without OCAF existing at all.
- **Whether OCAF is used internally, and for which subset of
  responsibilities, remains prototype-driven** and is explicitly not
  decided by this RFC. Prototyping work belongs under
  `project/experiments/` (see its `README.md`) and must produce evidence
  before `cad-occt-bridge`/`cad-references` commit to a specific internal
  persistence mechanism.

## 9. Alternatives considered

- **A fourth resolution outcome** (e.g. "best-effort resolved with a
  confidence score") — rejected (DL-8); this is exactly the arbitrary-
  selection behavior invariant 8 forbids, merely relabeled.
- **Enabling automatic geometry-fingerprint fallback from the start** —
  rejected (DL-8); this is the specific "silent wrong resolution" failure
  class the Stage-4 hard gate exists to eliminate
  (`docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md` §5).
- **Building semantic identity directly on OCAF** — rejected (DL-9); would
  violate the kernel-independence contract (RFC-0002 §3) and couple
  durable references to one backend's persistence framework.
- **Banning OCAF outright even as an internal implementation detail** —
  rejected (DL-9); forecloses a legitimate prototyping question (does OCAF
  help implement lineage evidence inside the OCCT bridge?) without
  evidence either way.

## 10. Open questions (intentionally not resolved here)

- Exact resolution precedence when multiple construction strategies (§3)
  could apply to the same entity — this RFC fixes the *outcome contract*
  (§6) but not a full precedence ordering across all seven strategies;
  that ordering is Stage-4 implementation work (AICAD-088) informed by the
  fixture corpus (AICAD-096), not a Stage-0 freeze.
- The extent of internal OCAF usage (§8) — explicitly prototype-driven,
  not decided here.
- Future automatic fingerprint-recovery policy and its acceptable
  silent-wrong-resolution threshold (§6.1) — explicitly deferred to a
  future, separate owner decision.

## 11. Impact

- `crates/cad-references`, `crates/cad-query`: implement §2-8 in Stage 4
  (AICAD-080..095).
- `crates/cad-feature-graph`: implements the lineage-storage and
  invalidation-replay contract in §5 (Stage 3, then extended Stage 4).
- `benchmarks/topology-naming/`: implements the fixture/perturbation/
  metrics work (AICAD-096-099) that is the actual evidence this RFC's
  fail-closed contract holds.
- `crates/cad-occt-bridge`: any internal OCAF prototyping (§8) stays
  behind this crate's boundary and is tracked via `project/experiments/`.
