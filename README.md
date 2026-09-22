# AICAD

AICAD is a programmable, source-first mechanical-engineering platform. Design intent is represented as typed, inspectable `.aicad` source and compiled into exact CAD geometry.

The current system combines typed engineering semantics, parametric/incremental computation, persistent fail-closed semantic topology references, advanced programmable geometry, a kernel-neutral geometry architecture, and an Open CASCADE Technology (OCCT) backend. Source and AICAD-owned semantic state are authoritative; B-rep and STEP files are derived products rather than the editable source of truth.

> **Status:** Stage 5 is complete, owner-approved, and merged. AICAD is pre-1.0 and its language/API surface is still evolving. The Stage-6 assembly/configuration queue is finalized for owner review, but Stage-6 implementation has **not** begun.

## What works today

Current AICAD includes:

- `.aicad` lexing, parsing, AST/HIR lowering, name binding, type checking, structured diagnostics, and bounded interpretation;
- dimension-aware engineering quantities and units;
- `param` declarations, derived expressions, dependency ordering, cycle diagnostics, and incremental rebuilds;
- typed RuntimeBuiltin calls for the supported Safe CAD catalogue;
- exact OCCT-backed B-rep operations including primitives, booleans, transforms, fillet/chamfer/shell, extrude/revolve, holes/pockets, mirror, and patterns;
- kernel-neutral spatial values including `Point3`, `Vector3`, `Direction`, `Axis3`, `Frame3`, and `Plane`;
- advanced analytic/freeform curve and surface representations and operations;
- trimmed geometry;
- intersection, projection, and distance query families;
- kernel-neutral topology construction plus bounded sewing/healing and deterministic topology inspection/traversal;
- controlled raw geometry, functional editing, validation, and raw-to-safe adoption;
- persistent `VertexRef` / `EdgeRef` / `WireRef` / `FaceRef` / `ShellRef` / `SolidRef` semantics above the kernel;
- source-declared persistent references using `query name : EntityKind in scope { ... }`;
- fail-closed reference outcomes: `Resolved`, `Ambiguous`, or `Broken`—ambiguity is never guessed through;
- feature/provenance plus topology-change lineage as resolver evidence while durable reference identity remains AICAD-owned;
- `cad refs check` reference-health inspection;
- epoch-bound raw topology handles and deliberately non-authoritative geometric-fingerprint evidence;
- exact geometry validation and STEP export/re-import paths;
- maintained executable examples covering getting started, parametrics, references, advanced geometry, and realistic models.

The sketch/entity/constraint/profile subsystem is implemented internal modeling substrate, but direct `.aicad` `sketch { ... }` authoring is not supported today. Some modeling functions still accept raw integer edge/face selectors; those remain topology-local selectors, not durable identity.

## Small model

```aicad
param width: Length = 60mm;
param depth: Length = 40mm;
param thickness: Length = 8mm;
param hole_diameter: Length = 6mm;

part MountingPlate {
    let base: Geometry = box(width, depth, thickness);
    let body: Geometry = hole(
        base,
        Axis3(
            origin = Point3(x = width / 2, y = depth / 2, z = 0mm - 1mm),
            direction = Vector3(x = 0.0, y = 0.0, z = 1.0),
        ),
        hole_diameter,
        thickness + 2mm,
    );
}
```

Build the named output as STEP:

```sh
cargo run -p cad-cli -- build plate.aicad \
  --output plate.step \
  --name MountingPlate.body
```

Declare a persistent semantic reference:

```aicad
query hole_wall : Face in MountingPlate.body {
    generated_by(body);
    cylindrical();
    unique();
}
```

Inspect source-declared reference outcomes:

```sh
cargo run -p cad-cli -- refs check plate.aicad
```

See [`examples/`](examples/) for the maintained executable example set.

## Architecture

```text
.aicad source
  -> parse / type / lower
  -> ParamModel + FeatureGraph + runtime execution
  -> backend-neutral Geometry IR
  -> geometry runtime / kernel-neutral API
  -> OCCT adapter
  -> exact B-rep

feature/provenance + kernel lineage evidence
  -> scoped persistent-reference resolver
  -> Resolved / Ambiguous / Broken
  -> reference health / diagnostics
```

Source semantics, HIR, public APIs, feature/provenance identity, and persistent reference recipes remain kernel-neutral. OCCT is isolated behind `cad-kernel-api`, `cad-occt-bridge`, and `native/occt_bridge`; kernel lineage contributes evidence but does not become durable semantic identity.

See [`docs/developer/architecture/`](docs/developer/architecture/) and [`docs/developer/parametrics/`](docs/developer/parametrics/).

## Getting started

Prerequisites include Rust **1.98.1**, CMake **3.16+**, a C++17-capable compiler, and an OCCT development installation discoverable by CMake.

```sh
git clone https://github.com/insightlabs38-pixel/AICAD.git
cd AICAD
cargo build --workspace
cargo test --workspace
```

Build a maintained example:

```sh
cargo run -p cad-cli -- build \
  examples/brackets/stage3_l_bracket.aicad \
  --output l-bracket.step \
  --name LBracket.body
```

More detail:

- [Installation and first run](docs/user/getting-started/)
- [User guide](docs/user/)
- [Modeling and advanced geometry](docs/user/modeling/)
- [Persistent references](docs/user/modeling/persistent-references.md)
- [CLI](docs/user/cli/)
- [Examples](examples/)
- [Developer guide](docs/developer/)
- [Current limitations](docs/user/current-limitations.md)
- [Contributing](CONTRIBUTING.md)

## Current limitations

AICAD is an evolving pre-1.0 engineering platform, not a drop-in replacement for every mature interactive CAD system. Important current boundaries include:

- assemblies, mates/joints, assembly solving, configurations, BOM, and assembly interference are **not implemented yet**; they are Stage-6 work;
- automatic fingerprint-based semantic-reference repair is intentionally not authoritative; fingerprints remain evidence only;
- source-declared references require explicit candidate scope and fail closed on ambiguity;
- direct source sketch authoring remains incomplete even though sketch/constraint substrate exists internally;
- some Safe CAD operations still expose raw topology-local integer selectors alongside the separate persistent-reference system;
- Bezier/B-spline curves and surfaces, plus trimmed surfaces, are supported as freeform semantic/runtime values but cannot yet be converted into real kernel topology through `make_edge` / `make_face_on_surface`; those paths fail explicitly with `UNSUPPORTED_TOPOLOGY_CONSTRUCTION` rather than silently degrading;
- `List<Geometry>` builtin parameters are currently invisible to `geometry_inputs` in `FeatureGraph` / `TraceFeatureGraph`, so incremental dirty-set propagation through that generic path is not claimed as proven behavior;
- kernel numerical/non-convergence coverage is bounded by tested operation classes; pathological cases outside that evidence can still fail;
- a full IDE/GUI, mature package/plugin ecosystem, and broad release-stability guarantees are not current claims.

See [`docs/user/current-limitations.md`](docs/user/current-limitations.md) for the categorized list.

## Roadmap

**PLANNED / NOT YET IMPLEMENTED:** Stage 6 establishes semantic assemblies and configurations: distinct definition/instance/occurrence identity, cross-instance semantic references, reusable mechanical interfaces, solver-neutral mates/joints, deterministic pose and DOF behavior, immutable configuration overlays, suppression/replacement, external assets, BOM/interference, realistic/adversarial coverage, and structured tooling.

Stage 7 remains provisional and focuses on verification, requirements/traceability, evidence strength, and realistic/adversarial/performance/AI gates. Provisional later-stage plans are not product commitments.

Internal planning and evidence live under [`project/`](project/); current product documentation lives under [`docs/user/`](docs/user/) and [`docs/developer/`](docs/developer/).

## Repository layout

| Path | Purpose |
| --- | --- |
| `crates/` | Compiler, runtime, geometry, parametric, reference, validation, and CLI crates |
| `native/occt_bridge/` | Narrow native OCCT implementation boundary |
| `specs/`, `rfcs/` | Accepted/current language and architecture contracts according to each file's status |
| `examples/` | Maintained current user-facing examples |
| `tests/`, `project/benchmarks/` | Integration, regression, stress, and benchmark fixtures |
| `docs/user/` | Current user guide |
| `docs/developer/` | Current architecture/testing/contributor guide |
| `project/` | Development governance, evidence, transitions, reports, and roadmap planning |
| `docs/plan/` | Frozen foundation planning retained for historical traceability |

## Contributing and security

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for contribution workflow and [`SECURITY.md`](SECURITY.md) for responsible vulnerability reporting guidance.

## License

AICAD is licensed under the [Apache License 2.0](LICENSE). OCCT is an external dependency with its own licensing requirements.
