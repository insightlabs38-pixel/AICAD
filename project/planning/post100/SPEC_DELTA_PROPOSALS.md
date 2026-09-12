# Specification delta proposals

These are **proposals only**. Existing RFCs/specifications/plans were not edited. Each delta identifies stale or insufficient normative material that should ideally be corrected before the dependent stage starts.

## Priority legend

- **MUST-BEFORE-S5** — ambiguity can distort Stage5 architecture/source semantics.
- **MUST-BEFORE-S6** — may wait through Stage5, but not assembly implementation.
- **MUST-BEFORE-S7** — may wait through Stage6, but not verification implementation.
- **SHOULD-BEFORE-S8+** — later-stage compatibility/clarity work.

---

## SPEC-DELTA-001 — Restore the canonical language-specification set

- **Target:** `specs/language/README.md` and new canonical files it already names.
- **Section:** canonical artifact list / status.
- **Current assumption:** README says `grammar.ebnf`, `semantics.md`, `types.md`, and `diagnostics.md` are the language specification.
- **Problem:** only README + grammar exist at the audited revision; semantics/types/diagnostics therefore have no canonical home.
- **Downstream:** 5, 6, 7 and all later source compatibility.
- **Proposed normative behavior:** define source evaluation/value semantics, declaration semantics, type invariants/generic/interface rules, unsafe capability rules, and language diagnostic guarantees in versioned canonical specs. Plans/examples remain informative unless promoted through RFC/owner process.
- **Compatibility:** documentation/spec completion; should describe existing behavior before extending it.
- **Owner decision required:** **Yes** — confirm authority/approval model.
- **Urgency:** **MUST-BEFORE-S5**.

**Proposed addition:**
> The language specification consists of grammar, semantics, type-system, and diagnostics documents. Implementation behavior not represented in these artifacts is not automatically a source-language guarantee. Roadmap examples are non-normative unless explicitly incorporated by an accepted RFC/owner decision and reflected here.

## SPEC-DELTA-002 — Apply D2 functional/value semantics to all future geometry examples

- **Target:** `docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`, `07...`, `18_REFERENCE_EXAMPLES.md`; canonical semantics spec once restored.
- **Current assumption:** several future examples read like in-place edits or state mutation.
- **Problem:** D2/DL-2 establishes functional/value semantics; method sugar must not imply object mutation.
- **Downstream:** 5-7, diff/incremental/provenance.
- **Proposed normative behavior:** every modeling/topology/configuration operation returns a new semantic value/resolved model. Method-call syntax, if used, lowers to an ordinary functional call. Raw kernel implementation may mutate private native objects internally, but no alias-observable mutation enters AICAD semantics.
- **Compatibility:** compatible with current functional core; examples change, not capability.
- **Owner:** No new decision if applying D2 literally; escalate only if true source mutation is proposed.
- **Urgency:** **MUST-BEFORE-S5**.

## SPEC-DELTA-003 — Replace D6-incompatible pointer/kernel examples with opaque epoch-bound AICAD handles

- **Target:** future-plan low-level examples (`05...`, `18...`) and restored type/semantics spec.
- **Current assumption:** illustrative raw examples include pointer-looking/kernel-shaped types.
- **Problem:** D6/RFC-0002 forbids kernel-specific public handles; Stage4 AICAD-093 owns epoch-bound raw-handle semantics.
- **Downstream:** 5, 9/10 tooling, future alternate kernels.
- **Proposed normative behavior:** raw topology values are opaque AICAD-owned handle types scoped to a kernel session/shape epoch, nonserializable/nonsemantic, rejected when stale. No pointer arithmetic, pointer identity, OCCT class names, or backend APIs are source semantics.
- **Compatibility:** replaces only illustrative spellings; preserves raw capability.
- **Owner:** No if consistent with Stage4 decision; yes if raw handles gain additional persistence/effect semantics.
- **Urgency:** **MUST-BEFORE-S5**.

## SPEC-DELTA-004 — Extend D7 semantic-reference rules explicitly to advanced topology transformation

- **Target:** RFC-0003 + semantic-reference spec after Stage4; `docs/plan/05...`.
- **Current assumption:** Stage4 defines baseline lineage/resolution for ordinary feature changes; Stage5 introduces trim/sew/heal/split/merge/delete/edit operations.
- **Problem:** advanced operations can invalidate baseline lineage assumptions.
- **Downstream:** 5-9.
- **Proposed normative behavior:** every topology-changing operation must emit a standardized lineage classification (`unchanged/new/modified/split/merged/deleted` or accepted successor model) sufficient for resolver replay. When a recipe cannot resolve uniquely under approved precedence, resolution is ambiguous/broken, never guessed. Fingerprints remain evidence/fallback only according to owner-approved policy.
- **Compatibility:** extends Stage4 semantics without changing source recipes.
- **Owner:** Maybe; required if advanced cases change resolver precedence or recovery policy.
- **Urgency:** **MUST-BEFORE-S5 topology editing completion**.

## SPEC-DELTA-005 — Clarify D11 as a family of semantic relation IRs plus solver lowerings, not one monolithic solver IR

- **Target:** D11/DL-20 explanatory text; `docs/plan/08...`; future constraint spec.
- **Current assumption:** “AICAD constraint IR authoritative; solver numerical only” can be misread as requiring sketch, assembly and engineering requirements to share one concrete equation struct.
- **Problem:** current `SketchConstraint` has sketch-specific entity IDs and relations; Stage6 joints and Stage7 requirements have different evaluator semantics.
- **Downstream:** 6-7.
- **Proposed normative behavior:** AICAD owns semantic relation/constraint representations. Domains may have typed payloads and may lower to one or more numerical solver models. Solver-native variable IDs, penalty weights, ordering and residual implementation are not public semantics. A small cross-domain obligation/evidence envelope may be shared where useful.
- **Compatibility:** preserves D11, avoids overgeneralizing current sketch structs.
- **Owner:** **Yes** for canonical domain/common boundary.
- **Urgency:** **MUST-BEFORE-S6**.

## SPEC-DELTA-006 — Apply D14 naming consistently to artifacts and packages

- **Target:** `docs/plan/09_FILE_FORMAT_INTERCHANGE_RECONSTRUCTION.md`, `15_IMPLEMENTATION_ROADMAP.md`, older examples mentioning CAD-IR/cad/native `.aicad` artifact.
- **Current assumption:** older names coexist with later AICAD naming.
- **Problem:** `.aicad` source and native packaged artifact can become ambiguous; audit brief expects `.aicadpkg`.
- **Downstream:** 8, packages, IDE/tooling.
- **Proposed normative behavior:** `.aicad` = source; `aicad.toml` (per D14 naming decision) = project/package manifest as applicable; `.aicadpkg` = versioned deterministic native bundle/artifact if owner confirms; internal IR names are not file-format promises.
- **Compatibility:** artifact format not yet frozen, so correction is low-cost now.
- **Owner:** **Yes** only for the exact `.aicadpkg` role if D14 did not already freeze it.
- **Urgency:** cleanup now desirable; **SHOULD-BEFORE-S8**.

## SPEC-DELTA-007 — Finish D16 deterministic collection semantics needed by public APIs

- **Target:** RFC-0004 / language types spec.
- **Current assumption:** `Map`/`Set` and iteration are accepted capabilities, but external ordering/serialization details are not sufficiently concrete for evidence/configuration/query APIs.
- **Problem:** hash iteration order can leak into canonical artifacts, diagnostics or solver/query results.
- **Downstream:** 5-8 and AI/CLI.
- **Proposed normative behavior:** Map/Set semantic equality is independent of hash-table layout. Any externally observable iteration, diagnostic candidate list, canonical serialization or evidence output must use a specified deterministic order. Implementations may use unordered internal storage if they sort at observation boundaries.
- **Compatibility:** no source break before implementation.
- **Owner:** Yes for default iteration-order semantics; internal storage remains implementation detail.
- **Urgency:** **MUST-BEFORE-S5** if public Stage5 maps/sets ship; otherwise before first public use.

## SPEC-DELTA-008 — Finish D17 interface/generic-bound semantics before Stage6 depends on them

- **Target:** RFC-0004 + language type spec/grammar.
- **Current assumption:** general generics/interfaces are desired; future assembly examples use `T: Interface` and `implements`.
- **Problem:** no implemented bounded-generic/conformance semantics; assembly could accidentally create a one-off protocol system.
- **Downstream:** 6, 11, 12.
- **Proposed normative behavior:** define interface member requirements, explicit/structural conformance policy, `implements` spelling if any, bound satisfaction, generic inference, coherence/orphan policy if relevant, and diagnostics. Mechanical interfaces are ordinary interfaces/protocol values plus assembly-domain semantics, not compiler-special builtins.
- **Compatibility:** new language surface.
- **Owner:** **Yes**.
- **Urgency:** **MUST-BEFORE-S6**.

## SPEC-DELTA-009 — Scale D18 runtime-backed standard functions without converting the geometry catalogue into language magic

- **Target:** RFC-0001 explanation / D18/DL-15 / `docs/API/safe-cad-api.md` architecture section.
- **Current assumption:** trusted runtime standard functions are ordinary calls represented by a closed `BuiltinFnId` catalogue.
- **Problem:** Stage5 near-kernel completeness makes a hand-coded compiler enum/dispatch surface large; a future agent may either expose IR 1:1 or invent compiler intrinsics.
- **Downstream:** 5+.
- **Proposed normative behavior:** privileged runtime implementations remain a closed trusted capability, but declarations/signatures/effects/docs may be generated from a declarative AICAD-owned catalogue/registry. Adding an ordinary runtime-backed function does not require new grammar/typing semantics. Packages/plugins cannot inject arbitrary native callbacks absent separate D12/D15 authorization.
- **Compatibility:** can preserve existing `BuiltinFnId` identities as generated IDs.
- **Owner:** **Yes**, interpreting D18 scalability.
- **Urgency:** **MUST-BEFORE-S5 broad API expansion**.

## SPEC-DELTA-010 — Establish a tolerance taxonomy; explicitly keep D19/D5 equivalence separate

- **Target:** validation/tolerance specification spanning `docs/plan/05...`, `08...`, `23...`; D19 explanatory material.
- **Current assumption:** “tolerance” appears in multiple subsystems without one taxonomy.
- **Problem:** current code already has D5 comparison tolerance, sketch-solver convergence tolerance, and spatial validity thresholds; Stage5/7 add more.
- **Downstream:** 5-8+.
- **Proposed normative behavior:** define at least:
  1. **geometry-operation tolerance** — robustness/acceptance for construction, sewing, intersection, healing;
  2. **approximation tolerance** — permitted approximation error (e.g. curve/surface approximation/tessellation where semantically relevant);
  3. **solver tolerance** — numerical convergence/residual criteria for sketch/assembly numerical backends;
  4. **verification tolerance** — requirement/test acceptance band selected explicitly or by profile;
  5. **D5 equivalence tolerance** — validation comparison between results/environments;
  6. optionally **representation-validity epsilon** — private invariant guard for normalized vectors/rigid transforms, not a project acceptance tolerance.

  Each value has units/dimensionality, scope, default/version source and evidence reporting. No category silently inherits another merely because numeric values happen to match.
- **Compatibility:** current D5 and solver profiles remain valid members of distinct categories.
- **Owner:** **Yes** for source/profile defaults/override hierarchy.
- **Urgency:** **MUST-BEFORE-S5**.

**Proposed normative sentence:**
> Numerical tolerances are purpose-typed policy values. A tolerance selected for construction robustness, approximation, numerical convergence, verification acceptance, or equivalence validation MUST NOT be reused for another purpose unless the governing specification explicitly maps the two policies.

## SPEC-DELTA-011 — Freeze spatial invariants above the kernel adapter

- **Target:** language type/semantics spec, `docs/plan/03...`, AICAD-075A output/API docs.
- **Current assumption:** source `Axis3`/`Frame3` are passive structs while kernel adapter equivalents enforce normalized/right-handed orthonormal invariants.
- **Problem:** downstream APIs need reliable frames independent of kernel adapter validation details.
- **Downstream:** 5-7.
- **Proposed normative behavior:** define source-semantic `Direction3`, axis, frame, rotation and rigid transform invariants; construction failure behavior; composition/inverse; units; equality/approximate comparison boundaries. Adapter conversion validates but does not define semantics.
- **Compatibility:** source types are new/incomplete, ideal time to fix.
- **Owner:** if AICAD-075A does not resolve it.
- **Urgency:** **MUST-BEFORE-S5**.

## SPEC-DELTA-012 — Specify feature-DAG identity/effects through user functions, parts and control flow

- **Target:** `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` and feature-DAG architecture spec/RFC.
- **Current assumption:** plan describes a semantic feature DAG; Stage3 implementation recognizes a deliberately narrower set of direct top-level supported calls.
- **Problem:** future programmable geometry can bypass graph visibility simply by moving calls into helpers/parts/branches.
- **Downstream:** 5, 7, 9, 10.
- **Proposed normative behavior:** geometry/feature-producing effects executed through ordinary source abstractions must retain deterministic feature identity, dependencies, source call/definition provenance and cache invalidation. The graph need not mirror every AST call; it must represent every semantically material geometry operation/result required for refs/provenance/rebuild. Define identity under repeated calls/loops/branches.
- **Compatibility:** extends an intentionally narrow Stage3 checkpoint.
- **Owner:** **Yes**.
- **Urgency:** **MUST-BEFORE-S5 advanced-library integration gate**.

## SPEC-DELTA-013 — Define kernel-backed source-query evaluation semantics

- **Target:** D18 API docs / runtime architecture / `docs/plan/05...` query operations.
- **Current assumption:** current interpreter builds a geometry graph first; kernel dispatch happens later. Safe API explicitly notes that making source control flow depend on kernel queries is a future architecture decision.
- **Problem:** Stage5 projection, intersection, distance, validation and topology inspection naturally return values source code may branch on.
- **Downstream:** 5+.
- **Proposed normative behavior:** choose one explicit evaluation model: staged demand evaluation with deterministic memoized kernel query effects, a compile/build phase that can realize required geometry subgraphs, or a restricted query service boundary. Query results must have dependency/cache keys and deterministic evidence; arbitrary file/network effects remain excluded.
- **Compatibility:** current graph-building calls remain valid.
- **Owner:** **Yes**.
- **Urgency:** **MUST-BEFORE-S5** if query results are source-visible.

## SPEC-DELTA-014 — Define assembly identity independently of solver/topology identity

- **Target:** `docs/plan/07...`; future assembly spec.
- **Current assumption:** component/instance examples imply named identities but do not normatively separate all identity domains.
- **Problem:** configs/BOM/refs/diff depend on stability rules.
- **Downstream:** 6-10.
- **Proposed normative behavior:** component-definition identity, assembly-definition identity, logical instance identity, nested instance path, concrete variant identity, BOM identity and geometry semantic-reference identity are distinct. Solver variables and raw topology handles are never instance identity. Pose changes do not change logical instance identity. Replacement behavior explicitly states which logical identity survives.
- **Compatibility:** no implementation yet.
- **Owner:** **Yes**.
- **Urgency:** **MUST-BEFORE-S6**.

## SPEC-DELTA-015 — Define deterministic assembly pose/DOF semantics independent of numerical backend

- **Target:** `docs/plan/07...`; D11 extension.
- **Current assumption:** mates/joints are solver-neutral in principle, but underconstrained pose/gauge and diagnostic semantics are not fixed.
- **Problem:** two valid solver outputs could differ by arbitrary rigid motion or redundant-coordinate choices.
- **Downstream:** 6-7.
- **Proposed normative behavior:** specify grounding/root conventions, initial-pose influence, underconstrained-family representation, DOF counting policy, residual/evidence normalization and deterministic tie-breaking that is part of AICAD layer rather than solver internals.
- **Compatibility:** new behavior.
- **Owner:** **Yes**.
- **Urgency:** **MUST-BEFORE-S6 solver task**.

## SPEC-DELTA-016 — Define configuration overlay semantics rather than mutation/deletion

- **Target:** `docs/plan/07...`; future configuration spec.
- **Current assumption:** suppression/replacement examples do not define identity/ref/provenance consequences.
- **Problem:** naive delete/swap behavior destroys stable model identity.
- **Downstream:** 6-9.
- **Proposed normative behavior:** a configuration is an immutable named overlay/rule set resolved against a base semantic model. Suppression changes active/resolved status without silently reassigning logical IDs. Replacement binds a logical slot to a compatible concrete definition/variant and records provenance. Semantic-reference and BOM behavior are explicit for inactive/replaced subjects.
- **Compatibility:** no implementation yet.
- **Owner:** **Yes**.
- **Urgency:** **MUST-BEFORE-S6 configurations**.

## SPEC-DELTA-017 — Define minimum privileged verification language vs library/runner responsibilities

- **Target:** `docs/plan/08...`; grammar/language specs; `cad-requirements` design.
- **Current assumption:** examples mix declarations, operators, predicates, contracts and orchestration.
- **Problem:** implementing all behavior as syntax/interpreter cases would create compiler bloat; implementing all as ordinary functions loses discovery/stable requirement identity/source traceability.
- **Downstream:** 7+.
- **Proposed normative behavior:** core language should own only semantics requiring universal understanding (likely `test`/`requirement` declarations, stable IDs/source metadata, possibly minimal assertion capture). Domain predicates remain typed library functions; matrices/sweeps/profiles/discovery execution belong to build/test orchestration. Contracts/invariants are separate opt-in language proposals.
- **Compatibility:** new behavior.
- **Owner:** **Yes**.
- **Urgency:** **MUST-BEFORE-S7**.

## SPEC-DELTA-018 — Version verification evidence independently from human diagnostics

- **Target:** `docs/plan/08...`, `17_CLI_DIAGNOSTICS_SCHEMA.md`, future verification schema.
- **Current assumption:** structured diagnostics exist, but verification evidence is broader than errors.
- **Problem:** a passing test/requirement can still require measurements, subject identity, tolerance/profile/configuration and provenance; diagnostics alone cannot represent it.
- **Downstream:** 7-10+.
- **Proposed normative behavior:** define versioned verification-result/evidence schema with test/requirement/case IDs, status, subjects, source spans, semantic refs, observed/expected values, effective tolerance/policy, configuration/profile, build identity and diagnostic links. Human CLI is a rendering of the same results.
- **Compatibility:** additive.
- **Owner:** yes for schema compatibility/versioning policy.
- **Urgency:** **MUST-BEFORE-S7 CLI/AI benchmark**.

## SPEC-DELTA-019 — Correct stale RFC decision-status text

- **Target:** RFC-0002, RFC-0004, RFC-0005 and any plan review tables whose “open” statuses predate the decision log.
- **Current assumption:** some RFC text still describes D5/D10/D11 or related questions as open.
- **Problem:** future agents may re-litigate resolved decisions or implement stale alternatives.
- **Downstream:** all future stages.
- **Proposed normative behavior:** annotate superseded status lines with the resolving decision-log entry while preserving historical RFC text where desired.
- **Compatibility:** documentation/status correction only.
- **Owner:** No new semantic ruling if merely recording existing decisions.
- **Urgency:** **MUST-BEFORE-S5** for audit clarity.

## SPEC-DELTA-020 — Officially split Stage 9A from evidence-driven Stage 9B

- **Target:** `docs/plan/10_IDE_HUMAN_UX.md`, `15_IMPLEMENTATION_ROADMAP.md` Stage9.
- **Current assumption:** old Stage9 heading can be read as requiring a full conventional visual-authoring environment in one gate.
- **Problem:** this over-couples essential source-first semantic tooling to expensive visual creation workflows and pressures the UI toward a second source of truth.
- **Downstream:** 9-10.
- **Proposed normative behavior:**
  - **Stage 9A — source-first engineering environment:** LSP/editor integration, diagnostics, navigation, viewport, selection, semantic/ref/provenance inspection, measurement, source↔geometry navigation, parameter edits as source transactions, feature-DAG inspection, model diff/review.
  - **Stage 9B — evidence-driven visual authoring:** sketch/feature/assembly creation/manipulation only after usability/usage evidence shows priority; every edit commits a deterministic source transaction; no hidden model state.
- **Compatibility:** capability-preserving sequencing change; does not ban visual authoring.
- **Owner:** **Yes** because it changes roadmap gate shape.
- **Urgency:** **SHOULD-BEFORE-S8/9 planning**, safe to record now.

## SPEC-DELTA-021 — Separate native semantic artifact from caches/interchange

- **Target:** `docs/plan/09...`, artifact spec.
- **Current assumption:** source/package artifact, B-rep caches and external formats appear near each other in plan.
- **Problem:** implementation convenience could make native B-rep or STEP authoritative.
- **Downstream:** 8-13.
- **Proposed normative behavior:** authoritative reconstructable state is source + manifest/lock + immutable external asset identities/settings. `.aicadpkg` may bundle sources, lock, evidence and optional caches. B-rep/mesh/interchange are derived artifacts unless explicitly imported as external assets with provenance. Cache absence must not change semantics.
- **Compatibility:** consistent with AGENTS/project principles.
- **Owner:** yes for exact artifact contents/versioning.
- **Urgency:** **SHOULD-BEFORE-S8**.

## SPEC-DELTA-022 — Make external asset identity available before Stage8 serialization

- **Target:** plans 07/09/14, future provenance/artifact specs.
- **Current assumption:** artifact/import provenance is staged later than assemblies, but Stage6 already expects vendor/external components.
- **Problem:** Stage6 could adopt an ad hoc path/index identity that Stage8 must replace.
- **Downstream:** 6, 8, 9.
- **Proposed normative behavior:** define minimal external asset record before Stage6 imported components: stable logical asset ID, content digest, origin locator metadata (non-authoritative), importer/version/settings, optional healing report. Stage8 later defines package serialization.
- **Compatibility:** additive.
- **Owner:** yes if locator/security policy becomes public.
- **Urgency:** **MUST-BEFORE-S6 imported-component task**.
