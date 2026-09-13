# Case 06 — Fillet/chamfer viability changes

**Split:** public. **Expected classification:** `kernel_failure`.

## Fixtures

- `baseline.aicad` — a `20mm x 20mm x 10mm` block (`part Block`), edge
  index `1` filleted with `fillet_radius = 8mm`.
- `perturbed.aicad` — identical except `fillet_radius = 12mm`.

## Intended query target

A reference intended to name **the fillet face created by `fillet(...,
[1], fillet_radius)`**.

## Measured evidence

`baseline.aicad` builds successfully (`{"status":"ok","diagnostics":[]}`),
re-imported to a valid B-rep with `7` faces, `15` edges, `10` vertices, and
volume `3725.31 mm^3` — matching the closed form `20*20*10 - (8^2 * (1 -
pi/4)) * 20 = 4000 - 274.69 = 3725.31 mm^3` (an `8mm`-radius quarter-circle
fillet along the block's own `20mm`-long edge `1`). `perturbed.aicad`
fails at the kernel dispatch stage:

```
{"status":"failed","diagnostics":[{"code":"GEOM-E005","severity":"error",
"category":"geometry-dispatch","title":"GEOMETRY_KERNEL_OPERATION_FAILED",
"message":"%1 (Fillet) failed in the kernel: kernel backend could not
complete the operation", ...}]}
```

A binary search across `fillet_radius` values on this exact fixture
(`3, 4, 4.5, 4.9, 5, 6, 8, 9.9` mm all succeed; `10, 12, 15, 19, 19.9, 20,
25` mm all fail identically) locates the real, empirically-confirmed
viability boundary for this specific edge/block geometry at exactly
`fillet_radius = box_z = 10mm` (the block's own thickness) — consistent
with the well-known fillet-radius constraint that a round cannot exceed
the extent of the material it rounds into. `perturbed.aicad`'s
`fillet_radius = 12mm` is chosen safely past that boundary (not directly
on it), so this case does not depend on exact floating-point tolerance
behavior at the threshold itself — see case `10` for a fixture
deliberately placed *at* a viability boundary instead.

## Reasoning

This is not a semantic-reference resolution question at all: the geometry
construction itself is infeasible at the perturbed parameter value, and
the *kernel* — not any future reference resolver — is what reports the
failure. A correct Stage-4 implementation must classify this outcome as
`kernel_failure`, distinct from `explicit_broken_reference` (which means
the *build succeeded* but the referenced entity no longer exists) and
distinct from `unrelated_build_failure` (which means the fixture itself is
broken for reasons unrelated to the geometry actually attempted here — it
is not; this is exactly the geometry the case intends to attempt, and it
is expected to fail). Conflating "the kernel could not build this
perfectly reasonable-looking parameter change" with a resolver defect would
misattribute a geometry-engine limitation to the reference-resolution layer
this benchmark is scoped to.

## Commands run (for reproduction)

```
cad build baseline.aicad  --output baseline.step --json   # succeeds
cad build perturbed.aicad --output perturbed.step --json  # GEOM-E005
```

`crates/cad-cli/tests/stage4_reference_benchmark_fixtures.rs` proves
`baseline.aicad` builds to a valid B-rep and proves `perturbed.aicad`
fails with exactly `GEOM-E005` (not merely "some error") — a negative test
in the same sense `AGENTS.md`'s "add focused positive and negative tests"
calls for.
