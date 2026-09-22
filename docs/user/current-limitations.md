# Current Limitations and Maturity

AICAD is pre-1.0. The supported surface is substantial but intentionally bounded; current documentation should not be read as a claim of parity with mature interactive CAD systems.

## Unsupported today

- Semantic assemblies, mates/joints, assembly solving, configurations, suppression/replacement, BOM, and assembly interference. These are Stage-6 work and are not implemented yet.
- A full interactive CAD IDE/GUI.
- A mature package/plugin ecosystem.
- Automatic authoritative semantic-reference repair from geometric fingerprints.
- Direct `.aicad` sketch authoring as a complete public workflow; sketch/entity/constraint/profile machinery exists as internal modeling substrate.

## Intentionally deferred or bounded

- Some Safe CAD APIs still use topology-local integer face/edge selectors. Persistent semantic references are a separate durability layer; integer selectors are not durable identity.
- Source-declared semantic references require an explicit candidate scope and fail closed when resolution is ambiguous or broken.
- Generic freeform `List<Geometry>` production is not treated as a general source-native B-rep construction path.
- First-class dependency invalidation through arbitrary generic `List<Geometry>` flows is not claimed as independently proven behavior.
- Some broader language/native ABI capabilities remain bounded to the forms required by current modeling stages.

## Kernel / numerical limitations

- Exact B-rep behavior ultimately depends on bounded OCCT operations behind AICAD's kernel-neutral contract. Numerical or non-convergence failures remain possible outside the operation classes and adversarial cases covered by current evidence.
- Tolerance domains are explicit; there is no claim that one global epsilon can make every geometric operation robust.
- Raw geometry requires validation/adoption evidence before becoming safe authoritative geometry.

## Product usability limitations

- Installation currently expects a Rust toolchain, CMake/C++ build environment, and an OCCT development installation discoverable by CMake.
- The source language/API can change before 1.0; current examples and docs track the supported surface, but broad backward-compatibility guarantees are not yet a project claim.
- Several advanced raw/lineage capabilities are represented primarily by automated integration tests rather than standalone teaching examples because they require a live kernel context.

## Planned next-stage capability

Stage 6 is planned to add semantic assemblies/configurations while preserving distinct identity domains, solver-neutral mate/joint semantics, deterministic observable pose/DOF/conflict behavior, immutable configuration overlays, content/provenance identity for external assets, BOM/interference outputs, and structured assembly tooling.

Planning is not an implementation claim. See the root [README](../../README.md) for roadmap context.
