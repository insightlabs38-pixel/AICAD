# 06 — Semantic References, Queries, Topological Naming, and Feature DAG

## 1. Why this subsystem is foundational

A parametric system fails if downstream features bind to transient entity numbers. Changing an upstream dimension can split, merge, reorder, create, or remove edges and faces. Therefore reference stability must be designed before the feature library becomes large.

## 2. Reference classes

### Stable semantic references

```text
VertexRef
EdgeRef
WireRef
FaceRef
ShellRef
SolidRef
AxisRef
FeatureRef
ComponentRef
```

These do **not** mean "store a kernel pointer forever." They mean "store a reproducible semantic recipe plus lineage context for resolving the intended entity after regeneration."

### Raw handles

```text
Vertex* Edge* Face* ...
```

These represent current topology in the current geometry epoch only.

## 3. Reference construction strategies

A stable reference can derive from one or more of:

1. **Feature lineage:** generated/modified by a named feature.
2. **Explicit export:** a feature intentionally names an output.
3. **Semantic query:** geometric/topological criteria.
4. **Structural role:** e.g. outer boundary, top mounting surface, bore axis.
5. **Ancestry:** descendant of a stable prior entity.
6. **Geometric fingerprint:** approximate position/normal/area/radius as a fallback discriminator.
7. **User confirmation:** when ambiguity cannot be solved reliably.

## 4. Explicit semantic exports

```aicad
feature base = extrude(profile, 5mm) expose {
    top_face = query result.faces {
        generated_by(this);
        planar;
        normal ~= +Z;
        largest(area);
    };

    vertical_edges = query result.edges {
        generated_by(this);
        direction ~= +Z;
    };
};
```

Consumers use:

```aicad
base.top_face
base.vertical_edges
```

## 5. Query model

Queries should be **criteria objects**, not permanently materialized lists. This mirrors a proven idea in programmable CAD systems and is important for regeneration.

Example:

```aicad
query body.faces {
    planar;
    normal ~= +Z within 0.1deg;
    area > 500mm^2;
    generated_by(base);
    largest(area);
}
```

## 6. Core query predicates

### Geometry predicates

- `planar`
- `cylindrical`
- `conical`
- `spherical`
- `toroidal`
- `bspline`
- `radius == ...`
- `area > ...`
- `length > ...`
- `normal ~= direction`
- `axis ~= axis`
- `curvature ...`

### Topology predicates

- `generated_by(feature)`
- `modified_by(feature)`
- `descended_from(ref)`
- `adjacent_to(ref/query)`
- `boundary(outer|inner)`
- `convex`, `concave`
- `manifold`, `nonmanifold`
- `connected_to(...)`
- `contains(point)`
- `intersects(...)`

### Spatial predicates

- `nearest_to(point|ref)`
- `farthest_from(...)`
- `above/below/left/right` relative to a frame
- `inside(volume)`
- `within(distance, target)`

### Ranking/disambiguation

- `first()` only when deterministic ordering is defined;
- `largest(area)`;
- `smallest(radius)`;
- `nearest(target)`;
- `unique()`;
- `expect_count(n)`.

## 7. Ambiguity must be explicit

A query intended to resolve a single reference should not silently choose an arbitrary entity.

Diagnostic example:

```text
REF-E102 AMBIGUOUS_REFERENCE

Reference: housing.mounting_face
Expected: exactly 1 face
Resolved: 2 faces

Candidates:
  face lineage=base/extrude area=642.1mm^2 normal=+Z
  face lineage=rib/union   area=610.9mm^2 normal=+Z

Suggested fixes:
  add generated_by(base)
  add adjacent_to(perimeter)
  use explicit semantic export
```

## 8. Topology lineage graph

Every topology-changing operation should report lineage if the kernel makes it available or the wrapper can infer it:

```text
old entity -> unchanged/new/modified/split/merged/deleted -> new entities
```

Store lineage at the feature node, not only in backend-native structures.

Example:

```text
FaceRef housing.outer_wall
  created_by: base_extrude
  modified_by: usb_cut
  split_by: vent_pattern
  current_resolution: [face A, face B, face C]
```

A semantic reference may intentionally refer to the set or use a discriminator to select one descendant.

## 9. Feature DAG

Do not force a purely linear feature history.

Each feature node includes:

```text
id
source_span
kind
parameters
input feature refs
input semantic refs
input external assets
geometry outputs
semantic outputs
lineage data
validation result
cache key
provenance
```

Example:

```text
             Base
           /      \
       Holes       Ribs
          \        /
           Boolean
              |
            Shell
              |
            Fillet
```

Independent branches can evaluate/cache in parallel.

## 10. Incremental invalidation

When a parameter changes:

1. find parameter dependents;
2. mark affected feature nodes dirty;
3. preserve unaffected cached nodes;
4. rebuild dirty subgraph;
5. replay semantic-reference resolution;
6. compare old/new topology lineage;
7. rerun affected requirements/tests only.

## 11. Reference durability levels

Expose reference confidence/durability in diagnostics and tooling:

| Level | Meaning |
|---|---|
| `explicit` | Feature exported the entity by semantic name |
| `lineage` | Resolved through tracked creation/modification history |
| `query_strong` | Unique result from robust semantic/topological criteria |
| `query_geometric` | Uses geometry fingerprint/spatial heuristics |
| `raw` | Current topology only; not persistent |

AI and code review tooling can warn when a critical downstream feature uses weak references.

## 12. New feature: reference health report

Add:

```text
cad refs check
```

Output:

```text
342 semantic references
329 explicit/lineage
11 strong queries
2 geometric fallbacks
0 ambiguous
0 broken
```

This is useful for production robustness and AI-generated designs.
