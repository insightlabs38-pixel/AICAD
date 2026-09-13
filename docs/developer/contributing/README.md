# Contributing / development workflow

Before implementation work, read `CLAUDE.md`, `AGENTS.md`, and the active project-control files (`CURRENT_STAGE.md`, `TASKS.yaml`, owner decisions, decision log, session handoff). Retrieve historical reports/gates only when the current task needs their evidence.

Current technical documentation belongs under `docs/developer/`; user guidance belongs under `docs/user/`; implementation diaries, gate evidence, architecture audits, and transition records belong under `project/`.

Do not treat the frozen post-100 audit in `project/planning/roadmap/post100/` as approved architecture. Do not treat `docs/plan/` examples as proof that a later-stage feature is implemented.

Use repository-required Git identity and hooks; never bypass checks or force-push transition/implementation branches.
