# Command-line interface

The current CLI intentionally implements one command family:

```text
cad build <path.aicad> [--json] [--output <path>] [--name <binding>[.<field>]]
```

From the repository, run it through Cargo:

```sh
cargo run -p cad-cli -- build model.aicad
```

## Build without an artifact

```sh
cargo run -p cad-cli -- build model.aicad
```

This runs the source through parsing, lowering, type checking, and top-level execution. A successful human-readable invocation ends with:

```text
build succeeded
```

## Export STEP

```sh
cargo run -p cad-cli -- build model.aicad --output model.step
```

When an output path is requested, geometry is dispatched to the OCCT-backed runtime and the chosen result is exported as STEP. Without `--name`, the original default is to export the last geometry-producing node.

For models with explicit named outputs, prefer selecting one:

```sh
cargo run -p cad-cli -- build \
  examples/brackets/stage3_l_bracket.aicad \
  --output l-bracket.step \
  --name LBracket.body
```

## Named output selection

`--name` accepts:

```text
<binding>
<part-binding>.<field>
```

A top-level `Geometry` binding can be selected directly. A `part` field can be selected as `PartName.field`. If a part has several geometry fields and no field is specified, AICAD reports an ambiguity with the candidate names instead of guessing.

This mechanism does not query faces/edges and does not provide persistent topology identity.

## JSON output

Add `--json` for machine-readable build output:

```sh
cargo run -p cad-cli -- build model.aicad --json
```

The current JSON report contains build status, diagnostics, and artifacts actually written. Later-roadmap fields are not emitted as empty placeholders.

## Exit codes

The current binary uses:

- `0` for a successful build;
- `1` for a build that completed with error diagnostics;
- `2` for command-line argument parsing/usage errors.

## Commands that do not exist yet

Long-term planning describes a broader CLI, but commands such as `cad test`, `cad inspect`, `cad package`, `cad diff`, and interactive parameter-edit/watch workflows are not part of the current CLI. Use only the command forms documented above unless the executable's source has changed.
