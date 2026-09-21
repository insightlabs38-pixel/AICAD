# 04 — trimmed freeform patch

**Category:** "trimmed surfaces" (docs/plan/16 §6 / AICAD-127 acceptance).

**Construction:** a non-planar (`degree_u=degree_v=1`) `bspline_surface`
base (four non-coplanar corners), trimmed (`trim_surface`) by a circular
loop of radius 0.3 centered at `(u,v) = (0.5, 0.5)`, entirely inside the
base's own `[0,1]x[0,1]` natural domain. Per AICAD-115, a trim loop is
read directly as a `(u, v)` parameter-space curve.

**Checked evidence:** `evaluate_surface(trimmed, 0.6, 0.5)` (0.1 inside
the loop) is bit-identical to `evaluate_surface(base, 0.6, 0.5)` — an
exact consequence of `AnalyticSurface::evaluate`'s own `Trimmed` arm
(`crates/cad-geometry-api/src/surface.rs`), which delegates directly to
`base.evaluate(u, v)` once the point is confirmed inside the outer loop.

**Tolerance domain:** none — bit-identical equality (`assert_eq!` on the
raw `f64` magnitudes), since this is a direct code-path delegation, not a
numerical approximation.

**Held-out counterpart:** `held_out/05_out_of_domain_trim_rejected` —
the same construction, evaluated outside the trim loop, and the required
evidence there is an explicit build failure.
