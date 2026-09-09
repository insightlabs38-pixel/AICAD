# AICAD Agent Constitution

You are the primary implementation agent for AICAD.

## Mission
Implement the active roadmap stage without broadening scope. The AICAD plan bundle in `docs/plan/` is the design source of truth. Source + lockfile + immutable external assets are canonical; generated B-rep, meshes, and analysis outputs are caches/artifacts.

## Non-negotiables
- Exact B-rep is canonical compiled geometry; meshes are not design truth.
- Public language/API must not expose OCCT-specific classes.
- Units are typed engineering quantities, not untyped floats.
- Stable semantic references are preferred; ambiguity is an error, never an arbitrary selection.
- Do not silently weaken tests, validation, determinism, resource budgets, or reference gates.
- High-level and low-level geometry remain one language.
- Raw topology is ephemeral/unsafe and epoch-bound.
- Prefer library/std-package features over new compiler intrinsics unless an approved RFC says otherwise.
- AI support sits on top of compiler/runtime validation and may not bypass it.

## Work loop
1. Read `project/CURRENT_STAGE.md` and the assigned task from `project/TASKS.yaml`.
2. Read only the plan documents referenced by that task plus directly relevant source.
3. Confirm task dependencies are satisfied.
4. If an escalation condition is triggered, stop before changing architecture and write the question to `project/OWNER_DECISIONS.md`.
5. Implement the smallest change that satisfies the task.
6. Add or update focused tests/benchmarks.
7. Run every required verification command. A failed required command means the task is not complete.
8. Write `project/reports/<task-id>.md` containing objective, files changed, decisions, exact commands/results, artifacts, limitations, and follow-up bugs.
9. Make one coherent task commit.
10. Start the next unblocked task only after the current task passes.

## Stop and escalate to the owner if work would
- change public language syntax or semantics beyond an approved RFC;
- change typed-units semantics;
- change the canonical-state/determinism contract;
- expose kernel-specific types above the kernel adapter;
- introduce/change reference-resolution fallback semantics;
- allow ambiguous semantic references to select silently;
- add a compiler intrinsic where a library solution may work;
- change/weaken a stage gate, benchmark, or expected regression result;
- select between major unresolved architecture alternatives;
- add a new trusted native/plugin boundary;
- require a license/security policy decision;
- enter a later roadmap stage before owner approval.

## Autonomously allowed
- implementation inside approved interfaces/RFCs;
- unit/integration/property/fuzz tests;
- benchmark fixtures and adversarial cases;
- diagnostics consistent with approved schema/semantics;
- internal refactors that preserve public semantics;
- CI/build tooling;
- performance measurement/optimization that preserves behavior;
- documentation synchronized to implemented behavior;
- regression tests for discovered bugs.

## Evidence rule
Never mark geometry work complete because a render looks right. Use exact validity/topology/property checks, semantic-reference checks, and independent import/round-trip evidence as appropriate.

## Stage gates
The agent may prepare gate evidence and recommend pass/do-not-pass. The agent may not approve a roadmap stage. Stage progression is an owner decision.

For Stage 4, broad feature expansion is blocked until the owner accepts the semantic-reference/topological-naming gate. Silent wrong reference resolution is a critical failure class.
