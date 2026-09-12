# Session Handoff

## Canonical branch and HEAD

`origin/claude/aicad-stage3-dev`. Commits on this branch since Stage-2
closed, in order:

1. `0a5202d` — Stage-3 setup (DL-16..DL-20, `CURRENT_STAGE.md` advance,
   `TASKS.yaml` additions).
2. `1865855` — `AICAD-064A`: D5 v1 comparison-profile implementation +
   calibration (`crates/cad-validation`).
3. `799a957` — `AICAD-065`: first-class `param` declarations/derived
   expressions (`crates/cad-runtime`).
4. `f91fe80` — `AICAD-066` + `AICAD-067`: the Stage-3 Feature DAG's node
   identity/dependency model and its construction from the currently
   supported modeling operations (`crates/cad-feature-graph`).
5. `2958ae3` — `AICAD-068`: feature-DAG cache keys and dirty propagation
   (`crates/cad-feature-graph/src/cache.rs`, new).
6. `8803326` — `AICAD-069`: feature-DAG source-to-feature mapping and
   provenance (`crates/cad-feature-graph/src/provenance.rs`, new).
7. (this invocation's final commit) — `project/gates/
   STAGE3-A_PARAMETRIC_GRAPH.md`: the Batch S3-00/S3-01/S3-02 checkpoint,
   recommending PASS.

All seven commits are pushed to `origin/claude/aicad-stage3-dev` at the
end of this invocation (verified — see "Push verification" below).

## Active stage / current batch

Stage 3, `status: active` (`project/CURRENT_STAGE.md`). **Batch S3-02 is
now COMPLETE** (`AICAD-068` done, `AICAD-069` done, `project/gates/
STAGE3-A_PARAMETRIC_GRAPH.md` prepared and recommends PASS — all three
`project/TASKS.yaml` entries/checkpoint requirement for this batch are
satisfied).

Per the campaign brief: each invocation works on exactly ONE fixed batch,
stopping once it is complete/verified/committed/pushed. **This invocation
is stopping here.** The next invocation begins **Batch S3-03**
(`AICAD-070`: "Create safe language-facing geometry types", then
`AICAD-071`: "Implement part concept plus box/cylinder/plate standard
helpers"), required order `070 -> 071`, no checkpoint at the end of S3-03
itself (the next checkpoint, `STAGE3-B_SKETCH_CONSTRAINTS.md`, lands after
Batch S3-05).

## Last completed task

`AICAD-069` (source-to-feature mapping and provenance), immediately
preceded by `AICAD-068` (cache keys and dirty propagation) in this same
invocation, both in `crates/cad-feature-graph`. Summary:

- `crates/cad-feature-graph/src/cache.rs` (new, `AICAD-068`):
  `CacheKey` — a purely structural content hash (operation name +
  `geometry_inputs`' own already-computed keys + `parameters`
  expressions), computed with a hand-rolled deterministic FNV-1a hasher
  (`StableHasher`, never `std::collections::hash_map::DefaultHasher` —
  see that module's own "Why a hand-rolled hasher" doc-comment section for
  the `DECISION_LOG.md#DL-12` Level-1 rationale). `FeatureNode` gained
  `cache_key: CacheKey` and `binding_refs: Vec<BindingId>`.
  `FeatureGraph::dirty_set(changed: &HashSet<BindingId>) ->
  HashSet<FeatureId>` implements `docs/plan/06_REFERENCES_QUERIES_
  FEATURE_DAG.md` §10's incremental-invalidation algorithm in one linear
  pass (relies on `FeatureGraph::nodes()` already being
  dependency-respecting build order).
- `crates/cad-feature-graph/src/provenance.rs` (new, `AICAD-069`):
  `Declaration` (`Let`/`Const`/`Anonymous`) and `Provenance`
  (`declared_as` + `transitive_bindings: Vec<BindingId>`, the full
  top-level-binding dependency closure — the forward-looking complement to
  `dirty_set`'s reverse direction). `FeatureGraph::feature_at(position:
  u32) -> Option<FeatureId>` resolves the innermost feature node
  containing a source byte offset. Deliberately scoped narrowly against
  `docs/plan/14_COLLABORATION_PROVENANCE_SECURITY.md`'s much larger
  Git/AI-governance "provenance" concept — see that module's own "What
  this deliberately does not do".
- `project/gates/STAGE3-A_PARAMETRIC_GRAPH.md` (new): the Batch
  S3-00/S3-01/S3-02 checkpoint (first Stage-3 checkpoint; covers
  `AICAD-064A` through `AICAD-069` since there is no earlier one),
  recommending PASS.

`crates/cad-feature-graph` test count: 0 (pre-`AICAD-066`) -> 11
(`AICAD-066`/`067`) -> 24 (`AICAD-068`) -> 34 (`AICAD-069`).

## Partial task

None. `AICAD-068`, `AICAD-069`, and the `STAGE3-A_PARAMETRIC_GRAPH.md`
checkpoint all completed cleanly in this invocation, in the required
order.

## Next task

`AICAD-070` ("Create safe language-facing geometry types"), first task of
Batch S3-03. Per `project/CURRENT_STAGE.md`'s own batch list, read
`docs/plan/04_HIGH_LEVEL_MODELING_API.md` and `docs/plan/06...` again
before starting, plus `DECISION_LOG.md#DL-15` (D18 — the already-approved
`RuntimeBuiltin`/"Safe CAD API sits above Geometry IR" design this task
must stay consistent with: "the public Safe CAD function catalogue must
not automatically expose every `GeometryOp`/`GeometryQuery` variant
one-for-one"). `crates/cad-feature-graph` (this batch's own output) is a
directly relevant, reusable input for whatever `AICAD-070`/`AICAD-071`
build, but wiring the feature graph into an actual runtime/`Part`
execution pipeline is not something either this batch or (based on its own
title, "create safe language-facing geometry types") `AICAD-070` itself
necessarily needs to do — that judgment call belongs to `AICAD-070`'s own
session, not pre-empted here.

## Exact recent test status

- `cargo fmt --all -- --check` → clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings, full workspace.
- `cargo test --workspace` → 853 passed, 0 failed (`cad-feature-graph`:
  34, up from 11 at the start of this invocation; every other crate
  unchanged from Batch S3-01's own last-verified counts).
- `cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1` →
  3/3 (Stage-2 gate proof unaffected).

## Regressions/failures

None.

## Unresolved owner decisions

Unchanged from Batch S3-01's own handoff — `D7`/`D8`/`D12`/`D15` remain
open/partially-resolved, none blocking through at least S3-08 (see
`project/OWNER_DECISIONS.md` for each). This batch closed none and opened
none.

## D5/D19 calibration status

Unchanged — complete (`project/OWNER_DECISIONS.md#D19`, `DECISION_LOG.md
#DL-17`, `project/reports/AICAD-064A.md`). No further D5/D19 work is
scheduled. `STAGE3-A_PARAMETRIC_GRAPH.md` §3.1/§3.7 re-confirms this batch
introduced no new D5 Level-1 nondeterminism (specifically checked
`crate::cache::CacheKey`'s hand-rolled hasher and every `HashMap`/
`HashSet` usage in `crates/cad-feature-graph`).

## Current checkpoint status

`project/gates/STAGE3-A_PARAMETRIC_GRAPH.md` prepared and recommends
PASS — covers Batches S3-00, S3-01, and S3-02 (`AICAD-064A` through
`AICAD-069`). The next checkpoint, `STAGE3-B_SKETCH_CONSTRAINTS.md`, is
created after Batch S3-05 (`AICAD-073`/`074`/`075`), not before.

## Environment

Rust 1.98.1 (edition 2024, auto-installed via rustup this invocation, no
`Cargo.toml`/toolchain file changes), container Linux environment
(`uname -r` reports a Fedora CoreOS-style kernel; the underlying repo
targets Ubuntu 24.04.4 LTS x86_64 per `AICAD-064A`'s own recorded
environment — no toolchain/target changes made this invocation). No new
third-party dependency was added; `crates/cad-feature-graph`'s two new
modules (`cache.rs`, `provenance.rs`) use only already-in-workspace
crates (`cad-ast`, `cad-hir`) plus the existing `cad-parser` dev-dependency
for tests.

## Git identity

`core.hooksPath` remained active and was not modified, disabled, or
bypassed. Every commit set `GIT_AUTHOR_NAME`/`GIT_AUTHOR_EMAIL`/
`GIT_COMMITTER_NAME`/`GIT_COMMITTER_EMAIL` to `insightlabs38-pixel`/
`insightlabs38@gmail.com` as process-local environment variables for the
`git commit` invocation only. No hook bypassed; `--no-verify` never used.

## Push verification

Before the final push, `git fetch origin claude/aicad-stage3-dev` is
re-run and the remote branch confirmed unchanged from this invocation's
own prior state (no concurrent writer) before `git push -u origin
claude/aicad-stage3-dev` runs as a fast-forward.

## Recommended next action

Start Batch S3-03 (`AICAD-070` then `AICAD-071`, no checkpoint at the end
of this particular batch) on the next invocation, reading `docs/plan/
04_HIGH_LEVEL_MODELING_API.md` and `DECISION_LOG.md#DL-15` first per "Next
task" above. Do not start any later batch. Do not re-open `D19`/Stage-2
approval or any of Batch S3-00/S3-01/S3-02's own already-complete tasks.
