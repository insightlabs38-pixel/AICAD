# Case 11 — Extrusion resize (resolver-execution corpus extension)

**Split:** n/a — this directory is not part of the frozen `AICAD-079A`
corpus (see `../README.md`). **Expected classification:**
`correct_resolved_reference`.

## Fixtures

- `baseline.aicad` — a `20mm x 20mm x 20mm` box's own top face
  (`extrude`'s `face` argument `5`, empirically confirmed by
  `AICAD-096`'s own probe to be the `+Z`-normal face of `box(dx, dy,
  dz)`) extruded `10mm` further along `+Z` (`part Boss`). The base box is
  deliberately **inlined**, never bound to its own `let` — see the
  fixture's own header comment for why: a separate `base` binding would
  leave its own `+Z` top face permanently resolvable alongside `body`'s
  own (`cad_cli::ParametricBuildSession::candidates`'s already-established
  "every top-level binding's own shape stays live" design), making the
  query below ambiguous by construction rather than by the perturbation
  this case means to test.
- `perturbed.aicad` — identical except `extrude_distance = 16mm`.

Empirically, `extrude(target, face, direction, distance)` here replaces
`target` with the swept volume between the selected face's own plane and
`distance` further along `direction` (not a boolean union with `target`) —
confirmed by the measured evidence below, not assumed.

## Intended query target

A reference intended to name **the boss's own outward end cap**: `planar;
normal ~= +Z; unique()` — pure geometry, no lineage needed (`extrude` is
not one of the five lineage-capable ops `cad_cli::reference_replay`
classifies, so a `generated_by` query has no evidence source for this
feature regardless of the separate `part`-scoping limitation
`project/reports/AICAD-096.md` records).

## Measured evidence

Checked directly against the live `Shape`
`cad_cli::ParametricBuildSession` produces (`crates/cad-cli/tests/
stage4_resolver_execution.rs`'s own `new_fixtures_build_to_their_recorded_
measured_evidence` test):

| variant | faces | edges | vertices | volume (mm^3) |
|---|---|---|---|---|
| baseline (`extrude_distance = 10mm`) | 6 | 12 | 8 | 4000.0 |
| perturbed (`extrude_distance = 16mm`) | 6 | 12 | 8 | 6400.0 |

Closed-form cross-check: `base_x * base_y * extrude_distance` —
`20 * 20 * 10 = 4000` / `20 * 20 * 16 = 6400`, matching exactly (the swept
volume is a plain box of the extruded face's own footprint times the
extrusion distance).

## Reasoning

`extrude`'s own outward end cap is always the *only* face in this
fixture's own topology whose normal is `+Z` and which is `Planar` — the
five side faces are vertical planes (`Normal` is horizontal), and the
inward end (the plane the extrude started from) has normal `-Z`, not
`+Z`. Growing `extrude_distance` moves this face further along `+Z` and
changes the enclosed volume, but never changes *which* face satisfies
`planar; normal ~= +Z` — a query bound to it before the perturbation
should still resolve to exactly one candidate afterward:
`correct_resolved_reference`.

## Resolver-execution result

`crates/cad-cli/tests/stage4_resolver_execution.rs`'s own
`case11_extrusion_resize_resolves_correctly_in_both_variants` test runs
the real `cad_query::resolve` resolver (via `cad_cli::
ParametricBuildSession::resolve`) against both real builds with exactly
the query above and asserts `Resolved(1)` in both — this case's own
`correct_resolved_reference` ground truth, reproduced by real, current
Stage-4 production code, not asserted from prose alone.
