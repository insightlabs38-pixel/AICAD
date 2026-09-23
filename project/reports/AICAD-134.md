# AICAD-134 — component-definition/logical-instance semantic IR

## Result

`cad-assemblies` now has a real assembly semantic IR built on `AICAD-133`'s identity primitives: `ComponentDefinition`, `ChildInstance`, `ParameterDeclaration`, and `ComponentDefinitionRegistry`. Definitions and instances are distinct types; a definition referenced by many children is stored once and shared, never cloned; no realized geometry/topology field exists anywhere in the IR.

## Changes

- `component.rs`: `ComponentDefinition { id, parameters, children: Vec<ChildInstance> }`; `ChildInstance { instance: LogicalInstanceId, arguments: Vec<(String, ParameterValue)> }` (no pose field — `AICAD-135` adds that); `ParameterDeclaration { name, ty: OperandType, default }`.
- `ComponentDefinitionRegistry`: `BTreeMap<ComponentDefinitionId, ComponentDefinition>` (never `HashMap`, so iteration order is `Ord`-deterministic, not insertion/hash-dependent). `define()` accepts an identical re-declaration as a no-op, rejects a conflicting one under the same id.
- `value.rs`: `ParameterValue { magnitude: f64, ty: cad_units::OperandType }` — a small local duplicate of `cad_geometry_api::ir::Quantity`'s shape rather than a dependency on that crate (which additionally pulls in `cad-ast`), so parameter arguments stay typed engineering quantities rather than bare `f64`.
- `Cargo.toml`: added `cad-units`/`cad-types` dependencies (both dependency-light; no `cad-ast`/kernel coupling).
- `children`/`arguments` are plain `Vec`s throughout — no container whose iteration order could leak into semantics.

## Verification

- `cargo fmt --all -- --check` — PASS
- `cargo clippy -p cad-assemblies --all-targets --all-features -- -D warnings` — PASS
- `cargo test -p cad-assemblies` — 50/50 PASS (40 unit + 4 new `component_ir.rs` integration + 6 pre-existing `identity_domains.rs`)
- `cargo build --workspace` — PASS

## Limitations

- No graph resolution/reuse-across-definitions validation or cycle detection yet (`AICAD-136`). `ComponentDefinitionRegistry` accepts a `ChildInstance` referencing an undefined `ComponentDefinitionId` at `define()` time; resolving/validating the full nested tree is `AICAD-136`'s job.
- No pose/frame on `ChildInstance` yet (`AICAD-135`).
- Parameter arguments are looked up by name only, with no cross-check against the target definition's own `ParameterDeclaration`s (no interpreter/typechecker integration exists at this IR layer yet) — out of this task's scope.

## Next

`AICAD-135` (local/world frames and rigid instance poses, batch S6-02) depends on this task.
