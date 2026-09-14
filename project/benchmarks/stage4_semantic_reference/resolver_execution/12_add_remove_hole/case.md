# Case 12 — Add/remove hole (resolver-execution corpus extension)

**Split:** n/a — this directory is not part of the frozen `AICAD-079A`
corpus (see `../README.md`). **Expected classification:**
`explicit_broken_reference`.

## Fixtures

- `baseline.aicad` — a `30mm x 20mm x 10mm` block (`part Tab`) with one
  `5mm`-diameter through-hole centered on the top face.
- `perturbed.aicad` — identical except the `hole(...)` call is removed
  entirely; `body` is the plain, un-holed block.

The frozen `AICAD-079A` ten name no case whose own perturbation is
literally "a hole appears/disappears" — case `02` ("disappearing entity")
tests a *different* face (a chamfer) disappearing as a side effect of an
unrelated later hole, not a hole itself being added or removed.

## Intended query target

A reference intended to name **the bore's own cylindrical wall face**:
`cylindrical; radius == 2.5mm; unique()` — pure geometry, no lineage
needed.

## Measured evidence

Checked directly against the live `Shape`
`cad_cli::ParametricBuildSession` produces:

| variant | faces | edges | vertices | volume (mm^3) |
|---|---|---|---|---|
| baseline (hole present) | 7 | 15 | 10 | 5803.650459150631 |
| perturbed (hole removed) | 6 | 12 | 8 | 6000.0 |

Closed-form cross-check: baseline `30 * 20 * 10 - pi * 2.5^2 * 10 = 6000 -
196.35 = 5803.65`, matching the measured value; perturbed
`30 * 20 * 10 = 6000.0` exactly (a plain block).

## Reasoning

While the hole exists, exactly one face satisfies `cylindrical; radius ==
2.5mm` (the bore wall). Once the `hole(...)` call is removed from the
*source* entirely, `body`'s own current shape is a plain box with no
cylindrical faces at all, and no other top-level binding in this fixture
holds one either — the reference's own intended target no longer exists
in any form: `explicit_broken_reference`, not a silent "found nothing, so
say nothing" outcome.

## Resolver-execution result

`crates/cad-cli/tests/stage4_resolver_execution.rs`'s own
`case12_add_remove_hole_resolves_then_reports_broken` test runs the real
resolver against both real builds with exactly the query above and
asserts `Resolved(1)` (baseline) then `Broken` (perturbed) — this case's
own `explicit_broken_reference` ground truth, reproduced by real, current
Stage-4 production code.
