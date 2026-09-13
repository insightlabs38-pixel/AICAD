# Transforms, mirror, and patterns

## Translation

The current source-level `transform` function performs rigid translation only:

```aicad
let moved: Geometry = transform(body, 20mm, 0mm, 5mm);
```

```text
transform(target: Geometry, dx: Length, dy: Length, dz: Length) -> Geometry
```

The kernel-neutral geometry layer supports richer rigid transforms internally, but the Safe CAD source catalogue does not expose a general `rotate` function today.

## Mirror

Mirror uses an explicit plane:

```aicad
let mirrored: Geometry = mirror(
    body,
    Plane(
        origin = Point3(x = 0mm, y = 0mm, z = 0mm),
        normal = Vector3(x = 1.0, y = 0.0, z = 0.0),
    ),
);
```

`mirror` returns the mirrored geometry; it does not automatically union it with the input.

## Linear pattern

```text
linear_pattern(target: Geometry,
               direction: Vector3<Float>,
               count: Int,
               spacing: Length) -> Geometry
```

The result is the union of `count` copies. The first remains in place and subsequent copies are translated by successive multiples of `spacing` along the normalized direction.

## Radial pattern

```aicad
let copies: Geometry = radial_pattern(
    tool,
    Axis3(
        origin = Point3(x = 30mm, y = 30mm, z = 0mm),
        direction = Vector3(x = 0.0, y = 0.0, z = 1.0),
    ),
    6,
    360deg,
);
```

```text
radial_pattern(target: Geometry,
               axis: Axis3,
               count: Int,
               angle: Angle) -> Geometry
```

For a full-circle `360deg` pattern, copies occupy `0, angle/count, ... (count-1)*angle/count`, avoiding a duplicate at the seam. A pattern count below 1 is rejected at runtime.

Patterns currently operate on already-built `Geometry` values; there is no source-level feature/function-valued pattern item or persistent feature identity implied by these functions.
