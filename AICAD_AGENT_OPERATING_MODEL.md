# AICAD 24/7 Agent Operating Model

This operating model is intended to sit beside the AICAD implementation-plan bundle. The bundle remains the design source of truth; this file controls how an always-on implementation agent consumes it.

## 1. Core rule

Do **not** prompt the agent with only: "read the bundle and implement it." The bundle is a reference library and roadmap, not a single executable prompt.

Use four layers:

1. **Project constitution** — stable rules in `AGENTS.md`.
2. **Current stage brief** — one short file defining the active roadmap stage and its exit gate.
3. **Task ticket** — one bounded task with dependencies, acceptance criteria, commands, and escalation conditions.
4. **Run report** — evidence produced by the agent after each task.

The agent may read the full bundle during initial orientation, but normal tasks should load only the relevant plan sections plus the active task ticket.

## 2. Suggested repository control files

```text
AGENTS.md
project/
  CURRENT_STAGE.md
  OWNER_DECISIONS.md
  DECISION_LOG.md
  TASKS.yaml
  reports/
  experiments/
  gates/
docs/plan/                 # verbatim copy of the implementation-plan bundle
specs/
benchmarks/
```

Keep `docs/plan/` immutable except when you deliberately replace it with a new version. Store a checksum/version so the agent can report which plan revision it used.

## 3. Permanent AGENTS.md instructions

Recommended content:

```text
You are the primary implementation agent for AICAD.

MISSION
Implement the active roadmap stage without broadening scope. The AICAD plan bundle is the design source of truth. Source + lockfile + immutable assets are canonical; generated B-rep/meshes are caches/artifacts.

NON-NEGOTIABLES
- Exact B-rep is canonical compiled geometry; meshes are not design truth.
- Public language/API must not expose OCCT-specific classes.
- Units are typed engineering quantities, not untyped floats.
- Stable semantic references are preferred; ambiguity is an error, never an arbitrary selection.
- Do not silently weaken tests, validation, determinism, resource budgets, or reference gates.
- High-level and low-level geometry remain one language.
- Raw topology is ephemeral/unsafe and epoch-bound.
- Prefer library/std-package features over new compiler intrinsics unless an approved RFC says otherwise.
- AI support sits on top of compiler/runtime validation and may not bypass it.

WORK LOOP
1. Read CURRENT_STAGE.md and the assigned task.
2. Read only the plan documents referenced by the task, plus any directly relevant source.
3. Confirm dependencies are satisfied.
4. Make the smallest implementation that satisfies the task.
5. Add/adjust tests before considering the task complete.
6. Run the task's required verification commands.
7. Record evidence and changed behavior in project/reports/<task-id>.md.
8. Commit one coherent task at a time.
9. Select the next unblocked task only after the current task passes.

STOP AND ESCALATE TO OWNER IF THE WORK WOULD:
- change public language syntax or semantics beyond an approved RFC;
- change the typed-units model;
- change the canonical-state/determinism contract;
- expose kernel-specific types above the kernel adapter;
- introduce or change reference-resolution fallback semantics;
- permit ambiguous semantic references to select silently;
- add a compiler intrinsic where a library solution may work;
- change a stage exit gate or weaken a benchmark;
- select between major unresolved architecture alternatives (for example OCAF-only vs a kernel-independent semantic graph);
- add a new trusted native/plugin boundary;
- require a license/security decision;
- expand into a later roadmap stage before the current gate is approved.

AUTONOMOUSLY ALLOWED
- implementation inside approved interfaces/RFCs;
- unit/integration/property/fuzz tests;
- benchmark fixtures and adversarial cases;
- diagnostics that preserve approved schemas/semantics;
- internal refactors that do not change public semantics;
- CI/build tooling;
- performance measurement and optimization that preserves behavior;
- documentation synchronized to implemented behavior;
- adding regression tests for discovered bugs.

NEVER MARK COMPLETE FROM RENDERED APPEARANCE ALONE.
Use validity, topology, analytic properties, semantic-reference checks, and independent import/round-trip evidence as appropriate.
```

## 4. Task ticket template

Each task should look like this:

```yaml
id: AICAD-042
stage: 2
objective: "Implement dimensional multiplication/division in cad-units."
references:
  - docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md
  - RFC-0004
  - crates/cad-units/
depends_on: [AICAD-041]
allowed_scope:
  - crates/cad-units/**
  - tests/typecheck/**
non_goals:
  - "Do not add user-facing syntax."
  - "Do not implement tolerances yet."
acceptance:
  - "Length / Time produces Velocity dimension."
  - "Length + Time is rejected with a stable diagnostic."
  - "Unit conversion round-trips property test passes."
commands:
  - cargo fmt --all -- --check
  - cargo clippy --workspace --all-targets --all-features -- -D warnings
  - cargo nextest run -p cad-units
escalate_if:
  - "RFC-0004 is ambiguous about dimension canonicalization."
output:
  - project/reports/AICAD-042.md
```

## 5. Initial bundle-ingestion prompt

Use this once at project bootstrap:

```text
Read the complete AICAD implementation-plan bundle as a design specification. Do not implement features yet.

Produce only:
1. a document map showing which file governs which subsystem;
2. a dependency graph of roadmap stages and work packages;
3. a list of decisions explicitly marked unresolved/prototype-before-freezing;
4. a list of non-negotiable invariants;
5. a traceability matrix from Stage 0-4 exit gates to tests/benchmarks needed to prove them;
6. any contradictions you can point to with exact source locations.

Do not resolve contradictions or make architecture choices yourself. Put them in OWNER_DECISIONS.md for review.
```

After this orientation pass, stop making the full bundle part of every prompt.

## 6. Per-task execution prompt

```text
Execute exactly task <ID> from project/TASKS.yaml.
Read AGENTS.md, project/CURRENT_STAGE.md, the task ticket, and only the referenced plan documents/source necessary for the task.

Before coding, state in the task report:
- the task objective;
- dependencies checked;
- acceptance criteria;
- any ambiguity requiring owner approval.

If no owner decision is required, implement, test, benchmark as specified, and write the evidence report. Do not start later-stage work. Do not mark complete if any required command fails.
```

## 7. Human-owned architectural decisions

The owner should personally approve these before the agent freezes them:

1. Canonical surface-syntax family and source/bundle extension choice.
2. Mutation semantics: functional core vs method/builder sugar and exact lowering semantics.
3. Type/units semantics, especially dimension canonicalization, implicit conversions, tolerance/range behavior.
4. Canonical-state and deterministic-equivalence contract.
5. Kernel abstraction boundary and what lineage information the bridge must expose.
6. Semantic-reference resolution model, durability levels, ambiguity policy, and fallback restrictions.
7. OCAF usage versus a kernel-independent semantic graph above it.
8. Raw topology epoch/lifetime rules.
9. What belongs in compiler intrinsics versus standard packages.
10. Public diagnostic schema/error-code stability rules.
11. Constraint IR semantics and solver-independence rules before sketch/assembly solving expands.
12. Trusted native extension boundary and plugin security model later.

The agent can prototype alternatives and collect evidence, but the owner chooses.

## 8. Stage progression policy

The agent may complete tasks inside a stage autonomously. It may **not** declare a roadmap stage passed. Stage gates are owner decisions backed by an evidence packet.

A gate packet should contain:

- exact git commit/revision;
- test/benchmark commands and results;
- known failures and limitations;
- representative artifacts;
- regression counts;
- performance baselines where relevant;
- unresolved decisions;
- recommendation: pass / do not pass.

For Stage 4, include the topological-naming metrics explicitly, especially silent wrong resolution. A visually correct render is not sufficient.

## 9. Cadence for one always-on agent

Use short tasks, usually 1-6 agent-hours. If a task is likely to exceed one working block or spans multiple subsystem boundaries, split it before implementation.

Recommended rhythm:

- continuous: implementation + task tests;
- after every task: one commit + evidence report;
- nightly: full workspace test suite, geometry regression suite, formatter/lints;
- several times per week once available: fuzz/property tests under bounded CPU/time budgets;
- weekly owner review: architecture decisions, benchmark trends, stage-gate progress;
- at stage gate: freeze feature work until owner passes or rejects the gate.

## 10. Branching/review

One task per short-lived branch or one task per atomic commit on an agent integration branch. Do not accumulate a week of unrelated changes.

Recommended prefixes:

```text
agent/AICAD-023-boolean-bridge
agent/AICAD-081-reference-lineage
```

Automerge is reasonable only for tasks that do not trigger an escalation category and whose required checks pass. Architecture/RFC changes should wait for owner approval.

## 11. First proof sequence

Do not aim immediately at the Stage-7 serious MVP. First prove these increasingly strong slices:

### Proof A — Kernel slice

```text
Rust/native call -> narrow OCCT bridge -> valid exact bracket B-rep -> STEP -> independent CAD import
```

### Proof B — Language slice

```text
AICAD source -> parse/type/units/HIR -> Geometry IR -> OCCT -> valid bracket -> STEP
```

### Proof C — Parametric slice

```text
AICAD source -> parameters + feature DAG -> edit parameter -> incremental rebuild -> correct valid geometry
```

### Proof D — Reference slice (hard gate)

```text
AICAD source -> semantic refs -> upstream perturbation/split/merge -> rebuild -> correct ref OR explicit ambiguity/broken result, never silent wrong selection
```

Only after Proof D is credible should the feature library expand aggressively toward advanced geometry, assemblies, and verification.
