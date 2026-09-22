# 06 (held out) — near-degenerate freeform patch, surface topology construction

**Category:** near-degenerate adversarial (docs/plan/16 §5) +
capability-gap disclosure (surface side), revisited after `AICAD-131`.

**Construction:** a `bspline_surface` whose `u=0` edge is only 0.001mm
long (a genuine, constructible, but extremely thin sliver — not a literal
zero-length edge, which `line_curve`/`trim_curve` cannot express at all),
bounded by a real 4-edge wire, passed to `make_face_on_surface`, plus
`is_valid(sliver_face)`.

**Original outcome (Stage 5):** the build failed before ever reaching
OCCT — `make_face_on_surface` rejected *any* Bezier/B-spline surface
(`UNSUPPORTED_TOPOLOGY_CONSTRUCTION`, `crates/cad-runtime/src/
interp.rs`'s `surface_to_spec`), so this fixture's own near-degeneracy
never actually reached the kernel.

**Current outcome (`AICAD-131`):** `surface_to_spec` now materializes a
Bezier/B-spline surface into a real `Geom_BSplineSurface` kernel face —
the originally-intended near-degenerate-geometry question is now
exercised for real: the build succeeds structurally (`BRepBuilderAPI_
MakeFace` reports done), but `sliver_face_valid` is measured `false` —
OCCT's own `BRepCheck_Analyzer` rejects the 0.001mm sliver as an invalid
B-rep. This is still the required adversarial evidence — "explicit,
structured, measurable outcome, never a panic/hang/silent-wrong success"
— now via the actual numerical/topological mechanism this fixture was
designed to probe, not merely an interface-level rejection. Confirmed by
`crates/cad-cli/tests/stage5_freeform_corpus.rs::
freeform_surface_topology_construction_now_builds_and_exposes_the_near_degeneracy`.
