# Troubleshooting

## CMake cannot find OpenCASCADE

Install the OCCT development packages required by `native/occt_bridge/CMakeLists.txt` or point `OpenCASCADE_DIR` at `OpenCASCADEConfig.cmake`. Missing modules are reported explicitly.

## A value has the wrong physical dimension

AICAD requires typed engineering quantities:

```aicad
let r: Length = 5mm;
```

A dimensionless number is not accepted merely because a kernel operation eventually consumes a floating-point magnitude.

## A spatial argument is rejected

`Axis3`, `Frame3`, and `Plane` values are validated when converted to kernel-neutral spatial values. Degenerate directions and invalid/non-orthonormal frames are errors.

## A raw index selects the wrong topology

`fillet`, `chamfer`, `shell`, `extrude`, and `revolve` still have APIs that use raw integer edge/face indices. These are topology-local selectors and can change meaning after shape-changing edits.

Use [persistent semantic references](../modeling/persistent-references.md) when you need a durable reference recipe. Persistent references do not change a raw-index-taking modeling function into a reference-taking function.

## `cad refs check` reports Ambiguous

Ambiguity is intentional fail-closed behavior. Narrow the source query with a more meaningful scoped recipe or predicates/cardinality; do not rely on candidate order.

## `cad refs check` reports Broken

Check that the declared scope still exists and that the predicates still match the intended topology. A missing required scope and a no-match query fail closed. Automatic geometric-fingerprint repair is intentionally disabled.

## A query clause is rejected

Only the current closed source vocabulary documented in [persistent references](../modeling/persistent-references.md) is accepted. Several Rust-level predicates require source-language work planned for the Stage-5 prelude; plan-only syntax is not current grammar.

## `--name` cannot resolve an output

`--name` uses exact source output names. For a part with several geometry outputs, specify the field explicitly, for example `--name LBracket.body`. This is separate from semantic topology-reference resolution.

## I tried `sketch { ... }` and it does not compile

The engine contains sketch entities, constraint semantics, a solver, and solved-profile lowering, but direct source-level sketch construction is not integrated. Use the current Safe CAD source functions.

## I expected another `cad` subcommand

The implemented CLI currently supports `cad build` and `cad refs check`. Planning material describes additional future commands but is not the current CLI contract.
