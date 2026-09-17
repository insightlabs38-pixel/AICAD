# Lightweight stage-promotion and reconciliation policy

This policy governs Stage 5 -> Stage 6 and Stage 6 -> Stage 7. Its purpose is to keep later transitions evidence-driven without reflexively reopening settled architecture.

## Authority

Current owner decisions and `project/DECISION_LOG.md` remain authoritative. Accepted specs/RFCs and actual preceding-stage implementation/gate evidence follow them. Historical post-100 audits and provisional queues are planning input, not independent semantic authority.

## Stage 5 -> Stage 6

At the Stage-5 owner gate, compare actual Stage-5 evidence against the assumptions recorded in `STAGE6_PROVISIONAL.yaml`, especially spatial/frame semantics, geometry query cardinality/order/tolerance behavior, semantic-reference/lineage behavior, source abstraction/provenance behavior, and structured machine-facing geometry APIs.

If **no material architecture or semantic assumption is invalidated**:

1. do not perform another broad roadmap audit;
2. perform a short reconciliation pass limited to actual evidence differences;
3. keep the existing `S6-*` decomposition/dependency order except for concrete affected details;
4. assign final global `AICAD-*` IDs to the promoted Stage-6 queue;
5. update acceptance criteria only where final Stage-5 evidence makes them more precise;
6. record any still-required Stage-6 owner decision;
7. obtain owner approval, merge the transition, and create the Stage-6 development branch from that exact merged HEAD.

If a **material conflict exists**, adjust only the affected Stage-6 tasks/dependencies/specs. A material conflict means actual evidence invalidates a public semantic assumption, identity model, dependency edge, capability boundary, or gate criterion—not merely that an implementation API name/file/crate differs from the provisional plan.

Do not reopen D26-D30 or other settled decisions merely because historical planning files describe equivalent questions as open.

## Stage 6 -> Stage 7

Apply the same process against `STAGE7_PROVISIONAL.yaml`. Reconcile actual Stage-6 evidence for assembly/configuration identity, relation/solver evidence, external assets, structured APIs, and realistic/adversarial results. Resolve OD-S7-01..OD-S7-05 at their latest safe decision points.

If no material assumption is invalidated, promote the existing `S7-*` queue with final global IDs and only local acceptance-detail corrections. If a material conflict exists, modify only the affected tasks/dependencies/specs.

## What does not justify broad replanning

The following normally require only local reconciliation:

- crate/module/file names differing from a provisional `likely_crates` guess;
- an internal helper or representation changing while preserving the accepted public contract;
- performance tuning that does not alter deterministic semantics, resource policy, or evidence;
- additional regression tests or examples;
- splitting one task for bounded execution while preserving capability/dependency order;
- merging tightly coupled implementation subtasks without erasing a checkpoint or acceptance obligation.

## What does justify targeted replanning

Examples include:

- Stage-5 evidence showing that the final spatial/reference/query contract cannot support the planned Stage-6 identity or mate semantics;
- Stage-6 evidence showing that verification cannot consume stable assembly/configuration subjects without a different public semantic model;
- an owner decision selecting a materially different language/evidence contract;
- a discovered silent-wrong/false-pass class requiring a different dependency ordering or hard gate;
- a preceding-stage limitation making a supposedly available prerequisite unavailable.

Even then, change only the impacted portion unless evidence demonstrates systemic inconsistency.

## Fixed invariants across promotions

- A later stage never begins before owner approval of the preceding gate and transition state.
- Provisional local IDs are non-executable and are not zero-context worker authorization.
- Stage-gate recommendations by an agent are advisory; owner approval remains explicit.
- Settled owner decisions are not reopened by stale audit language.
- Exact B-rep remains canonical compiled geometry; meshes are not design truth.
- Public semantics remain kernel-neutral.
- Ambiguity and broken semantic references remain fail-closed; no automatic fingerprint recovery is inferred from a later stage.
- Raw handles remain ephemeral/epoch-bound and are never durable identity.
- Tolerance domains remain explicit; no global epsilon is introduced through promotion.
- Current examples remain maintained executable product surfaces rather than aspirational syntax samples.
