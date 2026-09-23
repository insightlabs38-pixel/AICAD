# AICAD-135 — local/world frames and rigid instance poses

## Result

`cad-assemblies` now has typed instance poses reusing Stage-5's kernel-neutral `cad_kernel_api::Transform` for the actual rigid-motion math: `LocalPose` (an occurrence's transform relative to its immediate parent) and `WorldPose` (relative to the assembly root), composed/inverted deterministically. Every `ChildInstance` now carries its own `LocalPose`. Pose changes are structurally incapable of altering logical/occurrence identity, since neither `LogicalInstanceId` nor `OccurrencePath` gained a pose field.

## Changes

- `crates/cad-kernel-api/src/geometry.rs`: added `Transform::invert` (rotation transpose + un-rotated negated translation) — purely additive, no existing behavior changed.
- `frame.rs` (new): `LocalPose`/`WorldPose` newtypes wrapping `Transform`. `WorldPose::root`/`LocalPose::under` are the only ways to produce a `WorldPose`, so every world pose is traceably a fold of an explicit parent chain, never raw numbers (there is deliberately no `WorldPose::new`). `LocalPose::invert`/`WorldPose::invert` reuse `Transform::invert`.
- `component.rs`: `ChildInstance` gained a required `local_pose: LocalPose` field/constructor argument, placed between `instance` and `arguments`.
- `ChildInstance`/`LocalPose` both got `to_json()` (the pose serializes as its row-major 3x4 matrix).

## Verification

- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — PASS
- `cargo test -p cad-assemblies` — 62/62 PASS (46 unit incl. 5 new `frame::tests`, `component_ir.rs` 4/4, `identity_domains.rs` 6/6, new `instance_pose.rs` 2/2: nested-frame composition through a real `ComponentDefinitionRegistry`/`OccurrencePath` chain, and moving a root pose changing the resolved world pose while leaving the occurrence path unchanged)
- `cargo test -p cad-kernel-api` — 38/38 PASS (4 new `Transform::invert` round-trip tests: identity, translation, off-origin rotation, composed frame-to-frame transform)
- `cargo test --workspace` — PASS

## Limitations

- No tree-walking pose resolution over an entire nested assembly yet — `WorldPose` composition is exercised by hand-walking one chain in tests; `AICAD-136`'s occurrence-tree expansion is expected to do this generically.
- Frame conventions (local/parent/world) are established structurally by these two types; no named "semantic interface frame" concept exists yet (`AICAD-138`).

## Next

`AICAD-136` (nested-assembly dependency graph and cycle diagnostics, batch S6-02) depends on this task and `AICAD-134`.
