# Case 03 — Symmetric candidates (HELD OUT)

**Read `held_out/HELD_OUT_README.md` before using this case.**

**Split:** held out. **Expected classification:** `explicit_ambiguity`.

## Fixtures

- `baseline.aicad` — a `60mm x 30mm x 8mm` plate (`part Plate`), two
  through-holes (`5mm` diameter): "left" at `x = 15mm` (`15mm` from the
  plate's own mid-plane at `x = 30mm`) and "right" at `x = 50mm` (`20mm`
  from mid-plane) — deliberately asymmetric.
- `perturbed.aicad` — identical except the right hole moves to `x = 45mm`
  (now also exactly `15mm` from mid-plane) — the two holes are now exactly
  symmetric about the plate's own `x = 30mm` mid-plane.

## Intended query target

A reference intended to name **the hole nearest the plate's own
mid-plane**: `nearest_to(x = 30mm plane); unique()`.

## Measured evidence

| variant | faces | edges | vertices | volume (mm^3) |
|---|---|---|---|---|
| baseline (asymmetric: `15mm`/`20mm`) | 8 | 18 | 12 | 14085.84 |
| perturbed (symmetric: `15mm`/`15mm`) | 8 | 18 | 12 | 14085.84 |

Both builds have identical topology and volume (as expected — moving a
hole `5mm` along `x` with no interaction with anything else changes
nothing but its own position, and the two holes are the same diameter in
both builds). The distances themselves (`15mm` vs. `20mm` baseline;
`15mm` vs. `15mm` perturbed) are given directly by the fixture's own
declared `param` values, not something requiring kernel introspection to
confirm.

## Reasoning

Baseline: the left hole (`15mm` from mid-plane) is strictly closer than the
right hole (`20mm`) — `nearest_to(...)` has exactly one correct answer,
`correct_resolved_reference` is the right *baseline* outcome (recorded here
for contrast; this case's own frozen "expected classification" above
describes the **perturbed** build, per this benchmark's own convention of
naming the perturbation's effect).

Perturbed: both holes are now **exactly** equidistant from the mid-plane
— there is no correct single answer to "nearest," by construction, not by
measurement noise. This is `docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md`
§5's own headline risk in its purest form: a resolver that breaks the tie
arbitrarily (e.g. "whichever the kernel happens to enumerate first") would
produce `silent_wrong_resolution` exactly half the time from an author's
perspective, since either "half" is indistinguishable from the query's own
stated criteria. A correct Stage-4 resolver must report
`explicit_ambiguity`, naming both candidates, rather than ever picking one
— this is precisely `AGENTS.md`'s own non-negotiable ("ambiguity is an
error, never an arbitrary selection") stated as a concrete, buildable test
case.

## Commands run (for reproduction)

```
cad build baseline.aicad  --output baseline.step  --name Plate.body --json
cad build perturbed.aicad --output perturbed.step --name Plate.body --json
```

Re-imported/checked identically to the public cases (see
`public/01_topology_split_merge/case.md`'s own "Commands run" section).
