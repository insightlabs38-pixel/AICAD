# Examples

Use examples that are explicitly backed by the current compiler/runtime rather than assuming every category directory represents an implemented source feature.

## Recommended current examples

### Stage-3 L bracket

`examples/brackets/stage3_l_bracket.aicad`

Demonstrates:

- typed parameters;
- `part` execution;
- `box`, `fillet`, `union`, and `hole`;
- `Axis3` / `Point3` / `Vector3`;
- `mirror` with `Plane`;
- multiple named geometry outputs;
- export selection with `--name LBracket.body` or `--name LBracket.mirrored`.

### Stage-3 bearing mount

`examples/plates/stage3_bearing_mount.aicad`

Demonstrates:

- a parametric plate and raised boss;
- `pocket`;
- central `hole`;
- `radial_pattern` for a bolt circle;
- raw-index filleting on a deliberately simple pre-boolean box;
- a named final part output.

### Stage-2 mounting plate

`examples/brackets/stage2_mounting_plate.aicad`

Still useful as a language/control-flow example. It demonstrates:

- helper functions;
- derived engineering-unit expressions;
- `if`, `while`, `for`, and `match`;
- `var` rebinding;
- primitives, translation, booleans, fillet, and chamfer.

## Run an example

```sh
cargo run -p cad-cli -- build \
  examples/brackets/stage3_l_bracket.aicad \
  --output l-bracket.step \
  --name LBracket.body
```

## About other example directories

The repository reserves directories for later roadmap areas such as assemblies, verification, and freeform work. Their presence does not mean those user-facing capabilities are implemented. The current guide treats Stage-2/Stage-3 tested `.aicad` examples as the authoritative examples of source syntax available today.
