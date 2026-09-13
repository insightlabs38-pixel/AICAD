# Case 08 — Upstream suppression

**Split:** public. **Expected classification:** `explicit_broken_reference`.

## Fixtures

- `baseline.aicad` — a `40mm x 20mm x 10mm` block (`part Bracket`), edge
  index `1` filleted (`fillet_radius = 3mm`), then a `5mm`-diameter
  through-hole cut at `x = 20mm`.
- `perturbed.aicad` — the fillet step removed entirely (the block is
  holed directly, with no fillet at all).

**Proxy note:** AICAD has no source-level feature-suppression/configuration
toggle yet (that is Stage 5/6+ scope, `project/CURRENT_STAGE.md`'s own
"not allowed yet" list). This case models "upstream suppression" the only
way expressible in Stage-3 source today: the upstream feature call is
removed from the program entirely. The observable effect — a downstream
reference's own upstream feature no longer exists at rebuild time — is the
same effect a real suppression toggle would have; only the *mechanism* by
which the feature stopped existing differs, and that difference is noted
here rather than silently assumed away.

## Intended query target

A reference intended to name **the fillet face created by `fillet(...,
[1], fillet_radius)`**.

## Measured evidence

| variant | faces | edges | vertices | volume (mm^3) |
|---|---|---|---|---|
| baseline (fillet + hole) | 8 | 18 | 12 | 7765.02 |
| perturbed (hole only, fillet suppressed) | 7 | 15 | 10 | 7803.65 |

Volume delta: `7803.65 - 7765.02 = 38.63 mm^3`, matching the closed-form
volume of a `3mm`-radius quarter-circle fillet along a `20mm`-long edge
(`(3^2 * (1 - pi/4)) * 20 = 38.66 mm^3`, matching to the precision expected
from the kernel's own curve tessellation — the same `~1e-6`-scale
fillet/chamfer curve-fitting noise `project/OWNER_DECISIONS.md#D19`'s own
evidence already documents for this class of feature).

## Reasoning

The fillet feature, and the face it produced, are entirely absent from the
perturbed build's own feature graph — not merely reshaped (case `09`) or
consumed by a later feature acting on a still-present prior feature (case
`02`), but never constructed at all this rebuild. A reference bound to
"the fillet face" by lineage has no feature node left to trace back to.
A correct Stage-4 resolver must report `explicit_broken_reference` and,
per `docs/plan/06...` §7's own diagnostic shape, should be able to name
*which* upstream feature disappeared (the fillet step), not merely that
"some" reference failed to resolve — this is exactly the scenario
`docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md` §5's own perturbation list
labels "suppress configuration feature."

## Commands run (for reproduction)

```
cad build baseline.aicad  --output baseline.step  --name Bracket.body --json
cad build perturbed.aicad --output perturbed.step --name Bracket.body --json
```

Re-imported/checked identically to case `01`.
