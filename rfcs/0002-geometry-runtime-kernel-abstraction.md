# RFC-0002: Geometry/Runtime and Kernel Abstraction

- Status: **Accepted Stage-0 baseline** (`project/DECISION_LOG.md#DL-10`).
- Owner rulings incorporated/clarifying this RFC: DL-5 (D6 kernel boundary/lineage), DL-6 (development-phase OCCT/licensing policy), DL-12/DL-17+AICAD-064A (determinism/equivalence), DL-15 (RuntimeBuiltin source invocation), DL-21 (type-closed standard builtin environment).
- Depends on: RFC-0001 and the canonical current language specs under `specs/language/`.

## 1. Summary

AICAD separates source-language semantics, backend-neutral geometry intent, a kernel-neutral operation API, and the concrete OCCT adapter. Exact B-rep is the primary compiled geometry representation, but kernel object identity is never allowed to become language/HIR identity.

This RFC also preserves a deliberate distinction between safe source modeling and future raw/unsafe topology capability. The semantic requirement for a raw tier is accepted; exact Stage-5 source syntax for that tier is **not** defined by this Stage-3 cleanup.

## 2. Layering and authority

The current geometry path is:

```text
.aicad source
  -> typed HIR / ordinary RuntimeBuiltin calls
  -> backend-neutral GeometryGraph / Geometry IR
  -> geometry runtime dispatcher
  -> cad-kernel-api
  -> cad-occt-bridge
  -> native OCCT bridge
  -> exact B-rep
```

A higher layer may depend only on the stable AICAD/kernel-neutral contract below it. OCCT classes, `TopoDS_*` objects, native pointers, OCAF labels, and kernel enumeration order are implementation details and may not be public/HIR semantic identities.

## 3. Runtime-backed source modeling

The current Safe CAD source surface uses D18's RuntimeBuiltin mechanism: ordinary `.aicad` calls resolve/type-check/lower as ordinary calls, then append backend-neutral geometry operations. The human-readable catalogue is `docs/developer/geometry/safe-cad-api.md`.

The public catalogue is intentionally narrower than Geometry IR or the kernel adapter. Internal availability of an operation/query does not automatically expose a source function. Source-visible kernel queries that influence control flow require a separate future evaluation-architecture decision; this RFC does not invent one.

## 4. Kernel-neutral public boundary — D6/DL-5

`cad-kernel-api` is capability-driven and kernel-neutral. The adapter may expose the minimum operation-local lineage evidence required by higher semantic layers, but durable identity is owned above the kernel.

Terms such as `KernelShape*`, `Face*`, `Edge*`, or a native OCCT address must not define the public/HIR model. Where future low-level/raw geometry requires a topology handle, the semantic requirement is an **opaque AICAD-owned epoch/context-local handle**. Such a raw handle is not an OCCT pointer and is not a persistent semantic topology reference. Exact Stage-5 spelling/source syntax remains deferred.

## 5. Raw/unsafe capability tier

The accepted architecture retains a future low-level/raw tier for nearly kernel-complete advanced geometry. Its invariants are:

- raw topology is explicit rather than silently treated as durable identity;
- raw handles are valid only in their owning kernel context/epoch;
- they are kernel-neutral AICAD abstractions, never public OCCT pointers;
- operations that cross from raw/unchecked geometry into trusted higher-level geometry must use explicit validation/adoption semantics;
- source-visible syntax/effect spelling is not frozen by this cleanup.

Older Stage-0 examples using `unsafe geometry { ... }`, `validate(...)`, or `adopt_validated(...)` document the intended **separation of trust tiers**, not current `.aicad` syntax. The current parser exposes no general `unsafe geometry` construct. Stage-5 must decide whether the boundary is expressed through an unsafe/effect construct, opaque raw types/namespaces, or another approved mechanism before implementation.

Validation/adoption itself follows value semantics: it returns a validated/adopted value; it does not semantically mutate an existing source object in place.

## 6. Topology identity and lineage

Current Stage-3 raw `EdgeIndex`/`FaceIndex` values are enumeration selectors into realized topology. They are topology/epoch local and fragile across topology-changing rebuilds.

Operation-local lineage evidence may be propagated upward from the kernel adapter where useful. That evidence is not itself persistent identity. Stage-3 feature IDs/provenance/named outputs likewise do not become face/edge identity merely because they are stable within their own semantic domains.

Persistent fail-closed semantic topology references are Stage-4 work governed by RFC-0003 and D7. AICAD-080 remains unstarted on the transition branch.

## 7. Exact geometry, canonical state, and determinism

Source/AICAD-owned semantics are canonical. Generated exact B-rep is the canonical compiled geometry representation but not a source of language identity.

D5/D19 resolve the old determinism question:

- AICAD-owned canonical semantic/compiler serialization is deterministic/byte-identical where defined.
- Same-kernel and supported cross-platform geometry use validity plus versioned semantic/numerical equivalence rather than byte-identical B-rep.
- Kernel face/edge enumeration order is never canonical identity.
- Kernel-version changes are compatibility comparisons, not deterministic identity.

The D5 profile is solely an equivalence/comparison profile. It does not silently define solver convergence, OCCT construction tolerances, approximation tolerances, assertion tolerances, or private geometric validity epsilons.

## 8. OCCT isolation and lifecycle

Only the OCCT adapter/native bridge may include/own OCCT-specific classes and state. Higher Rust layers use kernel-neutral values and operations. Native context lifetime/resource safety is an implementation responsibility and must never leak raw ownership/pointer semantics into source/HIR.

OCAF may be prototyped as an internal OCCT-side persistence/labeling/lineage aid, but D8/DL-9 requires the AICAD kernel-independent semantic graph to remain authoritative. The exact internal extent of OCAF remains open and prototype-driven.

## 9. Licensing boundary — D13/DL-6

OCCT is an external dependency behind the adapter; AICAD does not incorporate OCCT implementation source as AICAD-owned code. Applicable notices/licensing must be preserved and the architecture must permit compliant shared/dynamic-library distribution. Standards-derived functionality must be independently implemented without copying protected ISO/ASME prose/tables/figures absent appropriate licensing.

A formal public/commercial distribution licensing review remains an explicit future gate; DL-6 is a development-phase policy, not that final review.

## 10. Historical rationale and alternatives

Stage-0 deliberately rejected exposing concrete kernel classes/pointers above the bridge because doing so would couple source semantics, testing, serialization, and a future second kernel to OCCT representation details. It also rejected treating mesh output as design truth and rejected conflating raw topology handles with persistent semantic references.

The accepted tiering keeps ordinary modeling ergonomic while preserving an escape hatch for future advanced geometry. This cleanup narrows only the unsupported **syntax claims** around the future raw tier; it does not remove that capability from the architecture.

## 11. Still-open boundaries

Still unresolved here: exact Stage-5 raw/unsafe source mechanism; source-visible kernel-query execution/evaluation model; modeling/construction/approximation tolerance defaults; exact internal OCAF usage; trusted plugin/native extension boundaries; and the final public-distribution license policy. None may be silently decided by implementing Stage 4 or by this specification cleanup.
