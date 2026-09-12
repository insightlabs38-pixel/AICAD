# Session Handoff

## Canonical branch and HEAD

`origin/claude/aicad-stage3-dev`. Local commits on this branch, in order:
1. `0a5202d` — Stage-3 setup (DL-16..DL-20, `CURRENT_STAGE.md` advance,
   `TASKS.yaml` additions).
2. `1865855` — `AICAD-064A`: D5 v1 comparison-profile implementation +
   calibration (`crates/cad-validation`).
3. `799a957` — `AICAD-065`: first-class `param` declarations/derived
   expressions (`crates/cad-runtime`).
4. (this invocation's final commit) — `AICAD-066` + `AICAD-067`: the
   Stage-3 Feature DAG's node identity/dependency model and its
   construction from the currently supported modeling operations
   (`crates/cad-feature-graph`, new content — the crate itself already
   existed as an `AICAD-002`/`AICAD-003` placeholder).

All four commits are pushed to `origin/claude/aicad-stage3-dev` at the end
of this invocation (verified — see "Push verification" below).

## Active stage / current batch

Stage 3, `status: active` (`project/CURRENT_STAGE.md`). **Batch S3-01 is
now COMPLETE** (`AICAD-066` done, `AICAD-067` done — both `project/
TASKS.yaml` entries marked `status: done`).

Per the campaign brief: each invocation works on exactly ONE fixed batch,
stopping once it is complete/verified/committed/pushed. **This invocation
is stopping here.** The next invocation begins **Batch S3-02**
(`AICAD-068`: "Implement cache keys and dirty propagation", then
`AICAD-069`: "Implement source-to-feature mapping and provenance", then
`project/gates/STAGE3-A_PARAMETRIC_GRAPH.md`), required order
`068 -> 069 -> gate`.

## Last completed task

`AICAD-066`/`AICAD-067` (Feature DAG node identity + construction from
supported modeling operations). Both tasks share one commit and one
implementation — see `project/reports/AICAD-066.md`'s "Why one commit"
section for why, and `project/reports/AICAD-067.md` for the full design.
Summary: `crates/cad-feature-graph/src/graph.rs` (new) gives every call to
one of the eight closed `cad_hir::builtins::BuiltinFnId` supported
modeling operations (`box`/`cylinder`/`transform`/`union`/`cut`/
`intersect`/`fillet`/`chamfer`) its own stable `FeatureId`, records its
`Geometry`-typed arguments as ordered `geometry_inputs` dependency edges
(reusing an already-built node rather than duplicating one for a repeated
name reference), and keeps every scalar argument's own expression
(unevaluated) as a `parameters` entry — `FeatureGraph::build` walks an
already-lowered `HirProgram`'s top-level `let`/`const` items to construct
it. 11 tests in `cad-feature-graph` (was 0 — the crate was an unfilled
placeholder before this task).

## Partial task

None. `AICAD-066` and `AICAD-067` both completed cleanly in this
invocation, in the required order (as one shared design/commit — see
above).

## Next task

`AICAD-068` ("Implement cache keys and dirty propagation"), first task of
Batch S3-02. Read `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §10
("Incremental invalidation") again before starting, plus `crates/
cad-feature-graph/src/graph.rs`'s own module doc comment section "What
this module deliberately does not do" (cache keys/dirty propagation are
named there explicitly as this next task's own job). `AICAD-065`'s
`ParamModel`/`ParamOverrides` (edit/rebuild primitive) and `AICAD-067`'s
`FeatureGraph` (dependency edges: `geometry_inputs` between features,
`parameters` — the scalar argument expressions a cache key will need to
hash/compare) are both direct, deliberately-reusable inputs — read both
before designing cache-key/dirty-propagation data structures from
scratch. `AICAD-068` will need a way to know which top-level `param`/`let`
bindings a feature's own scalar parameters reference (not currently
tracked by `FeatureNode::parameters`, which stores raw expressions, not
extracted binding references) — deciding whether to add that to
`FeatureNode` itself or compute it separately in `AICAD-068` is that
task's own design call, not pre-empted here.

## Exact recent test status

- `cargo fmt --all -- --check` → clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings, 28 workspace crates (unchanged count).
- `cargo test --workspace` → 0 failures across every crate with tests
  (`cad-feature-graph`: 11, all new; every other crate unchanged from
  Batch S3-00's own last-verified counts: `cad-runtime` 101,
  `cad-validation` 7 + 2 integration).
- `cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1` →
  3/3 (Stage-2 gate proof unaffected).

## Regressions/failures

None.

## Unresolved owner decisions

Unchanged from Batch S3-00's own handoff — `D7`/`D8`/`D12`/`D13`/`D15`
remain open/partially-resolved, none blocking through at least S3-08 (see
`project/OWNER_DECISIONS.md` for each). This batch closed none and opened
none.

## D5/D19 calibration status

Unchanged — complete (`project/OWNER_DECISIONS.md#D19`, `DECISION_LOG.md
#DL-17`, `project/reports/AICAD-064A.md`). No further D5/D19 work is
scheduled.

## Current checkpoint status

No Stage-3 checkpoint has been reached yet. The next one,
`project/gates/STAGE3-A_PARAMETRIC_GRAPH.md`, is created at the end of
Batch S3-02 (after `AICAD-068`/`AICAD-069`), not before.

## Environment

Rust 1.98.1 (edition 2024, auto-installed via rustup this invocation, no
`Cargo.toml`/toolchain file changes), Ubuntu 24.04.4 LTS x86_64. No new
third-party dependency was added; `crates/cad-feature-graph`'s new
dependencies (`cad-ast`, `cad-diagnostics`, `cad-hir`, and dev-dependency
`cad-parser`) are all existing in-workspace crates, not external
additions.

## Git identity

`core.hooksPath` remained active and was not modified, disabled, or
bypassed. Every commit set `GIT_AUTHOR_NAME`/`GIT_AUTHOR_EMAIL`/
`GIT_COMMITTER_NAME`/`GIT_COMMITTER_EMAIL` to `insightlabs38-pixel`/
`insightlabs38@gmail.com` as process-local environment variables for the
`git commit` invocation only. No hook bypassed; `--no-verify` never used.

## Push verification

Before the final push, `git fetch origin claude/aicad-stage3-dev` was
re-run and the remote branch confirmed unchanged from this invocation's
own prior state (no concurrent writer) — `git push -u origin
claude/aicad-stage3-dev` completed as a fast-forward.

## Recommended next action

Start Batch S3-02 (`AICAD-068` then `AICAD-069`, then the
`STAGE3-A_PARAMETRIC_GRAPH.md` checkpoint) on the next invocation, reading
`crates/cad-feature-graph/src/graph.rs`'s module doc comment and
`crates/cad-runtime/src/params.rs` first per "Next task" above. Do not
start any later batch. Do not re-open `D19`/Stage-2 approval or Batch
S3-00/S3-01's own already-complete tasks.
