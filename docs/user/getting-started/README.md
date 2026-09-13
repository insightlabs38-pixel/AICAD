# Getting started

AICAD currently runs from the repository as a Rust workspace with a native OCCT bridge. The fastest path from clone to a generated STEP file is:

1. [Install the required toolchain and OCCT development libraries](installation.md).
2. Build and test the workspace.
3. [Create and build a small `.aicad` part](first-part.md).

The current CLI surface is intentionally small:

```text
cad build <path.aicad> [--json] [--output <path>] [--name <binding>[.<field>]]
```

When working from the repository without installing a separate executable, invoke it through Cargo:

```sh
cargo run -p cad-cli -- build model.aicad --output model.step
```

Use [the CLI reference](../cli/) for output selection and diagnostics.
