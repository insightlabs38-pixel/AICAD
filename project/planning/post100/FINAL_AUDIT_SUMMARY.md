# Final post-AICAD-100 architecture / roadmap audit summary

**Owner-read-first document.**  
**Audit basis:** `insightlabs38-pixel/AICAD`, implementation branch `claude/aicad-stage3-dev`, exact revision `d82bf82e53a98f3117d3dc17707f1756822e593d` (`AICAD-074`).  
**Planning branch:** `planning/aicad-post100-audit`.  
**Status:** recommendations/drafts only. Nothing here is an approved roadmap/spec/RFC/owner decision.

---

# 1. Executive conclusion

AICAD's existing architecture is fundamentally compatible with the intended post-100 roadmap. The important foundations are already pointed in the right direction:

- source/AICAD semantic state is authoritative;
- public source semantics are separated from HIR, geometry IR, runtime dispatch and OCCT adapter layers;
- D2 value/functional semantics prevent kernel mutation from becoming source mutation;
- D6/RFC-0002 establishes kernel neutrality plus a deliberate safe/raw geometry split;
- D7/RFC-0003 establishes fail-closed semantic references instead of transient topology identity;
- D11 makes AICAD relation semantics authoritative over numerical solvers;
- D18 gives native runtime capabilities ordinary typed call semantics rather than compiler-intrinsic syntax;
- Stage3 already establishes parameters, a feature DAG/cache/invalidation substrate, safe geometry values and solver-independent sketch constraints;
- Stage4 has a well-defined task sequence to harden semantic references before post-100 work.

The project therefore does **not** need a broad rewrite before Stage5.

However, Stages5-7 should **not** be implemented directly from the old planning examples as written. The audit found several specification/architecture gaps that are small enough to fix now but expensive if allowed to become implementation precedent:

1. the canonical language-specification set is incomplete even though its README claims semantics/types/diagnostics specs exist;
2. Stage5 needs source-level spatial invariants, tolerance taxonomy, query-evaluation semantics, raw/safe boundary details and a scalable D18 runtime-function catalogue before broad API implementation;
3. the Stage3 feature DAG currently recognizes a deliberately shallow subset of geometry construction and will become an architectural blocker if advanced geometry inside user functions/parts/control flow remains invisible;
4. Stage6 depends on general language features (interfaces/`implements`/generic bounds/configuration surface) and stable assembly identity semantics that do not yet exist;
5. Stage6 solver work needs deterministic pose/DOF semantics above the numerical backend;
6. Stage7 needs a normalized obligation/evidence architecture and a clean split between language declarations, library predicates, runtime evaluation and build/test orchestration;
7. multiple older plan/RFC passages are stale relative to later owner decisions, especially naming and “open decision” status;
8. Stage9 should be split into mandatory source-first semantic environment (9A) and evidence-driven visual authoring (9B), preserving capability while avoiding an unnecessary all-or-nothing GUI gate.

**Bottom line:** Stage5 is safely task-plannable after a short pre-stage architecture/spec correction pass. Stage6 and Stage7 are also task-plannable now as drafts, but their queues must explicitly front-load the language/identity/semantic decisions they rely on. The 28/30/30 task drafts in this directory do that.

---

# 2. Can Stages 5-7 currently be task-planned safely?

## Stage 5 — YES, conditionally

The underlying architecture supports it, and the audit produced a 28-task dependency-ordered draft with three internal checkpoints and a final owner gate.

**Do not start implementation until the `MUST_FIX_BEFORE_STAGE_5` items below are resolved.** Most are specification/owner-boundary work, not large production-code projects.

Stage5 should remain the broad advanced-geometry stage envisioned by the canonical plan: analytic/freeform curves and surfaces, trimming, projection/intersection/distance, topology construction, healing, controlled low-level/raw topology, raw→safe adoption, lineage/provenance and difficult freeform benchmarks. It must not be narrowed to existing OCCT bridge functions.

## Stage 6 — YES as a draft, NOT as an immediate implementation queue

The 30-task draft is executable after Stage5 gate and Stage6 owner decisions. It deliberately makes language/interface and identity foundations explicit.

The most important correction is that Stage6 is **not only an assembly-solver project**. Its public semantics depend on:

- component/definition/instance/nested-path identity;
- shared frame/transform semantics;
- general interfaces/protocols and possibly bounded generics;
- cross-instance semantic refs;
- solver-neutral mate/joint relations;
- deterministic pose/DOF/conflict semantics;
- immutable configuration overlays, suppression and replacement identity;
- external asset provenance;
- BOM and interference evidence.

Skipping those and starting with a solver demo would create the largest future rewrite risk in the current roadmap.

## Stage 7 — YES as a draft, NOT until the verification language boundary is approved

The 30-task draft separates:

- language declarations (`test`, `requirement`, minimum evidence-capture semantics);
- common obligation/evidence identity/status model;
- ordinary library predicates;
- numerical/domain evaluators;
- build/test discovery and orchestration;
- parameter/configuration matrices;
- traceability/evidence schemas;
- coverage/mutation/AI repair benchmarks.

Contracts/invariants are deliberately not assumed to be mandatory for the core Stage7 gate. The owner should decide whether they are Stage7-critical or additive later capabilities.

---

# 3. Top architecture gaps

## MUST_FIX_BEFORE_STAGE_5

### A. Canonical language specification is incomplete

**CURRENT IMPLEMENTATION FACT:** `specs/language/README.md` names grammar, semantics, types and diagnostics as the canonical language specification; only README + grammar exist at the frozen revision. Future agents currently have no canonical semantic/type document to update.

**Consequence if ignored:** Stage5/6/7 examples become accidental specs, and implementation choices become compatibility promises without review.

**Recommendation:** restore/confirm the canonical spec set before new source-visible semantics.

### B. Spatial semantics are split between passive source structs and validated kernel-adapter types

AICAD-075A is already intended to close much of this gap. Stage5 must consume its accepted result, not duplicate frame/rotation rules. Source semantics should define Direction/Axis/Frame/Transform invariants; adapter math enforces/converts them but does not own the public definition.

### C. Feature DAG/provenance does not yet follow geometry through general programming abstractions

Stage3's current builder correctly proves the initial feature DAG on supported top-level direct runtime calls. It explicitly does not model geometry-producing user functions, branching or part bodies.

That is acceptable for Stage3. It is not acceptable as a permanent architecture because Stage5's low-level/freeform API will be most useful inside reusable functions/libraries. If abstraction causes geometry to disappear from the DAG, AICAD would lose references, provenance, incremental invalidation, source↔geometry navigation and AI inspectability exactly where programmability grows.

### D. D18 runtime function mechanism needs a Stage5 scaling decision

The current closed `BuiltinFnId` catalogue is safe and semantically clean for the nine Stage2/3 functions. Near-kernel-complete Stage5 can preserve the same **closed trusted ordinary-call semantics** while moving declarations/signatures/effect metadata to a generated/declarative registry. This is a scaling improvement, not permission for arbitrary native plugin registration.

### E. Tolerance taxonomy is absent

The project already demonstrates why one epsilon is wrong:

- D5 geometry equivalence profile;
- sketch-solver convergence tolerance;
- low-level vector/frame validity thresholds.

Stage5 adds construction/sewing/intersection tolerance and approximation tolerance; Stage7 adds verification tolerance. Freeze categories/ownership now. A full centralized engine can defer.

### F. Source-visible kernel query evaluation is unresolved

The current runtime largely builds a geometry graph before later kernel dispatch. Stage5 projection/intersection/distance/topology queries naturally produce values that source algorithms want to branch on. The Safe CAD API already flags this as a future architecture decision. It must be resolved deliberately rather than smuggled in operation by operation.

### G. Raw/unsafe geometry surface needs concrete semantics

The capability itself is already part of RFC-0002 and should **not** be removed. What remains is the language/type boundary: explicit `unsafe` syntax vs opaque raw types/namespaces or both; raw handle scope/epoch; functional editing; validation/adoption evidence.

## SHOULD_FIX_BEFORE_RELEVANT_STAGE

### H. Advanced geometry must extend Stage4 lineage/reference semantics

Every trim/sew/heal/edit operation must emit resolver-consumable lineage. Reconstructing identity later from geometry similarity is unsafe and conflicts with D7's fail-closed posture.

### I. Assembly identity domains are absent

Definition identity, logical instance identity, nested instance path, concrete variant, BOM line and geometry semantic reference must remain distinct. Topology IDs and solver variables cannot substitute.

### J. Solver-neutral relations need deterministic pose/DOF policy

D11 solves the “solver owns semantics” problem at principle level, but Stage6 still needs explicit grounding/gauge/underconstraint and diagnostic rules so two solver backends do not return observably different arbitrary poses.

### K. Configuration semantics are underspecified

Suppression/replacement should be immutable semantic overlays preserving logical slots/identity/provenance, not destructive deletion and swap operations.

### L. Verification needs a common obligation/evidence envelope, not a universal numerical constraint struct

The current sketch constraint IR is correctly domain-specific. Reusing it literally for requirements would be overgeneralization. Share identity/source/scope/status/evidence concepts; keep domain payloads/evaluators and solver lowerings typed.

### M. Artifact and external-asset identity needs a coherent Stage6→8 seam

Stage6 imported components need minimal content-addressed external-asset records before Stage8 freezes `.aicadpkg`. Otherwise file paths/import indices become accidental identity.

---

# 4. Top dangerous simplifications

The highest-risk simplifications found are:

1. **Public API = OCCT/GeometryOp API.** Reject. It would freeze backend/internal implementation into source semantics.
2. **Remove raw/unsafe geometry because Safe CAD handles most cases.** Reject. It deletes a deliberate near-kernel-complete systems layer needed by advanced algorithms/reconstruction/debugging.
3. **Use raw topology indices/handles as semantic identity.** Reject. Stage4 exists specifically to eliminate this fragility.
4. **One global tolerance.** Reject. It aliases five or more different numerical policies.
5. **Keep feature DAG top-level-builtins-only.** Reject as architecture; safe only as Stage3 implementation checkpoint.
6. **Make every Stage5 operation a compiler intrinsic.** Reject. D18 ordinary-call mechanism exists precisely to avoid this.
7. **Use kernel-api spatial structs directly as source language types.** Reject. Adapter representation is not source semantics.
8. **Assembly = transformed geometry + solver state.** Reject. It designs out logical instances/interfaces/configurations/BOM/diff.
9. **Initial solver's variables/penalty weights define mate/joint semantics.** Reject under D11.
10. **Configurations destructively delete/swap model nodes.** Reject. It breaks cross-config identity/references/provenance.
11. **Verification implemented entirely as interpreter magic.** Reject. Discovery/profiles/matrices/evidence/CI are orchestration concerns; predicates should often be libraries.
12. **Diagnostic JSON is sufficient as verification evidence.** Reject. Passing measurements, tolerance/profile/config/build identity require a dedicated evidence schema.
13. **B-rep cache becomes project authority.** Reject. Source/manifest/lock remain reconstructable authority.
14. **Full conventional GUI is an unconditional Stage9 gate.** Reject as roadmap architecture; split 9A/9B.
15. **Domain catalogs become compiler syntax.** Reject. Gears/fasteners/bearings/profiles/etc. are first-party libraries unless privileged access is proven necessary.

Safe simplifications do exist: defer the centralized tolerance engine after freezing the taxonomy; use a generated closed runtime catalogue; defer contracts/invariants if verification core is extensible; defer multiple representations/distributed caching; defer visual authoring while preserving source transaction architecture.

---

# 5. Specification changes that should happen before implementation

The detailed proposals are in `SPEC_DELTA_PROPOSALS.md`. The highest-priority changes are:

1. restore canonical language semantics/types/diagnostics specs;
2. rewrite mutation-looking future examples to D2 functional returns/method sugar;
3. remove pointer/kernel-specific raw topology examples under D6;
4. extend D7 lineage/ref policy to advanced topology transformations;
5. clarify D11 as AICAD-owned domain relations + solver lowerings, not one monolithic solver struct;
6. apply D14 naming consistently and separate `.aicad` source from `.aicadpkg` native artifact if owner confirms;
7. specify deterministic public behavior for D16 Map/Set/iteration;
8. freeze interface/`implements`/generic-bound rules before Stage6;
9. scale D18 through a declarative closed trusted runtime-function catalogue;
10. freeze the tolerance taxonomy explicitly separating operation, approximation, solver, verification, D5 equivalence and private representation-validity thresholds;
11. make source spatial invariants canonical above the kernel adapter;
12. specify feature identity/effects through user functions/parts/control flow;
13. specify source-visible kernel-query evaluation;
14. freeze assembly identity domains;
15. freeze deterministic assembly pose/DOF semantics;
16. freeze configuration overlay/suppression/replacement semantics;
17. freeze the minimal privileged verification language surface;
18. create a dedicated versioned verification evidence schema;
19. annotate stale RFC “open” statuses with their resolving decisions;
20. split Stage9 into 9A/9B in the official roadmap;
21. separate native semantic artifact from caches/interchange;
22. establish external-asset identity before Stage6 imported-component support.

---

# 6. Owner decisions required and when

## Before Stage 5

- **OD-S5-01:** canonical language-spec authority/workflow.
- **OD-S5-02:** D18 runtime-standard-function scaling.
- **OD-S5-03:** raw/unsafe geometry language/type boundary.
- **OD-S5-04:** kernel-backed source query evaluation model.
- **OD-S5-05:** tolerance taxonomy/default/override policy.
- **OD-S5-06:** semantic feature identity/provenance through functions/control flow.

The first five should be resolved before substantive Stage5 API work; OD-S5-06 must be resolved before Stage5's programmable advanced-geometry integration depends on feature DAG visibility.

## Before Stage 6

- **OD-S6-01:** assembly identity domains/stability.
- **OD-S6-02:** interface/conformance/generic-bound semantics.
- **OD-S6-03:** solver-neutral relation + deterministic pose semantics.
- **OD-S6-04:** configuration suppression/replacement identity semantics.
- **OD-S6-05:** minimal external-asset identity.

## Before Stage 7

- **OD-S7-01:** normalized obligation/evidence model.
- **OD-S7-02:** minimal privileged verification declarations.
- **OD-S7-03:** requirement/test/case identity.
- **OD-S7-04:** evidence schema versioning/compatibility.
- **OD-S7-05:** whether contracts/invariants are gate-critical, optional or deferred.

## Before Stage 8+

- exact `.aicadpkg` role/schema envelope;
- official Stage9A/9B roadmap split;
- later trusted-native plugin boundary only when Stage11 needs it.

## Later/research

Do not force owner decisions now on multiple geometry representations, distributed content-addressed execution, real-time collaboration, or an OCCT fork. Preserve seams and collect evidence first.

---

# 7. Proposed Stage-5 task count and batches

**28 provisional tasks**, `POST100-S5-001..028`.

### Batch S5-A — semantic/runtime foundations and curves (`001..010`)

- freeze prerequisites;
- spatial bridge;
- tolerance primitives;
- runtime standard-function/query evaluation architecture;
- programmable feature DAG/provenance;
- advanced geometry value/IR families;
- analytic/freeform curves and operations;
- **Checkpoint A**.

### Batch S5-B — surfaces and multi-solution geometry queries (`011..016`)

- analytic/freeform surfaces;
- trimmed surfaces;
- surface operations;
- intersections/projection/distance;
- **Checkpoint B**.

### Batch S5-C — safe/raw topology and reference integrity (`017..024`)

- general topology construction;
- sewing/healing;
- inspection/traversal;
- raw handles;
- functional editing;
- raw→safe validation/adoption;
- advanced lineage/reference integration;
- **Checkpoint C**.

### Batch S5-D — realism/adversarial/product gate (`025..028`)

- difficult freeform corpus;
- adversarial numerical/robustness campaign;
- learnability/inspectability/core-vs-library proof;
- final owner gate.

The count is intentionally not optimized for a round AICAD ID boundary.

---

# 8. Proposed Stage-6 task count and batches

**30 provisional tasks**, `POST100-S6-001..030`.

### Batch S6-A — language, identity and semantic assembly model (`001..011`)

- freeze prerequisites;
- general interfaces/generic bounds;
- assembly semantic IDs;
- definitions/instances;
- frames/poses;
- nesting/dependency graph;
- assembly semantic references;
- mechanical interfaces;
- mate/joint semantic relations;
- **Checkpoint A** before any solver can become authority.

### Batch S6-B — solver adapter, pose, DOF, diagnostics, kinematics (`012..018`)

- numerical adapter contract;
- deterministic grounding/representative pose;
- baseline solver;
- DOF;
- redundancy/conflicts;
- kinematics;
- **Checkpoint B** proving backend independence.

### Batch S6-C — configurations and engineering outputs (`019..026`)

- immutable configuration overlays;
- rules;
- suppression;
- replacement/variants;
- imported components/external assets;
- BOM;
- interference;
- **Checkpoint C**.

### Batch S6-D — realistic/adversarial/tooling gate (`027..030`)

- canonical assembly benchmark;
- adversarial identity/solver/scale campaign;
- structured assembly CLI/API;
- final owner gate.

---

# 9. Proposed Stage-7 task count and batches

**30 provisional tasks**, `POST100-S7-001..030`.

### Batch S7-A — verification core (`001..010`)

- freeze architecture/layer split;
- normalized obligation/evidence core;
- test/requirement declarations;
- stable IDs;
- assertion/evidence primitive;
- approximate/dimensional semantics;
- geometry predicate library;
- discovery/runner;
- evidence schema;
- **Checkpoint A**.

### Batch S7-B — requirements, traceability and matrices (`011..019`)

- hard/soft requirements;
- traceability;
- parameterized tests;
- parameter sweeps;
- configuration matrices;
- assembly verification;
- profiles;
- CLI;
- **Checkpoint B**.

### Batch S7-C — advanced strength/evidence (`020..025`)

- optional contracts/invariants according to owner ruling;
- semantic coverage;
- mutation testing;
- reproducible build/environment evidence;
- **Checkpoint C**.

### Batch S7-D — realistic/adversarial/performance/AI gate (`026..030`)

- realistic verification-first fixture;
- adversarial false-pass/evidence campaign;
- performance/resource benchmark;
- structured AI repair benchmark;
- final owner gate.

---

# 10. Recommended Stage-8/9/10+ shape

## Stage 8 — artifacts/interchange/reproducibility

Keep as a capability envelope until Stage7 finishes. Core should include:

- native `.aicadpkg`-direction artifact envelope (owner-confirmed name/role);
- versioned manifest;
- lock/environment identity;
- deterministic canonical packaging/content identity;
- external asset provenance;
- STEP hardening;
- optional B-rep derived caches;
- mesh and selected 2D interchange;
- structured fidelity/healing reports;
- artifact migration/security/resource handling.

**Reconstruction from arbitrary external CAD remains research**, not Stage8 gate-critical.

## Stage 9A — source-first engineering environment

Mandatory semantic IDE/workspace:

- LSP/editor;
- project navigation;
- incremental builds/diagnostics;
- 3D viewport/selection;
- semantic/ref/provenance/feature-DAG inspection;
- measurement;
- source↔geometry navigation;
- parameter editing as source transactions;
- model semantic diff/review.

## Stage 9B — visual authoring conditional on evidence

Potential sketch/feature/assembly/configuration visual authoring, but promoted by workflow/usability evidence. Every committed visual operation must produce authoritative source/project transactions.

## Stage 10 — AI-native structured tooling

Stable inspect/query/build/verify/edit APIs; bounded context packets; explicit source transactions; AI repair benchmark; no AI-only language semantics or authority bypass.

## Stage 11 — packages/plugins/extensions

Deterministic package identity/resolution/lock; capability/security model; WASM/external-service preferred; trusted native remains owner-controlled. Keep libraries/extensibility general.

## Stage 12 — engineering modules

Materials, fasteners, bearings, gears, manufacturing rules, drawings/PMI, simulation integrations, optimization, etc. Most should be first-party libraries/plugins, with only minimal universal semantic primitives promoted to core after evidence.

## Stage 13+ — collaboration/production/scale

Semantic diff/merge, stronger provenance, policy/security, large assemblies, usage-driven distributed/content-addressed evaluation, real-time collaboration only when justified.

---

# 11. Conflicts between current implementation and old plan

These are primarily **staleness or stage-boundary differences**, not evidence that implementation is wrong.

### Repository task-state conflict

- `CURRENT_STAGE.md`/`TASKS.yaml`/commit/report evidence: completed through AICAD-074.
- `SESSION_HANDOFF.md`: still says AICAD-072 is last completed.

The handoff is stale. This audit did not edit it.

### Canonical spec conflict

- `specs/language/README.md` claims canonical semantics/types/diagnostics files.
- Those files are absent.

This is a real documentation/spec-governance defect, not an implementation behavior mismatch.

### Future grammar/examples vs implemented AST

The grammar/plans anticipate interface/assembly/requirement/test concepts, while current AST implements only earlier-stage item kinds. This is expected staging, but future examples must not be mistaken for implemented syntax.

### Spatial type mismatch

Source/HIR passive `Axis3`/`Frame3` structures are narrower/less invariant-rich than kernel adapter Direction/Frame/Transform types. AICAD-075A is scheduled to resolve the source semantic foundation before Stage5.

### Stage3 feature-DAG scope vs long-term programmable model

Current feature DAG is intentionally direct/supported-operation-focused. Old plan language can read as if general semantic source→feature mapping already exists. It does not yet cover geometry hidden behind ordinary source abstractions.

### Safe API raw indices vs Stage4 semantic refs

Stage2 fillet/chamfer use `List<Int>` raw edge indices. This is explicitly a temporary limitation and must not survive as long-term semantic selection after Stage4.

### Old raw pointer-looking examples vs D6

Any `KernelShape*`/kernel-class examples are stale illustration and conflict with later kernel-neutral/epoch-bound handle policy.

### Old mutation-looking examples vs D2

Future topology/configuration APIs must be rewritten as immutable/functional semantics even if kernel implementation mutates native objects internally.

### Old artifact naming vs D14

Legacy CAD-IR/cad/native `.aicad` artifact wording conflicts with later AICAD naming/source-extension decisions and proposed `.aicadpkg` native bundle role.

### Old RFC open-status statements vs resolved owner decisions

Several RFC/plan status lines predate later D5/D10/D11/etc. resolutions. They should be annotated as superseded to prevent future agents from reopening closed architecture.

---

# 12. Features at risk of being accidentally designed out

The audit found no current code change that definitively deletes these features, but several could be lost through “simpler” future implementation choices:

1. **Near-kernel-complete low-level geometry** — if Stage5 is scoped to existing Safe CAD/OCCT bridge calls only.
2. **Safe + unsafe/raw geometry tiers** — if raw topology is omitted because high-level modeling works for common parts.
3. **Backend-neutrality** — if low-level source types become OCCT classes/pointers.
4. **General programmability with inspectability** — if feature DAG visibility stops at top-level builtins.
5. **Durable semantic refs through advanced geometry** — if Stage5 topology editing/healing does not emit lineage.
6. **General interfaces/protocols** — if Stage6 invents assembly-only interface compiler behavior.
7. **Solver independence** — if Stage6's first solver defines mates/joints/pose semantics.
8. **Cross-configuration stable identity** — if suppression/replacement is destructive.
9. **Verification as a platform capability** — if tests are interpreter special cases without stable evidence/discovery/traceability.
10. **Future IDE/AI/plugin stability** — if public tooling APIs expose HIR/GeometryOp/solver/kernel internals.
11. **Source authority** — if Stage8 caches or Stage9 visual state become hidden model authority.
12. **Extensible engineering modules** — if domain catalogs are hard-coded into compiler semantics.
13. **Multiple representations/backends later** — if today's exact B-rep/OCCT details leak upward.
14. **Semantic diff/merge/provenance later** — if Stage5-8 discards identity/history because current UI does not need it yet.

These capabilities should be preserved architecturally even when their implementation is deliberately deferred.

---

# 13. Abstractions that are over-engineered without current roadmap justification

The project should **not** react to this audit by building speculative infrastructure everywhere. The following are not justified as immediate Stage5 work:

### A universal distributed content-addressed build system

Stage3 cache keys/dirty propagation plus deterministic identity are enough foundation now. Remote CAS/distributed execution can wait for large-model evidence.

### A universal multi-representation geometry framework

Keep kernel/source/IR boundaries representation-neutral, but do not implement mesh/SDF/subdivision/exact-representation polymorphism until a real workflow needs it.

### A global provenance ontology before concrete provenance systems mature

Source→feature provenance, topology lineage, external assets and verification evidence are concrete and should interoperate by stable IDs. A giant generic provenance framework is not needed in Stage5.

### Multiple assembly solver implementations during Stage6

A clean solver adapter plus a mock/contract test is enough to prove independence. Build a second production solver only if robustness/performance evidence demands it.

### Full contract/invariant language before core verification proves need

Do not make Stage7 hinge on sophisticated DbC semantics unless the owner promotes them based on a concrete engineering fixture.

### Full visual CAD authoring before Stage9A

Not justified as a prerequisite for source-first semantic tooling.

### Native plugin ABI before Stage11 use cases/security policy

First-party trusted runtime functions do not justify an arbitrary native third-party extension ABI.

### Full automatic external-CAD intent reconstruction in Stage8

Keep it research; imported/interchange fidelity and provenance are the production requirement.

The correct strategy remains: **correct extensible boundary + minimal first implementation**, not maximal framework construction.

---

# 14. Exact recommended next planning actions

## Action 1 — Complete current Stage3/Stage4 normally

Do not let this audit interrupt AICAD-075 onward. The frozen audit basis is AICAD-074, and Stage4's semantic-reference work is a hard prerequisite for the final Stage5 plan.

## Action 2 — Review this directory as a planning packet, not merge-by-default authority

Owner should read in this order:

1. `FINAL_AUDIT_SUMMARY.md`
2. `OPEN_DECISIONS.md`
3. `SPEC_DELTA_PROPOSALS.md`
4. `ARCHITECTURE_GAP_REGISTER.md`
5. `SIMPLIFICATION_RISK_REGISTER.md`
6. `LANGUAGE_SURFACE_AUDIT.md`
7. `STAGE5_TASK_DRAFT.yaml`
8. Stage6/7 drafts
9. Stage8/9/10+ capability envelopes
10. `REQUIREMENTS_MATRIX.md` for trace detail.

## Action 3 — Resolve the six pre-Stage5 owner decisions

Prefer doing this near the end of Stage4, when AICAD-080..100 evidence is available. Do not guess today where Stage4 results may answer the question.

## Action 4 — Land spec/status corrections before assigning final AICAD-101+ IDs

At minimum:

- restore canonical language spec set;
- annotate stale RFC decision statuses;
- apply D2/D6/D14 corrections to future examples/naming;
- freeze tolerance taxonomy/spatial/query/raw/runtime-function semantics;
- specify general programmable feature identity/provenance.

These should go through the normal RFC/spec/owner process, not by directly copying this audit text.

## Action 5 — Reconcile Stage5 draft against final Stage4 gate

Specifically verify:

- Stage4 resolver precedence/durability levels;
- raw handle epoch rules;
- fingerprint fallback policy;
- lineage representation;
- reference query/result/cardinality semantics;
- Stage4 benchmark failure modes.

Then adjust only the Stage5 tasks affected by actual Stage4 evidence.

## Action 6 — Assign final AICAD-101+ IDs only after owner approves the Stage5 queue

Do not reserve arbitrary counts or force future stages to end on round numbers. The current 28 Stage5 tasks are provisional decomposition, not an ID commitment.

## Action 7 — Keep Stage6/7 drafts provisional until preceding-stage evidence exists

Their semantic dependency order is valuable now, but implementation details/benchmarks should be revised using real Stage5/6 outputs before final IDs are assigned.

## Action 8 — Officially decide the Stage9A/9B roadmap split before Stage8 closes

There is no need to implement UI work now. The only near-term requirement is to avoid designing away the semantic IDs/provenance/source transaction boundaries 9A will need.

## Action 9 — Continue using explicit owner gates and adversarial stage checkpoints

For Stages5-7, retain:

- architecture-heavy work first;
- vertical-slice checkpoint(s);
- stage-specific adversarial campaign;
- realistic integration corpus;
- final owner gate;
- no automatic promotion to next stage.

## Action 10 — Preserve the audit branch; do not merge automatically

This branch is an analysis artifact. After owner review, accepted pieces should be promoted through normal spec/RFC/task processes. The audit itself should remain traceable to its frozen implementation revision.

---

# Severity ledger

## MUST_FIX_BEFORE_STAGE_5

1. Confirm/restore canonical language specification authority and missing semantics/types/diagnostics specs.
2. Ensure AICAD-075A finishes canonical source spatial semantics used by Stage5/6.
3. Resolve D18 runtime-backed standard-function scaling for Stage5 breadth.
4. Resolve source-visible kernel query evaluation model before such queries affect control flow.
5. Freeze tolerance categories/ownership; do not reuse D5/solver tolerance implicitly.
6. Resolve raw/unsafe geometry capability boundary/spelling before raw source APIs.
7. Resolve feature identity/provenance through ordinary functions/parts/control flow before advanced abstractions depend on the feature DAG.
8. Correct the most dangerous stale spec examples/statuses (D2 mutation, D6 pointers, stale owner-decision status) so implementers do not copy them as normative.

## SHOULD_FIX_BEFORE_RELEVANT_STAGE

1. Extend Stage4 lineage/reference contract for every Stage5 topology-changing operation.
2. Freeze Map/Set deterministic observation semantics before public use.
3. Freeze interface/`implements`/generic bounds before Stage6 mechanical interfaces.
4. Freeze assembly identity before Stage6 IR.
5. Freeze solver-neutral deterministic pose/DOF semantics before Stage6 solver.
6. Freeze configuration suppression/replacement semantics before Stage6 configs.
7. Establish minimal external-asset identity before Stage6 imported components.
8. Freeze verification obligation/evidence architecture and privileged declaration surface before Stage7.
9. Freeze requirement/test/case IDs and evidence schema before Stage7 evidence is persisted.
10. Resolve `.aicadpkg`/manifest/versioning before Stage8.
11. Officially split 9A/9B before detailed Stage9 task planning.

## SAFE_TO_DEFER

1. Centralized full tolerance-policy engine after taxonomy is established.
2. Multiple production assembly solvers after adapter contract proves independence.
3. Contracts/invariants if core verification can add them compatibly later.
4. Automatic arbitrary external-CAD reconstruction.
5. Full conventional visual authoring until usage evidence promotes Stage9B capabilities.
6. Trusted native third-party plugin ABI.
7. Distributed/remote content-addressed execution.
8. Multiple geometry representations.
9. OCCT fork/customization absent benchmark evidence.
10. Real-time multi-user collaboration.
11. Large-assembly optimizations beyond measured bottlenecks.
12. Broad engineering catalog/compiler integrations; prefer libraries/plugins.

---

# Final assessment

The roadmap's ambitious capabilities do **not** need to be simplified away to keep AICAD implementable. The current architecture already contains the important seams; the main requirement is to make a few semantic boundaries explicit before future implementation hardens temporary Stage2/3 shortcuts into permanent behavior.

The highest-value planning move is therefore not to reduce Stage5-7 scope. It is to **sequence the architecture/spec decisions before the tasks that depend on them**, preserve safe/raw and semantic identity boundaries, and keep domain breadth in libraries wherever core privilege is unnecessary.

With those corrections, the post-100 roadmap is technically coherent: Stage5 can establish a genuinely advanced programmable geometry substrate; Stage6 can build semantic assemblies/configurations without solver lock-in; Stage7 can make verification a first-class engineering capability; and Stages8-10+ can build artifacts, tooling and AI on stable semantic APIs rather than compiler/kernel internals.
