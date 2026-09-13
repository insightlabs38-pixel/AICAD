# RFC-0002: Geometry/Runtime and Kernel Abstraction

- Status: **Accepted Stage-0 baseline** (`project/DECISION_LOG.md#DL-10`).
- Owner rulings incorporated/clarifying this RFC: DL-5 (D6 kernel boundary/lineage), DL-6 (development-phase OCCT/licensing policy), DL-12/DL-17+AICAD-064A (determinism/equivalence), DL-15 (RuntimeBuiltin source invocation), DL-21 (type-closed standard builtin environment), DL-23 (D21 closed-catalogue scaling), DL-24 (D22 safe/raw geometry tiers), DL-25 (D23 source-visible kernel-backed queries), DL-26 (D24 tolerance domains), and DL-27 (D25 feature/dependency/provenance observability through abstraction).
- Depends on: RFC-0001 and the canonical current language specs under `specs/language/`.

## 1. Summary

AICAD separates source-language semantics, backend-neutral geometry intent, a kernel-neutral operation API, and the concrete OCCT adapter. Exact B-rep is the primary compiled geometry representation, but kernel object identity is never allowed to become language/HIR identity.

This RFC also preserves a deliberate distinction between safe source modeling and future raw/unsafe topology capability. D22/DL-24 freezes the semantic separation and explicit validation/adoption requirement; exact Stage-5 source syntax, raw-handle representation, and epoch encoding remain deferred.

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

D21/DL-23 permits future growth of that closed first-party catalogue through a declarative/single-source description while preserving D18/D20 ordinary calls, type closure, eager signature checking, and deterministic deliberately managed builtin identity. This does not create an open native/plugin registration ABI; exact catalogue-generation mechanics remain deferred.

The public catalogue is intentionally narrower than Geometry IR or the kernel adapter. Internal availability of an operation/query does not automatically expose a source function. D23/DL-25 resolves the architecture for future source-visible kernel-backed queries needed during ordinary evaluation: they remain ordinary typed runtime-backed calls, and the runtime may synchronously materialize the minimum required upstream geometry and execute the query through the kernel-neutral adapter before evaluation continues. Exact query-effect metadata, eager/lazy scheduling, cache representation, and Stage-5 API surface remain deferred.

## 4. Kernel-neutral public boundary — D6/DL-5

`cad-kernel-api` is capability-driven and kernel-neutral. The adapter may expose the minimum operation-local lineage evidence required by higher semantic layers, but durable identity is owned above the kernel.

Terms such as `KernelShape*`, `Face*`, `Edge*`, or a native OCCT address must not define the public/HIR model. D22/DL-24 requires any future low-level/raw geometry topology handle to be an **opaque AICAD-owned epoch/context-local handle**. Such a raw handle is not an OCCT pointer and is not a persistent semantic topology reference. Exact Stage-5 spelling, handle representation, and epoch encoding remain deferred.

## 5. Raw/unsafe capability tier — D22/DL-24

The accepted architecture retains a future low-level/raw tier for nearly kernel-complete advanced geometry. Its invariants are:

- safe semantic geometry, raw/unsafe handles, persistent semantic topology references, and kernel-native objects are distinct concepts;
- raw topology is explicit rather than silently treated as durable identity;
- raw handles are valid only in their owning kernel context/epoch and are explicitly invalid afterward;
- they are kernel-neutral AICAD abstractions, never public OCCT pointers;
- operations that cross from raw/unchecked geometry into trusted higher-level geometry must use explicit validation/adoption semantics that produce a new safe value and preserve appropriate validation/provenance evidence;
- source-visible syntax/effect spelling, raw representation, and epoch encoding are not frozen by this ruling.

Older Stage-0 examples using `unsafe geometry { ... }`, `validate(...)`, or `adopt_validated(...)` document the intended **separation of trust tiers**, not current `.aicad` syntax. The current parser exposes no general `unsafe geometry` construct. Stage 5 may decide whether the boundary is expressed through an unsafe/effect construct, opaque raw types/namespaces, explicit capability blocks, or another approved mechanism, informed by Stage-4 evidence.

Validation/adoption itself follows D2 value semantics: it returns a new validated/adopted safe value; it does not semantically mutate an existing source object in place. Internal kernel mutation remains an implementation detail below the public semantic boundary.

## 6. Topology identity, lineage, and abstraction — D25/DL-27

Current Stage-3 raw `EdgeIndex`/`FaceIndex` values are enumeration selectors into realized topology. They are topology/epoch local and fragile across topology-changing rebuilds.

Operation-local lineage evidence may be propagated upward from the kernel adapter where useful. That evidence is not itself persistent identity. Stage-3 feature IDs/provenance/named outputs likewise do not become face/edge identity merely because they are stable within their own semantic domains.

D25 requires future supported ordinary language abstractions to preserve engineering observability: wrapping geometry-producing/effectful operations in user functions, reusable helpers/parts, supported control flow, or libraries must not inherently erase feature identity, dependency/parameter information, dirty propagation, incremental rebuild behavior, source provenance, semantic-reference support, or source-to-geometry inspection. The exact graph encoding, node granularity, call-instance identity, and provenance serialization remain deferred and may be refined using Stage-4 evidence.

Persistent fail-closed semantic topology references are Stage-4 work governed by RFC-0003 and D7. AICAD-080 remains unstarted on the transition branch.

## 7. Exact geometry, canonical state, and tolerance domains

Source/AICAD-owned semantics are canonical. Generated exact B-rep is the canonical compiled geometry representation but not a source of language identity.

D5/D19 resolve the old determinism question:

- AICAD-owned canonical semantic/compiler serialization is deterministic/byte-identical where defined.
- Same-kernel and supported cross-platform geometry use validity plus versioned semantic/numerical equivalence rather than byte-identical B-rep.
- Kernel face/edge enumeration order is never canonical identity.
- Kernel-version changes are compatibility comparisons, not deterministic identity.

D24/DL-26 makes the broader tolerance taxonomy explicit. Representation/validity, modeling/construction, approximation, solver, verification, and D5 equivalence/comparison tolerances are distinct policy domains. They may interact but are not aliases; one domain's values must not silently become another's defaults, and no tolerance may be silently widened merely to make a failed operation/test pass. Existing D5/D19 values remain unchanged. Numerical defaults for future modeling/construction/approximation/solver/verification policies remain separately deferred.

## 8. OCCT isolation and lifecycle

Only the OCCT adapter/native bridge may include/own OCCT-specific classes and state. Higher Rust layers use kernel-neutral values and operations. Native context lifetime/resource safety is an implementation responsibility and must never leak raw ownership/pointer semantics into source/HIR.

OCAF may be prototyped as an internal OCCT-side persistence/labeling/lineage aid, but D8/DL-9 requires the AICAD kernel-independent semantic graph to remain authoritative. The exact internal extent of OCAF remains open and prototype-driven.

## 9. Licensing boundary — D13/DL-6

OCCT is an external dependency behind the adapter; AICAD does not incorporate OCCT implementation source as AICAD-owned code. Applicable notices/licensing must be preserved and the architecture must permit compliant shared/dynamic-library distribution. Standards-derived functionality must be independently implemented without copying protected ISO/ASME prose/tables/figures absent appropriate licensing.

A formal public/commercial distribution licensing review remains an explicit future gate; DL-6 is a development-phase policy, not that final review.

## 10. Historical rationale and alternatives

Stage-0 deliberately rejected exposing concrete kernel classes/pointers above the bridge because doing so would couple source semantics, testing, serialization, and a future second kernel to OCCT representation details. It also rejected treating mesh output as design truth and rejected conflating raw topology handles with persistent semantic references.

The accepted tiering keeps ordinary modeling ergonomic while preserving an escape hatch for future advanced geometry. D22 strengthens that semantic boundary without prematurely freezing its source spelling or representation.

## 11. Remaining deferred/open boundaries

D21-D25 resolve the previously open semantic baselines for closed catalogue scaling, raw/safe geometry separation, source-visible query execution, tolerance-domain ownership, and feature/provenance preservation through abstraction. Their explicitly deferred implementation details remain future work and are not authorized merely by these decisions.

Still genuinely open at owner level here: D8's exact internal OCAF usage, D12/D15 trusted/plugin/native extension boundaries, and D13's final public-distribution license policy. Stage-5 raw syntax/representation, query scheduling/cache/API details, future tolerance numeric defaults, and feature-graph encoding are deliberately deferred implementation/specification details under D21-D25 rather than unresolved semantic baselines. None may be silently decided by implementing Stage 4.
