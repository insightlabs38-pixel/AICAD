# Simplification risk register

This register distinguishes implementation simplification from architectural narrowing. The governing test is whether a smaller first implementation preserves a semantic boundary that can later grow without breaking source or corrupting identity/provenance.

## Register

| ID | Proposed/current simplification | Motivation | Immediate complexity saved | Current functionality lost | Future functionality constrained | Later migration/rewrite cost | Semantic/API compatibility risk | Reversibility | Verdict |
|---|---|---|---|---|---|---|---|---|---|
| SIMP-001 | Treat public source geometry API as a 1:1 mirror of `GeometryOp`/OCCT capabilities | Easy dispatch and documentation | Moderate | Little today | Backend independence, source compatibility, alternate kernels/representations | High: source API changes whenever IR/kernel changes | Very high | Low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-002 | Keep only safe opaque `Geometry`; drop raw/unsafe low-level tier | Avoid difficult topology APIs | High in Stage5 | Low-level custom construction/editing/introspection | Near-kernel completeness, reconstruction, advanced algorithms, debugger/tooling | Very high; requires adding a second conceptual system later | Very high | Low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-003 | Expose OCCT topology objects/pointers directly for raw tier | Fastest access to kernel power | High | Kernel neutrality and deterministic portability | Alternate kernels, stable handles, safe epochs, serialization boundaries | Extreme | Extreme | Very low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-004 | Use transient edge/face indices as semantic references beyond Stage4 | Existing Stage2 API already has indices | Moderate | Durable reference behavior | Feature edits, configurations, assemblies, verification, GUI selection | Extreme; models break silently | Extreme silent-wrong risk | Very low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-005 | Represent advanced geometry only with whatever OCCT bridge operations currently exist | Avoid expanding kernel-neutral IR | Moderate | Planned curves/surfaces/topology operations | Stage5 roadmap and non-OCCT implementation | High | High | Medium-low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-006 | Model topology edits as in-place source mutation | Maps directly to kernel algorithms | Moderate | D2 value/functional semantics | Determinism, provenance, incremental rebuild, undo/diff reasoning | High | High | Low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-007 | One global `tolerance` value for all numerical behavior | Simple configuration | Moderate | Correct distinct tolerance meanings | Robust freeform ops, solvers, verification, D5 calibration | High; old artifacts/source ambiguous | High | Low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-008 | Keep Stage3 feature DAG limited to direct top-level runtime builtins and advise users to avoid geometry helper functions | Avoid evaluation-aware graph work | High | Programmable geometry abstraction | Libraries, parts, branching, provenance, semantic refs, incremental eval, IDE/AI | Extreme | Extreme product narrowing | Low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-009 | Add every Stage5 geometry capability as a new compiler intrinsic | Easy privileged dispatch | Low-to-moderate | Ordinary-call extensibility | Language stability, package/library composition | High; compiler owns domain catalog forever | High | Medium-low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-010 | Keep closed trusted runtime functions but generate/declaratively define their catalogue | Reduce hand-maintained enum/signature duplication | Moderate | None if identity/security remain closed | None material; can still later generalize | Low | Low | High | SAFE_SIMPLIFICATION |
| SIMP-011 | Reuse `cad-kernel-api::Frame3/Transform` directly as source/HIR public types | Avoid duplicate conversion/model | Moderate | Separation of source semantics from adapter | Unit-safe evolution, non-kernel representations, compatibility | High | High | Medium-low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-012 | Defer a centralized tolerance-policy engine while freezing tolerance classes and evidence fields now | Avoid premature framework | High | No current user capability if each operation has explicit policy | Little; engine can aggregate later | Low | Low | High | SAFE_IF_IMPLEMENTATION_ONLY |
| SIMP-013 | Treat the initial assembly solver's variables, residuals and grounding choices as assembly semantics | Faster solver integration | High | Solver portability/deterministic engineering intent | Alternate solvers, explainable DOF/conflicts, kinematics | Extreme | Extreme | Very low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-014 | Use geometry occurrence IDs as component instance IDs | One identity system | Moderate | Stable logical assembly identity | Config replacement, BOM, refs, nested instances, diff | Extreme | Extreme | Very low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-015 | Implement configurations by deleting suppressed nodes and swapping replacements in-place | Simple resolved graph | Moderate | Stable logical slot/identity/provenance | Reference durability, BOM diffs, visual review | High | High | Low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-016 | Make interface/`implements` assembly-only special syntax rather than implementing/generalizing language interfaces | Shorter Stage6 compiler work | Moderate | General protocol capability | Packages, generic libraries, engineering modules | High; duplicate language concepts | High | Low | DEFER_IMPLEMENTATION_BUT_PRESERVE_ARCHITECTURE |
| SIMP-017 | Encode mates/joints as hard-coded solver API calls with no solver-neutral relation IR | Quick demo | High | Portable semantic relation representation | DOF analysis, alternate solvers, verification, serialization | Extreme | Extreme | Very low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-018 | Omit overconstraint/redundancy evidence and return only “solve failed” | Simpler solver adapter | Moderate | Explainability | Verification, AI repair, user debugging | High | Medium-high | Medium | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-019 | Treat test/requirement/assertion behavior as interpreter-only magic | Fast syntax implementation | Moderate | Reusable predicates/orchestration separation | CI, tooling, plugins, machine evidence, alternate runner | High | High | Low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-020 | Keep geometry assertion predicates in first-party libraries where ordinary typed functions suffice | Minimize privileged syntax | High | None | None; library can expand independently | Low | Low | High | SAFE_SIMPLIFICATION |
| SIMP-021 | Add dedicated compiler syntax for every hard/soft requirement kind and every geometry predicate | Attractive DSL readability | Low initially | Generality | Engineering-module growth, plugins, user-defined predicates | High | High | Medium-low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-022 | Defer contracts/invariants until Stage7 evidence shows they need privileged call-boundary semantics | Avoid speculative language work | High | Contract syntax in first Stage7 slice | Little if verification IR can represent equivalent obligations | Low | Low if capability remains in roadmap | High | DEFER_IMPLEMENTATION_BUT_PRESERVE_ARCHITECTURE |
| SIMP-023 | Treat diagnostic JSON as sufficient verification evidence | Reuse existing schema | Moderate | Stable subject/result/measurement/profile model | CI history, regulated traceability, AI repair, artifact evidence | High | Medium-high | Medium | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-024 | Make native B-rep cache authoritative project state | Faster load and easier artifact | Moderate | Source-authoritative model | Migration, reproducibility, multiple kernels, semantic diff | Extreme | Extreme | Very low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-025 | Require full traditional visual CAD authoring as an unconditional Stage9 gate | Familiar CAD product framing | None; adds work | Delays source-first environment | Forces dual-authority pressure and large UI scope before evidence | High sunk cost | High | Medium | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-026 | Split Stage9 into 9A source-first semantic environment and evidence-triggered 9B visual authoring | Sequence risk and validate demand | High near-term | No core source-first capability | None if source-edit transaction boundary is preserved | Low | Low | High | SAFE_IF_IMPLEMENTATION_ONLY |
| SIMP-027 | Defer multiple geometry representations/distributed content-addressed execution | Avoid architecture astronautics | Very high | No near-stage requirement | Could delay scale research only | Low if kernel/IR/cache boundaries remain clean | Low | High | DEFER_IMPLEMENTATION_BUT_PRESERVE_ARCHITECTURE |
| SIMP-028 | Collapse source AST/HIR/geometry IR/kernel representation into fewer shared structures | Fewer conversions | Moderate | Layer independence | compiler refactors, alternate runtime/kernel, stable tooling APIs | Extreme | Extreme | Very low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-029 | Use fingerprint similarity as automatic default recovery for broken semantic refs | Better apparent resilience | Moderate | Fail-closed certainty | Safety, traceability, verification trust | High and potentially silent | Extreme silent-wrong risk | Low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-030 | Keep fingerprints as evidence/ranking fallback disabled by default until benchmark/owner policy allows recovery | Preserve safety | Low | Automatic convenience | None architecturally | Low | Low | High | SAFE_IF_IMPLEMENTATION_ONLY |
| SIMP-031 | Put gears/fasteners/bearings/vendor catalogs into compiler semantics | Convenient built-ins | Low | Library extensibility | Domain growth, packages, third-party modules | High | High | Low | REJECT_ARCHITECTURAL_SIMPLIFICATION |
| SIMP-032 | Implement such domain mechanisms as first-party libraries over core geometry/interfaces/configuration | Keep compiler small | High | None if core primitives sufficient | None; improves extensibility | Low | Low | High | SAFE_SIMPLIFICATION |

## Simplifications that would create future roadmap dead ends

### 1. “Just expose OCCT”

**SOURCE REQUIREMENT:** RFC-0002/D6 require kernel-neutral public semantics and distinguish durable semantic identity from transient raw topology.  
**INFERENCE:** directly exposing OCCT seems attractive during Stage5 because the project currently has one production kernel bridge. It would turn current implementation circumstance into source semantics, making later backend replacement, deterministic replay, serialization, package ABI, and AI/tooling interfaces kernel-specific.  
**VERDICT:** `REJECT_ARCHITECTURAL_SIMPLIFICATION`.

### 2. “Advanced users can use raw handles everywhere”

This saves work on semantic topology operations but makes every advanced model epoch-sensitive and unfit for persistence/rebuild. The correct boundary is **raw power inside a controlled tier plus explicit promotion into safe semantic geometry**, not one universal raw handle type.

**VERDICT:** `REJECT_ARCHITECTURAL_SIMPLIFICATION`.

### 3. “The feature graph only needs known builtins”

The current Stage3 restriction is an implementation checkpoint, not a language principle. Keeping it permanent would make abstraction itself destroy inspectability. AICAD would become most transparent only when users avoid writing functions and reusable libraries, which contradicts the product.

**VERDICT:** `REJECT_ARCHITECTURAL_SIMPLIFICATION`.

### 4. “One epsilon is simpler”

A solver convergence threshold, NURBS approximation chord error, sewing tolerance, verification acceptance band, frame orthonormality threshold, and D5 cross-kernel equivalence bound answer different questions and often carry different units/scaling laws. One project epsilon is not simplification; it is semantic aliasing.

**VERDICT:** `REJECT_ARCHITECTURAL_SIMPLIFICATION`.

### 5. “Assembly = transformed parts + solver state”

This deletes the logical identity/interface/configuration/BOM layer that gives an assembly engineering meaning. Solver state must be a computed realization of semantic relations, not the authority.

**VERDICT:** `REJECT_ARCHITECTURAL_SIMPLIFICATION`.

### 6. “Tests are just special interpreter statements”

Stage7 needs discovery, matrices, profiles, traceability, stable evidence, CLI/CI and AI consumption. Those concerns live across language declarations, runtime/library evaluation and build orchestration. Putting all of them into interpreter syntax creates an untestable monolith and blocks alternative runners.

**VERDICT:** `REJECT_ARCHITECTURAL_SIMPLIFICATION`.

### 7. “Visual authoring is required for the IDE to count”

The product can deliver a complete source-first engineering environment with semantic viewport, selection, navigation, measurement, diagnostics, parameter transactions and diff/review without yet implementing conventional sketch/feature-tree visual creation. Treating visual authoring as evidence-driven 9B prevents a large UI program from becoming a prerequisite for semantic tooling.

**VERDICT:** `SAFE_IF_IMPLEMENTATION_ONLY` to defer visual authoring; `REJECT_ARCHITECTURAL_SIMPLIFICATION` if the source-edit transaction capability itself is removed.

## Safe implementation deferrals that preserve architecture

- Implement the **tolerance taxonomy** before a centralized policy engine.
- Define **raw/safe types and adoption semantics** before trying to cover every kernel algorithm.
- Define the **assembly relation IR** before optimizing or supporting multiple numerical solvers.
- Define the **verification evidence schema** before mutation-testing breadth or advanced contracts.
- Build **Stage9A semantic inspection/edit transactions** before evidence-driven visual creation tools.
- Preserve kernel/runtime/IR seams now while deferring multiple geometry representations, distributed caching, or custom OCCT forks until a benchmark proves need.

These save meaningful implementation effort while keeping future source semantics and compatibility intact.
