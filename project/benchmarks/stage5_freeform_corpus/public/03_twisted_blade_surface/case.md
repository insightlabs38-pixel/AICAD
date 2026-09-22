# 03 — twisted blade-like surface

**Category:** "custom impeller blade surface" (docs/plan/16 §6).

**Construction:** a single bilinear (`degree_u = degree_v = 1`)
`bezier_surface` between a root chord (leading/trailing edge at z=0) and
a shorter, twisted, spanwise-offset tip chord (z=80mm) — a genuine
doubly-ruled, non-planar freeform surface.

**Why no face here:** this fixture's own frozen evidence stays
value-level, same as `02`. `AICAD-131` closed the `make_face_on_surface`
capability gap that originally motivated this split; `cad-occt-bridge`/
`cad-runtime`'s own focused `AICAD-131` regression fixtures build a real
bilinear-patch kernel face instead.

**Checked evidence (exact, closed-form, independent of the kernel):**

- Clamped-corner identity: `evaluate_surface(blade, 0.0, 0.0)` /
  `(0.0, 1.0)` / `(1.0, 0.0)` / `(1.0, 1.0)` each reproduce the matching
  control point (`root_le`/`root_te`/`tip_le`/`tip_te`) exactly.
- Bilinear center identity: `evaluate_surface(blade, 0.5, 0.5)` is the
  exact average of all four control corners.

**Tolerance domain:** representation exactness (`1e-9`, floating-point
noise floor only — every check is a closed-form bilinear identity, not a
kernel measurement).
