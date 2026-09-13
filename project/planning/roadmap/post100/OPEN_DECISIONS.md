# Open owner decisions

This file does **not** decide owner-controlled questions. It separates architecture decisions from reversible implementation choices and gives the latest safe decision point.

## BEFORE STAGE 5

### OD-S5-01 — What is the canonical language-specification authority and update workflow?

- **Exact question:** Are `specs/language/{grammar,semantics,types,diagnostics}` the canonical compatibility surface, with RFCs/decisions promoted into them, as `specs/language/README.md` currently claims?
- **Evidence:** README claims those files; three are absent. Plans/RFCs contain partially stale/illustrative language material.
- **Options:**
  1. Restore those four canonical specs and require accepted semantic changes to land there. **Recommended.**
  2. Make accepted RFCs the normative language spec and change the README.
  3. Treat implementation/code as normative. Not recommended.
- **Trade-offs:** option 1 adds discipline/docs work but gives one compatibility target. Option 2 preserves history but fragments normative state. Option 3 minimizes docs and maximizes accidental language drift.
- **Downstream:** every Stage5-7 language addition; packages/migrations later.
- **Latest safe decision point:** before the first Stage5 source-visible API/spec task.
- **Recommendation:** option 1.
- **Type:** architecture decision.

### OD-S5-02 — How should trusted runtime-backed standard functions scale beyond the current closed nine-function catalogue?

- **Exact question:** Should the D18 closed runtime mechanism remain a hand-authored `BuiltinFnId` enum/dispatch table, or become a declarative/generated closed registry while preserving ordinary-call semantics and prohibiting untrusted native registration?
- **Evidence:** `cad-hir/src/builtins.rs` intentionally uses a closed enum. Stage5 calls for near-kernel-complete geometry without compiler intrinsics.
- **Options:** hand-maintained closed enum; generated/declarative closed registry; broader native extension registration (requires D12/D15 and is not recommended now).
- **Trade-offs:** manual is simplest but increasingly repetitive; declarative reduces compiler maintenance without weakening trust; dynamic registration prematurely expands the security/ABI boundary.
- **Downstream:** all advanced geometry, future standard library/runtime services.
- **Latest safe decision point:** before Stage5 API expansion creates dozens of catalogue entries.
- **Recommendation:** declarative/generated **closed trusted** registry, preserving existing IDs/ordinary calls.
- **Type:** architecture decision interpreting D18.

### OD-S5-03 — What language/runtime boundary defines unsafe/raw geometry?

- **Exact question:** Is raw geometry enforced by a language-level `unsafe` capability/block, by type separation and explicit raw namespaces alone, or a combination?
- **Evidence:** RFC-0002 requires a distinct unsafe/raw tier and epoch-bound handles; current language has no `unsafe` construct.
- **Options:** language-level unsafe block/effect; opaque raw types/functions with no new syntax; both.
- **Trade-offs:** language-level unsafe is explicit and future-proof but adds privileged semantics; type-only boundary is smaller but may be less visible/auditable if other unsafe capabilities arise.
- **Downstream:** Stage5 low-level topology, plugin/FFI policy later, AI safety/inspection.
- **Latest safe decision point:** before raw topology/editing APIs are exposed.
- **Recommendation:** first test whether opaque raw types + explicit `geometry.raw` functions can enforce all invariants. Add general `unsafe` syntax only if a concrete privilege/effect cannot be represented cleanly.
- **Type:** architecture decision.

### OD-S5-04 — What is the source evaluation model for kernel-backed geometry queries?

- **Exact question:** How may source control flow consume projection/intersection/distance/validation/topology-query results when current interpretation builds a geometry graph before later kernel dispatch?
- **Evidence:** Safe CAD API explicitly leaves this as a future architecture decision. Stage5 needs query results as values.
- **Options:** deterministic demand realization/memoization during evaluation; explicit staged query/evaluation construct; restrict source-visible queries until a later phase.
- **Trade-offs:** demand realization is ergonomic but changes evaluation architecture and resource accounting; explicit staging is clearer but heavier; restriction narrows Stage5 programmability.
- **Downstream:** Stage5 low-level algorithms, Stage7 geometry assertions, AI/query APIs.
- **Latest safe decision point:** before first source-visible kernel query that can influence control flow.
- **Recommendation:** deterministic memoized demand realization behind an explicit effect/service boundary, with cache/dependency/evidence records; exact design needs an RFC.
- **Type:** architecture decision.

### OD-S5-05 — What is the cross-system tolerance taxonomy and override policy?

- **Exact question:** Which tolerance categories exist, where are defaults/versioned profiles owned, and how may source/project/test code override them?
- **Evidence:** D5 profile, sketch solver tolerance and spatial validity epsilons are already distinct; Stage5 adds operation/approximation, Stage7 verification.
- **Options:** typed categories with explicit profiles/overrides; one global tolerance; per-op unstructured epsilons.
- **Trade-offs:** taxonomy adds naming/policy work but preserves reproducibility; global epsilon is simpler and semantically wrong; unstructured epsilons are flexible but non-auditable.
- **Downstream:** geometry robustness, assemblies, verification, artifacts.
- **Latest safe decision point:** before Stage5 intersection/sewing/offset APIs freeze.
- **Recommendation:** typed categories; central policy engine may be deferred.
- **Type:** architecture decision.

### OD-S5-06 — What is semantic feature identity across user functions/control flow?

- **Exact question:** How are feature IDs/dependencies/provenance assigned when geometry is created inside ordinary AICAD functions, parts, repeated calls, loops or branches?
- **Evidence:** Stage3 feature graph intentionally recognizes only a supported shallow set of top-level direct calls; future language/product is programmable.
- **Options:** evaluation/effect-trace-based semantic nodes; compiler inlining/monomorphized graph with provenance; permanent top-level restriction.
- **Trade-offs:** effect tracing aligns with executed semantics but requires stable call-context identity; inlining can preserve static analysis but may explode/complicate recursion; permanent restriction deletes product capability.
- **Downstream:** Stage5 libraries, references, incremental rebuild, IDE, AI.
- **Latest safe decision point:** before Stage5 advanced operations are commonly wrapped in user functions.
- **Recommendation:** semantic execution/effect trace or equivalent that keeps stable source/call provenance; reject permanent restriction.
- **Type:** architecture decision.

## BEFORE STAGE 6

### OD-S6-01 — What are the assembly identity domains and stability rules?

- **Exact question:** How are component definition, assembly definition, logical instance, nested instance path, concrete variant, BOM line and geometry reference identities represented and preserved?
- **Evidence:** plan07 requires all of these capabilities; no assembly implementation exists.
- **Options:** explicit semantic identities derived from source declarations/paths; runtime-generated opaque occurrence IDs; geometry/topology IDs reused.
- **Trade-offs:** explicit identities need design work but support configs/refs/diff; runtime occurrence IDs are simpler but not reproducible; geometry identity is incorrect for logical assemblies.
- **Downstream:** every Stage6 feature and Stage7/9/10 tooling.
- **Latest safe decision point:** before first assembly IR schema.
- **Recommendation:** explicit source-semantic identity domains; runtime IDs may exist only as internal execution handles.
- **Type:** architecture decision.

### OD-S6-02 — What general interface/protocol and bounded-generic semantics should Stage6 rely on?

- **Exact question:** Is conformance explicit (`implements`) or structural; how are generic bounds expressed/resolved; what is the compatibility rule for mechanical interfaces?
- **Evidence:** future plan examples rely on `implements` and `T: Interface`; current AST/type system does not implement them.
- **Options:** explicit nominal interfaces; structural protocols; hybrid.
- **Trade-offs:** nominal is predictable/stable; structural is ergonomic but can create accidental conformance/versioning issues; hybrid is more complex.
- **Downstream:** reusable mechanical interfaces, packages, first-party engineering libraries.
- **Latest safe decision point:** before Stage6 interface/component-generic tasks.
- **Recommendation:** explicit nominal conformance for public engineering interfaces unless evidence strongly favors structural typing; ordinary generic bounds over that mechanism.
- **Type:** language architecture decision.

### OD-S6-03 — What are solver-neutral assembly relation and deterministic pose semantics?

- **Exact question:** Which semantic mate/joint relations are core, how are underconstrained families represented, and what grounding/tie-breaking belongs to AICAD rather than solver implementation?
- **Evidence:** D11 forbids solver-native semantics; plan07 requires deterministic poses/DOF/diagnostics.
- **Options:** AICAD relation IR + solver adapters + deterministic grounding policy; expose solver variables/native constraints; fixed first solver as semantics.
- **Trade-offs:** neutral IR requires mapping work but keeps semantics portable; native model is faster and creates lock-in.
- **Downstream:** mates, joints, kinematics, verification, evidence.
- **Latest safe decision point:** before numerical assembly solver integration.
- **Recommendation:** neutral IR + explicit deterministic gauge/grounding rules.
- **Type:** architecture decision.

### OD-S6-04 — How do configurations preserve identity under suppression and replacement?

- **Exact question:** Does a suppressed/replaced instance retain its logical slot/identity; how do references, BOM entries and provenance behave across configuration resolution?
- **Evidence:** plan07 requires suppression/replacement but does not normatively answer identity consequences.
- **Options:** immutable overlay preserving logical slots; destructive resolved graph; new identity per configuration.
- **Trade-offs:** overlay is richer but makes comparison/ref behavior coherent; destructive/new-ID models are simpler and break cross-config identity.
- **Downstream:** configuration matrices, BOM, verification, visual diff/review.
- **Latest safe decision point:** before configuration implementation.
- **Recommendation:** immutable overlay preserving logical identity and recording concrete variant/provenance.
- **Type:** architecture decision.

### OD-S6-05 — What minimal external-asset identity exists before Stage8 artifacts?

- **Exact question:** How should Stage6 imported/vendor components identify and lock source assets before the full Stage8 artifact format exists?
- **Evidence:** Stage6 includes externally imported components; Stage8 owns broader packaging/provenance.
- **Options:** content digest + stable logical asset ID + importer/settings record; filesystem path only; postpone imported components.
- **Trade-offs:** minimal record is modest and avoids retrofit; path-only is nonreproducible; postponement narrows Stage6 benchmark.
- **Downstream:** imported assemblies, Stage8 serialization, supply-chain provenance.
- **Latest safe decision point:** before Stage6 imported-component task.
- **Recommendation:** minimal content-addressed external-asset record now; Stage8 serializes/extends it.
- **Type:** architecture decision at subsystem seam.

## BEFORE STAGE 7

### OD-S7-01 — What is the normalized verification/obligation model?

- **Exact question:** Which fields are common across sketch constraints, assembly relations, tests and requirements, and which remain domain-specific/lowered solver models?
- **Evidence:** plan08 proposes a normalized conceptual model; current sketch constraint IR is domain-specific.
- **Options:** small common obligation/evidence envelope + domain payloads; one universal concrete constraint enum; fully separate domains.
- **Trade-offs:** envelope shares identity/evidence without false numerical unification; universal enum becomes sprawling; separate domains duplicate traceability/status concepts.
- **Downstream:** tests, requirements, contracts, CLI, AI repair.
- **Latest safe decision point:** before Stage7 verification IR implementation.
- **Recommendation:** common obligation/evidence envelope + typed domain payload/evaluators + solver lowerings.
- **Type:** architecture decision.

### OD-S7-02 — Which verification constructs are privileged language declarations?

- **Exact question:** Should core language include `test`, `requirement`, `assert/expect`, contracts and invariants, or should some be ordinary library/runner mechanisms?
- **Evidence:** future examples mix all layers; AST currently implements none of these items.
- **Options:** minimal core (`test`, `requirement`, perhaps evidence-capturing assertion) + library predicates/runner; all as syntax; all as library functions.
- **Trade-offs:** minimal core preserves discovery/stable IDs without bloat; all syntax is rigid; all library loses universal discovery/source identity.
- **Downstream:** language compatibility, IDE, packages, verification runner.
- **Latest safe decision point:** before Stage7 parser/AST work.
- **Recommendation:** minimal core declaration set; defer contracts/invariants until justified.
- **Type:** language architecture decision.

### OD-S7-03 — How are stable requirement/test/case IDs assigned and evolved?

- **Exact question:** Are IDs explicit user labels, source-derived semantic IDs, generated UUID/content IDs, or a combination; what happens on rename/move/refactor?
- **Evidence:** Stage7 requires stable requirement IDs and traceability; future collaboration/artifacts depend on them.
- **Options:** mandatory explicit IDs; deterministic derived IDs with optional explicit pinning; runtime generated IDs.
- **Trade-offs:** mandatory IDs are stable but noisy; derived IDs are ergonomic but need rename semantics; random runtime IDs are unsuitable for reproducible evidence.
- **Downstream:** coverage, CI history, AI repair, artifact evidence, diff/merge.
- **Latest safe decision point:** before requirement/test declaration spec freezes.
- **Recommendation:** deterministic semantic identity with explicit stable ID/label override for long-lived engineering requirements; never runtime-random as authoritative identity.
- **Type:** architecture decision.

### OD-S7-04 — What compatibility/versioning policy governs verification evidence?

- **Exact question:** How are machine-readable result/evidence schemas versioned, and what build/profile/configuration identity is mandatory?
- **Evidence:** diagnostics schema exists, but verification evidence needs passing measurements and traceability too.
- **Options:** dedicated versioned evidence schema; reuse diagnostic schema; ad hoc JSON per command.
- **Trade-offs:** dedicated schema costs design but supports CI/AI/artifacts; diagnostics-only cannot represent successful evidence; ad hoc output becomes API debt.
- **Downstream:** CLI, CI, AI, Stage8 artifacts.
- **Latest safe decision point:** before Stage7 CLI/machine interface.
- **Recommendation:** dedicated versioned evidence schema referencing diagnostics where appropriate.
- **Type:** architecture/API compatibility decision.

### OD-S7-05 — Do contracts/invariants belong in the Stage7 gate or remain a later additive capability?

- **Exact question:** Must Stage7’s owner gate require `requires`/`ensures`/`invariant` language semantics, or is a verification-first system complete enough with tests/requirements/assertions/matrices/evidence?
- **Evidence:** plan08 includes contracts/invariants, but their privileged semantics are less mature than core tests/requirements.
- **Options:** gate on contracts/invariants; implement if time after core; defer to later language stage while preserving verification IR support.
- **Trade-offs:** requiring them expands compiler semantics substantially; deferral reduces risk without deleting capability if obligations can represent them later.
- **Downstream:** task count, language surface, package API contracts.
- **Latest safe decision point:** when Stage7 queue is approved.
- **Recommendation:** `OPTIONAL_WITHIN_STAGE` or later unless a concrete fixture proves them necessary for the verification-first gate.
- **Type:** roadmap scope decision, not irreversible architecture if obligation IR is extensible.

## BEFORE STAGE 8+

### OD-S8-01 — What exactly is `.aicadpkg`?

- **Exact question:** Is `.aicadpkg` the deterministic native bundle containing source/manifest/lock/evidence/optional caches, a distributable package only, or both project artifact and package?
- **Evidence:** D14 naming and audit brief favor `.aicadpkg`; older plan text is inconsistent.
- **Options:** one versioned bundle format with declared roles; separate project artifact/package formats; source directory only.
- **Trade-offs:** one format is simpler but must avoid conflating editable project and distributable dependency; separate formats are clearer but add tooling.
- **Downstream:** Stage8, packages, registry, IDE sharing.
- **Latest safe decision point:** before Stage8 format implementation.
- **Recommendation:** define a versioned native bundle envelope with explicit manifest `kind`/purpose if both use cases share mechanics; keep source authoritative and caches optional.
- **Type:** format architecture decision.

### OD-S9-01 — Should Stage9 officially split into 9A and evidence-driven 9B?

- **Exact question:** Is full conventional visual authoring an unconditional Stage9 gate, or should the mandatory gate be source-first semantic engineering tooling with visual authoring promoted only by evidence?
- **Evidence:** product principle says source/semantic model is authoritative; core Stage9A capabilities are independently valuable; full CAD GUI is large and risks hidden state.
- **Options:** split 9A/9B; keep one mandatory full-IDE gate.
- **Trade-offs:** split delivers semantic tooling sooner and validates demand; unified gate pursues conventional CAD completeness but greatly increases scope.
- **Downstream:** roadmap duration, UI architecture, source transaction API.
- **Latest safe decision point:** before Stage9 task planning.
- **Recommendation:** split 9A/9B.
- **Type:** roadmap architecture decision.

### OD-S11-01 — Trusted native plugin boundary (existing D12/D15 family)

- **Exact question:** Will third-party extensions ever receive in-process native runtime access, or remain WASM/external-service/capability-mediated by default?
- **Evidence:** package/plugin plan and owner decisions deliberately leave trusted-native boundary open.
- **Options:** no native third-party; explicitly trusted/signed native tier; unrestricted registration (not recommended).
- **Trade-offs:** safety/reproducibility vs maximum performance/integration.
- **Downstream:** packages/plugins, runtime standard-function registration, security.
- **Latest safe decision point:** before Stage11 native extension ABI work; **not** needed for Stage5 standard library functions.
- **Recommendation:** keep closed through Stages5-10; revisit with concrete plugin use cases/threat model.
- **Type:** architecture/security decision.

## LATER / RESEARCH

### OD-R-01 — Multiple geometry representations and evidence-driven OCCT customization

This is **not** a current architecture decision. Preserve kernel-neutral IR/source semantics, collect Stage5-9 robustness/performance evidence, then decide whether alternate exact/mesh/implicit representations or an OCCT fork/customization have measurable value. Choosing now would be speculative.

### OD-R-02 — Distributed/content-addressed evaluation architecture

Stage3 cache keys/dirty propagation are useful foundations, but remote/distributed execution is not required by Stages5-9. Preserve deterministic content identities and artifact provenance; choose storage/distribution architecture only after large-model/large-assembly benchmarks show a bottleneck.

### OD-R-03 — Full conventional visual authoring scope

After Stage9A, use observed user workflows to decide whether sketch creation, feature-tree manipulation, assembly mating, drawing creation, direct manipulation, or only selected visual transactions justify first-party 9B investment. This is usage-driven product architecture, not something Stage5 should pre-optimize for.
