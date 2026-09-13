# AICAD Plan Bundle — Orientation Pass

Performed per `project/FIRST_PROMPT.md`, after AICAD-001 (verbatim plan
import, see `project/reports/AICAD-001.md`) and before executing AICAD-002
onward. This is an analysis/reading pass only — no product features were
implemented, and no architecture decision was resolved. Unresolved owner
decisions surfaced here are recorded in `project/OWNER_DECISIONS.md`.

Scope read: all 24 documents in `docs/plan/` (`README.md` and
`00_PRINCIPLES_AND_SCOPE.md` through `23_CROSS_SYSTEM_PARAMETER_CATALOG.md`),
cross-referenced against `AGENTS.md`, `AICAD_AGENT_OPERATING_MODEL.md`,
`CURRENT_STAGE.md`, and `project/TASKS.yaml` (`aicad_tasks_001_100.yaml`,
tasks AICAD-001 through AICAD-100, covering Stages 0-4 only).

---

## 1. Document map — which plan document governs each subsystem

| Document | Governs |
|---|---|
| `README.md` | Program thesis, document index (see §6 for a gap in this index), recommended first-implementation stack, ten core success criteria |
| `00_PRINCIPLES_AND_SCOPE.md` | Product definition, foundational thesis, 13 non-negotiable language rules, design goals, explicit non-goals, canonical data hierarchy, high/low abstraction split, raw-topology safety model, STEP interoperability rule, five-question quality bar |
| `01_SYSTEM_ARCHITECTURE.md` | End-to-end architecture diagram, 10 major services, process-boundary recommendation, first-implementation stack (Rust/OCCT bridge), compiler IR layers, canonical state/cache model, incremental build strategy, kernel-independence contract, execution capability tiers (A-D) |
| `02_LANGUAGE_AND_COMPILER.md` | Syntax philosophy, source file forms, lexical elements, declaration forms, mutability model, functions, control flow, iterators, pattern matching, modules/imports, `comptime`, macros, purity/determinism, error model, execution budgets, raw-handle lifetime/epoch model, 17-phase compiler pipeline, editor (Tree-sitter) parser strategy, conformance artifacts |
| `03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` | Primitive types, dimensional quantity types, unit literals, quantity representation, tolerances/ranges/fits, geometry types (safe + raw), semantic engineering types, collections, structs/enums, interfaces/generics, ownership/value semantics, nullability, control flow, recursion, closures, generators, purity, unsafe blocks, numerical-precision policy, parameter/rationale metadata |
| `04_HIGH_LEVEL_MODELING_API.md` | The default engineering feature catalog (box/cylinder/.../datum_axis — ~35 operations with parameter tables), sketch constraint catalog, feature composition, named semantic outputs |
| `05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` | Systems-programming geometry layer: curve/surface constructors, topology construction (`make_edge`...`make_solid`), healing/validation, raw pointer-like access (`raw_face`/`raw_shape`/`adopt_validated`), continuity enum, query-over-index philosophy, versioned kernel escape hatch, introspection normalization schema |
| `06_REFERENCES_QUERIES_FEATURE_DAG.md` | **Foundational for Stage 4.** Stable reference classes vs. raw handles, 7 reference-construction strategies, explicit semantic exports, query model + predicates, ambiguity-must-be-explicit rule, topology lineage graph, feature DAG node shape, incremental invalidation algorithm, reference durability levels, `cad refs check` |
| `07_ASSEMBLIES_KINEMATICS_CONFIGURATIONS.md` | Assembly philosophy, component/instance/mate/joint types and parameter tables, DOF analysis, interfaces/ports, configuration system + rules, suppression/replacement, nested assemblies, contact model, kinematic evaluation API, BOM semantics, large-assembly strategy |
| `08_CONSTRAINTS_REQUIREMENTS_TESTS.md` | Unified constraint domains/IR, hard vs. soft constraints, solver diagnostics, executable `requirement` object, CAD unit `test` object, test categories, geometry assertion library, parameterized tests, requirement traceability, contracts/invariants, build profiles, test result schema, verification-coverage metric |
| `09_FILE_FORMAT_INTERCHANGE_RECONSTRUCTION.md` | `.aicad` bundle structure/manifest, deterministic packaging rules, STEP/BREP/3MF/STL/DXF/glTF export targets and export API, import model + healing/normalization, parametric reconstruction pipeline, round-trip test fixtures, external asset policy, artifact fidelity report |
| `10_IDE_HUMAN_UX.md` | IDE workspace layout, bidirectional source<->geometry mapping, bidirectional parameter editing, visual feature creation, sketch editor, feature DAG view, geometry debugger, time-travel, REPL, inspector, LSP feature list, code actions, visual node mode, diff/review UI, AI UX controls, "explain selection" |
| `11_AI_NATIVE_SKILLS_AND_AGENT_PROTOCOL.md` | **Formal AI-learnability requirement.** Skill hierarchy (core/geometry/engineering), progressive disclosure, tool protocol (`cad.build/test/inspect/query/docs/search/explain/render/diff/export/packages`), structured-diagnostic contract, agent loop, package skill contract, org/project skill layering, learnability benchmark design, context packets |
| `12_PACKAGES_PLUGINS_EXTENSIONS.md` | Package structure/manifest, package manager CLI, lockfile, 4 extension levels (pure source / WASM / external process / trusted native), capability manifest, plugin ABI principles, package-defined types/validators/UI/AI skill, API schema, package testing, registry governance, std-library promotion policy |
| `13_ENGINEERING_MODULES.md` | Materials/mass properties, drawings (sheets/views/dimensions), PMI/GD&T (datums, tolerance frames, surface finish), DFM (CNC/additive/injection-molding/sheet-metal), simulation (structural/thermal/CFD-EM-later), optimization, uncertainty/tolerance-stack, cost/sustainability metadata, requirement-linkage rule |
| `14_COLLABORATION_PROVENANCE_SECURITY.md` | Semantic diff levels, semantic merge/conflict types, Git integration, provenance fields (project + feature level), AI governance policy shape, deterministic-build identity, lockfiles, build attestation, sandbox model, resource budgets, geometry-DoS risk list, unsafe-block audit, package security, external-asset provenance, project policy schema, deterministic geometry fingerprint |
| `15_IMPLEMENTATION_ROADMAP.md` | The 14-stage roadmap (Stage 0 through Stage 13), each stage's build list and exit gate; cross-stage rule to maintain 3 proof tracks (language/architecture/AI) simultaneously |
| `16_TESTING_BENCHMARKS_ACCEPTANCE.md` | Test-layer taxonomy, geometry-correctness methods, golden-fixture categories, **topological-naming benchmark** (the Stage-4 gate metric definition), low-level-completeness benchmark, assembly/interoperability/determinism/performance benchmarks, fuzzing, property tests, security tests, AI-learnability benchmark, human-usability benchmark, release-gate ladder (alpha->1.0), mutation testing for requirements |
| `17_CLI_DIAGNOSTICS_SCHEMA.md` | Full CLI command surface, diagnostic code taxonomy (`PARSE-E###` etc.), diagnostic JSON schema, suggestion-confidence contract, build/query output schemas, exit codes, `cad doctor` |
| `18_REFERENCE_EXAMPLES.md` | 10 illustrative (non-normative) source examples spanning plate/configuration/low-level-hook/raw-topology-repair/assembly/requirement/query/optimization/drawing/package-skill usage |
| `19_RESEARCH_NOTES_AND_SOURCES.md` | External precedent (OCCT, OCAF, STEP AP242, Onshape FeatureScript, CadQuery, OpenSCAD, Tree-sitter) and design consequences drawn from each; explicit list of what is genuinely differentiating vs. precedented; research items intentionally deferred to implementation stages |
| `20_REVIEW_PASS_GAPS_AND_DECISIONS.md` | Deliberate second-pass review: completeness checklist, 7 features added during review, 7 highest technical risks + mitigations, 6 decisions to prototype before freezing, deliberately deferred feature list, suggested first vertical demonstration, overall quality assessment/discipline recommendation |
| `21_FEATURE_INVENTORY.md` | Master `[ ]`/`[x]` program-level feature checklist across 15 categories (A-O), for tracking implementation completeness over the life of the project |
| `22_REPOSITORY_WORK_PACKAGES.md` | Proposed monorepo layout, 19 work packages (WP-01..WP-19) with owns/must-not-own/gate definitions, WP dependency graph, first vertical-integration target, feature definition-of-done checklist |
| `23_CROSS_SYSTEM_PARAMETER_CATALOG.md` | Non-geometry API parameter contracts: `mate`, `joint`, `instance`, `requirement`, `test`, `query`, `validate`, `heal`, `import_step`, `export_step`, `render`, `inspect`, `optimize`, `simulation`, `drawing`, package-skill manifest, agent build tool, agent context packet |

---

## 2. Dependency graph — roadmap stages and work packages

### 2.1 Stage chain (`15_IMPLEMENTATION_ROADMAP.md`)

```text
Stage 0  Architecture constitution and RFC                    [ACTIVE — owner approval required to advance]
   |
Stage 1  Geometry kernel spike (OCCT bridge)
   |
Stage 2  Core language/compiler/runtime
   |
Stage 3  High-level parametric CAD MVP  (AI benchmark begins)
   |
Stage 4  Semantic references & regeneration robustness        [HARD RELEASE GATE]
   |
Stage 5  Advanced low-level geometry
   |
Stage 6  Assemblies and configurations
   |
Stage 7  Verification-first CAD                                [First serious MVP]
   |
Stage 8  Native .aicad artifact + interoperability
   |
Stage 9  Human CAD IDE
   |
Stage 10 AI-native tool layer                                  [AI proof gate]
   |
Stage 11 Package/plugin ecosystem
   |
Stage 12 Engineering semantics (12A materials/BOM/drawings -> 12B GD&T/AP242
         -> 12C DFM -> 12D simulation -> 12E optimization/uncertainty)
   |
Stage 13 Production collaboration and scale
```

`project/TASKS.yaml` (AICAD-001..100) currently only populates Stages 0-4,
consistent with `CURRENT_STAGE.md` ("stage: 0 ... Owner approval required to
advance: Yes") and the operating model's rule against expanding into a later
roadmap stage before the current gate is approved.

### 2.2 Work-package dependency graph (`22_REPOSITORY_WORK_PACKAGES.md` §21, verbatim)

```text
WP01 Kernel
  |
WP05 Geometry API ----+
  |                   |
WP07 Refs/queries     |
  |                   |
WP06 Feature DAG      |
  |                   |
WP09 Modeling --------+

WP02 Parser -> WP03 Types -> WP04 Runtime -> WP05

WP08 Constraints -> WP09/WP10
WP10 Assemblies -> WP11 Verification
WP11 Verification -> WP12 Release artifacts

WP13 CLI sits across services
WP14/15 IDE depend on stable service APIs
WP16 AI depends on WP13-like structured services
WP17 packages requires stable core language/API
WP18 engineering modules sit on packages + semantics
WP19 collaboration requires mature feature/ref graph
```

### 2.3 Stage-to-WP correspondence (inferred — see §6, contradiction 7)

The plan never states this mapping explicitly; it is reconstructed here
from each stage's "Build" list against each WP's "Owns" list, for planning
convenience only:

| Stage | Primary WPs |
|---|---|
| 0 | (RFCs/specs only — precedes WP work) |
| 1 | WP-01 Kernel bridge |
| 2 | WP-02 Parser, WP-03 Types, WP-04 Runtime, WP-05 Geometry API, WP-13 CLI (baseline) |
| 3 | WP-06 Feature DAG, WP-08 Constraints (sketch), WP-09 High-level modeling std lib |
| 4 | WP-07 Semantic references/queries (hard gate) |
| 5 | WP-05 extended (advanced low-level geometry) |
| 6 | WP-10 Assemblies/configurations |
| 7 | WP-11 Verification |
| 8 | WP-12 Interchange/artifacts |
| 9 | WP-14 Language server, WP-15 Viewport/IDE |
| 10 | WP-16 AI tools + skills |
| 11 | WP-17 Package/plugin system |
| 12 | WP-18A..E Engineering modules |
| 13 | WP-19 Collaboration/reproducibility |

### 2.4 Task-to-stage mapping actually present in `project/TASKS.yaml`

```text
Stage 0: AICAD-001..014   (import plan, skeleton, CI, owner-decisions/traceability
                            scaffolding, RFC-0001..0005, paper example, contradiction
                            review, Stage-0 gate packet)
Stage 1: AICAD-015..037   (OCCT bridge through STEP export, Stage-1 bracket,
                            independent STEP verification, fuzz harness, gate packet)
Stage 2: AICAD-038..064   (diagnostics crate, lexer/parser/types/units/runtime,
                            Geometry IR, CLI, Tree-sitter, end-to-end slice, gate packet)
Stage 3: AICAD-065..079   (params, feature DAG, sketch IR/constraints, extrude/hole/
                            pocket, patterns, fillet/chamfer/shell, semantic outputs +
                            AI benchmark seed — no explicit gate-packet task, see §6)
Stage 4: AICAD-080..100   (ref representations, query IR, predicates, lineage,
                            resolver, ambiguity/broken diagnostics, durability,
                            fingerprint fallback, epochs, refs-check report, topology-
                            naming benchmark corpus/runner/metrics, adversarial
                            bug-hunt, Stage-4 hard-gate packet)
```

---

## 3. Unresolved / prototype-before-freezing decisions

Full detail with plan references and blocking impact is in
`project/OWNER_DECISIONS.md` (15 numbered decisions, D1-D15, plus a
"non-decision items for awareness" section). Summary list:

1. **D1** Canonical surface syntax family (braces/semicolons vs. indentation).
2. **D2** Mutation semantics (functional core vs. method/builder sugar, exact lowering).
3. **D3** Sketch entity/object binding model.
4. **D4** Type/units semantics specifics (canonicalization, implicit conversion, tolerance arithmetic).
5. **D5** Canonical-state/deterministic-equivalence contract (what "equivalent" means, testably).
6. **D6** Kernel abstraction boundary and required lineage exposure.
7. **D7** Semantic-reference resolution precedence, durability rules, and fallback policy — **highest priority**, directly gates Stage 4 (AICAD-088..092).
8. **D8** OCAF usage vs. kernel-independent semantic graph.
9. **D9** Compiler-intrinsic vs. standard-package boundary enforcement process.
10. **D10** Diagnostic code/schema stability policy.
11. **D11** Constraint IR semantics and solver-independence rules.
12. **D12** Trusted native extension boundary and plugin security model.
13. **D13** Licensing/redistribution for OCCT and standards-derived content (GD&T/AP242) — explicit license/security escalation trigger, unaddressed anywhere in the plan.
14. **D14** File extension / product branding.
15. **D15** Package plugin runtime (WASM vs. external process) — prototype before committing.

Plus plan-acknowledged research items not yet requiring an owner ruling
(`19_RESEARCH_NOTES_AND_SOURCES.md` §10) and a list of deliberately
deferred product scope (`20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §5), both
carried into `project/OWNER_DECISIONS.md` for visibility.

---

## 4. Non-negotiable invariants

From `00_PRINCIPLES_AND_SCOPE.md` §3 (the 13 language rules — restated and
already adopted verbatim into `AGENTS.md`'s "Non-negotiables"):

1. The language is Turing-complete by design.
2. High-level and low-level CAD live in one language (no separate "advanced language").
3. Low-level geometry is nearly kernel-complete; advanced users must not hit an abstraction wall.
4. Raw topology access exists but is explicitly ephemeral/unsafe, epoch-bound, and validation-guarded.
5. Units are in the type system — not plain untyped floats.
6. Exact B-rep is the primary compiled geometry; meshes are views/export targets only.
7. Source is canonical design intent; cached B-rep never replaces source semantics.
8. **Semantic references are preferred to entity indices; ambiguity is an error, never an arbitrary selection.**
9. Determinism is the default (same source + lockfile + compiler/kernel versions -> equivalent geometry/validation) — see D5 for the open testability question.
10. Engineering assertions (requirements/tests/contracts) are executable and live beside design source.
11. GUI and source are two projections over one model; the GUI must not create irreducible hidden state.
12. Extensions are first-class (packages can add abstractions/APIs/validators/exporters/solvers/skills).
13. The platform must remain usable by general-purpose coding models, not require a CAD-specialized model.

Additional non-negotiable contracts found elsewhere in the plan:

- **STEP interoperability rule** (`00` §9): the language must not compete with STEP at the same layer — source keeps intent/parameters/tests/provenance; STEP is the downstream neutral artifact.
- **Kernel-independence contract** (`01` §8): public language may promise only vendor-neutral concepts (NURBS curve, planar face, manifold solid, boolean union, semantic face reference); `TopoDS_Face`, `BRepAlgoAPI_Fuse`, and OCCT-specific status enums must never be the *only* explanation in a diagnostic (a `backend_details` field may carry them).
- **Determinism-exclusion rule** (`14` §8): wall-clock timestamps are excluded from the deterministic geometry/build identity hash.
- **Ambiguity-must-be-explicit rule** (`06` §7): a query intended to resolve a single reference must never silently choose an arbitrary entity; this is the specific mechanism behind invariant 8 and the Stage-4 hard gate.
- **Validation-vs-healing separation** (`23` §7): `validate()` must not silently heal unless explicitly requested — healing is a distinct, explicit transformation.
- **Import fidelity rule** (`09` §8): imported geometry must never be silently mutated without a recorded healing/transformation report.
- **Capability-tier model** (`01` §9): execution is stratified into Tier A (pure language) / B (safe CAD) / C (unsafe geometry) / D (external capabilities, must be declared/approved) — packages cannot silently raise resource/capability limits (`02` §15).
- **Standard-library promotion bar** (`12` §16): promotion into `std.*` requires broadly applicable semantics, proven-stable API, understood interoperability mapping, and a higher test bar — not ad hoc inclusion.

---

## 5. Traceability matrix — Stage 0 through Stage 4 exit gates

| Stage | Exit gate (verbatim intent, `15_IMPLEMENTATION_ROADMAP.md`) | Tasks producing evidence (`project/TASKS.yaml`) | Evidence / benchmark definition |
|---|---|---|---|
| **0** | A realistic example containing parameters, function, loop, conditional, sketch, extrusion, semantic query, low-level geometry, assembly, constraint, and test can be written without semantic contradiction. | AICAD-007..011 (RFC-0001..0005 drafts); AICAD-012 (paper/spec example spanning the required concepts); AICAD-013 (contradiction review across RFCs + example); AICAD-014 (Stage-0 owner gate packet) | AICAD-013's report is the direct evidence of "no semantic contradiction"; no automated test exists yet at this stage — the check is a documented consistency review, per `AICAD-013`'s own required_checks (task-specific review, recorded exactly). |
| **1** | Complex bracket fixture -> valid B-rep -> STEP -> opens correctly in multiple external CAD viewers/tools. | AICAD-015..033 (OCCT bridge through STEP export primitives); AICAD-034 (build the Stage-1 bracket directly via Rust/native API); AICAD-035 (independent STEP verification harness — this *is* the "opens correctly in external tools" check); AICAD-036 (bounded geometry regression/fuzz harness); AICAD-037 (Stage-1 owner gate packet) | `16_TESTING_BENCHMARKS_ACCEPTANCE.md` §2-3 geometry-correctness methods (validity/manifold, volume/area, bounding box, independent round-trip/import) apply directly; AICAD-035's independent-importer result is the exit-gate proof artifact. |
| **2** | Ordinary programs can construct geometry and compute parameterized patterns without special interpreter shortcuts. | AICAD-038 (`cad-diagnostics` + schema conformance); AICAD-039..058 (lexer/parser/AST/types/units/name-binding/HIR/typecheck/runtime/control-flow/collections/recursion/budgets); AICAD-059/060 (Geometry IR + dispatch); AICAD-061 (`cad-cli build`); AICAD-062 (Tree-sitter grammar); AICAD-063 (end-to-end source -> exact bracket -> STEP slice — the direct exit-gate proof); AICAD-064 (Stage-2 owner gate packet) | Compiler conformance artifacts required by `02_LANGUAGE_AND_COMPILER.md` §19 (`spec/grammar.ebnf`, `spec/semantics.md`, `tests/parser`, `tests/typecheck`, `tests/runtime`, `tests/geometry`); AICAD-063's slice result is the concrete exit-gate evidence. |
| **3** | A human and a fresh coding model (given only `cad-core.skill.md`) can create a representative set of ordinary mechanical parts concisely; AI benchmark begins. | AICAD-065..069 (params, feature-graph identity/DAG/cache/provenance); AICAD-070/071 (safe geometry types, part/box/cylinder/plate helpers); AICAD-072..075 (sketch IR, constraint IR/adapter, common constraints, profile lowering); AICAD-076..078 (extrude/revolve/hole/pocket, patterns/mirror, fillet/chamfer/shell); AICAD-079 (named semantic outputs baseline + ordinary-part examples **+ AI benchmark seed**) | `11_AI_NATIVE_SKILLS_AND_AGENT_PROTOCOL.md` §10 "Core only" tier (mounting plate, L bracket, enclosure, bearing mount, patterned flange, basic assembly, configuration variant, fix compiler error, fix failing requirement) is the concrete benchmark task list; AICAD-079's "AI benchmark seed" is the first materialization. **Gap:** no explicit Stage-3 owner-gate-packet task exists in the current task file — flagged in `project/OWNER_DECISIONS.md` for confirmation rather than resolved silently. |
| **4** | Benchmark models survive defined upstream parameter changes without incorrect silent downstream selections. **Hard release gate** — do not scale feature count before this is credible. | AICAD-080/081 (stable ref representations, query AST/IR); AICAD-082..084 (geometry/topology/spatial predicates); AICAD-085 (explicit semantic exports); AICAD-086/087 (lineage capture/storage); AICAD-088 (resolver using approved precedence — **depends on D7**); AICAD-089/090 (ambiguity-as-error / broken-reference diagnostics); AICAD-091 (durability levels); AICAD-092 (fingerprint fallback **only under approved policy** — **depends on D7**); AICAD-093 (raw-handle epochs/stale rejection); AICAD-094 (replay refs during incremental regeneration); AICAD-095 (`cad refs check`); AICAD-096 (fixture corpus); AICAD-097 (perturbation runner); AICAD-098 (benchmark metrics); AICAD-099 (adversarial bug-hunt, preserve every silent-wrong reproducer); AICAD-100 (Stage-4 hard-gate packet) | `16_TESTING_BENCHMARKS_ACCEPTANCE.md` §5 "Topological naming benchmark": perturbations (resize extrusion, add/remove hole, change pattern count, change fillet radius, split/merge face, reorder branches, suppress configuration feature) against metrics (correct resolution / ambiguous-but-detected / broken-but-detected / **silent-wrong-resolution must approach zero** / durability distribution). AICAD-098 implements exactly these metrics; AICAD-099/100 are the closing evidence. Per `AGENTS.md` and `CURRENT_STAGE.md`, the agent may prepare this gate evidence but may **not** approve the stage — that is an owner decision. |

---

## 6. Contradictions and underspecified decisions (exact source locations)

1. **Stale document index.** `README.md` lines 26-51 (the "Documents" table)
   lists only `00_PRINCIPLES_AND_SCOPE.md` through `21_FEATURE_INVENTORY.md`
   and omits `22_REPOSITORY_WORK_PACKAGES.md` and
   `23_CROSS_SYSTEM_PARAMETER_CATALOG.md`, even though both files are present
   in the bundle (confirmed against `docs/plan/MANIFEST.txt`) and are cited
   as `plan_references` by every Stage-0 task in `project/TASKS.yaml` (e.g.
   AICAD-001 lines 9-14). Treat the index as stale, not as evidence those
   two files are non-canonical.

2. **File-extension convention disagreement.** `02_LANGUAGE_AND_COMPILER.md`
   §2 proposes `.cad` or `.aicad` for modules (with `.aicad` doubling as the
   bundle "if ambiguity is undesirable"), while
   `09_FILE_FORMAT_INTERCHANGE_RECONSTRUCTION.md` §1 states the module
   extension as `*.cadl` and the bundle as `*.aicad` without that
   conditional. Both `20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §4.6 and
   `README.md`'s "Working names" line acknowledge the branding/extension
   choice is an open placeholder, but the two normative documents still
   disagree with each other on the default. Tracked as D14.

3. **Mutation-style examples disagree without resolving which is canonical.**
   The method/builder style `base.cut(hole(center=p, diameter=5.5mm));`
   appears in `README.md`'s top-level example (line 22 area) while the
   functional style `body = cut(base, holes);` appears in
   `18_REFERENCE_EXAMPLES.md` Example 1 (line 31). `20_REVIEW_PASS_GAPS_AND_DECISIONS.md`
   §4.2 explicitly defers the choice, so this is a flagged-but-unresolved
   ambiguity rather than a silent contradiction — implementers must not
   infer a canonical style from the examples alone. Tracked as D2.

4. **Sketch-entity binding mechanism is never shown.**
   `04_HIGH_LEVEL_MODELING_API.md` §3 defines `sketch()` returning `Sketch`
   and separate `line`/`circle`/`arc`/`rectangle`/`polygon`/`slot` functions
   returning `SketchEntity`/`Profile`, but no example in the bundle (checked
   `02_LANGUAGE_AND_COMPILER.md`, `04_HIGH_LEVEL_MODELING_API.md`, and all
   ten examples in `18_REFERENCE_EXAMPLES.md`) shows how an entity produced
   by one of those free functions is registered onto a specific `Sketch`
   value or participates in its constraint solver. Matches the
   self-acknowledged open item in `20_REVIEW_PASS_GAPS_AND_DECISIONS.md`
   §4.3. Tracked as D3.

5. **"Equivalent geometry" is not defined to a testable tolerance.**
   `00_PRINCIPLES_AND_SCOPE.md` §3 invariant 9 requires "equivalent geometry
   and validation results" for identical source+lockfile+versions, while
   `14_COLLABORATION_PROVENANCE_SECURITY.md` §18 separately warns not to
   claim bit-identical B-rep across kernel/platform versions "unless
   actually guaranteed." Neither document, nor
   `16_TESTING_BENCHMARKS_ACCEPTANCE.md` §9 (the determinism benchmark),
   states the actual comparison tolerance/method that defines "equivalent."
   Tracked as D5.

6. **Missing Stage-3 gate-packet task.** `project/TASKS.yaml` gives Stage 0
   (AICAD-014), Stage 1 (AICAD-037), Stage 2 (AICAD-064), and Stage 4
   (AICAD-100) each an explicit terminal "prepare owner gate packet" task.
   Stage 3's range (AICAD-065..079) ends at AICAD-079 ("Implement named
   semantic outputs baseline + ordinary-part examples + AI benchmark seed")
   with no analogous gate-packet task, even though
   `15_IMPLEMENTATION_ROADMAP.md` gives Stage 3 its own exit gate and
   `CURRENT_STAGE.md` establishes that stage advancement requires owner
   approval. Likely a task-file omission rather than an intentional skip —
   flagged in `project/OWNER_DECISIONS.md` for owner confirmation rather
   than resolved by adding or skipping a gate task unilaterally.

7. **No explicit stage-to-work-package mapping.** `15_IMPLEMENTATION_ROADMAP.md`
   defines stages by capability and `22_REPOSITORY_WORK_PACKAGES.md` defines
   work packages by ownership boundary with only a WP-to-WP dependency graph
   (§21) — the plan never states which WPs belong to which stage. The
   mapping in §2.3 above is this report's own inference for planning
   convenience and is not a normative statement from the plan.

No other outright logical contradictions (as opposed to acknowledged open
decisions or documentation gaps) were found across the 24 documents.

---

## 7. Next steps

Per `project/FIRST_PROMPT.md`, proceed to execute AICAD-002 through AICAD-006
in dependency order, per `AGENTS.md`, `CURRENT_STAGE.md`, and
`project/TASKS.yaml`, escalating to `project/OWNER_DECISIONS.md` (already
seeded above) rather than resolving any of the D1-D15 decisions implicitly.
