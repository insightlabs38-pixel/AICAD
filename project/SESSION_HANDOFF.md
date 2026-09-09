# Session Handoff

## Latest: Batch 1C (hard geometry operations) complete and CANONICAL on main — AICAD-025 through AICAD-028 done

Batch 1C's work (`AICAD-025` through `AICAD-028`) plus a CI infrastructure
fix (see below) merged into `origin/main` via **PR #3**
(`https://github.com/insightlabs38-pixel/AICAD/pull/3`, merge commit
`903dceb`). `origin/main` now contains this work — a future invocation
can confirm with `git log origin/main` and finding commit `61a9520`
(AICAD-028) or `903dceb` (the merge) in its ancestry, rather than trusting
this note alone.

### CI was red on `main` since the PR #2 merge (`179afd8`) — found and fixed this session, confirmed green before merge

While driving PR #3 to green, this session discovered `main` itself had
been red since the prior PR (#2, which introduced Batch 1A/1B) — three
separate pre-existing infrastructure gaps in `.github/workflows/ci.yml`,
none caused by Batch 1C's own content:

1. `cad-occt-bridge/build.rs` (AICAD-018) runs CMake against
   `native/occt_bridge`, requiring OpenCASCADE — but neither
   `build-and-test` nor `clippy` ever had an OCCT install step.
2. `native-build-smoke` did install OCCT, but was missing
   `libocct-visualization-dev`, which Ubuntu's OCCT packaging uses to
   house one transitively-included header
   (`NCollection_AliasedArray.hxx`, pulled in by `BRepPrimAPI_MakeBox.hxx`).
3. Every OCCT module in use hardcodes a link dependency on
   `libtbb.so`/`libtbbmalloc.so` (confirmed by grepping OCCT's own
   installed `.cmake` target files); the transitively-pulled runtime
   `libtbb12` package doesn't provide the unversioned `.so` symlink
   needed at link time — only `libtbb-dev` does.

Fixed across commits `3e4d4a9`, `101ce12`, `7c00793` (each diagnosed from
an actual CI failure log, not guessed), with the install step ultimately
factored into a reusable composite action
(`.github/actions/install-occt-dev/action.yml`) used by all three jobs
that need it, after duplicating it inline caused two of the three gaps
above to go unnoticed in different jobs. **Confirmed all four checks
green and `mergeable_state: clean` via `pull_request_read`
(`get_check_runs`) before the PR merged** — not just the absence of a new
failure notification. This fix benefits every future PR against this repo
automatically now that it's on `main`.

## What this session did (AICAD-025 through AICAD-028)

| Task | Summary | Report |
|---|---|---|
| AICAD-025 | `sweep`/`loft` minimal supported forms (`BRepOffsetAPI_MakePipe`/`ThruSections`); straight-spine sweep matches extrude's own volume; loft matches an analytic frustum-of-a-pyramid formula; two degenerate-spine cases probed and their actual validity outcomes recorded | `project/reports/AICAD-025.md` |
| AICAD-026 | Boolean `union`/`cut`/`intersect` (`BRepAlgoAPI_Fuse`/`Cut`/`Common`); volumes checked against inclusion-exclusion identities; resolved the "epoch-bump-on-mutation" test deferred since AICAD-016/018/019 | `project/reports/AICAD-026.md` |
| AICAD-027 | `fillet`/`chamfer` (`BRepFilletAPI_MakeFillet`/`MakeChamfer`) plus the raw indexed edge-selection primitives (`shape_edge_count`/`get_edge`) they need; fillet-all-edges matches the analytic "rounded box" (Minkowski-sum-with-a-ball) volume formula; single-edge chamfer matches an exact triangular-prism formula | `project/reports/AICAD-027.md` |
| AICAD-028 | `shell`/`offset` spike (`BRepOffsetAPI_MakeThickSolid`/`MakeOffsetShape`) plus `shape_face_count`/`get_face`; shell volume matches an analytic hollow-cavity formula; offset volume matches the *same* Minkowski-sum formula as AICAD-027's fillet-all-edges (an unplanned, independent cross-check) | `project/reports/AICAD-028.md` |

**Batch 1C checkpoint:** `project/gates/STAGE1-C_HARD_OPS.md` — **PASS**,
all four tasks implemented and re-verified fresh together (12/12 native
`ctest`, 53/53 `cad-occt-bridge` tests, full workspace
`fmt`/`clippy`/`build`/`test`). Carries forward Batch 1A/1B's two open
gaps (G1: exception containment still unproven against a genuine OCCT
throw; G2: no fresh valgrind run this batch) plus new, explicitly
non-blocking Batch-1C notes (sweep spine G1-continuity limits, fillet/
shell failure-boundary mapping left unsystematic per AICAD-028's own
"spike" charter, and the raw/index-based — not persistent-reference —
nature of edge/face selection).

**The one genuine architectural question this batch raised** (how to
select *which* edges/faces to fillet/chamfer/shell without a persistent
semantic-reference system, which does not exist until Stage 3/4) was
resolved by implementing `docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`
§5-6's own already-approved raw/indexed topology-access design
(`raw_edge(f, 2)`), not by selecting among unresolved
`OWNER_DECISIONS.md` alternatives — so no owner escalation was needed, and
none was recorded (`project/OWNER_DECISIONS.md`/`project/DECISION_LOG.md`
remain exactly as they were at Stage-0 approval, DL-10; no new entry this
session).

**Per-invocation work budget:** this invocation completed exactly one
batch (1C), plus the CI-fix work that was required to actually get that
batch's own PR to a mergeable state (not scope creep — CI was blocking
this session's own PR). Stopping here for a clean handoff, per
`AGENTS.md`/the active scheduled-task brief.

## Current state / next action

- **Active stage:** Stage 1 (`project/CURRENT_STAGE.md`), Batch 1A, 1B,
  and 1C all complete, checkpointed (PASS), and now canonical on
  `origin/main` (merge commit `903dceb`).
- **Next task:** `AICAD-029`, the first task in **Batch 1D — Inspection /
  validation / interchange** (`AICAD-029` through `AICAD-033`), per
  `project/TASKS.yaml` and the scheduled-task brief's batch list. Read
  `AICAD-029`'s full `project/TASKS.yaml` entry and its `plan_references`
  before starting (not yet read this session).
- **After Batch 1D (AICAD-033):** create/update
  `project/gates/STAGE1-D_INTERCHANGE.md` before Batch 1E
  (`AICAD-034..037`, the Stage-1 proof + owner gate packet).
- **A likely early Batch-1D need:** a proper `explore_topology`-style
  query surface (per `docs/plan/05` §6, "prefer queries") may want to
  reconcile with AICAD-027/028's raw `shape_edge_count`/`get_edge`/
  `shape_face_count`/`get_face` additions from this session — read those
  two reports' "Limitations" sections first; Batch 1D is where a more
  general topology-exploration API is expected to land, and it should
  most likely be built as an *addition* alongside the existing raw
  edge/face accessors (which fillet/chamfer/shell already depend on),
  not a replacement, unless AICAD-029's own task ticket says otherwise.
- **CI health:** now green on `main` after this session's fix (see
  above). A future invocation opening a new PR should get a clean CI run
  from the start — if it doesn't, something regressed and is worth
  investigating before assuming it's the same pre-existing issue this
  session already fixed.
- No owner blockers. No regressions. All required workspace checks
  (`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, `cargo build --workspace --all-targets`,
  `cargo test --workspace`) and the native `ctest` suite
  (`native/occt_bridge/build`, 12 tests) pass as of the AICAD-028 commit
  and this checkpoint's own fresh re-run.
- Environment (Stage-1 kernel policy #15, unchanged across
  AICAD-015..028, reconfirm at the start of Batch 1D rather than
  assuming): Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1
  (`rust-toolchain.toml`), CMake 3.28.3, OCCT 7.6.3 (`libocct-*-dev`
  7.6.3+dfsg1-7.1build1; this session additionally confirmed `TKOffset`,
  `TKBO`, `TKFillet`, and the CI-only-needed
  `libocct-visualization-dev`/`libtbb-dev` packages).

## Important decisions this session

- No new `project/DECISION_LOG.md` or `project/OWNER_DECISIONS.md`
  entries were required. All four AICAD-025..028 tasks stayed within the
  approved Stage-1 kernel architecture (RFC-0002, DL-5, DL-10) and made
  only the kind of autonomous implementation decisions AGENTS.md's
  "Autonomously allowed" section permits (documented individually in each
  task report's "Implementation decisions" section) — most notably:
  - Sweep/loft/booleans/fillet/chamfer/shell/offset's target-shape
    arguments are **not** restricted to `TopAbs_SOLID` (unlike
    extrude/revolve, which require a `Face`): an empirical probe
    (AICAD-026) proved OCCT's own boolean operations always produce a
    `TopAbs_COMPOUND`, never a bare `TopAbs_SOLID`, so a Solid-only
    restriction on any operation that must remain chainable after a
    boolean would break ordinary usage. This same reasoning was reapplied
    consistently in AICAD-027 (fillet/chamfer) and AICAD-028
    (shell/offset).
  - Edge/face selection for fillet/chamfer/shell is raw and index-based
    (`shape_edge_count`/`get_edge`, `shape_face_count`/`get_face`),
    implementing `docs/plan/05`'s own already-frozen raw-topology-access
    design rather than inventing a new one.
  - Loft uses `ruled=true` (straight generatrices) rather than OCCT's
    smoothed/spline-fitted default, specifically so its volume could be
    checked against an exact closed-form formula (AGENTS.md's evidence
    rule) rather than merely asserting validity.
  - The CI fix (composite action for OCCT package installation) is
    CI/build tooling, explicitly within AGENTS.md's "Autonomously
    allowed" scope — not an application-code or architecture decision.

## Git identity

All commits this session used `insightlabs38-pixel
<insightlabs38@gmail.com>` (verified via the repository's
`commit-msg`/`prepare-commit-msg` hooks at `core.hooksPath`, which reject
any other identity or AI-attribution text). No hook was bypassed or
modified. This session's own local git config (`user.name`/`user.email`)
had to be explicitly set at session start — the environment's default/
global git config did not match the required identity — before any commit
in this session would pass the `prepare-commit-msg` hook; this is worth a
future session checking again early, since it is not guaranteed to
persist across container/environment resets.

## Note on this branch (`branch/pensive-hopper-5cbjby`)

This branch's PR (#3) is merged and closed. Per this session's own
operating instructions, a merged PR is finished and must not be reopened;
this branch was reset to `origin/main` (`git reset --hard origin/main`)
and this handoff commit added on top, rather than leaving a stale,
already-merged-content branch tip around. A future invocation starting
fresh Stage-1 work should still verify `origin/main`'s exact state itself
(per this file's own "STATE RECONSTRUCTION" instructions) rather than
trusting this note alone, but should not expect to find unmerged Batch 1C
work on this branch — it is all on `main` already.
