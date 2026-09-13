# Modeling — current Stage-3 surface

The current Safe CAD source surface uses ordinary typed function calls backed by the runtime. Geometry values are opaque source-level values; OCCT/native objects are not exposed as source values.

## Runtime-backed modeling functions

The final Stage-3 builtin catalogue contains 17 modeling functions:

- primitives/transforms: `box`, `cylinder`, `transform`, `plate`
- booleans: `union`, `cut`, `intersect`
- finishing: `fillet`, `chamfer`, `shell`
- feature-like operations: `extrude`, `revolve`, `hole`, `pocket`
- duplication/symmetry: `mirror`, `linear_pattern`, `radial_pattern`

For exact signatures, implementation narrowings, and the relationship to Geometry IR, see the developer reference at `../../developer/geometry/safe-cad-api.md`.

## Important Stage-3 limitations

Some operations still use raw enumeration-order integer selectors for faces/edges. Those integers are not durable semantic references. Stage 4 is explicitly responsible for semantic-reference infrastructure; do not treat the current raw indices as persistent model identity.

The high-level foundation plan often shows broader signatures than the Stage-3 implementation. Current builtin signatures in the runtime catalogue are authoritative for what is actually callable today.
