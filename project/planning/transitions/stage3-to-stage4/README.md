# Stage 3 → Stage 4 transition reconciliation

Status: **active transition; documentation/governance/normative cleanup complete for review; Stage 4 not started**.

This directory records the post-Stage-3 transition work on `claude/aicad-stage4-transition`, a strict descendant of approved Stage-3 `main` merge `15fc5a37e4382de717e426ccc5317a491be264cd`.

Earlier passes preserved Stage-0..3 history, imported the frozen post-100 audit as non-normative planning, established current user/developer documentation, recorded Stage-3 approval as DL-22, and clarified the source/internal sketch boundary plus the Stage-3 identity/Stage-4 persistent-reference boundary.

The normative pass restores `specs/language/{grammar,semantics,types,diagnostics}`, aligns accepted RFC wording with current decisions and implementation boundaries, preserves exact pre-cleanup RFC bodies under `rfcs/history/stage0/`, separates tolerance categories, and marks `docs/plan/` as frozen foundation planning rather than automatic current authority.

See `NORMATIVE_SPEC_CLEANUP.md` for the detailed audit and `DEFERRED_TRANSITION_ITEMS.md` for remaining work.

## D20 authority conflict

The live repository records D20 as `RESOLVED — DL-21` and current Stage-3 code follows that model. The directive for the normative-cleanup pass says D20 is open. This transition does not rewrite either source; explicit reconciliation is required before final Stage-4 initialization.

## Stage 4 remains unstarted

`AICAD-080` remains todo. No Stage-4 persistent-reference implementation, production behavior, or CI expansion is introduced by the normative pass.

Do not merge this transition branch automatically or begin AICAD-080 solely because the cleanup pass is complete.
