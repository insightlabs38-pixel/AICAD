# Case 02 — Disappearing/newly-created entities

**Split:** public. **Expected classification:** `explicit_broken_reference`.

## Fixtures

- `baseline.aicad` — a `40mm x 20mm x 10mm` block (`part Corner`), a
  `1.5mm` chamfer on edge index `1` (empirically confirmed below to be the
  `20mm`-long edge at `x = 0mm` — the corner *opposite* the hole), then a
  `4mm`-diameter through-hole centered at `x = 30mm` (well clear of the
  chamfered corner).
- `perturbed.aicad` — identical except `hole_diameter = 24mm` and
  `hole_center_x = 0mm` — the hole is now centered exactly on the
  chamfered corner, with a `12mm` radius comfortably covering the entire
  `1.5mm x 20mm` chamfer footprint.

## Intended query target

A reference intended to name **the chamfer face created by `chamfer(...,
[1], chamfer_distance)`**: `generated_by(the chamfer feature); unique()`.

## Measured evidence

Two additional control builds isolate exactly what the big hole consumes:
a "no-chamfer" variant (`hole` applied directly to a plain, un-chamfered
box, otherwise identical) at each of the two hole positions.

| variant | faces | edges | vertices | volume (mm^3) |
|---|---|---|---|---|
| baseline (chamfer intact, hole far away) | 8 | 18 | 12 | 7851.84 |
| baseline, closed-form check: `40*20*10 - (0.5*1.5*1.5*20) - (pi*2^2*10)` | — | — | — | `8000 - 22.5 - 125.66 = 7851.84` (matches) |
| perturbed (chamfer + big hole at the corner) | 7 | 15 | 10 | 5918.12 |
| control: big hole alone, same position, **no chamfer at all** | 7 | 15 | 10 | 5918.12 |

The perturbed build and its no-chamfer control are **identical** in face
count, edge count, vertex count, and volume (to the full precision `cad-
occt-bridge` reports). This was cross-checked at a *different* hole
position too (`hole_center_x = 40mm`, the corner where the chamfer is
*not* — there, the chamfer-then-hole and no-chamfer-then-hole builds
diverge, by exactly the chamfer's own un-touched `22.5mm^3`, confirming the
methodology: when the hole does not reach the chamfer, the chamfer's
contribution survives intact; when it does, it is entirely gone).

## Reasoning

The chamfer face (and its own two new boundary edges) exists as a distinct
piece of topology in the baseline build. In the perturbed build, the hole's
own boolean cut is large enough to remove every point of material the
chamfer ever touched — confirmed not by assumption but by the control
build showing the *exact same* resulting shape whether the chamfer step
ran at all. A reference bound to "the chamfer face" (by lineage,
`generated_by` the chamfer feature) has no candidate left to resolve to
after the perturbation: the entity was not merely reshaped or split, it
was **entirely subsumed** by a later feature. A correct Stage-4 resolver
must report `explicit_broken_reference` — never silently redirect the
reference to some other nearby face (e.g. the hole's own new cylindrical
wall), which would misrepresent a deleted entity as if it still existed
under a new identity.

## Commands run (for reproduction)

```
cad build baseline.aicad  --output baseline.step  --name Corner.body --json
cad build perturbed.aicad --output perturbed.step --name Corner.body --json
```

Re-imported/checked identically to case `01` (see that case's own
"Commands run" section); the no-chamfer control variants used to derive
this case's own evidence are not checked in (they exist only to justify
the reasoning above, exactly as `AICAD-079`'s own report documents doing
for its own raw-index selection work) — `crates/cad-cli/tests/
stage4_reference_benchmark_fixtures.rs` re-proves `baseline.aicad`/
`perturbed.aicad` themselves, which is the frozen, checked-in evidence.
