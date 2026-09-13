# Case 01 — Topology splits/merges

**Split:** public. **Expected classification:** `explicit_ambiguity`.

## Fixtures

- `baseline.aicad` — a `60mm x 30mm x 10mm` plate (`part Wall`) with two
  separate through-holes (`hole_diameter = 6mm`) centered at
  `x = 15mm` (`hole_center_a`) and `x = 45mm` (`hole_center_b`), `y =
  15mm` (mid-width) — `30mm` apart, edge-to-edge gap `24mm`, no interaction.
- `perturbed.aicad` — identical except `hole_center_b = 20mm` (`5mm` from
  `hole_center_a`, i.e. the two `6mm`-diameter (`3mm`-radius) bores now
  overlap by `1mm`).

## Intended query target

A reference intended to name **hole A's own cylindrical bore-wall face**:
`generated_by(hole_a); cylindrical; unique()` (`docs/plan/
06_REFERENCES_QUERIES_FEATURE_DAG.md` §6 predicate vocabulary — prose only,
no such syntax exists in this repository).

## Measured evidence

Built with the real `cad build` pipeline, re-imported through an
independent `cad-occt-bridge::OcctContext`:

| | faces | edges | vertices | volume (mm^3) |
|---|---|---|---|---|
| baseline | 8 | 18 | 12 | 17434.51 |
| perturbed | 9 | 21 | 14 | 17457.02 |

`cad build baseline.aicad --output out.step --name Wall.body --json` →
`{"status":"ok","diagnostics":[]}` (both variants).

**Volume cross-check (closed form):** baseline: `60 * 30 * 10 - 2 * (pi *
3^2 * 10) = 18000 - 565.49 = 17434.51 mm^3` — matches the measured value
exactly (holes verified non-interacting, as designed). Perturbed: the
measured volume (`17457.02`) is *greater* than baseline's, consistent with
two overlapping circular cuts removing strictly less combined area than
two disjoint ones of the same size (the overlap region is not double-
counted) — the expected direction and rough magnitude for a `1mm` overlap
of two `3mm`-radius circles, confirming the perturbation is a real,
non-degenerate topology interaction, not merely two coincidentally-similar
independent builds.

## Reasoning

Baseline: hole A's own bore wall is a single, untrimmed cylindrical face
with no interaction from hole B — the query's `unique()` expectation holds
trivially (exactly one candidate).

Perturbed: the measured face count rose from 8 to 9 (and edges 18→21,
vertices 12→14) even though only one new topological feature (an
overlapping second hole) was added — one more face than the "two
independent full cylinders" baseline shape would predict on its own. This
is the real-kernel signature of `docs/plan/06...` §8's own lineage vocabulary
("split_by"): hole A's own cavity boundary has been re-trimmed by its
intersection with hole B's cavity, so a `generated_by(hole_a); cylindrical`
query with no further discriminator will find **more than one** candidate
face descended from hole A's own boolean cut (the original full cylinder
having been divided by the new intersection curve) rather than the single
face it found in the baseline build. A correct Stage-4 resolver must
report this as `explicit_ambiguity` (or, per §8's own "a semantic reference
may intentionally refer to the set" option, resolve to the explicit *set*
of hole A's own descendant faces if the query was written to expect a set)
— never silently pick one fragment and call it "hole A's wall," which
would be exactly the `silent_wrong_resolution` outcome this benchmark
exists to catch.

## Commands run (for reproduction)

```
cad build baseline.aicad --output baseline.step --name Wall.body --json
cad build perturbed.aicad --output perturbed.step --name Wall.body --json
```

Each re-imported and checked with `cad_occt_bridge::OcctContext::
import_step` + `is_valid`/`validate`/`face_count`/`edge_count`/
`vertex_count`/`volume` (`crates/cad-cli/tests/
stage4_reference_benchmark_fixtures.rs` runs this exact check as part of
the ordinary workspace test suite).
