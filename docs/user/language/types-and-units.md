# Types and physical units

AICAD treats engineering quantities as typed values. A length is not merely a `Float` that happens to be interpreted as millimeters, and an angle is not interchangeable with a length.

## Common scalar and CAD types

Current source examples commonly use:

| Type | Purpose | Example |
| --- | --- | --- |
| `Bool` | Boolean condition | `true` |
| `Int` | Signed integer/count/index | `6` |
| `Float` | Dimensionless floating-point value | `1.0` |
| `String` | Text value | `"name"` |
| `Length` | Physical length | `60mm` |
| `Angle` | Physical angle | `360deg` |
| `Geometry` | Opaque modeled geometry value | `box(10mm, 20mm, 3mm)` |

The type system contains additional engineering dimensions beyond this small authoring table. The important rule is that dimensional compatibility is checked explicitly.

## Unit-aware arithmetic

Quantities of the same physical dimension can participate in arithmetic and comparison even when written in different compatible units. Canonical internal representation is independent of the spelling chosen in source.

```aicad
let total: Length = 5mm + 2cm;
let half: Length = total / 2;
```

Cross-dimension operations are rejected when they have no valid dimensional meaning. Geometry function signatures use dimensional types such as `Length` and `Angle`, so passing a bare dimensionless number where a length is required is a type error rather than an implicit convention.

## Spatial values

Stage 3 provides ordinary struct-shaped spatial values used by modeling functions:

```aicad
let origin = Point3(x = 10mm, y = 20mm, z = 0mm);
let z = Vector3(x = 0.0, y = 0.0, z = 1.0);
let axis = Axis3(origin = origin, direction = z);
```

The current standard spatial set includes `Vector2<T>`, `Vector3<T>`, `Point2`, `Point3`, `Axis3`, `Frame3`, and `Plane`. Runtime conversion validates spatial invariants; for example, a degenerate axis direction or invalid frame is rejected rather than silently normalized into arbitrary geometry.

## Geometry is opaque

A source-level `Geometry` value does not contain an OCCT object or persistent topology pointer. It identifies geometry construction within AICAD's backend-neutral execution representation. Real kernel shapes are created later by the geometry dispatcher.

That separation is why raw edge/face integers used by some current operations are selectors only, not durable semantic identities.
