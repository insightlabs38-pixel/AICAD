# Session Handoff

## Canonical working branch

`origin/claude/aicad-stage4-transition`

This branch is a strict descendant of the owner-approved Stage-3 `main`
merge `15fc5a37e4382de717e426ccc5317a491be264cd`; it must not be restarted
from `main` or rebased in a way that discards the transition history.

## Current status

**Stage 3 is complete, owner-approved, and merged.** The approval is now
recorded in `project/DECISION_LOG.md#DL-22`, following the same governance
pattern as the earlier stage approvals. `project/CURRENT_STAGE.md` has been
updated from the stale pre-approval Stage-3 state to the active Stage-3 ->
Stage-4 transition state.

**Stage 4 implementation has not started.** `AICAD-080` remains
`status: todo`. The current transition work does not authorize semantic-
reference implementation, Stage-5 work, CI expansion, or AICAD-101+ tasks.

## Accepted Stage-3 implementation

The Stage-3 `main` merge is:

`15fc5a37e4382de717e426ccc5317a491be264cd`

Its lineage includes final remediation commit:

`99fb0d3b17000b0a1c6a1a3c175ea16f0d450180`

That remediation connected `ParamModel` and `FeatureGraph` through the real
`cad_cli::parametric_build::ParametricBuildSession` production path, fixed
the discovered parameter-evaluation ordering defect, and added the
in-process selective geometry reuse/recompute proof. The final Stage-3 gate
records PASS after remediation.

The last Stage-3 verification recorded by that remediation remains
historical evidence, not a fresh test run for this documentation branch:

- `cargo fmt --all -- --check` — clean;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  zero warnings;
- `cargo test --workspace` — 1047 passed, 0 failed;
- Stage-3 parametric incremental rebuild integration — 4/4;
- Stage-3 ordinary-part integration — 7/7;
- frozen Stage-4 benchmark fixture integration — 20/20.

See the archived original at
`project/reports/archive/stage3/AICAD-079B-INCREMENTAL-REMEDIATION.md` for
exact commands/results.

## Transition work completed so far

### First reconciliation pass

- created/preserved `claude/aicad-stage4-transition` from the exact Stage-3
  `main` merge;
- imported the post-100 architecture/roadmap audit as a frozen, non-
  normative snapshot under `project/planning/roadmap/post100/`;
- preserved Stage-0..3 task reports and gate/checkpoint evidence under
  stage-specific archive directories while retaining compatibility pointers;
- established separate `docs/user/`, `docs/developer/`, and `project/`
  information surfaces;
- preserved the frozen `docs/plan/` bundle in place because moving it would
  create broad historical-reference churn;
- created the Stage-3 -> Stage-4 transition record.

### Second documentation pass

- rewrote the repository root README as a post-Stage-3 project landing page;
- populated the Stage-3 user guide with installation, first-part, language,
  parameters, current Safe CAD modeling, CLI, examples, and troubleshooting;
- documented the critical user boundary that the sketch/constraint engine
  exists but source-level `sketch { ... }` authoring is not yet integrated;
- populated current developer architecture docs for frontend/HIR/runtime,
  Geometry IR, the OCCT boundary, parametrics/incremental rebuild,
  constraints, testing, and contributing;
- documented the remediated incremental path accurately as an in-process
  session capability, not a persistent cross-process cache;
- recorded the already-issued Stage-3 approval as DL-22 without reopening
  or changing any owner architecture decision.

## Documentation sources and precedence

When current documentation needs verification, prefer:

1. current code;
2. accepted owner decisions;
3. accepted RFC/spec semantics;
4. final gate evidence;
5. relevant archived task reports;
6. old planning prose.

Completed reports/gates are evidence and should be retrieved when relevant;
they are not routine startup context or the current developer manual.

## Known current boundaries

- `.aicad` source has no supported `sketch { ... }` authoring syntax even
  though the sketch/constraint subsystem is implemented below the source
  layer.
- Stage-3 raw face/edge integer selectors are not persistent references.
- `FeatureGraph` does not yet provide general interprocedural flattening of
  arbitrary geometry-producing source functions/branches.
- `ParametricBuildSession` reuses geometry only within the live in-process
  session/kernel context; no persistent disk/remote cache is implied.
- the implemented CLI is `cad build <path.aicad> [--json] [--output <path>]
  [--name <binding>[.<field>]]`; broader planned commands are not current.

## Remaining transition work

After owner review of the documentation pass, the major transition work
still intentionally deferred is:

1. normative specification/decision cleanup identified by the post-100
   audit;
2. expanded Stage-4 CI/CD preparation;
3. final Stage-4 initialization and explicit implementation authorization.

Do not begin those merely because they are listed here; use the next owner-
provided transition instructions.

## Next action

**Owner review of this documentation/governance pass.** Do not start
AICAD-080, do not expand CI/CD, and do not begin the normative-spec pass in
this session.

## Git policy

Required commit author/committer identity is
`insightlabs38-pixel <insightlabs38@gmail.com>`. Keep repository hooks
active; never use `--no-verify`, never force-push transition history, and do
not add AI/session attribution metadata.
