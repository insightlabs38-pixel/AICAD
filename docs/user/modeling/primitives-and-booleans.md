# Primitives and booleans

The basic Safe CAD functions create exact solid geometry or combine existing geometry values.

## Primitives

```aicad
let block: Geometry = box(80mm, 60mm, 10mm);
let pin: Geometry = cylinder(5mm, 20mm);
let plate_body: Geometry = plate(80mm, 60mm, 10mm);
```

Current signatures:

| Function | Signature | Notes |
| --- | --- | --- |
| `box` | `(Length, Length, Length) -> Geometry` | Axis-aligned, corner at the origin |
| `cylinder` | `(radius: Length, height: Length) -> Geometry` | Capped, axis along +Z |
| `plate` | `(width: Length, depth: Length, thickness: Length) -> Geometry` | Stage-3 domain name over rectangular box construction |

`plate` is intentionally narrower than some long-term planning examples: no source parameters for rounded corners, arbitrary frame placement, or centered construction are implied by this current signature.

## Booleans

```aicad
let added: Geometry = union(base, boss);
let drilled: Geometry = cut(added, hole_tool);
let overlap: Geometry = intersect(a, b);
```

| Function | Meaning |
| --- | --- |
| `union(a, b)` | Boolean union |
| `cut(a, b)` | Subtract `b` from `a` |
| `intersect(a, b)` | Boolean common/intersection |

Boolean operations are functional: each produces a new geometry value.

## Positioning primitive tools

The current general `transform` source function is translation-only:

```aicad
let moved: Geometry = transform(pin, 20mm, 15mm, 0mm);
```

Rotation-oriented operations use explicit `Axis3`, `Frame3`, or `Plane` values through functions such as `revolve`, `mirror`, and `radial_pattern`; there is not a generic source-level `rotate(...)` function in the current Safe CAD catalogue.
