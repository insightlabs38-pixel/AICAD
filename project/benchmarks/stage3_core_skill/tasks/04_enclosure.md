# Task 4 — Enclosure

## Prompt

Design a small open-top enclosure/housing, 60mm x 40mm x 30mm outside,
with 2mm walls (hollow it out, leaving the top open). Add a single
cable-access hole (8mm diameter) through the center of one side wall.

## Reference solution

`examples/enclosures/stage3_enclosure.aicad` — part `Enclosure`, named
output `body`.

## Verify

```
cad build examples/enclosures/stage3_enclosure.aicad --output enclosure.step --name Enclosure.body --json
```

Expect `"status": "ok"`, empty `"diagnostics"`, one artifact.

## Sanity checks a candidate solution should pass

- Re-imports as one valid, manifold solid strictly more complex than a
  bare box.
- The interior cavity is genuinely hollow (positive volume strictly less
  than the outer box's own `60 * 40 * 30` mm³, by roughly the wall-
  thickness shell removed) and the top remains open (no top face sealing
  the cavity).
- The cable hole passes fully through the wall it is cut into.

Proven by `crates/cad-cli/tests/stage3_ordinary_parts.rs`'s
`enclosure_body_builds_to_a_valid_solid`.
