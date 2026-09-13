# Your first part

Create a file named `plate.aicad`:

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
            origin = Point3(
                x = width / 2,
                y = depth / 2,
                z = 0mm - OVERSHOOT,
            ),
            direction = Vector3(x = 0.0, y = 0.0, z = 1.0),
        ),
        hole_diameter,
        thickness + OVERSHOOT * 2,
    );
}
```

This uses only the current Stage-3 source surface. `param` values carry engineering types and units, `box` produces a `Geometry`, and `hole` returns a new geometry value rather than mutating `base` in place.

## Check the source

Run the build pipeline without requesting an artifact:

```sh
cargo run -p cad-cli -- build plate.aicad
```

A successful run ends with `build succeeded`. Parse, lowering, type, and runtime failures are emitted as structured AICAD diagnostics.

## Export the named part output

`MountingPlate` contains two geometry values, `base` and `body`, so select the intended final result explicitly:

```sh
cargo run -p cad-cli -- build plate.aicad \
  --output plate.step \
  --name MountingPlate.body
```

`--name` performs exact source-name selection. If a part has more than one `Geometry` field and you name only the part, AICAD reports the ambiguity rather than choosing one arbitrarily.

## Add another feature

Safe CAD geometry operations are functional: bind the result to a new name.

```aicad
let rounded: Geometry = fillet(body, [5], 2mm);
```

The `[5]` selector is a **raw edge index**. It is suitable only when you know the target's current edge enumeration. It is not a persistent face/edge reference; Stage 4 is intended to provide durable semantic references.

Continue with [types and units](../language/types-and-units.md) or the [modeling guide](../modeling/).
