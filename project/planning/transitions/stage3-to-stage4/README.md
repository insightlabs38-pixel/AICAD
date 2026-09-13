# Stage 3 → Stage 4 transition reconciliation

Status: **active transition; documentation/governance/normative cleanup and D21-D30 owner-decision recording complete for review; Stage 4 not started**.

This directory records the post-Stage-3 transition work on `claude/aicad-stage4-transition`, a strict descendant of approved Stage-3 `main` merge `15fc5a37e4382de717e426ccc5317a491be264cd`.

Earlier passes preserved Stage-0..3 history, imported the frozen post-100 audit as non-normative planning, established current user/developer documentation, recorded Stage-3 approval as DL-22, and clarified the source/internal sketch boundary plus the Stage-3 identity/Stage-4 persistent-reference boundary.

The normative pass restored `specs/language/{grammar,semantics,types,diagnostics}`, aligned accepted RFC wording with current decisions and implementation boundaries, preserved exact pre-cleanup RFC bodies under `rfcs/history/stage0/`, separated tolerance categories, and marked `docs/plan/` as frozen foundation planning rather than automatic current authority.

A narrow follow-up correction removed stale transition-brief wording that incorrectly treated D20 as open. D20 remains resolved by DL-21 and implemented by AICAD-076A.

## D21-D30 owner-decision recording

The owner has approved ten additional future-stage semantic baselines. Because DL-22 is already the Stage-3 approval record, they are recorded as:

- D21 → DL-23 — closed RuntimeBuiltin catalogue scaling;
- D22 → DL-24 — safe/raw geometry boundary;
- D23 → DL-25 — kernel-backed source-query execution;
- D24 → DL-26 — distinct tolerance domains;
- D25 → DL-27 — feature/dependency/provenance preservation through abstraction;
- D26 → DL-28 — assembly identity domains;
- D27 → DL-29 — general nominal interfaces/protocols and bounded generics;
- D28 → DL-30 — solver-neutral assembly relations/deterministic pose;
- D29 → DL-31 — immutable configuration/suppression/replacement semantics;
- D30 → DL-32 — external-asset content identity/provenance.

These rulings resolve frozen post-100 recommendations OD-S5-02..OD-S5-06 and OD-S6-01..OD-S6-05 at the semantic-baseline level. The frozen audit itself remains unchanged as a historical snapshot. Details explicitly marked deferred by D21-D30 remain future implementation/specification work and are listed in `DEFERRED_TRANSITION_ITEMS.md`.

Stage-4 evidence may refine only explicitly deferred representation/implementation details through explicit review; it may not silently weaken the approved invariants. See `NORMATIVE_SPEC_CLEANUP.md` for the detailed mapping and anti-drift rule.

## Stage 4 remains unstarted

`AICAD-080` remains todo. No Stage-4 persistent-reference implementation, production behavior, Stage-5/6 implementation, or CI expansion is introduced by the normative/governance passes.

Do not merge this transition branch automatically or begin AICAD-080 solely because the cleanup and owner-decision recording are complete.
