# Stage-5 difficult freeform exact-geometry corpus (`AICAD-127`)

A versioned corpus of difficult-but-realistic freeform/exact geometry
fixtures, per `docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md` §6's "Low-level
completeness benchmark" target categories (spline-defined hook,
variable-section sweep, twisted loft, custom blade/impeller surface,
manually trimmed surfaces) and `project/TASKS.yaml`'s `AICAD-127`
acceptance list. Proven through the real production path
(`ParametricBuildSession`) by `crates/cad-cli/tests/
stage5_freeform_corpus.rs`, mirroring `project/benchmarks/
stage4_semantic_reference/`'s own "frozen corpus, real kernel, real
evidence" discipline.

## Layout

- `public/` — tuning fixtures: positive-path geometry with independent
  (closed-form or exact-delegation) checks where derivable, or frozen
  kernel-measured ground truth otherwise.
- `held_out/` — the adversarial/checkpoint subset: fixtures whose
  *expected* outcome is an explicit failure (out-of-domain evaluation) or
  (historically, before `AICAD-131` closed it — see "Capability gap
  closed by `AICAD-131`" below) the discovered `make_edge`/
  `make_face_on_surface` capability gap, feeding `AICAD-128`'s
  numerical/robustness campaign.

| Fixture | Category | Outcome |
| --- | --- | --- |
| `public/01_spline_hook` | spline/NURBS hook or bend | builds; exact endpoint/query identities |
| `public/02_twisted_variable_section_solid` | variable-section sweep + twisted loft | builds; exact bilinear identities + real planar-cap topology |
| `public/03_twisted_blade_surface` | custom impeller-blade-like surface | builds; exact corner/center identities |
| `public/04_trimmed_freeform_patch` | trimmed surfaces | builds; exact trim-delegation identity |
| `held_out/05_out_of_domain_trim_rejected` | numerical/domain adversarial | build fails explicitly (`OutOfDomain`) |
| `held_out/06_near_degenerate_extreme_twist` | near-degenerate (revisited after `AICAD-131`) | builds; `sliver_face_valid` measured `false` |
| `held_out/07_spline_edge_construction_unsupported` | curve-side counterpart (revisited after `AICAD-131`) | builds; `hook_edge_valid` measured `true` |

## Discovered capability gap (the corpus's own central finding)

Building this corpus's own originally-planned positive-path fixtures (a
real spline-hook *face*, a real twisted-loft *solid*, a real trimmed
*face*) discovered that Stage-5 topology construction is narrower than the
Stage-5 value/query layer it sits on top of:

- `make_edge` (`crates/cad-runtime/src/interp.rs`'s `curve_to_edge_op`)
  accepts only `Circle`, `Arc`, and a trimmed `Line` — an `Ellipse`,
  `Bezier`, or `BSpline` curve (`AICAD-110`) is rejected with an explicit
  `UNSUPPORTED_TOPOLOGY_CONSTRUCTION` diagnostic.
- `make_face_on_surface` (`interp.rs`'s `surface_to_surface_spec`) accepts
  only the five original Stage-2 elementary-quadric families (`Plane`,
  `Cylinder`, `Cone`, `Sphere`, `Torus`) — a `Bezier`/`BSpline` surface
  (`AICAD-114`) or *any* `trim_surface` result (`AICAD-115`, even of an
  analytic base) is rejected the same way.

In other words: `AICAD-110`/`AICAD-114`/`AICAD-115` gave `.aicad` real,
kernel-neutral freeform curve/surface **values** — construction,
evaluation, derivatives, trimming, intersection, projection, distance all
work today and never touch OCCT at all (`stage5_curves_checkpoint.rs`/
`stage5_queries_checkpoint.rs` already prove this) — but `AICAD-119`'s own
topology-construction dispatch (`make_edge`/`make_face_on_surface`) was
never extended to cover them; it still only bridges the *original*
Stage-2 analytic families into real kernel B-rep topology. This is not a
crash, hang, or silent-wrong result anywhere in the pipeline — every
rejection is a clean, structured, immediately-surfaced diagnostic — but it
does mean the docs/plan/16 §6 target categories that fundamentally need a
real *face or solid* (not just an evaluable surface value) are not yet
reachable through the current public `.aicad` surface for anything beyond
the five analytic quadric families.

This is recorded as a non-decision item in `project/OWNER_DECISIONS.md`
(a real, disclosed Stage-5 scope gap, not requiring an architecture
ruling to *record* — closing it is ordinary future implementation work:
wiring `curve_to_edge_op`/`surface_to_surface_spec` to also dispatch
Bezier/B-spline/trimmed geometry through OCCT's own matching
`Geom_BSplineCurve`/`Geom_BSplineSurface`/pcurve-trim constructors). Per
this task's own acceptance criterion ("fixtures are executable through
current `.aicad`/public APIs ... no aspirational syntax is presented as
current usage"), every `public/` fixture here proves the freeform
geometry's own exact *value*-level math instead of a face/solid it cannot
yet construct, and `held_out/06`/`07` turn the gap itself into adversarial
evidence that the rejection is explicit, not silent.

## Capability gap closed by `AICAD-131`

`AICAD-131` (Stage 6, `S6-00`) closed the gap this section originally
recorded, per its own required prerequisite scope:

- `curve_to_edge_op` now also dispatches `AnalyticCurve::Bezier`/
  `BSpline` into a real `Geom_BezierCurve`/`Geom_BSplineCurve` kernel
  edge (`GeometryOp::BezierEdge`/`BSplineEdge`, `OcctContext::
  make_bezier_edge`/`make_bspline_edge`); `Ellipse` remains unsupported.
- `surface_to_spec` now also dispatches `AnalyticSurface::Bezier`/
  `BSpline` into a real `Geom_BezierSurface`/`Geom_BSplineSurface`
  kernel face (`SurfaceSpec::Bezier`/`BSpline`, `Shape::
  make_face_on_bezier_surface`/`make_face_on_bspline_surface`), and
  `AnalyticSurface::Trimmed` unwraps to its own `base` recursively
  (a trimmed surface's boundary is supplied separately, as
  `make_face_on_surface`'s own explicit `outer`/`holes` kernel wires —
  its embedded `TrimLoop` is a value-level domain restriction, not a
  second topology boundary).

`held_out/06`/`07` are revisited per their own prior "Follow-up" notes
(see each fixture's own `case.md`) rather than removed: the interface-
level rejection they originally proved no longer exists, so each now
proves the originally-intended question instead (does OCCT itself
reject/mishandle the geometry) — `06`'s 0.001mm sliver face is
structurally built but measured invalid; `07`'s ordinary hook curve
builds a valid edge. `public/01`-`03` remain intentionally value-level
(their own frozen checked evidence is unchanged); new focused positive-
path regression fixtures for the closed capability live in `cad-occt-
bridge`/`cad-runtime`'s own test suites and `native/occt_bridge/tests/
freeform_topology_test.cpp` instead of expanding this frozen corpus.

## Freezing and change control

Each fixture's `case.aicad` plus `case.md` (declared checked evidence and
tolerance domain) is frozen once merged: a future edit that silently
changes a fixture's own geometry or expected evidence will fail
`crates/cad-cli/tests/stage5_freeform_corpus.rs`. Adding new fixtures is
expected as the corpus grows (`AICAD-128`/`129` and beyond); removing,
weakening, or silently re-authoring an existing one is not (`AGENTS.md`'s
"do not silently weaken tests/... reference gates").
