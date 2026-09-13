# Task 2 — L bracket

## Prompt

Design an L-shaped mounting bracket: a flat base leg (60mm x 30mm x 6mm)
with an upright wall leg (6mm thick, 30mm wide, 50mm tall) rising from one
end of the base. Round one exterior top edge of the base, away from the
wall. Add one mounting hole through the base leg and one through the wall
leg (5mm diameter each), positioned so neither hole intersects the other
leg. Also produce a mirrored ("opposite hand") copy of the finished
bracket as a second named output, distinct from the first.

## Reference solution

`examples/brackets/stage3_l_bracket.aicad` — part `LBracket`, named
outputs `body` (the bracket) and `mirrored` (its opposite-hand twin).

## Verify

```
cad build examples/brackets/stage3_l_bracket.aicad --output body.step --name LBracket.body --json
cad build examples/brackets/stage3_l_bracket.aicad --output mirrored.step --name LBracket.mirrored --json
```

Expect `"status": "ok"`, empty `"diagnostics"`, one artifact each. Note
`--name LBracket` alone (no `.field`) is expected to **fail** here — the
part exposes two `Geometry` fields, so it is genuinely ambiguous; a
correct solution (and a correct benchmark runner) must name one
explicitly, exactly the case `crates/cad-cli/src/build.rs`'s
`NamedOutputError::AmbiguousFields` exists to catch.

## Sanity checks a candidate solution should pass

- Both `body` and `mirrored` independently re-import as valid, manifold
  solids strictly more complex than a bare box (`face_count() > 6`,
  `edge_count() > 12`, `vertex_count() > 8`).
- `mirrored`'s bounding box is the mirror image of `body`'s own across the
  chosen mirror plane.

Proven by `crates/cad-cli/tests/stage3_ordinary_parts.rs`'s
`l_bracket_body_builds_to_a_valid_solid`,
`l_bracket_mirrored_output_also_builds_to_a_valid_solid`, and
`l_bracket_body_is_ambiguous_without_a_field_name`.
