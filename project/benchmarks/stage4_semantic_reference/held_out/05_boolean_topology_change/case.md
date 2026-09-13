# Case 05 — Boolean topology changes (HELD OUT)

**Read `held_out/HELD_OUT_README.md` before using this case.**

**Split:** held out. **Expected classification:** `explicit_ambiguity`.

## Fixtures

Two `30mm x 20mm x 10mm` blocks, `A` (`x` in `[0, 30]`) and `B` (`x` in
`[30, 60]`), sharing the seam at `x = 30mm`; a single `10mm`-diameter
through-hole is cut centered exactly on that seam (`x = 30mm, y = 10mm`),
straddling both blocks.

- `baseline.aicad` (`cut after union`) — `union(A, B)` first, then the
  hole is cut once from the combined solid.
- `perturbed.aicad` (`cut before union`) — the hole is cut from `A` and
  from `B` **independently** (same axis/diameter against each block's own,
  still-separate, full geometry), and the two already-holed halves are
  unioned together afterward.

Both orderings are individually "valid" ordinary Safe CAD source — nothing
about either program is malformed — and both produce a geometrically
equivalent-volume solid (see below). This is deliberately *not* the same
shape as case `01`/`07`: those hold volume identical because the perturbed
features truly do not interact; this case's two features (the hole cut,
and the union) *do* interact (the hole straddles the union seam), and the
reordering changes which operation "sees" the seam first.

## Intended query target

A reference intended to name **the bore's own cylindrical wall face**:
`generated_by(the hole feature); cylindrical; unique()`.

## Measured evidence

| variant | faces | edges | vertices | volume (mm^3) |
|---|---|---|---|---|
| baseline (cut after union) | 11 | 29 | 18 | 11214.60 |
| perturbed (cut before union) | 13 | 31 | 18 | 11214.60 |

Volumes are **identical** (as expected — boolean set operations are
volume-associative regardless of order), but face count differs by
**exactly 2** and edge count by **exactly 2**, with vertex count unchanged.
This is real, measured kernel behavior, not an assumption: cutting the same
cylindrical tool from each half independently (before the seam is healed by
`union`) leaves the resulting cavity wall as **two** half-cylindrical
face fragments (one contributed by each original block) that a
cut-after-union build merges into fewer, differently-shaped faces around
the same seam.

## Reasoning

A query written as `generated_by(the hole feature); cylindrical;
unique()` finds exactly the single expected candidate in the
`baseline.aicad` build's own topology, but the *same* logical feature (cut
the same tool, at the same location, from the same nominal target) yields
**more** cylindrical face candidates in `perturbed.aicad` purely because
of an operation-ordering difference that produces an identical *volume*
but different *topology*. A resolver that assumes "same feature intent,
same reference behavior regardless of build-order internals" will silently
break on this case (`silent_wrong_resolution` in either of two ways: either
it picks one of the two fragments and misses the other, or — if it happens
to be order-aware for the wrong reason — it reports success on one build
order and failure on the other, without the caller ever having changed
their own query). The correct Stage-4 behavior is `explicit_ambiguity`
(the query's own `unique()` expectation is genuinely violated in the
`perturbed.aicad` build, and a resolver must say so) — or, if the resolver
is sophisticated enough to recognize both fragments as descendants of the
same single hole feature (`docs/plan/06...` §8's own "a semantic reference
may intentionally refer to the set" allowance), a `correct_resolved_reference`
to the *set* of both fragments would also be an acceptable, documented
Stage-4 design choice; what is never acceptable is silently picking one
fragment as if it were the query's single, unique answer.

## Commands run (for reproduction)

```
cad build baseline.aicad  --output baseline.step  --name Seam.body --json
cad build perturbed.aicad --output perturbed.step --name Seam.body --json
```

Re-imported/checked identically to the public cases.
