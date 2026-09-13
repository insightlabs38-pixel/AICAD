# Getting started

AICAD is currently built from this Rust workspace. The implemented command-line entry point is the `cad` binary from `crates/cad-cli`.

## Build the workspace

```bash
cargo build --workspace
```

## Build an AICAD model

The current CLI accepts one build command:

```bash
cargo run -p cad-cli -- build path/to/model.aicad
```

Useful options are documented under `../cli/`.

A successful build parses, lowers/type-checks, evaluates the supported source/modeling surface, realizes geometry through the geometry runtime/kernel bridge, and reports the build result. Output/export behavior depends on the options and model output selected.

## Current stage boundary

Stage 3 provides parametric foundations and a standard modeling baseline. Semantic-reference selection/resolution is Stage 4 and is **not** available merely because Stage-4 planning/benchmarks exist in the repository.
