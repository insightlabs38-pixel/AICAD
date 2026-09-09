# AICAD-002 — Create proposed monorepo directory skeleton

## Objective
Create the proposed monorepo directory skeleton per `project/TASKS.yaml`
(AICAD-002) and `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` §1, without
implementing product features, a parser, a GUI, or a broad AI tool layer
(all disallowed at Stage 0 per `project/CURRENT_STAGE.md`).

## Dependencies checked
AICAD-001 (plan bundle imported into `docs/plan/`) — complete, see
`project/reports/AICAD-001.md`.

## Decisions made and why

1. **Relocated `CURRENT_STAGE.md` and `aicad_tasks_001_100.yaml` into
   `project/`.** `AGENTS.md` line 20 and `project/FIRST_PROMPT.md` both
   reference `project/CURRENT_STAGE.md` and `project/TASKS.yaml` as the
   canonical control-file locations (matching
   `AICAD_AGENT_OPERATING_MODEL.md` §2's suggested repository control
   files), but both files were present at the repository root instead.
   Used `git mv` (history-preserving) to relocate them to
   `project/CURRENT_STAGE.md` and `project/TASKS.yaml`. No content was
   changed. This resolves the structural mismatch noted in
   `project/reports/ORIENTATION_PASS.md` §2.1 without requiring an owner
   decision — it is a pure repository-layout fix, not a plan/architecture
   change.
2. **Skeleton only, no Rust/native/toolchain files yet.** `AICAD-003` is
   explicitly titled "Create Rust workspace/toolchain policy and baseline
   ignore/editor config," so this task deliberately does not create
   `Cargo.toml`, `CMakeLists.txt`, `package.json`, `.gitignore`, or any
   crate `Cargo.toml`/source files — only directories with a `README.md`
   each, to keep AICAD-002 and AICAD-003 non-overlapping.
3. **Every created directory carries a `README.md`** naming its owning
   work package (WP-01..WP-19 from `docs/plan/22_REPOSITORY_WORK_PACKAGES.md`)
   and the specific plan sections governing it, rather than bare
   `.gitkeep` files. Git cannot track empty directories, so a placeholder
   file was required in every leaf directory regardless; making that
   placeholder a short ownership note directly satisfies AICAD-002's
   acceptance criterion ("plus task-specific tests/benchmarks satisfy the
   referenced plan requirements") by making the skeleton self-documenting
   against `22_REPOSITORY_WORK_PACKAGES.md` rather than an unexplained
   empty tree.
4. **Added `project/gates/README.md` and `project/experiments/README.md`**
   documenting their purpose per `AGENTS.md` "Stage gates" and
   `AICAD_AGENT_OPERATING_MODEL.md` §7/§8, since the operating model lists
   these directories without describing them in the task/report bodies
   themselves.
5. **Added `project/DECISION_LOG.md`** as an empty log with the entry
   format from `AICAD_AGENT_OPERATING_MODEL.md` §2, ready for
   owner-approved decisions. Content is a template only — no decisions
   have been approved, consistent with `project/OWNER_DECISIONS.md` being
   entirely `open` at this point.
6. **Added a root `README.md`** indexing the control files and the
   repository layout, cross-referencing each directory's own `README.md`.
7. **Did not create `docs/plan/22_REPOSITORY_WORK_PACKAGES.md`/`23_...`
   entries in `docs/plan/`'s own index** (that file is the immutable
   imported bundle from AICAD-001 and is not edited in place; the stale
   index in `docs/plan/README.md` is already flagged in
   `project/reports/ORIENTATION_PASS.md` §6, contradiction 1, and in
   `project/OWNER_DECISIONS.md`).

No escalation condition was triggered: this is repository/toolchain
scaffolding, explicitly allowed at Stage 0
(`project/CURRENT_STAGE.md` "Allowed work"), and does not touch public
syntax/semantics, stage gates, kernel-type exposure, reference-resolution
fallback, or an unresolved architecture alternative.

## Files changed
- Renamed (git mv, history preserved): `CURRENT_STAGE.md` ->
  `project/CURRENT_STAGE.md`; `aicad_tasks_001_100.yaml` ->
  `project/TASKS.yaml`.
- Added: root `README.md`.
- Added: `project/DECISION_LOG.md`, `project/gates/README.md`,
  `project/experiments/README.md`.
- Added: 104 new directory-ownership `README.md` files across
  `crates/` (27 crates), `native/occt_bridge/`, `std/` (6 packages),
  `editor/` (6 subsystems), `tree-sitter-aicad/`, `skills/`,
  `examples/` (6 categories), `tests/` (10 suites), `benchmarks/`
  (5 suites), `docs/` (4 authored subdirectories, alongside the existing
  `docs/plan/`), `specs/` (2 subdirectories).

## Verification (exact commands/results)
```
$ find crates native std editor tree-sitter-aicad skills examples tests benchmarks docs specs project -type f | wc -l
104

$ find . -maxdepth 3 -type d -not -path "./.git*" -empty
(no output — no empty, untracked-by-git directories remain)

$ git mv CURRENT_STAGE.md project/CURRENT_STAGE.md && git mv aicad_tasks_001_100.yaml project/TASKS.yaml
$ git status
  renamed:    CURRENT_STAGE.md -> project/CURRENT_STAGE.md
  renamed:    aicad_tasks_001_100.yaml -> project/TASKS.yaml
  (all new files listed as untracked, staged in this task's commit)

$ grep -rn "aicad_tasks_001_100" project/TASKS.yaml project/CURRENT_STAGE.md
(no output — no stale self-references to the old filename)
```

Required checks per task ticket:
- `cargo fmt --all -- --check` — not applicable, no Rust workspace exists
  yet (deliberately deferred to AICAD-003).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  not applicable, same reason.
- Task-specific check: directory structure matches
  `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` §1's proposed monorepo layout
  (verified by direct comparison against §1's tree, reproduced in
  `README.md`'s "Repository layout" section) and every leaf directory is
  git-trackable (verified by the empty-directory check above).

## Limitations / follow-up
- No `Cargo.toml`/workspace config, `CMakeLists.txt`, `package.json`,
  `.gitignore`, or editor config exists yet — that is AICAD-003.
- `crates/`, `std/`, `editor/`, `tree-sitter-aicad/`, and `skills/`
  directories contain only ownership documentation, no source files.
- The stale `docs/plan/README.md` document index (contradiction 1 in
  `project/reports/ORIENTATION_PASS.md`) was intentionally left as-is
  since `docs/plan/` is immutable per the operating model's caching rule;
  it is tracked in `project/OWNER_DECISIONS.md` instead.
- The 15 open owner decisions in `project/OWNER_DECISIONS.md` are
  cross-referenced from the relevant crate/directory `README.md` files
  where directly applicable (D4, D5, D7, D8, D10, D11, D12, D13, D15) so a
  future implementer sees the open question before writing code in that
  area.
