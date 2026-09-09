# Stage 0-4 exit-gate -> test/benchmark/evidence traceability matrix

Produced by AICAD-006, from the analysis in
`project/reports/ORIENTATION_PASS.md` §5. This is the canonical, standalone
copy of the matrix (the orientation report keeps its own copy as analysis
narrative; this file is the tracked deliverable future gate packets in
`project/gates/` should cite and this file is updated whenever a task list
or plan revision changes the mapping).

Scope: `project/TASKS.yaml` currently only defines Stages 0-4 (AICAD-001
through AICAD-100), consistent with `project/CURRENT_STAGE.md` (Stage 0,
owner approval required to advance) and the AGENTS.md rule against
expanding into a later roadmap stage before the current gate is approved.
This matrix therefore stops at Stage 4, the hard release gate — it will be
extended only after Stage 4 is owner-approved and Stage 5+ tasks exist.

| Stage | Exit gate (`docs/plan/15_IMPLEMENTATION_ROADMAP.md`) | Tasks (`project/TASKS.yaml`) | Evidence / benchmark definition | Non-negotiable invariant(s) directly tested |
|---|---|---|---|---|
| **0** | A realistic example containing parameters, function, loop, conditional, sketch, extrusion, semantic query, low-level geometry, assembly, constraint, and test can be written without semantic contradiction. | AICAD-007..011 (RFC-0001..0005); AICAD-012 (paper/spec example); AICAD-013 (contradiction review); AICAD-014 (Stage-0 gate packet) | AICAD-013's contradiction-review report is the direct evidence; the check is a documented consistency review, not an automated test (no compiler exists yet). | High-level/low-level-one-language rule (`00` §3.2-3); semantic-reference-over-index rule (`00` §3.8) as exercised in the paper example's "semantic query" requirement. |
| **1** | Complex bracket fixture -> valid B-rep -> STEP -> opens correctly in multiple external CAD viewers/tools. | AICAD-015..033 (bridge through STEP export); AICAD-034 (Stage-1 bracket via Rust/native API); AICAD-035 (independent STEP verification harness); AICAD-036 (fuzz harness); AICAD-037 (gate packet) | `16_TESTING_BENCHMARKS_ACCEPTANCE.md` §2-3 geometry-correctness methods (validity/manifold, volume/area, bounding box, independent round-trip/import); AICAD-035's independent-importer result is the exit-gate proof artifact. | Exact-B-rep-is-canonical rule (`00` §3.6); kernel-independence contract (`01` §8, checked via AICAD-035 not depending on OCCT-specific output). |
| **2** | Ordinary programs can construct geometry and compute parameterized patterns without special interpreter shortcuts. | AICAD-038 (`cad-diagnostics`); AICAD-039..058 (lexer/parser/types/units/HIR/runtime); AICAD-059/060 (Geometry IR + dispatch); AICAD-061 (`cad-cli build`); AICAD-062 (Tree-sitter grammar); AICAD-063 (end-to-end slice); AICAD-064 (gate packet) | Conformance artifacts per `02_LANGUAGE_AND_COMPILER.md` §19 (`spec/grammar.ebnf`, `tests/parser`, `tests/typecheck`, `tests/runtime`, `tests/geometry`); AICAD-063's source -> exact bracket -> STEP slice is the concrete proof. | Turing-completeness rule (`00` §3.1, via loops/functions/recursion tests in `tests/runtime/`); units-are-typed rule (`00` §3.5, via `tests/typecheck/`). |
| **3** | A human and a fresh coding model (`cad-core.skill.md` only) can create a representative set of ordinary mechanical parts concisely; AI benchmark begins. | AICAD-065..069 (params, feature-graph); AICAD-070/071 (geometry types, part helpers); AICAD-072..075 (sketch IR/constraints); AICAD-076..078 (extrude/hole/pocket, patterns, fillet/chamfer/shell); AICAD-079 (semantic outputs + AI benchmark seed) | `11_AI_NATIVE_SKILLS_AND_AGENT_PROTOCOL.md` §10 "Core only" tier task list (mounting plate, L bracket, enclosure, bearing mount, patterned flange, basic assembly, configuration variant, fix compiler error, fix failing requirement); AICAD-079's benchmark seed is the first materialization. **No explicit Stage-3 gate-packet task exists** in `project/TASKS.yaml` — see `project/OWNER_DECISIONS.md` non-decision item on this asymmetry; do not add or skip one without owner confirmation. | AI-learnability-without-specialized-model rule (`00` §3.13); source-canonical rule (`00` §3.7, via bidirectional example generation). |
| **4** | Benchmark models survive defined upstream parameter changes without incorrect silent downstream selections. **Hard release gate.** | AICAD-080/081 (ref representations, query IR); AICAD-082..084 (predicates); AICAD-085 (explicit exports); AICAD-086/087 (lineage); AICAD-088 (resolver — depends on D7); AICAD-089/090 (ambiguity/broken diagnostics); AICAD-091 (durability); AICAD-092 (fingerprint fallback — depends on D7); AICAD-093 (epochs); AICAD-094 (regeneration replay); AICAD-095 (`cad refs check`); AICAD-096 (fixture corpus); AICAD-097 (perturbation runner); AICAD-098 (metrics); AICAD-099 (adversarial bug-hunt); AICAD-100 (hard-gate packet) | `16_TESTING_BENCHMARKS_ACCEPTANCE.md` §5 topological-naming benchmark: perturbations (resize extrusion, add/remove hole, change pattern count, change fillet radius, split/merge face, reorder branches, suppress configuration feature) against metrics (correct / ambiguous-detected / broken-detected / **silent-wrong must approach zero** / durability distribution). AICAD-098 implements exactly these metrics; `benchmarks/topology-naming/` is where the corpus/results live (see its `README.md`). | **Ambiguity-is-an-error rule (`00` §3.8; `06` §7) — the single invariant this entire stage exists to prove.** Per `AGENTS.md`/`project/CURRENT_STAGE.md`, the agent may prepare this evidence and recommend pass/do-not-pass but may not approve the stage itself. |

## How to keep this current

- If `project/TASKS.yaml` gains, removes, or renumbers a task inside
  Stages 0-4, update the corresponding row here in the same commit.
- If a plan revision changes an exit gate's wording (`docs/plan/` is
  otherwise immutable per AICAD-001's import policy — a revision means a
  new verified import, not an in-place edit), update this matrix in the
  same commit that records the new plan version/checksum.
- Do not mark a stage's row "gate passed" here — that determination and
  its evidence packet belong in `project/gates/stage-<n>-gate.md`
  (see `project/gates/README.md`); this file only maps gates to the
  tests/tasks that produce evidence for them.
