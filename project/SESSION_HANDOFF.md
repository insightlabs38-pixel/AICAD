# Session Handoff

## Canonical branch and HEAD

`origin/claude/aicad-stage3-dev`. Commits on this branch since Stage-2
closed, in order (unchanged history through `72328ef` omitted — see prior
handoff revisions in git history for that detail; this handoff lists only
what changed this invocation and the current tip):

- `41283eb` — `AICAD-075A`: `cad_kernel_api::Plane3`, `cad_hir::
  geometry_types::Plane`, `cad_runtime::spatial` (the `Value ->
  cad_kernel_api` conversion boundary), and a geometry-backed integration
  test suite proving revolve/circular-pattern/mirror-plane/general-
  transform preparation. See `project/reports/AICAD-075A.md`.
- `8bf1a7c` — `AICAD-076`: `extrude`/`revolve`/`hole`/`pocket`
  `RuntimeBuiltin`s, `GeometryOp::GetFace`, `Frame3::from_z`, and a real
  architecture finding recorded as `project/OWNER_DECISIONS.md#D20` (a
  `Named` `cad_hir::geometry_types` struct broke every other program's
  type-checking when referenced in a builtin signature) — shipped with a
  temporary scalar-decomposed workaround pending an owner ruling.
- (this invocation's commit) — `AICAD-076A`: implements the owner's `D20`
  ruling (`project/DECISION_LOG.md#DL-21`, "Accepted"). `crate::lower::
  lower_program` now seeds `cad_hir::geometry_types`'s standard struct
  declarations unconditionally, alongside the builtin catalogue itself
  (`Lowerer::seed_standard_types`), making the always-seeded builtin
  environment type-closed — no `with_geometry_types` composition
  required. Migrated `revolve`/`hole`/`pocket` back to real `Axis3`/
  `Frame3`-typed signatures. Added the owner-requested catalogue-wide
  zero-diagnostics invariant test. Batch S3-06 is now complete.

All commits through this invocation's own are pushed to `origin/
claude/aicad-stage3-dev` (verified — see "Push verification" below).

## Active stage / current batch

Stage 3, `status: active` (`project/CURRENT_STAGE.md`). **Batch S3-06 is
now COMPLETE** (`AICAD-075A`/`AICAD-076`/`AICAD-076A` all `status: done`
in `project/TASKS.yaml`; `AICAD-076A` was inserted into this batch by
owner ruling `project/DECISION_LOG.md#DL-21` after `AICAD-076` found the
architecture gap `DL-21` resolves — see `project/CURRENT_STAGE.md`'s own
updated fixed batch list). No checkpoint is required between S3-06 and
S3-07 — `project/gates/STAGE3-C_MODELING.md` is prepared only after
Batches S3-06/S3-07/S3-08 all complete.

Per explicit owner instruction accompanying the `D20` ruling ("Keep this
as a short additional task. Stop here instead of continuing on to
AICAD-077"), this invocation stops at the end of `AICAD-076A`, even
though Batch S3-06 is now complete and the campaign brief would otherwise
call for continuing into Batch S3-07 in the same invocation.

## Last completed task

`AICAD-076A` ("Make the RuntimeBuiltin catalogue's standard type
environment type-closed"), interstitial task inserted into Batch S3-06.
See `project/reports/AICAD-076A.md` for full detail; summary:

- New `Lowerer::seed_standard_types` (`crates/cad-hir/src/lower.rs`),
  called from `lower_program` unconditionally: seeds `cad_hir::
  geometry_types::GEOMETRY_TYPES_SOURCE`'s struct declarations
  (`Point2`/`Point3`/`Vector2<T>`/`Vector3<T>`/`Axis3`/`Frame3`/`Plane`)
  into every compiled program's global scope, skipping any name the
  caller's own program already declares (idempotence with
  `with_geometry_types`, which remains available as an optional,
  backward-compatible composition helper).
- Migrated `revolve`/`hole`/`pocket` (`AICAD-076`) from their temporary
  scalar-decomposed signatures back to real `Axis3`/`Frame3`-typed ones
  (`cad_runtime::spatial::axis3_from_value`/`frame3_from_value`,
  `AICAD-075A`), restoring the arbitrary-origin capability the workaround
  had dropped. `extrude`'s `direction: Vector3<Float>` was unaffected
  either way (a `Generic` type reference was never part of the original
  problem).
- New catalogue-wide invariant test (`cad-hir`, the owner's own explicit
  request): the entire `BuiltinFnId` catalogue type-checks against an
  otherwise-empty program with zero diagnostics — catches a future bad
  signature directly, rather than via an unrelated test suite exploding
  (`AICAD-076`'s own 149-test regression during development).
- `project/OWNER_DECISIONS.md#D20` marked RESOLVED; `project/
  DECISION_LOG.md#DL-21` records the full ruling and rationale.

Test counts (delta from `AICAD-076`'s own baseline): `cad-hir` 226 -> 227
(+1), `cad-runtime` 126 -> 127 (+1). Every other crate's count is
unchanged from `AICAD-076`'s own last-verified totals.

## Partial task

None. `AICAD-076A` completed cleanly in this invocation.

## Next task

Batch S3-07 (`AICAD-077`, then `AICAD-078`). `AICAD-077` ("Implement
mirror and linear/circular pattern basics") now depends on `AICAD-076A`
(satisfied), not `AICAD-076` directly. **Before starting, read
`project/DECISION_LOG.md#DL-21` and `project/reports/AICAD-076A.md`'s own
"Next dependency" section**: `mirror(target, plane: Plane)` and
`radial_pattern`'s own axis parameter should use real struct-typed
parameters from the start (`plane3_from_value`/`axis3_from_value`, both
already available from `AICAD-075A`) — the type-closed standard
environment `AICAD-076A` established makes this safe now, with no
scalar-decomposition workaround needed (do not repeat `AICAD-076`'s own
temporary pattern; that was explicitly a stopgap, not the long-term
convention). `AICAD-077` is also the task that must add mirror's own
`Plane3`-based native/kernel-adapter operation — mirror cannot be
expressed as a `Transform` (`AICAD-075A`'s own "Rigidity (no reflection)"
finding: `Transform` is proper-rigid-only, determinant `+1`), and no
mirror kernel capability exists yet at any layer (confirmed by
`AICAD-075A`'s own audit).

## Exact recent test status

- `cargo fmt --all -- --check` → clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings, full workspace (29 crates).
- `cargo test --workspace` → 978 passed, 0 failed across every crate.
- `cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1` →
  3/3 (Stage-2 gate proof unaffected).

## Regressions/failures

None outstanding. `AICAD-076`'s own transient 149-test regression (from
its first draft, during a prior invocation) was never committed/pushed
and is fully resolved by this task — the pre-existing edge case that
regression risked recurring on (a user program declaring its own
unrelated `struct Point2`) was checked directly this invocation and
passes unmodified.

## Unresolved owner decisions

`D7`/`D8`/`D12`/`D15` remain open/partially-resolved, none blocking
through at least S3-08 (unchanged from prior batches). **`D20` is now
RESOLVED** (`DL-21`, this invocation) — no longer an open item.

## D5/D19 calibration status

Unchanged — complete (`project/OWNER_DECISIONS.md#D19`,
`DECISION_LOG.md#DL-17`, `project/reports/AICAD-064A.md`). Not touched by
this task.

## Current checkpoint status

`project/gates/STAGE3-B_SKETCH_CONSTRAINTS.md` (Batches S3-00-S3-05)
remains the most recent checkpoint (PASS). The next checkpoint,
`project/gates/STAGE3-C_MODELING.md`, is created after Batches
S3-06/S3-07/S3-08 all complete — Batch S3-06 finishing here does not by
itself trigger it; S3-07 (`AICAD-077`/`AICAD-078`) and S3-08 (`AICAD-079`)
still remain.

## Environment

Rust 1.98.1 (edition 2024), container Linux environment. No new
third-party dependency was added this invocation, and no `Cargo.toml` in
any crate changed.

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

Per explicit owner instruction, this invocation stops here rather than
continuing into Batch S3-07. The next invocation starts Batch S3-07
(`AICAD-077`, then `AICAD-078`) — read `project/DECISION_LOG.md#DL-21`
and `project/reports/AICAD-076A.md` first, per "Next task" above. Do not
re-open Batch S3-00 through S3-06's own already-complete tasks.
