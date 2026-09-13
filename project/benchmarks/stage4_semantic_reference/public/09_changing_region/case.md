# Case 09 — Changing sketch regions

**Split:** public. **Expected classification:** `correct_resolved_reference`.

## Fixtures

- `baseline.aicad` — a `40mm x 40mm x 10mm` plate (`part Plate`), an
  `8mm x 8mm x 3mm` rectangular pocket centered at `(x = 16mm, y = 16mm)`
  on the top face — fully contained within the top face's own boundary,
  clear of every edge.
- `perturbed.aicad` — identical except the pocket footprint grows to
  `16mm x 20mm` (still centered the same way, still fully contained within
  the top face — verified below by unchanged topology).

**Proxy note:** there is no `Sketch`/`Profile` source-level type yet
(`skills/cad-core.skill.md` §10 — the internal constraint-solver/profile-
lowering machinery `AICAD-072`-`075` built has no grammar/lowering
integration into `.aicad` source). This case models "a sketch region
changing shape" the closest way expressible in Stage-3 source today: a
single `pocket` feature's own footprint (its closest analog to a 2D
profile region) changes size while remaining the same feature. The
observable question this case actually tests — does a reference survive a
continuous change to a region's own boundary curve/area, without any
change in *which* topological entities exist — is the same question a real
sketch-region perturbation would raise; only the concrete mechanism used
to vary the region differs, and that difference is noted here rather than
silently assumed away.

## Intended query target

A reference intended to name **the pocket floor face created by
`pocket(...)`**: `generated_by(the pocket feature); planar; unique()`.

## Measured evidence

| variant | faces | edges | vertices | volume (mm^3) |
|---|---|---|---|---|
| baseline (`8mm x 8mm` pocket) | 11 | 24 | 16 | 15808.00 |
| perturbed (`16mm x 20mm` pocket) | 11 | 24 | 16 | 15040.00 |

Face, edge, and vertex counts are **identical** between the two builds —
only the pocket's own area (and therefore the plate's remaining volume)
changed. Volumes match their respective closed forms exactly: baseline
`40*40*10 - 8*8*3 = 16000 - 192 = 15808 mm^3`; perturbed `16000 - 16*20*3 =
16000 - 960 = 15040 mm^3`.

## Reasoning

Unlike cases `01`/`02`/`05` (where a perturbation changes *which* faces
exist), this perturbation changes only the pocket floor face's own
boundary curve and area, while the feature graph's own topology count is
provably unchanged (confirmed above, not assumed). A reference to "the
pocket floor face," bound by lineage to the pocket feature, should
therefore resolve to the **same face** in both builds —
`correct_resolved_reference` in both, even though that face's own concrete
geometry (its boundary, its area) is different each time. This is the
"positive control" this benchmark needs alongside its breakage/ambiguity
cases: `docs/plan/06...` §10 explicitly requires "preserve unaffected
cached nodes" and "replay semantic-reference resolution" on every parameter
edit, and a resolver that treated *every* geometric change as grounds to
re-resolve from scratch (or, worse, to report ambiguity/breakage whenever a
face's own shape changes at all) would fail this case despite the
reference being unambiguously trackable.

## Commands run (for reproduction)

```
cad build baseline.aicad  --output baseline.step  --name Plate.body --json
cad build perturbed.aicad --output perturbed.step --name Plate.body --json
```

Re-imported/checked identically to case `01`.
