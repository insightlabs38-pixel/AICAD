# 02 — twisted, variable-section lateral surfaces + planar caps

**Category:** combined "variable-section sweep" + "twisted loft"
(docs/plan/16 §6). No single sweep/loft builtin is exposed to `.aicad`
(`crates/cad-hir/src/builtins.rs`'s catalogue has no `sweep`/`loft` entry —
`GeometryOp::Sweep`/`Loft` exist only as older, lower-level dispatch
variants an earlier sketch/extrude pipeline uses, never mirrored into the
Stage-5 catalogue). A general `bspline_surface` control net stands in for
it instead, matching docs/plan/16 §6's own stated goal: proving the
low-level DSL is expressive enough without feature proliferation.

**Construction:** a 20mm square base (z=0) ruled to a 10mm square top
(z=30mm) rotated 45 degrees about the z axis. Each of the four lateral
faces (`side0..side3`) is a `degree_u=1, degree_v=1` (bilinear, ruled)
`bspline_surface` between one bottom edge and the corresponding
(differently sized, rotated) top edge — a genuine doubly-curved surface
wherever the two ruling edges are not parallel/coplanar.

**Why no solid here:** `make_face_on_surface` does not yet accept a
Bezier/B-spline surface family (`interp.rs`'s `surface_to_surface_spec`
only covers `Plane`/`Cylinder`/`Cone`/`Sphere`/`Torus`) — confirmed
empirically. See this corpus's own `README.md` "Discovered capability
gap" section and `held_out/06_near_degenerate_extreme_twist`. This
fixture proves the lateral surfaces' own exact bilinear value-level
identities, plus real kernel topology for the two planar caps (built from
`Line`-family edges, which *are* supported today).

**Checked evidence:**

- `side0`'s bilinear evaluation identities (exact, closed-form,
  independent of the kernel): `f(0.5,0.5)` is the average of its own four
  control corners (`b0`, `b1`, `t0`, `t1`); `f(0.0,0.5)`/`f(1.0,0.5)` are
  the exact midpoints of the bottom/top ruling edges.
- `bottom_face`/`top_face`: real kernel B-rep faces (`Line`-only
  topology), `is_valid` true, `area` exactly `400.0e-6 m^2` /
  `100.0e-6 m^2` (a 20mm/10mm square's own area — rotation does not
  change it).

**Tolerance domain:** representation exactness (`1e-12` for the bilinear
identities, bit-level closed-form math); D5/verification exactness for
the two square areas (`1e-12 m^2`, matching the kernel's own exact planar
quadrature for a simple polygon).
