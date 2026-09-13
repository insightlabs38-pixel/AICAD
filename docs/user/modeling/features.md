# Modeling features

Stage 3 extends the basic primitive/boolean surface with common single-part features. All functions below return a new `Geometry` value.

## Hole

```aicad
let drilled: Geometry = hole(
    body,
    Axis3(
        origin = Point3(x = 30mm, y = 20mm, z = -1mm),
        direction = Vector3(x = 0.0, y = 0.0, z = 1.0),
    ),
    6mm,
    10mm,
);
```

Signature:

```text
hole(target: Geometry, axis: Axis3, diameter: Length, depth: Length) -> Geometry
```

Depth is explicit. There is no current `ThroughAll`, counterbore, countersink, or thread-metadata source parameter.

## Pocket

```text
pocket(target: Geometry, frame: Frame3,
       width: Length, length: Length, depth: Length) -> Geometry
```

The pocket is rectangular and positioned/oriented by `Frame3`. Arbitrary profile pockets are not source-visible yet.

## Extrude and revolve

```text
extrude(target: Geometry, face: Int,
        direction: Vector3<Float>, distance: Length) -> Geometry

revolve(target: Geometry, face: Int,
        axis: Axis3, angle: Angle) -> Geometry
```

Both operate on a face selected from an existing geometry value. The `face` argument is a raw topology index, not a Stage-4 semantic `FaceRef`.

## Fillet and chamfer

```aicad
let rounded: Geometry = fillet(block, [5], 2mm);
let beveled: Geometry = chamfer(rounded, [12], 1mm);
```

Current signatures use `List<Int>` edge indices. The list must identify edges in the target's current topology; do not treat these values as stable identity after arbitrary topology-changing edits.

## Shell

```text
shell(target: Geometry, removed_faces: List<Int>, thickness: Length) -> Geometry
```

`removed_faces` uses raw face indices. An empty list is valid for a fully closed shell. The Safe CAD source operation interprets positive thickness as an inward hollowing thickness; the runtime maps that convention to the kernel operation.

## Named part outputs

Inside a `part`, top-level `let`/`const`/`param` values become named outputs of the executed part value. Geometry outputs can be selected exactly at export time:

```sh
cargo run -p cad-cli -- build part.aicad \
  --output body.step \
  --name MyPart.body
```

This is explicit source-name selection only. It does not search topology or track a face/edge through a rebuild.
