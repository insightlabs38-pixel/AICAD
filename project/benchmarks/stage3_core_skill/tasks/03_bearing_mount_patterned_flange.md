# Task 3 — Bearing mount / patterned flange

## Prompt

Design a square bearing-mount plate (60mm x 60mm x 8mm) with a raised
cylindrical boss on top (15mm radius, 10mm tall), a central through-bore
(12mm diameter) through both the boss and the plate, and an evenly spaced
6-hole bolt circle (4mm diameter holes, 22mm radius from the plate
center) cut through the plate. Round one exterior top edge of the plate.

## Reference solution

`examples/plates/stage3_bearing_mount.aicad` — part `BearingMount`, named
output `body`. (Also includes one small keyway pocket near a corner,
demonstrating `pocket` — not required by the prompt above, but a valid
elaboration a candidate solution may or may not include.)

## Verify

```
cad build examples/plates/stage3_bearing_mount.aicad --output mount.step --name BearingMount.body --json
```

Expect `"status": "ok"`, empty `"diagnostics"`, one artifact.

## Sanity checks a candidate solution should pass

- Re-imports as one valid, manifold solid.
- The bolt-circle holes are evenly spaced (60° apart for 6 holes) and none
  intersects the boss or the central bore (bolt-circle radius strictly
  greater than the boss radius plus the bolt-hole radius).
- The central bore passes fully through both the boss and the plate
  (no residual material at the bore's own axis).

Proven by `crates/cad-cli/tests/stage3_ordinary_parts.rs`'s
`bearing_mount_body_builds_to_a_valid_solid`.

## Note on task-tier correspondence

This single reference solution answers both `docs/plan/
11_AI_NATIVE_SKILLS_AND_AGENT_PROTOCOL.md` §10's "bearing mount" and
"patterned flange" benchmark items — see `../README.md`'s own note on
why no second, near-duplicate file was added.
