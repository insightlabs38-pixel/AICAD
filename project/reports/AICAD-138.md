# AICAD-138 — reusable mechanical-interface semantics

## Result

`cad-assemblies::interface` adds mechanical interfaces as ordinary Rust-level semantic values, D27-shaped (a named closed list of required typed fields, explicit nominal conformance) but built from Stage-6's own engineering value kinds — frames, cross-instance references, typed parameters — with static conformance and value-level compatibility kept as two distinct, separately callable checks.

## Changes

- `interface.rs` (new): `MechanicalInterface` (named contract of `InterfaceField { name, ty: InterfaceFieldType }`, where `InterfaceFieldType` is `Frame | Reference | Parameter(OperandType)`); `MechanicalInterfaceInstance` (declared interface name + named `InterfaceValue` bindings, mirroring `struct implements Interface` conformance).
- `check_conformance(instance, interface)` — purely structural: same declared interface name, every declared field present under the right name with the right field kind (and, for `Parameter`, the right dimension). Never inspects a bound value's magnitude, reference kind, or pose.
- `check_compatibility(left, left_field, right, right_field)` — a separate, later, value-level check between two named field bindings: `Reference` fields must resolve the same `cad_references::EntityKind`; `Parameter` fields must match exactly; `Frame` fields are compatible if both sides bind a frame (pose/orientation alignment itself is `AICAD-139`+'s mate-solving concern, not this task's). The caller names the field pair explicitly — this function never guesses a correspondence between two instances' own field names.
- `ConformanceError`/`CompatibilityError` each render a structured diagnostic (`ASM-E004`/`ASM-E005`, continuing the reserved `assembly` family after `AICAD-136`'s `ASM-E001`/`E002` and `AICAD-137`'s `ASM-E003`).
- Deliberately does not depend on `cad-hir`'s compiler-internal `Item::Interface`/typeck pipeline — that machinery is `.aicad`-source-parsing/type-checking plumbing this identity/IR crate has no other reason to pull in; the D27 shape is mirrored as plain data instead, matching `crate::value`'s own "small independent duplication beats a heavy cross-crate coupling" precedent. No assembly-only compiler intrinsic was introduced anywhere in the language/compiler crates.

## Verification

- `cargo fmt --all -- --check` — PASS
- `cargo clippy -p cad-assemblies --all-targets --all-features -- -D warnings` — PASS
- `cargo test -p cad-assemblies` — 90/90 PASS (72 unit incl. 11 new `interface::tests`, plus the four existing integration suites unchanged)
- `cargo build --workspace` / `cargo test --workspace` — PASS, no regressions

## Limitations

- Frame-field compatibility is type-level only (both sides bind a frame); no geometric alignment/orientation check exists here — by design, since real pose-alignment semantics belong to the mate/joint IR (`AICAD-139`/`140`) and the deterministic-pose policy (`AICAD-143`), not to a standalone interface-compatibility check.
- `MechanicalInterface`/`MechanicalInterfaceInstance` are pure Rust values with no `.aicad` source syntax or compiler wiring — consistent with every other Stage-6 IR module to date (no assembly grammar exists yet); a future task that exposes mechanical interfaces to `.aicad` source is new scope.

## Next

Batch `S6-03` (`AICAD-137`, `AICAD-138`) is complete. `AICAD-139` (solver-neutral mate relation IR, batch `S6-04`) depends on both and is the next executable task.
