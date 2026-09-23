# AICAD-140 — solver-neutral joint and coordinate IR

## Result

`cad-assemblies::joint` adds `Joint`/`JointKind`/`JointCoordinate` as AICAD-owned typed relations over an approved, bounded joint family, with explicit per-axis dimension/limit/home-position checking and zero solver dependency.

## Changes

- `joint.rs` (new): `JointId` (same construction precedent as `MateId`); `JointAxis { dimension, min, max, home }` (typed limits + required home position, validated at construction — dimension-consistency, `min <= max`, `home` within `[min, max]`); `JointKind::{Fixed, Revolute, Prismatic, Cylindrical, Planar}`, each smart-constructor (`JointKind::revolute`, etc.) enforcing the family's required per-axis `Dimension`; `JointCoordinate` (the candidate-value shape matching each family); `Joint { id, kind, parent, child }` over `OccurrenceTopologyRef` subjects.
- `Joint::validate_coordinate` checks a candidate `JointCoordinate` against the joint's own family and limits with no `Transform`/pose computation at all — mapping a coordinate to a resulting pose is kinematic evaluation (`AICAD-147`), layered on top of this module's typed shape, not this task's job.
- `JointError` (`ASM-E007`, continuing the reserved `assembly` family after `AICAD-139`'s `ASM-E006`) covers axis-dimension mismatch, `min > max`, out-of-range home, wrong coordinate family, and out-of-limit coordinate — every case fails closed rather than clamping/guessing.
- **Approved baseline family is deliberately bounded to five of `docs/plan/07`'s eight joint types** — `Fixed`, `Revolute`, `Prismatic`, `Cylindrical`, `Planar` — each exactly expressible with `cad-units`' existing 21 named dimensions under one unambiguous scalar-per-axis convention. Three are explicitly *not yet* implemented, each for a disclosed, concrete reason (documented in the module's own doc comment, not silently dropped): `helical` needs a `Length`-per-`Angle` lead/pitch quantity, which is not one of the 21 RFC-0004-governed named dimensions (adding one is a unit-semantics change, `AGENTS.md`'s own escalation trigger); `spherical`/`universal` have 2-3 rotational DOF with no single unambiguous scalar-per-DOF convention (Euler-angle axis-order/gimbal-lock choices — `AGENTS.md`'s "ambiguity is an error, never an arbitrary selection"); `custom` (arbitrary equations) is the general dynamics/constraint-solving scope `AGENTS.md`'s "NO SPECULATIVE FUTURE WORK" excludes. None of this weakens `AICAD-140`'s own acceptance criteria, which asks for "approved joint families," not exhaustive coverage of the frozen plan's illustrative table.

## Verification

- `cargo fmt --all -- --check` — PASS
- `cargo clippy -p cad-assemblies --all-targets --all-features -- -D warnings` — PASS
- `cargo test -p cad-assemblies` — PASS (16 new `joint::tests`)
- `cargo test --workspace` — PASS, no regressions

## Limitations

- Helical/spherical/universal/custom joint families are out of scope for the reasons above — extending the set later is ordinary additive IR work, not a redesign of what exists.
- No `.aicad` source syntax/compiler wiring exists yet.

## Next

`AICAD-141` (Checkpoint A) depends on `AICAD-137`..`AICAD-140` and is the next task, closing batch `S6-04`.
