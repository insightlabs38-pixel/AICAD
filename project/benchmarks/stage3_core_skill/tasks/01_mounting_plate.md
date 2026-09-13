# Task 1 — Mounting plate

## Prompt

Design a rectangular mounting plate, 80mm x 60mm x 10mm. Round one long
top edge and chamfer the opposite long top edge (2mm). Add a small
central cylindrical boss on top (8mm radius, 12mm tall). Cut two
clearance through-holes (4mm radius) along the plate's long axis, 15mm in
from each end, centered on the plate's width. Expose the finished solid
as a named output.

## Reference solution

`examples/brackets/stage2_mounting_plate.aicad` — its own top-level
`mounting_plate` binding is the named output (no `part` wrapper; this
fixture predates `AICAD-071`'s `part` concept, but a plain top-level `let`
is an equally valid named output per `skills/cad-core.skill.md` §5).

## Verify

```
cad build examples/brackets/stage2_mounting_plate.aicad --output plate.step --json
```

Expect `"status": "ok"`, empty `"diagnostics"`, one artifact. Proven
end-to-end (hand-derived closed-form volume, bounding box, mirror-symmetry
centroid, re-import validity) by `crates/cad-cli/tests/
stage2_end_to_end.rs`.

## Sanity checks a candidate solution should pass

- Re-imports through an independent kernel context as one valid, manifold
  solid (`is_valid`, `validate` all-zero invalid counts).
- Bounding box is (0,0,0)-(80,60,~22) mm (plate + boss height).
- Exactly one solid, no internal voids (both holes open to the exterior).
