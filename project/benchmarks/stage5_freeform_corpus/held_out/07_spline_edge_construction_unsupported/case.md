# 07 (held out) — spline edge construction, curve-side capability gap

**Category:** capability-gap disclosure (curve side, counterpart of `06`),
revisited after `AICAD-131`.

**Construction:** the same hook `bspline_curve` as
`public/01_spline_hook`, passed directly to `make_edge`, plus
`is_valid(hook_edge)`.

**Original outcome (Stage 5):** the build failed —
`UNSUPPORTED_TOPOLOGY_CONSTRUCTION` (`crates/cad-runtime/src/
interp.rs`'s `curve_to_edge_op`, which only covered
`Circle`/`Arc`/trimmed-`Line`; `Ellipse`/`Bezier`/`BSpline` all fell into
its shared rejection arm).

**Current outcome (`AICAD-131`):** `curve_to_edge_op` now materializes a
Bezier/B-spline curve into a real `Geom_BSplineCurve` kernel edge. This
hook curve is not itself near-degenerate (identical to `public/
01_spline_hook`'s own), so the build succeeds and `hook_edge_valid` is
`true` — a real, valid kernel edge, not a second invalidity data point
(contrast `06`'s own measured `false`). Confirmed by
`crates/cad-cli/tests/stage5_freeform_corpus.rs::
freeform_curve_edge_construction_now_builds_a_valid_edge`.
