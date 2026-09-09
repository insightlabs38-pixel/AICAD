# 05 — Low-Level Geometry, Topology, and Raw Access API

## 1. Purpose

This is the **systems-programming layer for CAD**. It is not a second language. It shares the same functions, loops, types, modules, generics, and compiler as the high-level modeling layer. Its objective is to make a very high majority of advanced geometric tasks possible without waiting for a new high-level feature or compiler intrinsic.

The public API exposes mathematical geometry and topology concepts rather than OCCT class names. The backend may initially use OCCT.

## 2. Coverage target

The mature low-level layer should cover roughly the conceptual capabilities needed to construct and manipulate:

- analytic and freeform curves;
- analytic and freeform surfaces;
- vertices/edges/wires/faces/shells/solids;
- boolean and splitting operations;
- transformations;
- projection/intersection/distance;
- surface/curve evaluation;
- topology construction and traversal;
- shape healing;
- local replacement/splitting/merging;
- raw topology handles;
- validation/promotion back into safe semantic geometry.

## 3. Baseline API catalog

### `point2`

Construct 2D point. **Returns:** `Point2`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `x` | `Length` | required |
| `y` | `Length` | required |

### `point3`

Construct 3D point. **Returns:** `Point3`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `x` | `Length` | required |
| `y` | `Length` | required |
| `z` | `Length` | required |

### `vector3`

Construct vector. **Returns:** `Vector3`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `x` | `Float|Quantity` | required |
| `y` | `Float|Quantity` | required |
| `z` | `Float|Quantity` | required |

### `direction`

Normalize vector into direction. **Returns:** `Direction3`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `v` | `Vector3` | required |

### `axis`

Construct axis. **Returns:** `Axis3`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `origin` | `Point3` | required |
| `direction` | `Direction3` | required |

### `frame`

Construct coordinate frame. **Returns:** `Frame3`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `origin` | `Point3` | origin |
| `x` | `Direction3` | required |
| `y` | `Direction3?` | derived |
| `z` | `Direction3?` | derived |

### `transform_from_frames`

Compute transform. **Returns:** `Transform`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `from` | `Frame3` | required |
| `to` | `Frame3` | required |

### `line_curve`

Infinite/trimmed line curve. **Returns:** `Curve3`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `origin` | `Point3` | required |
| `direction` | `Direction3` | required |
| `range` | `Range<Length>?` | none |

### `circle_curve`

Circle curve. **Returns:** `Curve3`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `center` | `Point3` | required |
| `normal` | `Direction3` | required |
| `radius` | `Length` | required |

### `ellipse_curve`

Ellipse curve. **Returns:** `Curve3`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `center` | `Point3` | required |
| `normal` | `Direction3` | required |
| `major_axis` | `Direction3` | required |
| `major_radius` | `Length` | required |
| `minor_radius` | `Length` | required |

### `bezier_curve`

Bezier curve. **Returns:** `Curve3`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `control_points` | `List<Point3>` | required |
| `weights` | `List<Float>?` | none |

### `bspline_curve`

B-spline/NURBS curve. **Returns:** `Curve3`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `degree` | `Int` | required |
| `control_points` | `List<Point3>` | required |
| `knots` | `List<Float>` | required |
| `multiplicities` | `List<Int>` | required |
| `weights` | `List<Float>?` | none |
| `periodic` | `Bool` | false |

### `interpolate_curve`

Interpolate points. **Returns:** `Curve3`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `points` | `List<Point3>` | required |
| `tangents` | `EndpointTangents?` | none |
| `periodic` | `Bool` | false |
| `tolerance` | `Length` | project tolerance |

### `trim_curve`

Trim parametric curve. **Returns:** `Curve`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `curve` | `Curve2|Curve3` | required |
| `u0` | `Float` | required |
| `u1` | `Float` | required |

### `offset_curve`

Offset curve. **Returns:** `Curve`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `curve` | `Curve2|Curve3` | required |
| `distance` | `Length` | required |
| `normal` | `Direction3?` | required for 3D where ambiguous |

### `plane_surface`

Plane surface. **Returns:** `Surface`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `frame` | `Frame3` | required |

### `cylinder_surface`

Infinite cylinder surface. **Returns:** `Surface`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `axis` | `Axis3` | required |
| `radius` | `Length` | required |

### `cone_surface`

Infinite conical surface. **Returns:** `Surface`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `axis` | `Axis3` | required |
| `semi_angle` | `Angle` | required |
| `reference_radius` | `Length` | 0mm |

### `sphere_surface`

Sphere surface. **Returns:** `Surface`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `center` | `Point3` | required |
| `radius` | `Length` | required |
| `frame` | `Frame3?` | default orientation |

### `torus_surface`

Torus surface. **Returns:** `Surface`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `axis` | `Axis3` | required |
| `major_radius` | `Length` | required |
| `minor_radius` | `Length` | required |

### `bezier_surface`

Bezier surface. **Returns:** `Surface`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `control_grid` | `Grid<Point3>` | required |
| `weights` | `Grid<Float>?` | none |

### `bspline_surface`

B-spline/NURBS surface. **Returns:** `Surface`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `u_degree` | `Int` | required |
| `v_degree` | `Int` | required |
| `control_grid` | `Grid<Point3>` | required |
| `u_knots` | `List<Float>` | required |
| `v_knots` | `List<Float>` | required |
| `u_mult` | `List<Int>` | required |
| `v_mult` | `List<Int>` | required |
| `weights` | `Grid<Float>?` | none |
| `u_periodic` | `Bool` | false |
| `v_periodic` | `Bool` | false |

### `surface_of_revolution`

Surface by revolution. **Returns:** `Surface`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `curve` | `Curve3` | required |
| `axis` | `Axis3` | required |

### `surface_of_extrusion`

Surface by translation. **Returns:** `Surface`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `curve` | `Curve3` | required |
| `direction` | `Vector3` | required |

### `offset_surface`

Offset surface. **Returns:** `Surface`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `surface` | `Surface` | required |
| `distance` | `Length` | required |

### `trim_surface`

Trim surface parameter range. **Returns:** `Surface`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `surface` | `Surface` | required |
| `u` | `Range<Float>` | required |
| `v` | `Range<Float>` | required |

### `project_point`

Project point. **Returns:** `ProjectionResult`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `point` | `Point3` | required |
| `onto` | `Curve3|Surface` | required |
| `mode` | `ProjectionMode` | nearest |

### `project_curve`

Project curve onto surface. **Returns:** `List<Curve3>`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `curve` | `Curve3` | required |
| `surface` | `Surface` | required |
| `direction` | `Direction3?` | normal/nearest depending mode |
| `mode` | `ProjectionMode` | nearest |

### `intersect_geometry`

Exact/numerical intersection. **Returns:** `IntersectionResult`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `a` | `Curve|Surface|Shape` | required |
| `b` | `Curve|Surface|Shape` | required |
| `tolerance` | `Length` | project tolerance |

### `closest_points`

Minimum distance pair. **Returns:** `DistanceResult`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `a` | `Geometry|Shape` | required |
| `b` | `Geometry|Shape` | required |

### `evaluate_curve`

Evaluate curve. **Returns:** `CurveEvaluation`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `curve` | `Curve` | required |
| `u` | `Float` | required |
| `derivatives` | `Int` | 0 |

### `evaluate_surface`

Evaluate surface. **Returns:** `SurfaceEvaluation`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `surface` | `Surface` | required |
| `u` | `Float` | required |
| `v` | `Float` | required |
| `derivatives` | `Int` | 0 |

### `curvature`

Evaluate curvature. **Returns:** `CurvatureResult`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `entity` | `Curve|Surface` | required |
| `at` | `Float|UV` | required |

### `make_vertex`

Create topological vertex. **Returns:** `Vertex`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `point` | `Point3` | required |
| `tolerance` | `Length?` | default |

### `make_edge`

Create edge. **Returns:** `Edge`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `curve` | `Curve3` | required |
| `range` | `Range<Float>?` | full |
| `start` | `Vertex?` | derived |
| `end` | `Vertex?` | derived |

### `make_wire`

Create ordered wire. **Returns:** `Wire`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `edges` | `List<Edge>` | required |
| `closed` | `Bool?` | infer |

### `make_face`

Create trimmed face. **Returns:** `Face`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `surface` | `Surface` | required |
| `outer` | `Wire` | required |
| `holes` | `List<Wire>` | [] |
| `orientation` | `Orientation` | forward |

### `make_shell`

Create shell. **Returns:** `Shell`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `faces` | `List<Face>` | required |
| `sew` | `Bool` | true |
| `tolerance` | `Length` | project tolerance |

### `make_solid`

Create solid from shell. **Returns:** `Solid`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `outer` | `Shell` | required |
| `voids` | `List<Shell>` | [] |
| `validate` | `Bool` | true |

### `compound`

Create compound. **Returns:** `Compound`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `shapes` | `List<Shape>` | required |

### `sew`

Sew faces/shells. **Returns:** `Shell|Compound`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `shapes` | `List<Face|Shell>` | required |
| `tolerance` | `Length` | project tolerance |
| `non_manifold` | `Bool` | false |

### `heal`

Repair shape. **Returns:** `HealingResult`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `shape` | `Shape` | required |
| `profile` | `HealingProfile` | default |
| `tolerance` | `Length?` | project tolerance |
| `max_tolerance` | `Length?` | policy max |

### `validate`

Validate B-rep/topology. **Returns:** `ValidationReport`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `shape` | `Shape` | required |
| `level` | `ValidationLevel` | standard |

### `remove_face`

Delete face and optionally heal. **Returns:** `Shape`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `solid` | `Solid` | required |
| `faces` | `List<FaceRef|Face*>` | required |
| `heal` | `Bool` | true |

### `replace_face`

Replace face surface/boundary. **Returns:** `Solid`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `solid` | `Solid` | required |
| `face` | `FaceRef|Face*` | required |
| `replacement` | `Face` | required |
| `heal` | `Bool` | true |

### `split_edge`

Split edge. **Returns:** `List<Edge>`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `edge` | `EdgeRef|Edge*` | required |
| `parameters` | `List<Float>` | required |

### `merge_faces`

Merge compatible adjacent faces. **Returns:** `Face|List<Face>`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `faces` | `List<FaceRef|Face*>` | required |
| `continuity` | `Continuity` | C1 |

### `topology_faces`

Enumerate faces. **Returns:** `Iterator<FaceRef>`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `shape` | `Shape` | required |

### `topology_edges`

Enumerate edges. **Returns:** `Iterator<EdgeRef>`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `shape` | `Shape` | required |

### `topology_vertices`

Enumerate vertices. **Returns:** `Iterator<VertexRef>`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `shape` | `Shape` | required |

### `adjacent_faces`

Topological adjacency. **Returns:** `List<FaceRef>`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `edge` | `EdgeRef` | required |

### `face_edges`

Boundary edges. **Returns:** `List<EdgeRef>`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `face` | `FaceRef` | required |

### `generated_by`

Select topology lineage. **Returns:** `Query`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `feature` | `FeatureRef` | required |
| `entity_type` | `EntityType?` | any |

### `raw_face`

Obtain ephemeral raw face. **Returns:** `Face*`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `shape` | `Shape` | required |
| `index` | `Int` | required |

### `raw_edge`

Obtain ephemeral raw edge. **Returns:** `Edge*`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `shape` | `Shape|Face*` | required |
| `index` | `Int` | required |

### `raw_shape`

Obtain kernel-grade raw shape. **Returns:** `KernelShape*`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `shape` | `Shape` | required |

### `adopt_validated`

Promote raw result. **Returns:** `Shape`.

| Parameter | Type | Default/Requirement |
|---|---|---|
| `raw` | `KernelShape*` | required |
| `semantic_outputs` | `Map<String,Query>?` | none |
| `validation` | `ValidationLevel` | strict |

## 4. Continuity and geometric quality

Define a common continuity enum:

```text
C0   positional
G1   tangent direction
C1   first-derivative
G2   curvature direction/magnitude relation
C2   second-derivative
```

Operations that promise continuity must report whether the requested continuity was achieved.

## 5. Raw pointer-like access

Raw handles exist because advanced geometry sometimes requires access below semantic abstractions. They are intentionally dangerous and ephemeral.

```aicad
unsafe geometry {
    let raw: KernelShape* = raw_shape(body);
    let f: Face* = raw_face(body, 17);
    let e: Edge* = raw_edge(f, 2);

    let modified = kernel_grade_operation(raw, e, ...);
    body = adopt_validated(modified);
}
```

Rules:

1. A raw handle belongs to a geometry epoch.
2. Topology mutation may invalidate handles.
3. Stale-handle access is trapped.
4. Raw handles cannot be serialized as persistent design references.
5. A raw result must pass validation before promotion.
6. Unsafe operations produce stronger provenance and audit records.

## 6. Query/selection rather than indices

Even the low-level language should prefer queries:

```aicad
for edge in body.edges()
    .where(|e| e.convex && e.length > 10mm)
{
    ...
}
```

Indices are permitted only when an algorithm intentionally depends on the current transient topology enumeration.

## 7. Kernel escape hatch

A mature implementation may expose a versioned `kernel` namespace for operations that have not yet received vendor-neutral wrappers. This should be **package/unsafe-only**, clearly backend-specific, and never used by the core standard library if a portable equivalent exists.

Example conceptual syntax:

```aicad
unsafe geometry {
    let out = kernel::occt::some_operation(...);
}
```

This is the absolute last escape hatch and is not part of portable source compatibility.

## 8. New requirement: geometry introspection normalization

Every backend must normalize introspection into a common schema:

```text
curve_kind
surface_kind
parameter_bounds
continuity
area/length
bounding_box
orientation
adjacency
provenance
tolerance
validity
```

This keeps AI tooling and the IDE backend-independent.
