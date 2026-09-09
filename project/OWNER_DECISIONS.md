# AICAD Owner Decisions

Unresolved architecture/product decisions requiring owner approval belong here.
Do not silently resolve them in implementation work.

Populated by the orientation pass over `docs/plan/` performed per
`project/FIRST_PROMPT.md`, before executing AICAD-002 onward. See
`project/reports/ORIENTATION_PASS.md` for the full analysis (document map,
invariants, traceability matrix, contradictions) this list was drawn from.
This file will be revisited and formally finalized by task AICAD-005
("Create OWNER_DECISIONS.md and DECISION_LOG.md from unresolved plan
decisions"); entries below are not to be resolved implicitly before then.

Status legend: `open` = no owner ruling yet. Once the owner rules, move the
decision (with rationale) to `project/DECISION_LOG.md` and mark it here as
`resolved -> DECISION_LOG#<n>`.

---

## D1. Canonical surface syntax family

**Question:** Rust/TypeScript-style braces-and-semicolons vs. Python-style
indentation for the canonical `.aicad`/`.cadl` grammar (RFC-0001 scope)?

**Plan references:** `docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §4.1;
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §1.

**Status:** The plan recommends Rust/TypeScript-style but explicitly labels
this a decision to "prototype before freezing," not a settled choice.

**Blocking impact:** RFC-0001 (AICAD-007) and every subsequent parser task
(AICAD-039+) depend on this being frozen first.

---

## D2. Mutation semantics: functional core vs. builder/method sugar

**Question:** Is `body = cut(base, holes);` (functional) or
`base.cut(hole(...));` (method/builder) the canonical surface form, and what
exactly does builder-style syntax lower to in HIR?

**Plan references:** `docs/plan/02_LANGUAGE_AND_COMPILER.md` §5;
`docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §4.2. The plan's own
examples are inconsistent on this without resolving it — see
`project/reports/ORIENTATION_PASS.md` §6 (contradiction 3).

**Status:** Plan recommends supporting both (functional core + ergonomic
sugar) but the exact desugaring/SSA lowering rule is not specified.

**Blocking impact:** RFC-0001, grammar (AICAD-039/041), HIR lowering
(AICAD-051).

---

## D3. Sketch entity/object model

**Question:** Are sketch entities (`line`, `circle`, `arc`, ...) explicit
named objects registered onto a `Sketch` value, or sugar over a declarative
block? How does an entity created by a free function actually attach to a
specific `Sketch`'s plane and constraint solver?

**Plan references:** `docs/plan/04_HIGH_LEVEL_MODELING_API.md` §3 (`sketch`,
`line`, `circle`, ... signatures); `docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md`
§4.3. No example in the bundle (checked 02, 04, 18) shows the binding
mechanism — see `project/reports/ORIENTATION_PASS.md` §6 (contradiction 4).

**Status:** Open; plan favors explicit objects internally with block syntax
as sugar, but the mechanism is undefined.

**Blocking impact:** Stage 3 sketch IR/constraint tasks (AICAD-072, AICAD-073,
AICAD-074, AICAD-075).

---

## D4. Type/units semantics specifics

**Question:** Exact dimension-canonicalization algorithm, whether/when
implicit unit conversions are allowed, and tolerance/range arithmetic
semantics (e.g. how `Tolerance<T> + Tolerance<T>` behaves).

**Plan references:** `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §3-6,
§21; `AICAD_AGENT_OPERATING_MODEL.md` §7 item 3.

**Status:** Baseline dimension list and quantity representation are given,
but canonicalization/conversion/tolerance-arithmetic rules are not fully
specified.

**Blocking impact:** RFC-0004 (AICAD-010), `cad-units` (AICAD-047-049). This
is an AGENTS.md escalation trigger ("change typed-units semantics") — must
not be resolved implicitly.

---

## D5. Canonical-state / deterministic-equivalence contract

**Question:** What does "equivalent geometry and validation results" mean
in testable terms across kernel/platform versions? What tolerance/comparison
method defines pass/fail for the determinism benchmark?

**Plan references:** `docs/plan/00_PRINCIPLES_AND_SCOPE.md` §3 invariant 9;
`docs/plan/14_COLLABORATION_PROVENANCE_SECURITY.md` §18 (explicitly warns
against claiming bit-identical B-rep across kernel/platform versions);
`docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md` §9. See
`project/reports/ORIENTATION_PASS.md` §6 (contradiction 5).

**Status:** Open. The two documents do not contradict each other outright,
but neither defines the actual comparison tolerance, and this is an
AGENTS.md escalation trigger ("change the canonical-state/determinism
contract").

**Blocking impact:** Not required for Stage 0-1, but should be settled
before Stage 2's deterministic evaluator work matures and well before the
Stage 8/13 determinism benchmarks.

---

## D6. Kernel abstraction boundary and lineage exposure

**Question:** Exact narrow bridge operation set and what lineage information
(entity split/merge/creation history) the kernel adapter must expose to the
semantic-reference layer above it.

**Plan references:** `docs/plan/01_SYSTEM_ARCHITECTURE.md` §2.6, §4, §8;
`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §8; `AICAD_AGENT_OPERATING_MODEL.md`
§7 item 5.

**Status:** A representative bridge operation list exists (§4 of doc 01),
but it is stated as "add capabilities only as required" — the boundary is
not frozen, and lineage-exposure requirements are described conceptually
without a concrete adapter-level contract.

**Blocking impact:** AICAD-016/017/018 (kernel bridge crates), and
AICAD-086/087 (lineage capture) in Stage 4.

---

## D7. Semantic-reference resolution model, durability, and fallback policy

**Question:** Exact resolution precedence across the reference-construction
strategies in `06_REFERENCES_QUERIES_FEATURE_DAG.md` §3, the precise
durability-level assignment rules (§11), and — critically — under what
"approved policy" (if any) a geometry-fingerprint fallback is permitted
before it must be treated as ambiguous/broken instead of silently resolved.

**Plan references:** `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §3, §7,
§11; `AICAD_AGENT_OPERATING_MODEL.md` §7 item 6. Task AICAD-092 in
`project/TASKS.yaml` ("Implement geometry-fingerprint fallback only under
approved policy") presupposes this decision exists by the time Stage 4
reaches it.

**Status:** Open. This is the single highest-priority decision in the plan:
`AGENTS.md` and `docs/plan/00_PRINCIPLES_AND_SCOPE.md` §3 invariant 8 both
state ambiguity must be an error, never an arbitrary selection, and Stage 4
is a hard release gate specifically about silent-wrong-resolution risk
(`docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md` §5,
`docs/plan/15_IMPLEMENTATION_ROADMAP.md` Stage 4).

**Blocking impact:** AICAD-088 through AICAD-092 directly; do not implement
a fallback-selection policy implicitly when that stage is reached.

---

## D8. OCAF usage vs. kernel-independent semantic graph

**Question:** Should persistent semantic references and lineage be built on
OCCT's OCAF framework, on a kernel-independent semantic graph layered above
it, or a hybrid — and what happens if OCAF's native topological-naming
capability is later found insufficient?

**Plan references:** `docs/plan/19_RESEARCH_NOTES_AND_SOURCES.md` §3;
`docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §4.5;
`AICAD_AGENT_OPERATING_MODEL.md` §7 item 7. This is also an explicit
AGENTS.md escalation trigger ("select between major unresolved architecture
alternatives").

**Status:** Plan leans toward "likely needs a kernel-independent semantic
graph above OCAF" but explicitly calls for prototyping both before
committing.

**Blocking impact:** WP-07 (semantic references), Stage 4 tasks broadly.

---

## D9. Compiler-intrinsic vs. standard-package boundary enforcement

**Question:** The plan gives a qualitative test ("could this be a library
instead of a compiler intrinsic?" — `docs/plan/00_PRINCIPLES_AND_SCOPE.md`
§10) but no enforced process (e.g. an RFC requirement or CI check) for new
intrinsics. Should one be adopted formally?

**Plan references:** `docs/plan/00_PRINCIPLES_AND_SCOPE.md` §10;
`docs/plan/12_PACKAGES_PLUGINS_EXTENSIONS.md` §16; `AICAD_AGENT_OPERATING_MODEL.md`
§7 item 9. AGENTS.md escalation trigger: "add a compiler intrinsic where a
library solution may work."

**Status:** Open, low urgency until the standard library work in Stage 3+
begins in earnest.

---

## D10. Diagnostic code/schema stability policy

**Question:** What is the exact stability/deprecation policy for diagnostic
codes (the `PARSE-E###`/`GEOM-E###`/etc. families) once published — can
codes be renumbered pre-1.0, and what is the process for adding new
families?

**Plan references:** `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §10-12;
`AICAD_AGENT_OPERATING_MODEL.md` §7 item 10.

**Status:** A taxonomy and JSON schema exist; a stability policy does not.

**Blocking impact:** AICAD-038 (`cad-diagnostics` crate + JSON-schema
conformance tests) is the first task that materializes this schema — should
be settled at or before that task.

---

## D11. Constraint IR semantics and solver-independence rules

**Question:** Exact constraint IR contract (`docs/plan/08_CONSTRAINTS_REQUIREMENTS_TESTS.md`
§4) and the precise boundary of what a pluggable solver backend may vs. may
not change about observable solving behavior (e.g. solution-branch
selection, conflict-set minimality guarantees).

**Plan references:** `docs/plan/01_SYSTEM_ARCHITECTURE.md` §2.4;
`docs/plan/08_CONSTRAINTS_REQUIREMENTS_TESTS.md` §4, §6;
`AICAD_AGENT_OPERATING_MODEL.md` §7 item 11.

**Status:** Open; needed before sketch/assembly solving expands
(Stage 3 AICAD-073/074, Stage 6).

---

## D12. Trusted native extension boundary and plugin security model

**Question:** Exact approval process and technical sandbbox boundary for
"Level 4 — Trusted native plugin" extensions (kernels, kernel-level
importers/exporters, native solvers).

**Plan references:** `docs/plan/12_PACKAGES_PLUGINS_EXTENSIONS.md` §6, §8;
`docs/plan/14_COLLABORATION_PROVENANCE_SECURITY.md` §11, §15;
`AICAD_AGENT_OPERATING_MODEL.md` §7 item 12. AGENTS.md escalation trigger:
"add a new trusted native/plugin boundary."

**Status:** Open, low urgency — relevant starting at Stage 11.

---

## D13. Licensing/redistribution for standards-derived content and OCCT

**Question:** (a) OCCT's license (LGPL-2.1-with-exception family) and its
implications for how the native bridge is built/distributed/linked; (b)
licensing/redistribution constraints for content derived from ISO/ASME
standards (STEP AP242, GD&T symbology/semantics in
`docs/plan/13_ENGINEERING_MODULES.md` §11) if the standard's text or tables
are reproduced in code, docs, or test fixtures.

**Plan references:** `docs/plan/19_RESEARCH_NOTES_AND_SOURCES.md` §10 ("licensing/
redistribution constraints for standards-derived libraries/data");
`docs/plan/13_ENGINEERING_MODULES.md` §11 ("implement against
licensed/authoritative standards work during the engineering stage").

**Status:** Open. This is an explicit AGENTS.md/operating-model escalation
trigger ("require a license/security policy decision") and has not been
addressed anywhere in the plan bundle.

**Blocking impact:** Relevant as soon as Stage 1 links against OCCT
(AICAD-015/016), and required before Stage 12B (GD&T) implementation.

---

## D14. File extension / project branding

**Question:** Final module and bundle file extensions (candidates: `.cadl`
source / `.aicad` bundle, vs. `.aicad` for both) and product branding
(`AICAD` / `CAD-IR` / `CADLang`).

**Plan references:** `docs/plan/README.md` ("Working names ... placeholder");
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §2;
`docs/plan/09_FILE_FORMAT_INTERCHANGE_RECONSTRUCTION.md` §1;
`docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §4.6. The two normative
documents (02, 09) do not fully agree with each other even though both
flag the choice as unresolved — see
`project/reports/ORIENTATION_PASS.md` §6 (contradiction 2).

**Status:** Open, explicitly deferred by the plan itself.

**Blocking impact:** Low urgency for Stage 0, but should be resolved before
RFC-0001 finalizes source-file conventions and before AICAD-002's repository
skeleton commits to concrete file extensions.

---

## D15. Package plugin runtime (WASM vs. external process) — prototype first

**Question:** Which sandboxed plugin runtime is the default "Level 2" ABI
before committing to one implementation.

**Plan references:** `docs/plan/12_PACKAGES_PLUGINS_EXTENSIONS.md` §6, §8;
`docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §4.4.

**Status:** Open; plan explicitly calls for prototyping both. Low urgency —
relevant starting at Stage 11.

---

## Non-decision items carried forward for awareness (not owner rulings needed yet)

These are plan-acknowledged gaps/research items that do not currently block
Stage 0 work but should stay visible:

- Task list asymmetry: Stage 0/1/2/4 each end with an explicit "prepare
  owner gate packet" task; Stage 3 (AICAD-065..079) does not. See
  `project/reports/ORIENTATION_PASS.md` §6 (contradiction 6). Recommend
  owner confirm whether a Stage-3 gate task should be added before Stage 3
  begins, rather than silently adding or omitting one.
- `docs/plan/README.md`'s document index does not list
  `22_REPOSITORY_WORK_PACKAGES.md` or `23_CROSS_SYSTEM_PARAMETER_CATALOG.md`
  even though both are present in the bundle and are actively cited by
  every Stage-0 task. Treat the index as stale, not as evidence those files
  are non-canonical. See `project/reports/ORIENTATION_PASS.md` §6
  (contradiction 1).
- No explicit stage-to-work-package mapping exists in the plan; any such
  mapping (including the one in `project/reports/ORIENTATION_PASS.md` §2)
  is inference for planning convenience only.
- Research items in `docs/plan/19_RESEARCH_NOTES_AND_SOURCES.md` §10
  (constraint-solver robustness, topological-naming algorithms beyond OCAF,
  AP242 PMI/GD&T mapping coverage, B-rep fingerprinting across kernel
  versions, WASM plugin overhead, large-assembly streaming, solver-neutral
  FEA/CFD schema, affine-temperature typing, incremental-compiler
  architecture) are flagged by the plan itself as pre-implementation
  research, not yet owner decisions.
- Deliberately deferred product scope (not to be pulled forward without an
  owner decision to widen scope): ECAD/schematic authoring, architecture/BIM,
  full CAM toolpaths, photorealistic rendering, in-house topology-optimization
  solver, cloud PLM, real-time multi-user editing, certified standards
  database, native mobile CAD UI, domain-specific aerospace/medical
  validation (`docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §5).
