# Case 04 — Pattern-count changes

**Split:** public. **Expected classification:** `correct_resolved_reference`.

## Fixtures

- `baseline.aicad` — a `25mm`-radius, `8mm`-thick disc (`part Flange`) with
  a `5`-hole bolt circle (`BOLT_COUNT = 5`, `radial_pattern(..., 360deg)`,
  `18mm` circle radius, `4mm` bolt-hole diameter).
- `perturbed.aicad` — identical except `BOLT_COUNT = 6`.

## Intended query target

A reference intended to name **bolt-pattern instance 0** — the original,
unrotated cutting tool, by *lineage* (`docs/plan/
06_REFERENCES_QUERIES_FEATURE_DAG.md` §3 item 1, "generated/modified by a
named feature," here "the un-rotated member of `bolt_pattern`"), not by a
structural/positional query like "the last hole" or "the hole nearest
angle 0" (see "Reasoning" for why that distinction matters here).

## Measured evidence

| variant | faces | edges | vertices | volume (mm^3) |
|---|---|---|---|---|
| baseline (5 holes) | 8 | 18 | 12 | 15205.31 |
| perturbed (6 holes) | 9 | 21 | 14 | 15104.78 |

Volume delta: `15205.31 - 15104.78 = 100.53 mm^3`, matching the closed-form
volume of exactly one additional `4mm`-diameter bolt hole through the
`8mm`-thick plate (`pi * 2^2 * 8 = 100.53 mm^3` — the hole tool's own
`HOLE_OVERSHOOT` extends past the plate on both ends and removes no
additional plate material there, so the *effective* per-hole volume uses
`plate_thickness`, not the tool's own longer length). This confirms exactly
one hole was added, cleanly, with no other topology disturbed.

## Reasoning

`radial_pattern(target, axis, count, angle)`'s own documented semantics
(`skills/cad-core.skill.md` §4: "the original, unrotated `target` is one of
the `count` copies") place instance 0 at a **fixed** location — `(x =
bolt_circle_radius, y = 0)` relative to the pattern axis — regardless of
`count`. This is a language/builtin-level guarantee, not something that
needs kernel introspection to confirm: changing `count` from `5` to `6`
only changes the angular spacing of the *other* instances; instance 0's own
position, and therefore the hole cut there, is unchanged by construction.

A reference bound to "pattern instance 0" by lineage should therefore
resolve to the **same physical hole** in both builds —
`correct_resolved_reference` in both. By contrast, a structural query like
"the pattern member farthest counter-clockwise from instance 0" would
correctly resolve to a *different* physical hole after the count changes
(a new member now occupies that structural role) — that is expected and
correct behavior for a *different* kind of reference, not evidence against
this case's own lineage-based target. This case deliberately picks the
lineage-based target specifically because `docs/plan/06...` §11 documents
`lineage` durability as one of the two strongest levels (`explicit` and
`lineage`), and pattern-count changes are exactly the perturbation category
where a lineage-based reference should demonstrably survive, in contrast to
weaker `query_geometric`-level references that a later, harder case in this
same perturbation family might not.

## Commands run (for reproduction)

```
cad build baseline.aicad  --output baseline.step  --name Flange.body --json
cad build perturbed.aicad --output perturbed.step --name Flange.body --json
```

Re-imported/checked identically to case `01`.
