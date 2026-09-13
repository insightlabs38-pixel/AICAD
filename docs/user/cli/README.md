# CLI

The implemented CLI command is:

```text
cad build <path.aicad> [--json] [--output <path>] [--name <binding>[.<field>]]
```

When developing from the repository, the equivalent is typically:

```bash
cargo run -p cad-cli -- build model.aicad
```

Options:

- `--json` — emit the structured/canonical JSON build report instead of the human renderer.
- `--output <path>` — select an output path supported by the current build pipeline.
- `--name <binding>[.<field>]` — select a named model output/binding (including a part field where supported).

CLI errors and build failures should be consumed through the structured diagnostics/report model rather than parsed from incidental debug output.
