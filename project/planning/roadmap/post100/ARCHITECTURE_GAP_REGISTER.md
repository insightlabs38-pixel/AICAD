# Architecture gap register

Audit basis: `d82bf82e53a98f3117d3dc17707f1756822e593d`. A “gap” does not necessarily mean the current implementation is wrong; many are intentional stage boundaries. It means a future-stage requirement cannot be implemented cleanly until the boundary is specified or extended.

## Summary

| Gap | Severity / timing | Stages | Owner ruling? |
|---|---|---|---|
| GAP-001 | MUST resolve before Stage 5 | 5-7 | Yes |
| GAP-002 | MUST resolve via AICAD-075A + spec before Stage 5 | 5-6 | If 075A leaves choices open |
| GAP-003 | MUST resolve early Stage 5 | 5, 7, 9, 10 | Yes |
| GAP-004 | MUST resolve before Stage 5 public surface expands materially | 5+ | Yes |
| GAP-005 | MUST define taxonomy before Stage 5; engine may defer | 5-8+ | Yes for policy |
| GAP-006 | MUST integrate during Stage 5 | 5-7 | Maybe after Stage4 evidence |
| GAP-007 | SHOULD resolve before Stage 5 raw tier | 5 | Yes |
| GAP-008 | MUST resolve before Stage 6 | 6-7 | Yes |
| GAP-009 | MUST resolve before Stage 6 solving | 6-7 | Yes |
| GAP-010 | MUST resolve before Stage 6 configurations | 6-9 | Yes |
| GAP-011 | MUST resolve before Stage 7 | 7+ | Yes |
| GAP-012 | MUST resolve before Stage 7 declarations | 7 | Yes |
| GAP-013 | SHOULD resolve before Stage 7 evidence persistence | 7-10+ | Yes |
| GAP-014 | SHOULD resolve before Stage 8 | 8+ | Yes |
| GAP-015 | SHOULD resolve before Stage 8 | 8-11 | Yes |
| GAP-016 | SHOULD resolve before Stage 9A | 9-10 | No/Maybe |
| GAP-017 | SAFE to defer implementation; preserve boundary | 10+ | 10-13+ | No |

---

## GAP-001 — Canonical language specification set is incomplete

**Affected stages:** 5, 6, 7.  
**Affected files:** `specs/language/README.md`, `specs/language/grammar.ebnf`; absent `specs/language/semantics.md`, `types.md`, `diagnostics.md`; future syntax in `docs/plan/02_LANGUAGE_AND_COMPILER.md`, `03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md`, `07_ASSEMBLIES_KINEMATICS_CONFIGURATIONS.md`, `08_CONSTRAINTS_REQUIREMENTS_TESTS.md`.

**SOURCE REQUIREMENT:** `specs/language/README.md` identifies grammar, semantics, types, and diagnostics as canonical language artifacts. Future plans rely on semantics beyond grammar alone.  
**CURRENT IMPLEMENTATION FACT:** at the frozen revision the directory contains only `README.md` and `grammar.ebnf`; a direct repository lookup for `semantics.md` fails. The grammar names some future item categories, while `cad-ast::Item` implements only let/const/param/fn/struct/enum/part/import.  
**Required future capability:** Stage5-7 additions need one canonical location for value semantics, type invariants, effect/unsafe boundaries, declaration behavior, diagnostics, and compatibility policy.  
**Why insufficient:** roadmap examples cannot safely become de facto specifications. Two agents could implement the same example with incompatible semantics and both plausibly claim conformance.

**Possible solutions:**
1. Create the missing canonical spec files before Stage5 and migrate only approved normative behavior into them. **Recommended.** Lowest long-term ambiguity; modest documentation cost.
2. Declare RFCs/plan docs canonical instead and revise `specs/language/README.md`. Possible, but spreads language authority and makes compatibility review harder.
3. Continue using grammar + code as de facto semantics. **Reject:** implementation becomes the language definition and future refactors become source-breaking by accident.

**Recommended timing:** before Stage5 implementation tasks that add any new source-visible semantic behavior.  
**Owner ruling:** **Yes**, to confirm canonical-spec authority and approval workflow.

## GAP-002 — Spatial math has two incomplete semantic layers

**Affected stages:** 5, 6.  
**Affected files:** `crates/cad-hir/src/geometry_types.rs`, `crates/cad-kernel-api/src/geometry.rs`, `docs/API/safe-cad-api.md`, task AICAD-075A, `docs/plan/03...`, `05...`, `07...`.

**SOURCE REQUIREMENT:** future geometry and assemblies need points, vectors, directions, axes, frames, transforms, rotations and composition.  
**CURRENT IMPLEMENTATION FACT:** source/HIR currently exposes Point2/3, Vector2/3, Axis3 and Frame3 passive structs; `Axis3.direction` and frame axes are generic float vectors without normalized/orthonormal invariants. Kernel API separately has `Direction3`, validated right-handed orthonormal `Frame3`, and `Transform`, with low-level numeric validity thresholds. AICAD-075A explicitly owns general axis/frame/rotation semantics and is not completed at the audit revision.  
**Required future capability:** one source-semantic spatial model, unit-safe and backend-neutral, with explicit invariants and lossless conversion into kernel adapter math.  
**Why insufficient:** Stage5 curve/surface coordinate systems and Stage6 instance poses cannot safely depend on passive structs whose validity differs from the adapter's types.

**Possible solutions:**
- Finish AICAD-075A with source-level `Direction3`/rotation/transform semantics, constructor validation and conversion contracts; keep adapter representations private. **Recommended.** 
- Reuse `cad-kernel-api` types directly in HIR/source. **Reject:** leaks backend adapter representation into public language semantics.
- Leave structs unconstrained and validate only at kernel dispatch. Saves work now but creates late runtime errors and inconsistent semantic equality. **Reject as architecture.**

**Recommended timing:** AICAD-075A / before Stage5 starts.  
**Owner ruling:** only if AICAD-075A encounters unresolved public semantics.

## GAP-003 — Feature DAG/provenance is shallower than the programmable geometry model

**Affected stages:** 5, 7, 9, 10.  
**Affected files:** `crates/cad-feature-graph/src/graph.rs`, Stage3 checkpoint, `docs/plan/06...`, `04...`, `05...`.

**SOURCE REQUIREMENT:** AICAD is a programming language, not a flat operation list; user functions, parts, control flow and libraries should be able to construct geometry while retaining inspectability, dependencies, incremental invalidation and provenance.  
**CURRENT IMPLEMENTATION FACT:** Stage3 `FeatureGraph::build` recognizes supported direct runtime-builtin calls in top-level bindings. Its documented scope excludes ordinary source functions that construct geometry, conditional/branching feature selection, and part bodies.  
**Required future capability:** a feature/effect trace representing semantically meaningful geometry construction across ordinary calls and control flow, with deterministic identity and source provenance.  
**Why insufficient:** Stage5 encourages reusable low-level geometry helpers. If geometry inside those helpers disappears from the DAG, references, provenance, incremental evaluation, IDE inspection and AI tooling become least reliable exactly where the model becomes more programmable.

**Possible solutions:**
1. Build feature nodes from evaluated/lowered geometry effects with stable call-site/context identity while preserving source spans and parameter dependencies. **Recommended direction; exact design needs RFC/owner review.**
2. Inline/monomorphize source functions before DAG construction while preserving provenance. Could work but may explode graph size and complicate recursion/control flow.
3. Restrict geometry-producing calls to top-level known builtins. **Reject:** narrows the language to a DSL and contradicts core product principles.

**Recommended timing:** architecture decision before Stage5; minimal implementation in early Stage5 before advanced library abstractions.  
**Owner ruling:** **Yes** — feature identity across calls/branches is public-semantic enough to affect references and incremental behavior.

## GAP-004 — Closed runtime-builtin catalogue may not scale to near-kernel-complete Stage 5

**Affected stages:** 5+.  
**Affected files:** `crates/cad-hir/src/builtins.rs`, `docs/API/safe-cad-api.md`, D18/DL-15, RFC-0001, RFC-0002.

**SOURCE REQUIREMENT:** Stage5 low-level geometry should be near-kernel-complete while remaining ordinary typed AICAD calls, not compiler intrinsics. D18 established a compiler/runtime-owned standard-function mechanism.  
**CURRENT IMPLEMENTATION FACT:** every runtime-backed function requires a `BuiltinFnId` enum variant and runtime dispatch arm; users/packages cannot register implementations. This is intentionally a closed mechanism.  
**Required future capability:** tens or hundreds of typed privileged runtime operations, grouped coherently, without changing grammar/type rules for each operation and without opening arbitrary native callbacks.  
**Why insufficient/ambiguous:** the *semantics* scale, but the catalogue's code ownership and namespace mechanism may become a maintenance bottleneck and tempt special cases.

**Possible solutions:**
- Keep a closed trusted runtime registry, but move identity/signature/effect metadata to declarative generated tables consumed by HIR/typecheck/runtime/docs; namespace by module; no user registration. **Recommended.** This preserves DL-15 security and ordinary-call semantics.
- Continue hand-extending one enum/dispatch match. Architecturally valid but operationally brittle; acceptable initially if generated/declarative migration remains compatible.
- Add geometry-specific syntax/intrinsics. **Reject unless an operation truly requires privileged language semantics and passes D9 RFC gate.**
- Allow arbitrary plugin native registration. **Reject before D12/D15 security decisions.**

**Recommended timing:** decide before broad Stage5 API buildout; implementation can begin with minimal compatible catalogue generalization.  
**Owner ruling:** **Yes**, because it interprets the intended scalability of D18.

## GAP-005 — No explicit tolerance taxonomy across geometry, approximation, solver, verification and equivalence

**Affected stages:** 5-8+.  
**Affected files:** D5/D19, `cad-validation`, `cad-constraints::sketch_solver`, `cad-kernel-api::geometry`, plans 05/08/23.

**SOURCE REQUIREMENT:** the planning brief explicitly forbids conflating operation tolerance, approximation tolerance, solver tolerance, verification tolerance, and D5 equivalence tolerance.  
**CURRENT IMPLEMENTATION FACT:** D5 has a versioned comparison profile; sketch solving has a separate versioned convergence tolerance; spatial validity code contains local normalization/rigidity thresholds. These are already semantically different.  
**Required future capability:** named tolerance classes with ownership, units, defaults, inheritance/override rules, serialization/versioning and diagnostic reporting.  
**Why insufficient:** Stage5 sewing/intersection/approximation and Stage7 approximate assertions will otherwise reuse whichever epsilon is convenient, producing nonportable and unstable semantics.

**Possible solutions:**
- Freeze a taxonomy now, leave a centralized policy engine until usage justifies it. **Recommended.** Each operation declares which class it consumes and records effective values in evidence.
- One global project tolerance. **Reject:** dimensionally and semantically wrong.
- Every operation accepts unrelated raw `Float` epsilon. Flexible but ungovernable and hard to reproduce; acceptable only as low-level explicit override beneath typed policy.

**Recommended timing:** taxonomy before Stage5 operation APIs; richer engine can defer.  
**Owner ruling:** **Yes** for default and override semantics.

## GAP-006 — Stage4 reference/lineage model has no frozen advanced-geometry extension contract yet

**Affected stages:** 5-7.  
**Affected files:** Stage4 tasks AICAD-080..100; RFC-0003; `docs/plan/05...`, `06...`.

**SOURCE REQUIREMENT:** Stage4 must establish semantic recipes, lineage, resolver precedence, durability and raw-handle epochs. Stage5 then introduces trimming, healing, sewing and explicit topology editing that can split/merge/delete topology.  
**CURRENT IMPLEMENTATION FACT:** Stage4 has not begun at the frozen revision. Stage3 provenance is source/feature dependency provenance, not topology lineage.  
**Required future capability:** every advanced topology-changing operation emits standardized lineage events sufficient for Stage4 resolver semantics; semantic references either survive according to declared durability or fail closed with evidence.  
**Why insufficient:** Stage5 can implement correct geometry but accidentally destroy the identity substrate Stage6/7/9 require.

**Possible solutions:**
- Add a Stage5 “advanced lineage contract” task immediately after Stage4 outputs are known, then require lineage from every topology-changing operation. **Recommended.**
- Reconstruct lineage afterward from geometry fingerprints. **Reject as primary design:** risks silent-wrong recovery and violates D7.

**Recommended timing:** Stage4 decides baseline; Stage5 extends it before topology editing/healing are considered complete.  
**Owner ruling:** maybe, if advanced cases require changing Stage4 resolver semantics.

## GAP-007 — Unsafe/raw geometry capability and validation/adoption semantics are not concretely specified

**Affected stages:** 5.  
**Affected files:** RFC-0002 §4, `docs/plan/05...`, future examples.

**SOURCE REQUIREMENT:** low-level raw topology is intentionally unsafe, epoch-bound, nonpersistent, and promotable only through validation/adoption.  
**CURRENT IMPLEMENTATION FACT:** Safe CAD has no raw-tier source surface; plan examples contain pointer-like/raw spellings that are illustrative and sometimes conflict with D6.  
**Required future capability:** explicit capability boundary: how raw values are created, scoped, invalidated, inspected/edited, validated and converted to safe semantic geometry; what provenance and semantic outputs adoption requires.  
**Why insufficient:** inventing syntax during implementation could either make raw handles accidentally durable or make unsafe operations indistinguishable from safe geometry.

**Possible solutions:**
- General `unsafe` language capability with raw-geometry library namespaces and opaque `Raw*Handle` values, if language-level unsafety is justified. 
- Explicit raw geometry value types/functions without an `unsafe` block, enforced by type separation. Could be sufficient if the compiler needs no privileged effect rule.
- Pointer-like `KernelShape*` API. **Reject:** violates kernel neutrality and falsely implies pointer semantics.

**Recommended timing:** spec decision before Stage5 raw-tier tasks.  
**Owner ruling:** **Yes**, especially if new `unsafe` language semantics are proposed.

## GAP-008 — Assembly semantic identity model is absent

**Affected stages:** 6-9.  
**Affected files:** `docs/plan/07...`, `cad-assemblies` placeholder, refs/provenance plans.

**SOURCE REQUIREMENT:** reusable component definitions, multiple instances, nested assemblies, configurations, BOMs and semantic references require stable identity.  
**CURRENT IMPLEMENTATION FACT:** no assembly IR exists.  
**Required future capability:** distinct identity domains for component definition, assembly definition, instance, nested instance path, configuration/variant slot, BOM line and referenced geometry. Identity must survive pose solving and ordinary rebuilds according to declared rules.  
**Why insufficient:** treating geometry occurrence, solver variable, array index or topology identity as “the instance ID” would break configurations, BOM, references and diff/merge.

**Possible solutions:** explicit semantic IDs allocated from source declarations/paths plus stable instance keys, with derived nested paths and provenance. **Recommended.** Random runtime IDs alone are insufficient for reproducible source semantics.

**Recommended timing:** before any Stage6 implementation beyond language prerequisites.  
**Owner ruling:** **Yes**.

## GAP-009 — Solver-neutral assembly relation model and deterministic pose policy are not specified

**Affected stages:** 6-7.  
**Affected files:** D11/DL-20; `docs/plan/07...`, `08...`.

**SOURCE REQUIREMENT:** AICAD constraint IR is authoritative; numerical solvers are backends. Assemblies require mates, joints, DOF, conflict diagnostics and repeatable pose semantics.  
**CURRENT IMPLEMENTATION FACT:** sketch constraints satisfy solver independence for sketches but are domain-specific; no assembly relation IR exists.  
**Required future capability:** relation types that state geometry/kinematic intent, variable/scoping model, solver adapter contract, residual/evidence schema, and deterministic grounding/gauge conventions.  
**Why insufficient:** a first solver can otherwise leak its variable ordering, penalty weights or arbitrary underconstrained pose into public behavior.

**Possible solutions:** normalized constraint envelope + assembly relation payloads + solver adapters + deterministic post/initialization conventions. **Recommended.** Do not force joint kinematics into sketch equation types.

**Recommended timing:** spec before Stage6 solver implementation.  
**Owner ruling:** **Yes** for public relation and deterministic pose semantics.

## GAP-010 — Configuration suppression/replacement semantics are undefined across identity, refs, provenance and BOM

**Affected stages:** 6-9.  
**Affected files:** `docs/plan/07...`, `23...`, configurations placeholder.

**SOURCE REQUIREMENT:** configurations select variants, enforce rules, suppress components/features and replace components.  
**CURRENT IMPLEMENTATION FACT:** no configuration declaration or runtime exists; examples are illustrative.  
**Required future capability:** define logical configuration identity, rule evaluation order, suppression visibility, retained identity, replacement compatibility, reference outcomes, BOM effects and provenance.  
**Why insufficient:** naive “delete when suppressed / insert replacement” semantics create unstable IDs and refs.

**Possible solutions:** immutable base assembly + configuration overlay producing a resolved semantic assembly while preserving logical slots and explicit status/provenance. **Recommended.**

**Recommended timing:** before Stage6 configuration implementation.  
**Owner ruling:** **Yes**.

## GAP-011 — No normalized cross-domain verification relation/evidence model

**Affected stages:** 7+.  
**Affected files:** `docs/plan/08...`, D11, `cad-constraints`, `cad-requirements` placeholder.

**SOURCE REQUIREMENT:** sketch constraints, assembly relations and engineering requirements share conceptual fields but not necessarily a numerical solver. Verification needs stable IDs, evaluators, tolerance, strength, scope, status and structured evidence.  
**CURRENT IMPLEMENTATION FACT:** `SketchConstraint` and `ConstraintSet` are sketch-specific. No general requirement/test model exists.  
**Required future capability:** common envelope for identity/source/scope/relation/evidence/status, with domain-specific payload/evaluator interfaces and optional solver mapping.  
**Why insufficient:** making current sketch structs universal would encode sketch entity IDs/solver assumptions into requirements; making verification wholly separate loses shared diagnostics/evidence concepts.

**Possible solutions:** small domain-neutral “relation/obligation + evidence” core with adapters. **Recommended.** Keep numerical solver IRs as lowerings where needed.

**Recommended timing:** architecture/spec before Stage7, informed by Stage6 relation experience.  
**Owner ruling:** **Yes**.

## GAP-012 — Test/requirement/contract language constructs are named but not semantically frozen or implemented

**Affected stages:** 7.  
**Affected files:** grammar, AST, lexer, `docs/plan/08...`.

**SOURCE REQUIREMENT:** roadmap expects tests, requirements, assertions and possibly contracts/invariants.  
**CURRENT IMPLEMENTATION FACT:** `test` and `requirement` are reserved/mentioned in grammar, but AST `Item` has no such variants; `requires`, `ensures`, `invariant` are not established implemented constructs.  
**Required future capability:** decide which features need privileged declarations and which are ordinary functions/library predicates/build-runner behavior.

**Possible solutions:** minimal `test` + `requirement` declarations as core because discovery/stable IDs/traceability need universal semantics; keep most assertions/predicates in libraries; add contracts only if their call-boundary semantics justify compiler/runtime privilege. **Recommended direction.**

**Recommended timing:** before Stage7 queue begins.  
**Owner ruling:** **Yes**.

## GAP-013 — Verification evidence persistence/versioning contract is absent

**Affected stages:** 7-10+.  
**Affected files:** `docs/plan/08...`, `14...`, `17...`, artifact plan.

**SOURCE REQUIREMENT:** structured evidence must be usable by CLI/CI/AI and trace back to source/geometry/requirements.  
**CURRENT IMPLEMENTATION FACT:** diagnostic schema exists, but no persistent verification evidence schema/build identity binding.  
**Required future capability:** versioned result IDs, subject IDs, effective tolerance/profile, configuration, build/toolchain identity, measurements, source spans, semantic refs and causal diagnostics.

**Possible solutions:** versioned machine-readable evidence schema in Stage7, embedded/referenced by Stage8 artifacts later. **Recommended.**

**Recommended timing:** before Stage7 CLI/AI integration; artifact container can defer.  
**Owner ruling:** yes for compatibility/version policy.

## GAP-014 — Native artifact naming/schema/versioning is stale/undefined

**Affected stages:** 8+.  
**Affected files:** D14/DL-4, `docs/plan/09...`, roadmap Stage8, `cad-artifact` placeholder.

**SOURCE REQUIREMENT:** D14 owns AICAD naming/extensions; audit brief specifically calls for `.aicadpkg`.  
**CURRENT IMPLEMENTATION FACT:** older plan passages use `cad`, CAD-IR or `.aicad` artifact language that conflicts with later naming where `.aicad` is source and `.aicadpkg` is native package/artifact.  
**Required future capability:** canonical manifest, schema version, source/package distinction, lock/environment identity, deterministic packaging and migration rules.

**Recommended timing:** spec cleanup can happen before Stage5; detailed artifact format before Stage8.  
**Owner ruling:** **Yes** if anything beyond applying D14 is ambiguous.

## GAP-015 — External asset identity/provenance crosses Stage6 and Stage8 but has no shared contract

**Affected stages:** 6, 8, 9.  
**Affected files:** plans 07/09/14; interchange/artifact/provenance placeholders.

**SOURCE REQUIREMENT:** vendor/imported components, STEP hardening and deterministic rebuilds require provenance.  
**CURRENT IMPLEMENTATION FACT:** lower layers can import STEP, but stable source-level external asset identity and locked import settings are not defined.  
**Required future capability:** content hash/URI policy, importer version/settings, healing/fidelity evidence, semantic wrapper identity and change invalidation.

**Possible solutions:** small external-asset identity record introduced when Stage6 first needs imported components, later serialized by Stage8. **Recommended.** Avoid waiting until Stage8 and retrofitting identity into assemblies.

**Recommended timing:** define minimal contract during Stage6; formal artifact serialization Stage8.  
**Owner ruling:** yes for URI/security/reproducibility policy if externally observable.

## GAP-016 — IDE/AI semantic APIs are not yet separated from compiler internals by a versioned service model

**Affected stages:** 9-10.  
**Affected files:** plans 10/11; `cad-lsp` and `cad-agent-tools` placeholders.

**SOURCE REQUIREMENT:** GUI and AI should use source/semantic model authority, semantic refs, provenance, diagnostics and structured queries rather than internal compiler data structures.  
**CURRENT IMPLEMENTATION FACT:** relevant crates are placeholders; current compiler/HIR APIs can evolve freely.  
**Required future capability:** stable semantic query/edit/navigation service boundaries, source transaction edits, versioned machine schemas.

**Possible solutions:** design services late, after Stage4-8 semantic models are real. **Recommended.** Only preservation requirement now: do not erase stable IDs/provenance or make source API equal internal IR.

**Recommended timing:** architecture during Stage9A; no need to build now.  
**Owner ruling:** only for externally stable protocol/compatibility.

## GAP-017 — Strategic multiple-representation/content-addressed/production capabilities need preserved seams, not present implementation

**Affected stages:** 10-13+.  
**Affected files:** system architecture, plans 06/14, strategic backlog.

**SOURCE REQUIREMENT:** long-term backlog includes semantic diff/merge, stronger provenance, content-addressed evaluation, multiple geometry representations, geometry debugger, large assemblies and possible OCCT customization.  
**CURRENT IMPLEMENTATION FACT:** kernel API/runtime boundaries, feature cache keys and provenance provide partial seams; no production-scale implementation is needed now.  
**Required future capability:** ability to add alternate representation/storage/execution policies without changing source semantics.

**Recommendation:** do not create speculative distributed/representation abstraction layers in Stage5. Preserve kernel-neutral source/IR, stable semantic identity, explicit provenance, deterministic cache keys and versioned artifacts; introduce further layers only when a concrete later-stage benchmark requires them.

**Recommended timing:** **SAFE_TO_DEFER**.  
**Owner ruling:** no current ruling required.
