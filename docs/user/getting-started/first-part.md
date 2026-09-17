# Your first part

Create `plate.aicad`:

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

    query hole_wall : Face in MountingPlate.body {
        generated_by(body);
        cylindrical();
        unique();
    }
}
```

This uses current supported source syntax. Geometry operations are functional: `hole` creates a new geometry value rather than mutating `base`.

## Check and export

```sh
cargo run -p cad-cli -- build plate.aicad
cargo run -p cad-cli -- build plate.aicad \
  --output plate.step \
  --name MountingPlate.body
```

`--name` performs exact source-output selection. It is separate from the semantic `FaceRef` created by `hole_wall`.

## Inspect reference health

```sh
cargo run -p cad-cli -- refs check plate.aicad
```

For this one-hole model, the scoped `generated_by(body)` + `cylindrical()` + `unique()` recipe resolves the hole wall as one persistent semantic reference. After topology/parameter changes, the recipe is replayed against regenerated evidence; if the contract no longer identifies exactly one entity, AICAD reports `Ambiguous` or `Broken` rather than guessing.

## Raw selectors still exist

Some modeling functions, such as `fillet`, still accept raw integer topology selectors:

```aicad
let rounded: Geometry = fillet(body, [5], 2mm);
```

`[5]` is a raw edge index tied to the target's current realized topology. Persistent references do not retroactively make raw indices stable.

Continue with [types and units](../language/types-and-units.md), the [modeling guide](../modeling/), or [persistent references](../modeling/persistent-references.md).
