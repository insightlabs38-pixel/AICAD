# Case 10 — Near-degenerate but valid geometry (HELD OUT)

**Read `held_out/HELD_OUT_README.md` before using this case.**

**Split:** held out. **Expected classification:** `correct_resolved_reference`.

## Fixtures

- `baseline.aicad` — a `30mm x 20mm x 10mm` block (`part Block`), edge
  index `1` filleted with a comfortable `fillet_radius = 5mm`.
- `perturbed.aicad` — identical except `fillet_radius = 9.99mm` — `0.01mm`
  away from the real, empirically-confirmed kernel-failure threshold this
  corpus's own `public/06_fillet_viability/case.md` establishes for this
  exact edge/block shape (`fillet_radius = box_z = 10mm`).

## Intended query target

A reference intended to name **the flat top face remaining between the
fillet and the block's own far edge**: `generated_by(the base block);
planar; normal ~= +Z; unique()`.

## Measured evidence

| variant | faces | edges | vertices | volume (mm^3) |
|---|---|---|---|---|
| baseline (`fillet_radius = 5mm`) | 7 | 15 | 10 | 5892.70 |
| perturbed (`fillet_radius = 9.99mm`) | 7 | 15 | 10 | 5571.65 |

Both builds are `is_valid`/`validate`-clean exact B-reps with **identical**
topology counts — the perturbation changes only the fillet's own radius
(and therefore the flat top face's own remaining width and the block's
overall volume), never which faces exist. `perturbed.aicad` is `0.01mm`
inside the actual measured kernel-failure boundary (`public/
06_fillet_viability/case.md`'s own binary search on the same edge/block
family locates that boundary at `fillet_radius = 10mm`) — this is
deliberately not a hand-picked "should probably still work" guess; it is
the closest value to the confirmed failure boundary that this benchmark
also confirms still builds.

## Reasoning

The flat top face is still a real, single, valid piece of topology in the
perturbed build (confirmed above — same face count as baseline, real
`is_valid`/`validate` pass) — it has simply become extremely thin (its own
remaining width shrinks toward, but never reaches, zero as `fillet_radius`
approaches `10mm`). A reference bound to this face by lineage
(`generated_by` the base block, `planar`, `normal ~= +Z`) should resolve to
it just as unambiguously as in the comfortable baseline case —
`correct_resolved_reference` in both builds. This is this corpus's own
"near-degenerate" stress case specifically because a *naive* resolution
heuristic based on absolute face area or size (e.g. "ignore faces smaller
than some small threshold, they're probably tessellation noise") would
incorrectly treat the perturbed build's own now-tiny-but-still-valid face
as `explicit_broken_reference` or filter it out of `largest(area)`/
`smallest(area)` ranking entirely — a `silent_wrong_resolution` outcome
this case exists to catch. The correct ground truth is that validity, not
absolute size, is what should gate whether an entity is a legitimate
resolution candidate.

## Commands run (for reproduction)

```
cad build baseline.aicad  --output baseline.step  --name Block.body --json
cad build perturbed.aicad --output perturbed.step --name Block.body --json
```

Re-imported/checked identically to the public cases.
