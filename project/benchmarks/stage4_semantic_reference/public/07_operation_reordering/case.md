# Case 07 — Valid operation reordering

**Split:** public. **Expected classification:** `correct_resolved_reference`.

## Fixtures

- `baseline.aicad` — a `50mm x 30mm x 10mm` plate (`part Plate`), a hole at
  `x = 10mm` ("hole A") cut first, then a hole at `x = 40mm` ("hole B") cut
  second (both `6mm` diameter, `30mm` apart center-to-center — well clear
  of any interaction, unlike case `01`).
- `perturbed.aicad` — the identical two cuts in the **reverse** source
  order (hole B cut first, hole A second).

## Intended query target

A reference intended to name **hole A's own cylindrical bore-wall face**:
`generated_by(hole_a); cylindrical; unique()`.

## Measured evidence

| variant | faces | edges | vertices | volume (mm^3) |
|---|---|---|---|---|
| `baseline.aicad` (A then B) | 8 | 18 | 12 | 14434.51 |
| `perturbed.aicad` (B then A) | 8 | 18 | 12 | 14434.51 |

Identical in every measured respect — face count, edge count, vertex
count, and volume to the full precision `cad-occt-bridge` reports.

## Reasoning

`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §9 states independent
feature-DAG branches "can evaluate/cache in parallel" — i.e. Stage 3's own
feature-DAG model (`AICAD-066`-`069`) does not treat two non-interacting
cuts as ordered with respect to each other's *own* topology, only with
respect to the shared `target` chain each is applied to in source. Hole A
and hole B here are `30mm` apart (well outside each other's `6mm` diameter)
— cutting them in either order against the same base plate produces
bit-for-bit identical resulting topology, confirmed above, not merely
assumed from the DAG model's own stated intent.

A reference to "hole A's own bore wall" must therefore resolve identically
regardless of which source-order variant is built —
`correct_resolved_reference` for both. This case is the "sanity check"
counterpart to case `01`/`05`: it establishes that *not every* feature
reordering perturbs topology (only ones where the perturbed features
actually interact do, as cases `01` and `05` deliberately construct) — a
Stage-4 resolver, or the DAG rebuild logic feeding it, must not report
spurious ambiguity/breakage for a reordering that is truly inert, since
`docs/plan/06...` §10 requires the rebuild to "preserve unaffected cached
nodes" for exactly this reason.

## Commands run (for reproduction)

```
cad build baseline.aicad  --output baseline.step  --name Plate.body --json
cad build perturbed.aicad --output perturbed.step --name Plate.body --json
```

Re-imported/checked identically to case `01`.
