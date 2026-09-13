# Troubleshooting

## CMake cannot find OpenCASCADE

AICAD's native bridge uses `find_package(OpenCASCADE REQUIRED CONFIG)`. Install your distribution's OCCT development packages or point `OpenCASCADE_DIR` at the directory containing `OpenCASCADEConfig.cmake`.

If CMake finds OCCT but reports a specific missing library/module, install the development package containing that module. The bridge intentionally fails with the missing module name rather than silently disabling the operation.

## A value has the wrong physical dimension

Geometry signatures require types such as `Length` and `Angle`. A bare dimensionless number is not accepted merely because a kernel operation eventually consumes a floating-point magnitude.

Prefer:

```aicad
let r: Length = 5mm;
```

rather than relying on an implicit unit convention.

## A spatial argument is rejected

`Axis3`, `Frame3`, and `Plane` values are validated when converted to the kernel-neutral spatial representation. Degenerate directions and invalid/non-orthonormal frames are errors; choose valid direction vectors and frame axes.

## `linear_pattern` or `radial_pattern` fails

The runtime rejects a pattern count below 1. Pattern count is an `Int`, so the numeric range check happens at runtime.

## Fillet/chamfer/shell/extrude/revolve selects the wrong topology

Current selection for these operations uses raw integer edge/face indices. Those indices are not stable semantic references and can become invalid or refer to different topology after shape-changing edits.

Keep raw-index operations on simple, controlled target topology where possible. Do not assume Stage-4 topology naming is already available.

## `--name` cannot resolve an output

`--name` uses exact source names. For a part with several geometry outputs, specify the field explicitly:

```sh
--name LBracket.body
```

Naming only `LBracket` is intentionally ambiguous when both `body` and `mirrored` are geometry fields; AICAD reports candidates instead of choosing one.

## I tried `sketch { ... }` and it does not compile

The Stage-3 engine contains sketch entities, constraint semantics, a solver, and exact solved-profile lowering, but source-level sketch construction syntax has not been integrated. Use the current Safe CAD solid/feature functions from `.aicad` source. Do not use aspirational sketch syntax from the frozen planning documents as current syntax.

## I expected a `cad` subcommand from the planning docs

The implemented CLI is currently limited to `cad build`. Planning material describes future commands but is not the current CLI contract. See the [CLI reference](../cli/) for the exact supported arguments.
