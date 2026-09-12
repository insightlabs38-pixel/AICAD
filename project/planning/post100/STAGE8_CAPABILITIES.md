# Stage 8 capability envelope — artifacts, interchange, reproducibility

**Planning status:** capability envelope only. Do **not** convert this file into a fixed task queue until the Stage-7 owner gate is complete and the artifact decisions below are resolved.

## Goal

Stage 8 should make an AICAD project/build portable, reproducible, inspectable and interoperable without changing the source-authoritative model. Its job is not to make a native B-rep file the new source of truth. The native artifact should package/refer to the information required to reconstruct and verify semantic state, while derived caches and external interchange remain explicitly derived/provenanced.

## Dependencies inherited from Stages 4-7

### From Stage 4

- semantic-reference recipe and resolver semantics;
- durability/fail-closed behavior;
- raw-handle epochs remain explicitly nonpersistent;
- fingerprint policy and reference diagnostics.

### From Stage 5

- kernel-neutral advanced geometry and topology semantics;
- safe/raw adoption boundary;
- lineage through advanced topology operations;
- typed numerical policies and structured operation/healing evidence;
- robust import/export/query foundations.

### From Stage 6

- component/instance/configuration identity;
- minimal external-asset identity/provenance for imported components;
- deterministic resolved assembly semantics;
- vendor/imported component handling.

### From Stage 7

- stable requirement/test/case IDs;
- reproducible build/profile/configuration identity;
- versioned verification evidence schema;
- traceability and machine-readable results.

Stage 8 should **serialize/package these established semantics**, not redefine them.

## Required capability groups

### 1. Native AICAD artifact/package envelope

A native artifact should have a canonical, versioned envelope. The current recommended naming direction is `.aicadpkg`, subject to final owner confirmation under D14/OD-S8-01.

Likely core properties:

- format/magic/schema version;
- artifact kind/purpose if one envelope serves both project snapshots and distributable packages;
- project/package identity and version where applicable;
- source file set or source-content references;
- normalized manifest metadata;
- lock/environment identity;
- compiler/language version;
- geometry runtime/kernel adapter identity needed by reproducibility policy;
- external asset records/digests/settings;
- package/dependency graph and digests;
- selected build/profile/configuration identities;
- structured verification evidence or content-addressed references to it;
- optional derived caches with explicit validity keys;
- provenance/audit metadata allowed by policy.

**Architectural rule:** an optional B-rep cache, tessellation, thumbnail, search index or other acceleration payload may be absent or invalidated without changing source semantics.

### 2. Manifest and schema/version model

The native format needs a versioning strategy before bytes are frozen:

- schema version is explicit and machine checked;
- unknown required fields/features fail clearly rather than being ignored silently;
- optional forward-compatible sections have explicit extension semantics;
- canonical field ordering/normalization exists where byte/content identity matters;
- migrations are explicit transformations between format/language/model versions, never implicit reinterpretation;
- format compatibility is distinct from source-language compatibility.

Avoid exposing internal HIR/GeometryOp layout as the artifact schema. Internal IR is allowed to evolve without artifact/source breakage.

### 3. Lock and environment identity

Reproducibility needs enough identity to explain why two builds may differ. The lock/environment record should be capability-driven, likely including:

- AICAD compiler/runtime version or content/build identity;
- package dependency versions/digests;
- kernel adapter/backend identity and relevant version;
- external assets with content digests;
- import/healing settings;
- verification profile identity;
- configuration/parameter case identity for produced evidence;
- optional platform/toolchain metadata where it can alter accepted semantics.

Do **not** promise byte-identical native B-rep output across environments unless a future decision explicitly requires it. D5 already distinguishes semantic/numerical equivalence from byte identity.

### 4. Deterministic packaging

Given the same authoritative inputs and the same packaging-version policy, the logical package manifest/content identity should be deterministic. Achieving this likely requires:

- normalized path representation;
- deterministic file/member ordering;
- no wall-clock timestamp in canonical identity unless explicitly excluded from hash;
- canonical encoding of maps/sets;
- normalized metadata fields;
- content digests over immutable members;
- bounded compression variability or hashing of canonical uncompressed content;
- clear separation between canonical identity and transport-level container bytes if compression metadata can vary.

### 5. External asset provenance

Stage 6 should already provide a minimal external-asset record. Stage 8 formalizes its serialization and portability:

- stable logical asset ID;
- content digest;
- original locator/origin metadata, treated as provenance rather than identity unless policy says otherwise;
- importer type/version;
- import options/unit assumptions;
- healing/tolerance policy;
- fidelity/healing report;
- license/security metadata where later package policy requires it.

A filesystem path alone is not reproducible identity.

### 6. STEP import/export hardening

STEP support should graduate from a kernel capability into a documented interoperability service:

- unit/schema detection and explicit assumptions;
- deterministic import settings;
- imported assembly/product structure where supported;
- source/external-asset provenance;
- healing report and topology changes;
- semantic names/metadata retention where possible;
- explicit loss report when AICAD semantics have no STEP equivalent;
- export validation/re-import benchmark;
- large and adversarial STEP fixtures.

Do not represent “STEP imported successfully” as semantic fidelity. A report should state what was preserved, approximated, healed, dropped or reinterpreted.

### 7. B-rep derived caches/artifacts

Exact kernel-native B-rep can be valuable for speed and exchange, but is derived state.

Required properties if cached/bundled:

- key includes all semantic inputs that affect geometry plus backend/kernel identity needed for validity;
- cache can be discarded and rebuilt;
- cache load validates version/key/shape integrity;
- raw kernel pointers/epochs never persist;
- semantic references are replayed/reconstructed from semantic recipes/lineage, not trusted from native topology IDs;
- cache corruption cannot change source semantics silently.

### 8. Mesh export/import

Likely formats may include STL/OBJ/3MF/glTF or whichever formats evidence shows useful, but format selection is an implementation/product decision at Stage8 planning time.

Semantic requirements:

- units and coordinate system explicit;
- tessellation/approximation policy recorded;
- normals/material/metadata preservation documented per format;
- mesh is not silently promoted to exact B-rep;
- imported mesh has a distinct semantic representation/capability unless a reconstruction process explicitly creates exact/approximate geometry with evidence.

### 9. 2D interchange

Likely needs include SVG/DXF and drawing/sketch interoperability, but detailed format scope should be decided from concrete workflows.

Core requirements:

- units/coordinate frames explicit;
- curve approximation recorded;
- layer/name metadata retained where supported;
- import does not silently invent constraints/design intent;
- 2D interchange remains distinct from future full drawing/PMI semantics.

### 10. Fidelity and healing reports

Every lossy or repair-capable interchange path should share a structured report vocabulary where practical:

- operation/importer/exporter version;
- source and output identity;
- effective tolerance policies;
- entities read/written/dropped/repaired/approximated;
- unit/coordinate conversions;
- topology validity before/after;
- warnings and unresolved defects;
- provenance/lineage mapping when available;
- validation metrics.

The report is evidence, not merely log text.

## Likely core abstractions

These abstractions are justified by concrete Stage8 requirements; do not generalize beyond them prematurely.

1. **ArtifactManifest** — stable format-level metadata and member table, independent of compiler HIR layout.
2. **ContentIdentity / Digest** — typed identity for source/assets/package members; algorithm/version explicit.
3. **EnvironmentIdentity** — compiler/runtime/kernel/package/tool policy identity needed for reproducibility.
4. **ExternalAssetRecord** — promoted serialization of Stage6 minimal asset model.
5. **DerivedArtifactDescriptor** — declares cache/interchange kind, derivation inputs, producer and validity key.
6. **InterchangeReport / FidelityReport** — structured preservation/loss/healing evidence.
7. **ArtifactMigration** — explicit version-to-version transform with validation/evidence.

Avoid a universal “everything artifact object” if typed records keep responsibilities clearer.

## Major risks

### Artifact accidentally becomes second source of truth

**Risk:** fast load paths begin trusting cached topology/compiled IR over source+manifest/lock.  
**Mitigation:** caches are keyed/validated/rebuildable; semantic IDs derive from source/model identity; authoritative reconstruction path remains continuously tested.

### Internal IR frozen as public file format

**Risk:** serializing HIR/GeometryOp structs directly makes every compiler refactor a migration.  
**Mitigation:** use purpose-designed versioned artifact schemas at semantic boundaries.

### Reproducibility overclaim

**Risk:** “deterministic package” is interpreted as byte-identical OCCT B-rep across all OS/kernel versions.  
**Mitigation:** explicitly define which layers require byte identity, content identity, semantic equivalence or numerical equivalence.

### Interchange silently loses design intent

**Risk:** imported/exported STEP/mesh appears successful while semantic names, refs, units or exactness are lost.  
**Mitigation:** fidelity reports and explicit semantic boundary; reconstruction is research, not automatic truth recovery.

### External assets are path-based

**Risk:** builds differ when the same path points at different contents.  
**Mitigation:** content digest + locked import policy is authoritative; path/URL is origin metadata.

### Security / untrusted archive handling

Artifacts and imported CAD can be adversarial. Stage8 planning should include archive traversal protection, decompression/resource limits, checksum validation, parser isolation as needed, and no execution of embedded content. This is a concrete packaging/interchange requirement, not a reason to design the entire plugin security model now.

## Known specification gaps to resolve before Stage 8 implementation

1. **OD-S8-01 / D14:** exact `.aicadpkg` role and naming.
2. **Manifest schema/version/migration policy.**
3. **Canonical content identity algorithm/version policy.**
4. **What environment/kernel metadata is semantic/reproducibility-critical versus diagnostic only.**
5. **Package dependency lock format and its relationship to Stage11 package management.** Stage8 needs enough lock identity for reproducible artifacts without prematurely designing a registry.
6. **Artifact signing/trust policy** — can be optional/later unless distribution security requires it at Stage8.
7. **Interchange support matrix and fidelity guarantees** per format.
8. **Native cache invalidation compatibility** across compiler/kernel versions.

## Likely Stage-8 gate

A Stage8 owner gate should demonstrate at least:

- a source-first project packaged into the accepted native artifact and reconstructed/verified without hidden local state;
- deterministic canonical manifest/content identity for identical accepted inputs;
- changed source/package/profile/configuration/external-asset input changes the appropriate identities;
- optional B-rep/mesh caches can be deleted and rebuilt without semantic change;
- hardened STEP round-trip/import fixture with structured fidelity/healing report;
- at least one mesh export/import path and one 2D interchange path at the scope accepted for Stage8;
- verification evidence and provenance remain linked after package/unpackage;
- malformed/corrupt/untrusted artifacts fail safely with bounded resources;
- no raw topology handle/native pointer/internal HIR identity is used as persistent semantic authority.

The gate should **not** require high-quality automatic reconstruction of arbitrary external CAD into idiomatic AICAD source.

## Optional / research items

### Reconstruction research

Keep reconstruction explicitly research-oriented:

- imported B-rep → candidate analytic/freeform features;
- topology/name recovery;
- inferred sketches/constraints;
- imported assembly relation inference;
- confidence/evidence scoring;
- human/AI-assisted source reconstruction.

Success should be measured against curated corpora; reconstructed intent must not be presented as original ground truth without evidence.

### Signing / attestations

Potential later capability for package distribution/regulated workflows. Preserve manifest extensibility; do not gate Stage8 unless a concrete security/distribution requirement exists.

### Remote/content-addressed artifact store

The manifest/digest design should make this possible later, but remote storage, registry and distributed build are not Stage8 core unless usage/performance evidence demands them.

### Additional CAD formats

Parasolid/native commercial formats, IFC, JT, glTF, 3MF, etc. should be chosen based on real interoperability use cases and licensing/tool availability. The architecture should add format adapters under one provenance/fidelity contract, not freeze a huge format checklist now.
