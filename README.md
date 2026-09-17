# AICAD

AICAD is a programmable, source-first mechanical-engineering environment. Design intent is represented as typed, inspectable `.aicad` source and compiled into exact CAD geometry.

The current system combines typed engineering semantics, a parametric/incremental dependency model, persistent fail-closed semantic topology references, a kernel-neutral geometry pipeline, and an Open CASCADE Technology (OCCT) backend. Source and AICAD-owned semantic state are authoritative; B-rep and STEP files are derived products rather than the editable source of truth.

> **Status:** Stage 4 is complete and owner-approved. Persistent semantic references are implemented through the production build path, including source-declared `query` references and `cad refs check`. The repository is currently in the Stage-4 -> Stage-5 transition; the Stage-5 queue is finalized for owner review, but Stage-5 implementation has not begun. AICAD is pre-1.0.

## What works today

Current AICAD includes:

- `.aicad` lexing, parsing, AST/HIR lowering, name binding, type checking, structured diagnostics, and bounded interpretation;
- dimension-aware engineering quantities and units;
- `param` declarations, derived expressions, dependency ordering, and cycle diagnostics;
- ordinary typed RuntimeBuiltin calls for the closed Safe CAD catalogue;
- exact OCCT-backed B-rep operations including boxes/cylinders, booleans, translation, fillet/chamfer/shell, extrude/revolve, hole/pocket, mirror, and linear/radial patterns;
- kernel-neutral spatial values including `Point3`, `Vector3`, `Axis3`, `Frame3`, and `Plane`;
- `part` bodies with named outputs and `--name <binding>[.<field>]` export selection;
- `ParamModel` + `FeatureGraph` incremental rebuilds with dirty propagation and reuse of unaffected realized geometry;
- persistent `VertexRef` / `EdgeRef` / `WireRef` / `FaceRef` / `ShellRef` / `SolidRef` semantics above the kernel;
- source-declared persistent references using `query name : EntityKind in scope { ... }`;
- fail-closed reference outcomes: `Resolved`, `Ambiguous`, or `Broken`—ambiguity is never guessed through;
- feature/provenance lineage as resolver evidence while durable reference identity remains AICAD-owned;
- `cad refs check` reference-health inspection;
- epoch-bound raw topology handles and deliberately non-authoritative geometric-fingerprint evidence;
- exact geometry validation and STEP export/re-import paths.

The sketch/entity/constraint/profile subsystem is implemented internal modeling substrate, but direct `.aicad` `sketch { ... }` authoring is not supported today. Several modeling functions still accept raw integer edge/face selectors; those remain topology-local selectors, not durable identity.

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

A persistent semantic reference can be declared in source:

```aicad
query hole_wall : Face in MountingPlate.body {
    generated_by(body);
    cylindrical();
    unique();
}
```

Inspect all source-declared reference outcomes with:

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

- [User guide](docs/user/)
- [Developer guide](docs/developer/)
- [Examples](examples/)
- [CLI](docs/user/cli/)
- [Persistent references](docs/user/modeling/persistent-references.md)

## Current limitations

The current system deliberately does **not** claim:

- automatic fingerprint-based reference repair; fingerprints remain evidence only;
- broad unscoped resolution as a safe authoring default; source-declared references require explicit candidate scope;
- complete source syntax for every already-implemented Rust-level Stage-4 predicate;
- complete nested-`part` execution semantics or Area-dimension source construction;
- Stage-5 advanced curves/surfaces, intersection/projection/distance families, general topology construction/healing, or controlled raw-geometry editing/adoption;
- assemblies/configurations, the later verification framework, a full IDE/GUI, or a package/plugin system.

## Roadmap

**PLANNED / NOT YET IMPLEMENTED:** Stage 5 starts with the remaining language/query completeness work, then adds runtime/query/tolerance foundations; kernel-neutral advanced geometry representations; curves and surfaces; intersection/projection/distance; general topology construction/healing/inspection; controlled raw geometry and raw-to-safe adoption; advanced lineage/reference integrity; and realistic/adversarial hardening.

Assemblies/configurations and verification-first engineering remain later-stage work. Provisional later-stage plans are not product commitments.

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

## License

AICAD is licensed under the [Apache License 2.0](LICENSE). OCCT is an external dependency with its own licensing requirements.
