# Command-line interface

The current CLI has two supported command families:

```text
cad build <path.aicad> [--json] [--output <path>] [--name <binding>[.<field>]]
cad refs check <path.aicad> [--json]
```

From the repository, invoke them through Cargo.

## Build and type-check a model

```sh
cargo run -p cad-cli -- build model.aicad
```

This runs parsing, lowering, type checking, and execution. A successful human-readable invocation ends with `build succeeded`.

## Export STEP

```sh
cargo run -p cad-cli -- build model.aicad --output model.step
```

For a part with explicit outputs, select one:

```sh
cargo run -p cad-cli -- build \
  examples/brackets/stage3_l_bracket.aicad \
  --output l-bracket.step \
  --name LBracket.body
```

`--name` accepts `<binding>` or `<part-binding>.<field>`. If a part has several geometry fields and the requested name is ambiguous, AICAD reports the candidates instead of guessing.

`--name` is source-output selection. It is separate from persistent topology references declared with `query`.

## Check persistent references

```sh
cargo run -p cad-cli -- refs check model.aicad
```

`cad refs check` builds the model through the production `ParametricBuildSession`, collects the program's actual source-declared references, resolves them against the current build, and reports reference health. Outcomes are fail-closed: each reference is `Resolved`, `Ambiguous`, or `Broken`.

A file with no `query` declarations honestly reports a zero-reference set. AICAD does not fabricate implicit references from ordinary source bindings.

Use JSON when consuming the report programmatically:

```sh
cargo run -p cad-cli -- refs check model.aicad --json
```

See [persistent references](../modeling/persistent-references.md) for current query syntax and supported clauses.

## JSON build output

```sh
cargo run -p cad-cli -- build model.aicad --json
```

The report contains build status, diagnostics, and artifacts actually written.

## Exit codes

Both command families use `0` for success and `1` for a completed operation with error diagnostics. CLI argument/usage errors use `2`.

## Not current CLI surface

Planning material describes additional commands such as `cad test`, `cad inspect`, `cad package`, `cad diff`, and interactive watch/parameter-edit workflows. Those are not implemented CLI contracts today.
