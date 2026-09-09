# native/occt_bridge

WP-01 (Kernel bridge). Narrow C++ bridge around Open CASCADE Technology
(OCCT), exposing coarse, domain-shaped operations only:

```text
create_box, create_cylinder, make_edge, make_wire, make_face, extrude,
revolve, sweep, loft, boolean_union, boolean_cut, boolean_intersect,
fillet, chamfer, offset, shell, heal, validate, explore_topology,
surface_info, curve_info, export_step
```

Add capabilities only as required by the low-level API in
`docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`. `crates/cad-occt-bridge`
is the only crate that may call into this bridge. OCCT's license and
redistribution terms are an open owner decision — see
`project/OWNER_DECISIONS.md` D13.

Plan references: `docs/plan/01_SYSTEM_ARCHITECTURE.md` §4;
`docs/plan/19_RESEARCH_NOTES_AND_SOURCES.md` §1-3.
