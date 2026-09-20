# Maintained AICAD examples

Every file listed as **ACTIVE** below uses current supported `.aicad` syntax and is registered in `crates/cad-cli/tests/active_examples.rs`.

| Status | Example | Purpose |
| --- | --- | --- |
| ACTIVE | `getting_started/simple_box.aicad` | Minimal primitive/part/named output |
| ACTIVE | `parametric/derived_plate.aicad` | Typed parameters and derived values |
| ACTIVE | `brackets/stage2_mounting_plate.aicad` | Control flow, derived engineering values, ordinary parametric modeling |
| ACTIVE | `brackets/stage3_l_bracket.aicad` | Multi-feature part, hole, fillet, mirror, multiple outputs |
| ACTIVE | `plates/stage3_bearing_mount.aicad` | Transform, pocket, radial pattern, realistic mechanical part |
| ACTIVE | `enclosures/stage3_enclosure.aicad` | Shell + hole mechanical part |
| ACTIVE | `references/hole_wall_reference.aicad` | Scoped persistent reference; replayed after a parameter edit |
| ACTIVE | `references/ambiguous_reference.aicad` | Expected fail-closed ambiguity |
| ACTIVE | `references/broken_reference.aicad` | Expected fail-closed broken reference |
| ACTIVE | `curves/circle_curve_basics.aicad` | Minimal analytic curve construction/evaluation |
| ACTIVE | `curves/cable_routing_path.aicad` | Realistic curve use: rational-Bezier fillet arc, line segments, closest-point clearance check |
| ACTIVE | `surfaces/surface_query_basics.aicad` | Minimal surface/surface intersection and point-to-surface projection |
| ACTIVE | `surfaces/pipe_clearance_check.aicad` | Realistic surface-query use: curve-to-surface clearance distance and curve/surface crossing point |
| ACTIVE | `topology/topology_construction_basics.aicad` | Vertex/edge/wire/face/face-on-surface/shell/solid/compound/sew/heal/topology-traversal construction, and the "construction success is not validity" contract |

Historical `stage2_*` / `stage3_*` filenames are retained for evidence/test compatibility; ACTIVE means they still represent current supported syntax.

Stage-0 paper/spec examples and roadmap-only freeform/verification placeholders were classified **ARCHIVE** and moved to `project/archive/examples/`. They remain available for design archaeology but are not presented as current usage.

## Validation contract

The automated active-example test:

- builds every ACTIVE file through the real compiler/runtime path;
- asserts the three reference fixtures' `Resolved`, `Ambiguous`, and `Broken` health outcomes;
- edits the canonical reference example's `hole_diameter`, performs a real incremental rebuild, and re-resolves the same source reference.

Representative mechanical examples retain their stronger exact-B-rep/STEP integration checks in existing tests.

## Maintenance policy

Public language/modeling changes must update affected ACTIVE examples in the same stage/batch. Each major stage checkpoint should add or refresh representative examples. Stale examples are updated or archived, never silently left as historical syntax.
