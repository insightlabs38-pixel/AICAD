# RFC-0003: Semantic References

- Status: **Accepted Stage-0 design contract** (`project/DECISION_LOG.md#DL-10`).
- Implementation status: **Stage 4 has not started** on `claude/aicad-stage4-transition`; AICAD-080 remains todo.
- Owner rulings incorporated: DL-8 (D7 fail-closed reference resolution, partial) and DL-9 (D8 kernel-independent semantic graph, directional).
- Depends on: RFC-0002's kernel-neutral/epoch-local topology boundary.

## 1. Purpose

Persistent topological naming is a foundational CAD risk. AICAD therefore defines semantic topology references above the geometry kernel rather than treating kernel enumeration order, pointer identity, or operation-local lineage as durable source identity.

This RFC is a Stage-4 **target contract**, not a claim that `FaceRef`/`EdgeRef`/other persistent reference types already exist in the current Stage-3 implementation.

## 2. Stage-3 mechanisms are not Stage-4 references

Stage 3 already contains useful semantic identity/provenance mechanisms:

- source/binding/parameter identities;
- feature identities and feature/source provenance;
- named part/model outputs;
- GeometryGraph/GeomId execution identity;
- sketch/entity/constraint identities inside the Stage-3 semantic subsystem;
- operation-local kernel lineage where available;
- raw/index-based face/edge selection in limited modeling APIs.

These concepts serve different layers. None is automatically a persistent topology reference. In particular:

- `Part.body` selects a named source/model output, not a face or edge;
- `FaceIndex(3)` is an enumeration selector in one realized topology, not `FaceRef` identity;
- a Stage-3 sketch entity ID belongs to sketch IR, not arbitrary regenerated solid topology;
- operation-local lineage is evidence used by a resolver, not durable identity by itself.

## 3. Stage-4 semantic-reference contract

Stage 4 is responsible for durable, kernel-independent references to semantic topology classes such as vertex/edge/wire/face/shell/solid. Reference identity is owned by the AICAD semantic layer above the kernel.

A reference resolves against a model/build context through deterministic evidence/recipes rather than raw kernel pointer identity. The exact serialized recipe/schema is Stage-4 task scope, but it must preserve the following D7 contract.

### Fail-closed outcomes — D7/DL-8

Resolution may produce only:

- `Resolved` — exactly one valid entity satisfies the reference contract;
- `Ambiguous` — multiple candidates remain, with evidence/candidate information;
- `Broken` — no valid entity can satisfy the reference, with a reason/evidence.

A resolver must never silently choose an arbitrary candidate. Ambiguity is an observable semantic failure state, not permission to pick whichever face/edge happens to appear first.

## 4. Fingerprints and fallback

Geometry fingerprints may be useful evidence for diagnostics, candidate ranking shown to a user/tool, and benchmarking/experiments. They are **not** an automatic fallback in the first Stage-4 implementation.

Whether automatic fingerprint-based recovery may ever be enabled remains an explicit open D7 sub-question. A later owner decision must be supported by benchmark evidence showing an acceptably negligible silent-wrong-resolution rate. This cleanup does not change that policy.

An explicit authored geometric query/selector may participate in a future reference recipe when approved. That is not the same thing as silently invoking fingerprint recovery after another identity mechanism fails.

## 5. Kernel-independent semantic graph — D8/DL-9

AICAD owns a kernel-independent semantic graph that is authoritative for source/model identity, features, dependencies, and durable references. Public/serialized reference semantics must not depend on OCAF or any other OCCT-specific identity mechanism.

OCAF may still be useful internally for OCCT-side labeling/persistence/lineage experiments. The exact extent of internal OCAF use is intentionally still open and prototype-driven; it must not leak into the public reference contract.

## 6. Kernel handles and raw indices

Kernel shapes/edges/faces and future raw topology handles are epoch/context local. A raw handle may eventually be represented as an opaque AICAD-owned value, but:

```text
raw topology handle != OCCT pointer != persistent semantic reference
```

Likewise, current raw integer indices are temporary selectors and should be treated as fragile after topology-changing edits.

## 7. Source syntax/status boundary

Older Stage-0 examples and foundation-plan material show illustrative reference/query/export syntax. Those examples remain design targets, not literal current `.aicad` grammar, unless promoted into `specs/language/grammar.ebnf` by an authorized Stage-4 task.

`AICAD-100A` is that promotion for this section's own reserved `query { ... }` surface: `specs/language/grammar.ebnf`'s `query_decl` production (`query name : EntityKind in scope { clause* }`) is now real, parsed, lowered, and executable `.aicad` source syntax — see `cad_ast::item::Item::Query`'s own doc comment for the exact shape, and `crates/cad-cli/src/query_lowering.rs`'s own module doc comment for the closed clause-name vocabulary it currently interprets. This is the smallest completion consistent with this section's own promotion clause: no new expression syntax (comparison operators, a `within` modifier, direction literals) was introduced, `scope` is mandatory (never defaulting to the whole-session unscoped candidate universe — §3's fail-closed contract applies to a source-declared reference exactly as it does to one constructed directly against `cad-query`/`cad-references`), and a persistent reference declared this way is an ordinary typed `VertexRef`/`EdgeRef`/`WireRef`/`FaceRef`/`ShellRef`/`SolidRef` value (`cad_references::AnyRef`), never an OCCT-specific type.

Illustrative syntax this promotion does *not* cover (`adjacent_to`, `boundary`, `connected_to`, `contains`, `intersects`, `area`, spatial predicates needing a point/frame/nested-reference argument, ranking `nearest`/`farthest`, an `expose { ... }` block) remains a design target only, not current grammar, pending a later authorized promotion.

## 8. Diagnostics/tooling target

Stage-4 tooling should expose reference status/evidence in structured machine-readable form and support explicit checks for unresolved/ambiguous/broken references. `cad refs check` (`AICAD-095`, wired to real source-declared `query { ... }` references by `AICAD-100A`) is real, current CLI tooling meeting this target — not a planned concept — built through the same `ParametricBuildSession` pipeline `cad build` itself uses.

## 9. Mutation/regeneration durability

A durable semantic reference exists to survive or explicitly fail across model regeneration/topology changes. It must not quietly retarget because a kernel's enumeration order changed. Regeneration may preserve the reference (`Resolved`) or make it observably ambiguous/broken; silent arbitrary retargeting is forbidden.

## 10. Historical rationale and rejected alternatives

Stage-0 chose semantic references because direct kernel handles/indices cannot support robust parametric editing, reproducible automation, semantic diff/merge, or trustworthy AI-generated edits. The plan considered lineage, authored queries, naming/provenance, and geometric evidence as candidate ingredients rather than one universal magic identifier.

The later D7 ruling rejects best-effort automatic candidate selection for the first implementation, and D8 rejects making OCAF/public kernel identity authoritative. Those rulings narrow the original design risk without discarding the rationale.

## 11. Still-open questions

Still open: whether/when benchmark evidence can justify automatic fingerprint recovery; the exact internal use of OCAF; the exact Stage-4 reference-recipe/schema/API spelling; and any later reference mechanisms needed by Stage-5+ programmable geometry. Those questions must be resolved in their authorized stage rather than through this cleanup pass.
