# Stage 10+ capability envelopes

These stages are deliberately less prescriptive. The purpose is to preserve important architectural capabilities without pretending current evidence is sufficient to freeze distant task queues.

Classification used here:

- **LIKELY CORE** — universally understood semantics or privileged infrastructure that ordinary libraries cannot implement cleanly.
- **FIRST-PARTY LIBRARY** — useful domain capability implementable over stable public semantics without compiler privilege.
- **PLUGIN / SERVICE** — capability better isolated or independently deployable/replaceable.
- **RESEARCH** — uncertain feasibility/value; benchmark before roadmap commitment.
- **USAGE-DRIVEN FUTURE WORK** — valuable only if real scale/workflows justify it.

---

# Stage 10 — AI-native structured tooling

## Goal

Make AICAD unusually effective for AI-assisted engineering by exposing the same typed semantic model, diagnostics, references, provenance, verification evidence and source transactions that human tooling uses. AI is a client of structured APIs and normal verification, not a privileged authority that can bypass source, constraints, safety boundaries or evidence.

## Dependencies

- Stage4 reference/query service;
- Stage5 advanced geometry, feature/provenance and structured operation evidence;
- Stage6 assembly/configuration semantic services;
- Stage7 verification/evidence/traceability;
- Stage8 project/artifact/environment identity;
- Stage9A semantic service, navigation, source transaction and model diff APIs.

Stage10 should not require Stage9B visual authoring.

## Capability groups

### Structured project context packets — LIKELY CORE/API

A model should request bounded, typed context rather than scrape files blindly. Context packet construction may include:

- project manifest and dependency summary;
- relevant source declarations/spans;
- selected semantic feature/part/assembly subjects;
- parameter values/units;
- reference recipes/outcomes;
- feature DAG slice;
- geometry/measurement summaries;
- provenance/lineage;
- active configuration/profile/build identity;
- requirement/test results and failing evidence;
- relevant diagnostics;
- external asset/fidelity metadata.

Packets need size/resource budgets, stable schemas and explicit provenance.

### Structured inspection/query tools — LIKELY CORE/API

Examples:

- list/find symbols/parts/features/params;
- inspect semantic subject;
- resolve/reference explain;
- measure/query geometry;
- inspect topology lineage;
- inspect assembly instance/mate/joint/DOF/config/BOM;
- inspect requirements/tests/evidence/coverage;
- compare model snapshots;
- explain rebuild/cache dependencies.

These should invoke Stage4-9 service boundaries, not private HIR/OCCT/solver data.

### Source/project transaction API — LIKELY CORE/API

AI edits must use the same validated transaction mechanism as IDE tooling:

- precondition against build/source revision;
- typed/semantic target where possible;
- textual patch or structured transformation;
- parse/type/build/verify result;
- semantic diff;
- provenance of tool/model/user actor as policy allows;
- rollback/reject on failure.

AI must not silently mutate generated geometry or evidence as authoritative state.

### Plan/build/verify/repair workflow — LIKELY CORE TOOLING

AICAD can support agent workflows that:

1. inspect project and requirements;
2. propose a bounded source change;
3. build;
4. inspect structured diagnostics;
5. run targeted verification;
6. inspect semantic diff/evidence;
7. iterate within resource/permission policy;
8. leave an auditable change/evidence trail.

The orchestration can live in `cad-agent-tools` or a service; it does not require new source-language AI constructs.

### AI-facing skill/schema catalog — LIKELY CORE/API

Each action should have:

- stable name/version;
- typed input/output schema;
- capability/permission scope;
- resource cost/limits where relevant;
- deterministic vs nondeterministic classification;
- mutating vs read-only classification;
- source/build preconditions;
- evidence/provenance returned.

### Bounded AI repair benchmark — LIKELY CORE QUALITY GATE

Continue Stage7 seeded-failure benchmark into more complex Stage10 cases:

- broken semantic reference;
- unit/type error;
- geometry construction failure;
- assembly overconstraint;
- invalid configuration;
- failed requirement;
- package/external asset issue.

Measure correctness, verification pass after repair, unnecessary edit size, invalid attempts, and whether the agent used stable APIs rather than bypassing them.

## Safety/trust boundary

AI output is untrusted input. At minimum:

- no direct native-kernel pointer access;
- no hidden evidence rewriting;
- all source/project mutations are explicit transactions;
- external command/network/plugin capability is separately permissioned;
- resource budgets apply;
- package/plugin trust policy applies identically to AI-triggered actions;
- destructive/project-wide transformations require explicit scope and review policy;
- normal verification is authority for technical acceptance, not the model's claim.

## What should not enter compiler semantics

- prompt templates;
- model/provider names;
- “AI feature” source keywords;
- domain-specific repair heuristics;
- agent memory format;
- proprietary tool invocation protocol.

Those belong to tooling/services over stable semantic APIs.

---

# Stage 11 — packages, plugins and extensions

## Goal

Allow reusable libraries and controlled extensions to grow the ecosystem without turning package code into compiler internals or weakening determinism/security.

## Likely core capabilities

### Package identity/resolution — LIKELY CORE

- package name/version/content identity;
- dependency graph and lock semantics;
- deterministic resolution policy;
- source module visibility/export rules;
- package feature/capability metadata if justified;
- compatibility with Stage8 artifact/manifest identity.

### Package registry/provenance contract — LIKELY CORE + SERVICE

Core understands package identity/digest/signature metadata; a hosted registry can be a replaceable service.

### Extension capability/security model — LIKELY CORE

Extensions must declare capabilities such as:

- pure source/library only;
- artifact/interchange access;
- filesystem/project read/write;
- network;
- external process/service;
- WASM sandbox;
- trusted native (only if owner later authorizes D12/D15 path).

Default should remain least privilege.

### Extension semantic integration — LIKELY CORE/API

Plugins may add library functions, validators, import/export adapters, UI panels, agent skills, or domain services only through stable extension points. They must not patch internal HIR/kernel objects as an ABI.

## First-party libraries

Good candidates include:

- fasteners;
- bearings;
- gears;
- springs;
- standard structural profiles;
- vendor component libraries;
- manufacturing-rule sets;
- reusable mechanical interfaces;
- material databases where licensing/data policy permits.

These are useful but do not deserve compiler syntax merely because they are common.

## Plugin/service candidates

- proprietary CAD translators;
- PLM/PDM connectors;
- vendor catalog search;
- simulation backends;
- cloud render/meshing;
- organization policy/validation integrations;
- external optimization services;
- model-provider AI connectors.

## Trusted native plugins — unresolved / owner controlled

Keep D12/D15 open until a concrete performance/integration use case, ABI lifecycle and threat model exist. WASM/external-service boundaries should be preferred by default. Stage5 trusted **first-party runtime builtins** do not imply third parties may register arbitrary native functions.

## Stage11 risks

- package resolution becomes nondeterministic;
- plugin APIs expose internal compiler/kernel structs and freeze implementation;
- native extension crashes/corrupts process;
- untrusted extension gains network/filesystem unexpectedly;
- package-defined functions bypass resource accounting;
- semantic IDs conflict across packages;
- dependency supply-chain provenance is insufficient.

---

# Stage 12 — engineering semantics and modules

## Goal

Build richer mechanical-engineering workflows over the stable language/geometry/assembly/verification/package substrate. Most capabilities should be first-party libraries or plugins unless they require universally understood semantics or privileged geometry/runtime access.

## Likely first-party library areas

### Materials and physical properties — FIRST-PARTY LIBRARY

- material records/properties with units and provenance;
- density-driven mass properties;
- material assignment to semantic part/body subjects;
- temperature/process metadata if needed.

Core may need only a stable semantic metadata/subject attachment mechanism if libraries cannot implement attachment cleanly.

### Fasteners, bearings, gears, springs, profiles — FIRST-PARTY LIBRARY

Use general geometry, parameters, interfaces, configurations and BOM. Do not add compiler keywords for these catalogs.

### Manufacturing rules / DFM — FIRST-PARTY LIBRARY + PLUGIN

Examples:

- minimum wall/thickness;
- hole/tool accessibility;
- draft/undercut checks;
- additive overhang/min feature rules;
- sheet-metal constraints if that domain is prioritized.

Rules should produce normal Stage7 requirements/evidence.

### Drawings / annotations / PMI — LIKELY LIBRARY + SOME CORE SEMANTICS

The difficult question is durable association of annotations/dimensions/tolerances with semantic geometry and configurations. If universal document/annotation identity is required, a small core semantic layer may be justified. Rendering/layout/export belong to libraries/tooling.

### GD&T — FIRST-PARTY LIBRARY / POSSIBLE CORE-ADJACENT TYPES

Typed geometric tolerance semantics may need standardized core-adjacent value types and semantic references, but specific standards/catalogs can remain libraries. Gate any compiler involvement on demonstrated inability to express it with normal types/requirements.

### Simulation integration — PLUGIN / SERVICE

FEA/CFD/multibody tools should consume semantic geometry/material/load/boundary-condition definitions and return versioned evidence/artifacts. Solvers remain external backends unless AICAD later develops first-party solvers.

### Optimization / generative design — LIBRARY / SERVICE

Parameter search, constraints, objective functions and verification can reuse Stage7; heavy optimizers can be services. Avoid a generative-design compiler subsystem.

## Potential core-adjacent abstractions to evaluate only with evidence

- standardized semantic metadata attachment;
- engineering quantity/property schemas;
- document/drawing/annotation stable IDs;
- manufacturing/simulation subject sets and boundary-condition reference model;
- richer tolerance/fit/GD&T value types.

Do not add these preemptively in Stage5-7.

---

# Stage 13+ — collaboration, production scale and usage-driven expansion

## Collaboration and semantic review — LIKELY CORE + SERVICE

Potential capabilities:

- semantic diff/merge;
- change sets tied to source transactions;
- model review comments anchored to stable semantic subjects;
- verification evidence attached to revisions;
- approval/review states where organizations need them;
- branch/merge conflict visualization;
- audit/provenance history.

The source/VCS remains fundamental. Collaboration tooling should augment semantic understanding rather than invent a proprietary hidden model history.

## Production provenance/security — LIKELY CORE + POLICY/SERVICE

- signed artifacts/packages if use cases require;
- provenance attestations;
- policy-as-verification profiles/rules;
- organization package allowlists;
- external asset/license/security records;
- plugin capability governance;
- audit retention/export.

## Large-assembly scale — USAGE-DRIVEN FUTURE WORK

Potential techniques:

- lazy geometry realization;
- assembly spatial indexes;
- hierarchical DOF/solver decomposition;
- level-of-detail visualization;
- selective verification;
- distributed/cache-backed build;
- partial artifact materialization.

Do not design the Stage6 semantic model around a particular scale optimization. Preserve identity/dependency boundaries, then optimize based on profiling.

## Multi-user / real-time collaboration — RESEARCH / USAGE-DRIVEN

Only pursue after semantic diff/merge and source transactions are mature. Real-time editing raises difficult source merge, semantic conflict and ownership questions; it should not force early CRDT/OT architecture into core.

---

# Strategic backlog that must remain visible

## Semantic diff/merge — LIKELY CORE later

Depends on stable IDs for params/features/refs/instances/configs/requirements/artifacts. Stage5-8 must avoid destroying those identities. Diff should combine source change with semantic consequence, not replace text/VCS.

## Stronger provenance — LIKELY CORE later

Stage3/5 provenance, topology lineage, external assets and verification evidence are separate concrete needs. Later collaboration can unify query/indexing across them. Avoid a speculative universal provenance engine now, but preserve IDs/links.

## Incremental/content-addressed evaluation — USAGE-DRIVEN FUTURE WORK

Stage3 already has cache keys/dirty propagation. Stages5-8 should keep deterministic keys and dependency evidence. Remote/distributed CAS is later optimization, not current semantic requirement.

## Tolerance-policy engine — LIKELY CORE-adjacent later

The **taxonomy** is required early. A full hierarchical project/package/profile policy engine can emerge from actual Stage5-7 usage. It should preserve purpose-typed tolerance categories.

## Geometry debugger — LIKELY TOOLING

Built on Stage9 semantic inspector, feature DAG, reference resolution, topology lineage and operation/healing evidence. Useful views may include operation inputs/outputs, intermediate topology, tolerance decisions, candidate refs and kernel error normalization. This belongs to tooling, not source semantics.

## Fingerprint/equivalence infrastructure — LIKELY CORE SUPPORT

D5 validation and Stage4 reference fallback need fingerprints/equivalence evidence. Preserve versioned algorithms and never confuse similarity with semantic identity. Additional fingerprints may support diff/cache/import matching later.

## Multiple geometry representations — RESEARCH until justified

Potential exact B-rep, mesh, implicit/SDF, subdivision or specialized analytic representations. Kernel-neutral source/IR boundaries make future experimentation possible. Do not require a universal representation framework before a real workload needs it.

## Language/model migration — LIKELY CORE TOOLING later

Once source/artifact schemas evolve, provide explicit migrations with semantic diff/verification evidence. Avoid indefinite backwards compatibility implemented as parser/runtime special cases.

## Evidence-driven OCCT customization — RESEARCH

Only fork/customize/replace upstream OCCT components when reproducible benchmark failures show the adapter cannot meet robustness/performance/semantic needs. Maintain minimal patch sets and upstreamability where practical. “We use OCCT today” is not a reason to make a fork strategic work.

## Large assemblies — USAGE-DRIVEN

Profile realistic assembly counts and relation graphs after Stage6. Optimize only observed bottlenecks; keep public identity/relation semantics stable.

## Advanced first-party libraries — FIRST-PARTY LIBRARY

Candidate areas grow with users: sheet metal, frames/weldments, piping, cams, mechanisms, lattice/generative patterns, optics fixtures, robotics components, manufacturing templates. Libraries should exercise the extensibility architecture rather than become reasons to add compiler intrinsics.

---

# Cross-stage rules for Stage 10+

1. **Do not promote internal compiler/kernel structures into public IDE/AI/plugin APIs.** Stable semantic services/schemas are the boundary.
2. **Do not use AI as an authority.** Source, semantic rules, references and verification evidence remain authoritative.
3. **Do not add domain concepts to compiler semantics when a library can implement them cleanly.**
4. **Do not add plugin trust merely because first-party runtime builtins are native.** They are different security boundaries.
5. **Do not prebuild distributed/real-time/multiple-representation infrastructure without a benchmark/use case.** Preserve seams instead.
6. **Do preserve stable semantic identity, provenance and versioning information needed by later diff/merge, artifacts, IDE and AI.**
7. **Do keep source/project state reconstructable headlessly.** No later UI/service may become a required hidden source of truth.
8. **Do make migrations explicit.** Compatibility should be engineered, not accumulated as silent interpretation branches.
