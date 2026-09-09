# 04 — High-Level Modeling API

## 1. Purpose

This is the **default engineering layer**. It should express the large majority of ordinary mechanical parts without forcing the author or AI to manage raw B-rep details. Most constructs may be implemented in the standard library and lower to the small geometry core.

The signatures below are the proposed baseline contracts, not immutable final syntax. Every operation must support source attribution, semantic output naming, deterministic evaluation, cancellation, and structured diagnostics.

## 2. Common conventions

- Length values always carry units.
- Geometry operations accept semantic references/queries where selections are needed.
- Any operation that can fail returns a structured geometry diagnostic; `try_*` forms can expose `Result<T,E>` explicitly.
- Optional `name`/`expose` mechanisms attach stable semantic identifiers.
- High-level operations lower to low-level geometry and retain provenance.

## 3. Feature catalog

### `box`

Create an axis-aligned or framed rectangular solid. **Returns:** `Solid`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `size` | `Vector3<Length>` | required | X/Y/Z extents |
| `center` | `Point3` | [0,0,0] | center point |
| `frame` | `Frame3` | world | construction frame |
| `name` | `String?` | none | semantic output name |

### `cylinder`

Create cylindrical solid. **Returns:** `Solid`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `radius` | `Length` | required | radius |
| `height` | `Length` | required | axial height |
| `axis` | `Axis3` | +Z through origin | axis |
| `capped` | `Bool` | true | closed ends |

### `cone`

Create cone/frustum. **Returns:** `Solid`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `r1` | `Length` | required | start radius |
| `r2` | `Length` | 0mm | end radius |
| `height` | `Length` | required | height |
| `axis` | `Axis3` | +Z | axis |

### `sphere`

Create sphere or angular sector. **Returns:** `Solid`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `radius` | `Length` | required | radius |
| `center` | `Point3` | origin | center |
| `theta` | `Range<Angle>` | full | azimuth range |
| `phi` | `Range<Angle>` | full | polar range |

### `torus`

Create torus. **Returns:** `Solid`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `major_radius` | `Length` | required | centerline radius |
| `minor_radius` | `Length` | required | tube radius |
| `axis` | `Axis3` | +Z | axis |
| `angle` | `Angle` | 360deg | sweep angle |

### `plate`

High-level rectangular/rounded plate. **Returns:** `Solid`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `size` | `Vector2<Length>` | required | planar dimensions |
| `thickness` | `Length` | required | thickness |
| `corner_radius` | `Length` | 0mm | corner fillet |
| `frame` | `Frame3` | XY | placement |
| `centered` | `Bool` | true | center vs corner origin |

### `sketch`

Create constrained 2D sketch. **Returns:** `Sketch`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `plane` | `Plane|FaceRef|Frame3` | required | support plane |
| `name` | `String?` | none | semantic name |
| `solver` | `SolverProfile` | default | solver settings |

### `line`

Sketch line segment. **Returns:** `SketchEntity`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `start` | `Point2` | required | start point |
| `end` | `Point2` | required | end point |
| `construction` | `Bool` | false | construction geometry flag |

### `circle`

Sketch circle. **Returns:** `SketchEntity`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `center` | `Point2` | required | center |
| `radius` | `Length` | required | radius |
| `construction` | `Bool` | false | construction flag |

### `arc`

Sketch arc. **Returns:** `SketchEntity`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `center` | `Point2` | required | center |
| `radius` | `Length` | required | radius |
| `start_angle` | `Angle` | required | start |
| `end_angle` | `Angle` | required | end |
| `direction` | `RotationDirection` | CCW | orientation |

### `rectangle`

Sketch rectangle. **Returns:** `Profile`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `size` | `Vector2<Length>` | required | width/height |
| `center` | `Point2` | origin | center |
| `rotation` | `Angle` | 0deg | rotation |
| `corner_radius` | `Length` | 0mm | optional rounded corners |

### `polygon`

Sketch polygon. **Returns:** `Profile`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `points` | `List<Point2>` | required | ordered points |
| `closed` | `Bool` | true | close contour |

### `slot`

Sketch slot. **Returns:** `Profile`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `start` | `Point2` | required | centerline start |
| `end` | `Point2` | required | centerline end |
| `width` | `Length` | required | slot width |

### `extrude`

Extrude profile/face. **Returns:** `Solid`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `profile` | `Profile|Sketch|FaceRef` | required | source |
| `distance` | `Length` | required unless to target | distance |
| `direction` | `Vector3` | support normal | direction |
| `symmetric` | `Bool` | false | both directions |
| `taper` | `Angle` | 0deg | draft/taper |
| `up_to` | `FaceRef?` | none | terminate on face |

### `revolve`

Revolve profile. **Returns:** `Solid`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `profile` | `Profile|Sketch|FaceRef` | required | source |
| `axis` | `Axis2|Axis3` | required | axis |
| `angle` | `Angle` | 360deg | rotation |
| `symmetric` | `Bool` | false | split angle |

### `sweep`

Sweep profile along path. **Returns:** `Solid|Surface`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `profile` | `Profile|FaceRef` | required | cross section |
| `path` | `Curve3|Wire` | required | trajectory |
| `orientation` | `SweepOrientation` | minimum_twist | frame rule |
| `scale` | `Fn<Float>?` | none | optional variable scale |
| `twist` | `Angle|Fn<Angle>` | 0deg | optional twist |

### `loft`

Loft through sections. **Returns:** `Solid|Surface`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `sections` | `List<Profile|Wire|FaceRef>` | required | ordered sections |
| `solid` | `Bool` | true | solid vs surface |
| `ruled` | `Bool` | false | ruled interpolation |
| `continuity` | `Continuity` | C1 | target continuity |
| `guides` | `List<Curve3>` | [] | optional guide curves |

### `union`

Boolean union. **Returns:** `Solid`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `a` | `Solid` | required | first body |
| `b` | `Solid|List<Solid>` | required | body/bodies |
| `tolerance` | `Length?` | kernel default | fuzzy tolerance |

### `cut`

Boolean subtract. **Returns:** `Solid`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `target` | `Solid` | required | body |
| `tool` | `Solid|List<Solid>` | required | subtractive bodies |
| `tolerance` | `Length?` | kernel default | fuzzy tolerance |

### `intersect`

Boolean intersection. **Returns:** `Solid`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `a` | `Solid` | required | first |
| `b` | `Solid|List<Solid>` | required | second |
| `tolerance` | `Length?` | kernel default | fuzzy tolerance |

### `split`

Split body without discarding pieces. **Returns:** `List<Shape>`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `target` | `Solid|Shell|Face` | required | target |
| `tools` | `List<Surface|Face|Solid>` | required | splitters |
| `keep` | `SplitKeep` | all | which regions |

### `fillet`

Round selected edges. **Returns:** `Solid`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `target` | `Solid` | required | body |
| `edges` | `Query<EdgeRef>|List<EdgeRef>` | required | edges |
| `radius` | `Length|Fn<EdgeRef,Length>` | required | constant/variable radius |
| `continuity` | `Continuity` | C1 | target continuity |

### `chamfer`

Chamfer edges. **Returns:** `Solid`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `target` | `Solid` | required | body |
| `edges` | `Query<EdgeRef>|List<EdgeRef>` | required | edges |
| `distance` | `Length` | required | primary distance |
| `distance2` | `Length?` | none | asymmetric second distance |
| `angle` | `Angle?` | none | distance-angle alternative |

### `shell`

Hollow a solid. **Returns:** `Solid`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `target` | `Solid` | required | body |
| `thickness` | `Length` | required | wall thickness |
| `remove` | `List<FaceRef>|Query<FaceRef>` | [] | faces to open |
| `inward` | `Bool` | true | offset direction |
| `join` | `OffsetJoin` | arc | corner behavior |

### `offset`

Offset face/shell/solid. **Returns:** `Shape`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `target` | `Face|Shell|Solid` | required | target |
| `distance` | `Length` | required | signed offset |
| `join` | `OffsetJoin` | arc | corner method |
| `tolerance` | `Length?` | default | kernel tolerance |

### `draft`

Apply draft angle. **Returns:** `Solid`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `target` | `Solid` | required | body |
| `faces` | `Query<FaceRef>` | required | faces |
| `pull_direction` | `Vector3` | required | mold pull |
| `neutral_plane` | `Plane|FaceRef` | required | neutral |
| `angle` | `Angle` | required | draft angle |

### `hole`

Semantic hole feature. **Returns:** `Feature`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `center` | `Point2|Point3` | required | location |
| `diameter` | `Length` | required | diameter |
| `depth` | `Length|ThroughAll` | ThroughAll | depth |
| `direction` | `Vector3` | support normal | direction |
| `counterbore` | `CounterboreSpec?` | none | counterbore |
| `countersink` | `CountersinkSpec?` | none | countersink |
| `thread` | `ThreadSpec?` | none | thread metadata/geometry mode |
| `fit` | `FitSpec?` | none | fit intent |

### `pocket`

Semantic subtractive pocket. **Returns:** `Feature`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `profile` | `Profile|Sketch` | required | profile |
| `depth` | `Length|ThroughAll` | required | depth |
| `direction` | `Vector3` | support normal | direction |
| `draft` | `Angle` | 0deg | wall draft |

### `boss`

Semantic protruding boss. **Returns:** `Feature`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `profile` | `Profile` | required | base profile |
| `height` | `Length` | required | height |
| `draft` | `Angle` | 0deg | draft |
| `fillet` | `Length` | 0mm | root fillet |

### `rib`

Create structural rib. **Returns:** `Feature`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `path` | `Curve2|Curve3` | required | center/path |
| `thickness` | `Length` | required | rib thickness |
| `height` | `Length|ToFace` | required | height/termination |
| `draft` | `Angle` | 0deg | side draft |
| `root_fillet` | `Length` | 0mm | root radius |

### `thread`

Create thread semantic/geometry. **Returns:** `Feature`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `axis` | `Axis3` | required | thread axis |
| `standard` | `ThreadStandard` | required | ISO/UNC/etc. |
| `designation` | `String` | required | e.g. M5x0.8 |
| `length` | `Length` | required | thread length |
| `mode` | `ThreadMode` | semantic | semantic/cosmetic/modeled |

### `linear_pattern`

Pattern geometry/features. **Returns:** `FeatureGroup`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `item` | `Feature|Shape|Fn` | required | source |
| `direction` | `Vector3` | required | pattern direction |
| `count` | `Int` | required | instances |
| `spacing` | `Length` | required | pitch |
| `centered` | `Bool` | false | center distribution |

### `rectangular_pattern`

2-axis pattern. **Returns:** `FeatureGroup`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `item` | `Feature|Shape|Fn` | required | source |
| `dir1` | `Vector3` | required | axis 1 |
| `count1` | `Int` | required | count 1 |
| `spacing1` | `Length` | required | spacing 1 |
| `dir2` | `Vector3` | required | axis 2 |
| `count2` | `Int` | required | count 2 |
| `spacing2` | `Length` | required | spacing 2 |

### `radial_pattern`

Angular pattern. **Returns:** `FeatureGroup`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `item` | `Feature|Shape|Fn` | required | source |
| `axis` | `Axis3` | required | axis |
| `count` | `Int` | required | instances |
| `angle` | `Angle` | 360deg | total angle |
| `include_endpoint` | `Bool` | false | endpoint policy |

### `path_pattern`

Pattern along curve. **Returns:** `FeatureGroup`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `item` | `Feature|Shape|Fn` | required | source |
| `path` | `Curve3|Wire` | required | path |
| `count` | `Int?` | none | instance count |
| `spacing` | `Length?` | none | distance spacing |
| `orientation` | `PathOrientation` | follow | orientation rule |

### `mirror`

Mirror shape/feature. **Returns:** `Shape|Feature`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `item` | `Shape|Feature` | required | source |
| `plane` | `Plane|FaceRef` | required | mirror plane |
| `merge` | `Bool` | false | merge with source if touching |

### `transform`

Apply rigid/affine transform. **Returns:** `same as input`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `item` | `Shape|Feature|Component` | required | source |
| `transform` | `Transform` | required | transform |
| `copy` | `Bool` | true | copy vs relocate semantics |

### `bearing_seat`

Semantic bearing seat. **Returns:** `Feature`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `bearing` | `BearingSpec` | required | bearing definition |
| `axis` | `Axis3` | required | axis |
| `fit` | `FitSpec` | required | fit/tolerance intent |
| `depth` | `Length` | required | seat depth |
| `retention` | `RetentionSpec?` | none | shoulder/snap ring/etc. |

### `o_ring_groove`

Semantic seal groove. **Returns:** `Feature`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `standard` | `String?` | none | standard/table source |
| `cross_section` | `Length` | required | o-ring cross section |
| `diameter` | `Length` | required | groove center diameter |
| `compression` | `Ratio` | required | target squeeze |
| `type` | `GrooveType` | radial | radial/face/etc. |

### `gear`

High-level standard gear. **Returns:** `Part`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `type` | `GearType` | spur | spur/helical/bevel/etc. |
| `module` | `Length` | required | module |
| `teeth` | `Int` | required | tooth count |
| `pressure_angle` | `Angle` | 20deg | pressure angle |
| `helix_angle` | `Angle` | 0deg | helix angle |
| `face_width` | `Length` | required | width |
| `bore` | `Length?` | none | bore diameter |

### `enclosure`

High-level enclosure scaffold. **Returns:** `Part|Assembly`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `outer_size` | `Vector3<Length>` | required | overall envelope |
| `wall` | `Length` | required | nominal wall |
| `corner_radius` | `Length` | 0mm | corner radius |
| `split` | `EnclosureSplit` | lid_base | split strategy |
| `clearance` | `Length` | 0mm | internal allowance |

### `datum_plane`

Create semantic datum plane. **Returns:** `Datum`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `source` | `Plane|FaceRef` | required | reference |
| `offset` | `Length` | 0mm | offset |
| `rotation` | `Angle` | 0deg | optional rotation |
| `name` | `String` | required | datum name |

### `datum_axis`

Create semantic datum axis. **Returns:** `Datum`.

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `source` | `Axis3|EdgeRef|CylinderRef` | required | reference |
| `name` | `String` | required | datum name |

## 4. Sketch constraints

The sketch API should not invent a separate scripting universe. Sketch entities are ordinary typed objects and constraints are values registered with the unified constraint engine. Baseline constraints:

| Constraint | Parameters |
|---|---|
| `coincident(a,b)` | points/point-on-curve references |
| `horizontal(entity)` | line/points |
| `vertical(entity)` | line/points |
| `parallel(a,b)` | lines/axes |
| `perpendicular(a,b)` | lines/axes |
| `tangent(a,b)` | curves |
| `concentric(a,b)` | circles/arcs/axes |
| `equal(a,b)` | lengths/radii/entities |
| `symmetric(a,b,about)` | entities + line/axis |
| `distance(a,b,value)` | entities + `Length` |
| `angle(a,b,value)` | lines/axes + `Angle` |
| `radius(entity,value)` | circle/arc + `Length` |
| `diameter(entity,value)` | circle/arc + `Length` |
| `fixed(entity)` | entity |
| `midpoint(point,line)` | point + line |

The solver must report remaining degrees of freedom and conflicting constraints.

## 5. Feature composition

High-level features must be ordinary values/functions, making custom abstractions possible without compiler changes:

```aicad
fn four_corner_mount(
    size: Vector2<Length>,
    inset: Length,
    hole_d: Length
) -> FeatureGroup {
    let pts = corner_points(size=size, inset=inset);
    return pts.map(|p| hole(center=p, diameter=hole_d));
}
```

## 6. Semantic outputs

Features should be able to expose named semantic outputs:

```aicad
feature base = extrude(... ) expose {
    top_face = query result.faces { normal ~= +Z; largest(area); };
    perimeter_edges = query result.edges { generated_by(this); boundary(outer); };
}
```

Downstream code refers to `base.top_face`, not an unstable integer face number.
