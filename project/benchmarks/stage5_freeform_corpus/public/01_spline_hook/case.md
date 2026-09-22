# 01 — spline-defined hook/bend

**Category:** spline/NURBS hook or bend (docs/plan/16 §6).

**Construction:** a clamped cubic B-spline (`bspline_curve`, degree 3, one
interior knot at u=0.5) through five control points forming a hook/bend
profile in the z=0 plane, plus a real `closest_point_on_curve` query
against a fixed obstacle point — the same realistic-curve-use pattern as
`examples/curves/cable_routing_path.aicad`.

**Why no `make_edge`/face here:** `make_edge` does not yet accept a
Bezier/B-spline curve family (`crates/cad-runtime/src/interp.rs`'s
`curve_to_edge_op` only covers `Circle`/`Arc`/trimmed-`Line`) — confirmed
empirically while building this fixture, not assumed. See this corpus's
own `README.md` "Discovered capability gap" section and
`held_out/07_spline_edge_construction_unsupported`. This fixture proves
the spline's own exact value-level math instead.

**Checked evidence (exact, independent, no kernel measurement):**

- `evaluate_curve(hook, 0.0)` == first control point `(0, 0, 0)` exactly
  (clamped-B-spline endpoint identity).
- `evaluate_curve(hook, 1.0)` == last control point `(30mm, 15mm, 0)`
  exactly.
- `closest_point_on_curve(hook, obstacle)` returns at least one solution;
  each result's own `distance` field equals the plain Euclidean distance
  between its own `point` and the obstacle, recomputed independently in
  the Rust test (`crates/cad-cli/tests/stage5_freeform_corpus.rs`).

**Tolerance domain:** D5/representation exactness (bit-level identity for
the clamped endpoints; `1e-9` m floating-point-noise floor for the
distance recomputation) — not a modeling/construction tolerance.
