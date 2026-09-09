# Stage gate evidence packets

Each roadmap stage's exit-gate evidence packet lives here as
`stage-<n>-gate.md`, produced by that stage's final "prepare owner gate
packet" task (e.g. AICAD-014 for Stage 0, AICAD-037 for Stage 1, AICAD-064
for Stage 2, AICAD-100 for Stage 4).

Per `AGENTS.md` ("Stage gates") and `AICAD_AGENT_OPERATING_MODEL.md` §8, a
gate packet must contain: exact git commit/revision, test/benchmark commands
and results, known failures and limitations, representative artifacts,
regression counts, performance baselines where relevant, unresolved
decisions, and a pass/do-not-pass recommendation.

The implementation agent may prepare this evidence and recommend pass/do-not-pass.
It may **not** approve a roadmap stage — stage progression is an owner
decision recorded in `project/DECISION_LOG.md`.
