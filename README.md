# AICAD

AICAD is a programmable mechanical-engineering environment in which design intent is represented as typed, inspectable source code and compiled into exact CAD geometry.

The project combines a general-purpose engineering language, a parametric dependency model, a backend-neutral geometry pipeline, and an Open CASCADE Technology (OCCT) kernel adapter. Humans and automated tools operate on the same source and semantic model; generated B-rep and STEP output are products of that model rather than the source of design truth.

> **Status:** Stage 3 is complete and owner-approved. The parametric single-part CAD foundation is implemented. Stage 4—durable semantic/topological references—is next, but Stage-4 implementation has not started. AICAD is pre-1.0 and under active development.

## Why AICAD

Most CAD systems make the editable model primarily a GUI-owned feature tree. AICAD instead makes engineering intent explicit in source:

- **Typed design intent.** Dimensions and physical units participate in type checking instead of being untyped numeric conventions.
- **Exact geometry.** Source-level operations lower through a backend-neutral Geometry IR and are realized as exact OCCT-backed B-rep geometry.
- **Inspectable semantics.** Parameters, bindings, feature dependencies, source provenance, diagnostics, and named outputs are represented explicitly rather than hidden inside kernel objects.
- **Reproducible execution.** AICAD-owned state follows deterministic language/compiler rules; kernel results are validated by semantic and numerical equivalence rather than assumed byte-identical.
- **One model for humans and automation.** Safe CAD operations use ordinary typed function-call semantics. Automation does not need a privileged geometry language that bypasses the compiler/runtime model.

## What works today

The post-Stage-3 repository implements:

- `.aicad` lexing, parsing, AST/HIR lowering, name binding, type checking, structured diagnostics, and bounded interpretation;
- primitive and generic language constructs used by current examples, including functions, structs/enums, lists/ranges, conditionals, loops, and pattern matching;
- engineering quantities and physical units with dimension-aware arithmetic and conversion;
- top-level `param` declarations, derived parameter expressions, dependency ordering, and cycle diagnostics;
- a closed RuntimeBuiltin Safe CAD catalogue invoked through ordinary typed calls—not compiler geometry intrinsics;
- exact OCCT-backed geometry for boxes/cylinders, booleans, translation, fillet/chamfer/shell, extrude/revolve, hole/pocket, mirror, and linear/radial patterns;
- kernel-neutral spatial values including `Point3`, `Vector3`, `Axis3`, `Frame3`, and `Plane`;
- `part` bodies with named outputs and CLI selection through `--name <binding>[.<field>]`;
- a feature DAG with source provenance, structural cache keys, parameter dependencies, dirty propagation, and deterministic affected-feature calculation;
- a production in-process parametric rebuild session that connects `ParamModel` → `FeatureGraph` → dirty propagation → selective Geometry-IR redispatch and reuse;
- an **internal** solver-independent sketch/entity/constraint model, a concrete relaxation solver, and solved closed-profile lowering to exact kernel faces; this is implemented modeling substrate, not public `.aicad` sketch syntax;
- exact B-rep validation/property queries in the geometry stack and STEP export/re-import verification paths;
- `cad build` with human-readable or JSON diagnostics.

Two boundaries matter when evaluating that list. First, Stage-3 named outputs, feature identity/provenance, and raw face/edge selectors are **not** the persistent `VertexRef` / `EdgeRef` / `WireRef` / `FaceRef` / `ShellRef` / `SolidRef` topology-reference capability planned for Stage 4. Second, the sketch/constraint/profile subsystem is implemented and geometry-backed, but `.aicad` does **not** yet expose a supported `sketch { ... }` authoring construct; current source-level part examples use the Safe CAD builtin surface.

## Small example

This is a reduced form of the tested Stage-3 part style used by the repository examples:

```aicad
param width: Length = 60mm;
param depth: Length = 40mm;
param thickness: Length = 8mm;
param hole_diameter: Length = 6mm;

const OVERSHOOT: Length = 1mm;

part MountingPlate {
    let base: Geometry = box(width, depth, thickness);
    let body: Geometry = hole(
        base,
        Axis3(
            origin = Point3(x = width / 2, y = depth / 2, z = 0mm - OVERSHOOT),
            direction = Vector3(x = 0.0, y = 0.0, z = 1.0),
        ),
        hole_diameter,
        thickness + OVERSHOOT * 2,
    );
}
```

Build the named `body` output as STEP with:

```sh
cargo run -p cad-cli -- build path/to/plate.aicad \
  --output plate.step \
  --name MountingPlate.body
```

For larger, repository-tested examples, see [`examples/brackets/stage3_l_bracket.aicad`](examples/brackets/stage3_l_bracket.aicad) and [`examples/plates/stage3_bearing_mount.aicad`](examples/plates/stage3_bearing_mount.aicad).

## Architecture

```text
.aicad source
     │
     ▼
lexer / parser / AST
     │
     ▼
HIR + binding + type/unit checking
     │
     ├──────────────► ParamModel / FeatureGraph
     │                    │
     ▼                    │ dirty propagation
bounded runtime ◄─────────┘
     │
     ▼
backend-neutral Geometry IR
     │
     ▼
geometry runtime / dispatcher
     │
     ▼
kernel-neutral API
     │
     ▼
OCCT adapter + narrow C++ bridge
     │
     ▼
exact B-rep / validation / STEP
```

AICAD source and AICAD-owned semantic state remain authoritative above the kernel. Public language types, HIR, feature identities, and Geometry IR do not expose OCCT classes. OCCT is isolated behind `cad-kernel-api` / `cad-occt-bridge` / `native/occt_bridge` and is responsible for exact geometric realization, not language-level identity.

See [`docs/developer/architecture/`](docs/developer/architecture/) for the layer contracts and [`docs/developer/parametrics/`](docs/developer/parametrics/) for the Stage-3 incremental path.

## Repository layout

| Path | Purpose |
| --- | --- |
| `crates/` | Rust compiler, runtime, geometry, parametric, validation, and CLI crates |
| `native/occt_bridge/` | Narrow C++/C ABI bridge to OCCT; the only native OCCT implementation boundary |
| `specs/` | Canonical language/schema material according to each file's status |
| `rfcs/` | Accepted/proposed architecture and language RFCs |
| `examples/` | Current and future example families; use Stage-2/3 `.aicad` files for implemented source syntax |
| `tests/`, `benchmarks/` | Integration, regression, acceptance, and future-stage benchmark fixtures |
| `docs/user/` | Current user guide: how to write/build Stage-3 AICAD |
| `docs/developer/` | Current implementation architecture, testing, and contributor guide |
| `project/` | Development governance, task state, owner decisions, archived evidence, gates, transitions, and roadmap planning |
| `docs/plan/` | Frozen foundation planning retained for historical traceability; not the current user/developer manual |

## Getting started

### Prerequisites

- Rust **1.98.1** (the repository's `rust-toolchain.toml` selects it, including `rustfmt` and Clippy);
- CMake **3.16+** and a C++17-capable compiler;
- an OCCT development installation discoverable by CMake as `OpenCASCADE` and containing the modules required by `native/occt_bridge/CMakeLists.txt`.

On Debian/Ubuntu, the native bridge documents the base packages as:

```sh
sudo apt install cmake build-essential \
  libocct-foundation-dev \
  libocct-modeling-data-dev \
  libocct-modeling-algorithms-dev
```

Depending on distribution packaging, the additional OCCT modeling/data-exchange modules required by the full bridge may be split into additional `libocct-*-dev` packages. CMake reports the specific missing module when discovery is incomplete.

### Build and test

```sh
cargo build --workspace
cargo test --workspace
```

The Rust `cad-occt-bridge` build script configures and builds `native/occt_bridge` automatically, so an OCCT installation is required even when invoking Cargo from the workspace root.

### Build an AICAD part

```sh
cargo run -p cad-cli -- build \
  examples/brackets/stage3_l_bracket.aicad \
  --output l-bracket.step \
  --name LBracket.body
```

Use `--json` for structured diagnostics/output metadata. If `--output` is omitted, the source still runs through parsing, lowering, type checking, and execution but no STEP artifact is requested.

More detail: [`docs/user/getting-started/`](docs/user/getting-started/) and [`docs/user/cli/`](docs/user/cli/).

## Documentation

- **Using AICAD:** [`docs/user/`](docs/user/)
- **Architecture and contributing:** [`docs/developer/`](docs/developer/)
- **Safe CAD developer reference:** [`docs/developer/geometry/safe-cad-api.md`](docs/developer/geometry/safe-cad-api.md)

`project/` is intentionally separate from the manual. It contains development state and historical evidence—useful for audits and design archaeology, but not required to learn the current product surface.

## Roadmap

The following are **planned**, not implemented claims:

- **Stage 4 — semantic references:** durable, fail-closed topology/reference resolution and the hard naming benchmark.
- **Stage 5 — advanced geometry:** broader analytic/freeform geometry and controlled low-level topology operations.
- **Stage 6 — assemblies/configurations:** component identity, mates/joints, kinematics, configurations, BOM/interference foundations.
- **Stage 7 — verification-first engineering:** requirements/tests/evidence across geometry and assemblies.
- **Later:** native artifacts/interchange hardening, source-first engineering tooling, AI-native tools, packages/plugins, engineering-domain modules, collaboration, and scale work.

Future architecture is maintained as internal planning under [`project/planning/`](project/planning/); it is not current API documentation.

## Development and contributing

Start with [`docs/developer/contributing/`](docs/developer/contributing/) and [`AGENTS.md`](AGENTS.md). Architectural changes must respect the existing layer boundaries, accepted RFCs/owner decisions, stage gates, and the repository's evidence-first verification policy.

Stage-0 through Stage-3 task reports and gate evidence are preserved under `project/reports/archive/` and `project/gates/archive/`. They are evidence and history, not the primary developer manual.

## License

AICAD is licensed under the [Apache License 2.0](LICENSE). OCCT is an external dependency with its own licensing requirements; the repository's current policy keeps it behind the native adapter and requires a separate distribution/license review before a public binary/commercial distribution policy is frozen.
