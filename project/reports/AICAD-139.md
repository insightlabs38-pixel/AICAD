# AICAD-139 — solver-neutral mate relation IR

## Result

`cad-assemblies::mate` adds `Mate` as an AICAD-owned typed semantic relation between exactly two occurrence subjects, inspectable and fully constructible with zero solver dependency anywhere in the crate.

## Changes

- `mate.rs` (new): `MateId` (stable declared name, same construction precedent as `ComponentDefinitionId`); `MateKind` (`Coincident | Concentric | Parallel | Perpendicular | Tangent | Distance | Angle | Lock` — `docs/plan/07_ASSEMBLIES_KINEMATICS_CONFIGURATIONS.md` §5's baseline table minus its three joint-coupling entries `gear_ratio`/`rack_pinion`/`screw`, which couple two *joint* coordinates rather than two occurrence subjects and are left for a later relation shape, not silently dropped); `Mate { id, kind, subjects: [OccurrenceTopologyRef; 2], parameter: Option<ParameterValue> }`.
- `Mate::new` fail-closed validates `kind`'s own declared parameter shape: `Distance`/`Angle` require a `Length`/`Angle`-dimensioned parameter respectively, every other kind requires none — `MateError::{MissingParameter, UnexpectedParameter, WrongParameterDimension}`, rendered as `ASM-E006` (continuing the reserved `assembly` diagnostic family after `AICAD-138`'s `ASM-E005`).
- `ParameterValue::dimension() -> Option<Dimension>` added to `value.rs` — a small shared accessor both `mate.rs` and `joint.rs` (`AICAD-140`) use instead of re-matching `OperandType` independently.
- Subjects are `OccurrenceTopologyRef` (`AICAD-137`'s own D26 topology-reference domain), so two instances of the same component definition never collapse to the same mate subject — proven by a dedicated test.

## Verification

- `cargo fmt --all -- --check` — PASS
- `cargo clippy -p cad-assemblies --all-targets --all-features -- -D warnings` — PASS
- `cargo test -p cad-assemblies` — PASS (11 new `mate::tests`, plus 1 new `value::tests` case)
- `cargo test --workspace` — PASS, no regressions

## Limitations

- No geometric validation of subject compatibility (e.g. that a `Concentric` mate's subjects are actually axis-like) — that evidence only exists once a resolver/solver runs, which this module has no dependency on by design.
- No `.aicad` source syntax/compiler wiring exists yet, consistent with every other Stage-6 IR module to date.

## Next

`AICAD-140` (solver-neutral joint/coordinate IR) depends on this and is the next task in batch `S6-04`.
