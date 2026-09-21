# 07 (held out) — spline edge construction, curve-side capability gap

**Category:** capability-gap disclosure (curve side, counterpart of `06`).

**Construction:** the same hook `bspline_curve` as
`public/01_spline_hook`, passed directly to `make_edge`.

**Expected/measured outcome:** the build fails —
`UNSUPPORTED_TOPOLOGY_CONSTRUCTION` (`crates/cad-runtime/src/
interp.rs`'s `curve_to_edge_op`, which only covers
`Circle`/`Arc`/trimmed-`Line`; `Ellipse`/`Bezier`/`BSpline` all fall into
its shared rejection arm). Confirmed by
`crates/cad-cli/tests/stage5_freeform_corpus.rs::
freeform_curve_edge_construction_is_explicitly_rejected_never_silent`.
Never a panic, hang, or silently-wrong edge.
