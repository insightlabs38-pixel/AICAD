# Task 5 — Fix a compiler error

## Prompt

The following program fails to build:

```
part SimplePlate {
    param width: Length = 50mm;
    param depth: Length = 30mm;
    param thickness: Length = 5mm;

    let margin: Length = width + thickness * thickness;

    let body: Geometry = box(width, depth, thickness);
}
```

Run `cad build` on it, read the diagnostic it reports, and fix the source
so it builds cleanly with zero diagnostics — without changing `width`,
`depth`, or `thickness`'s own declared values, and without removing the
`margin` binding.

## Fixture files

- `../broken/simple_plate_broken.aicad` — the broken program above.
- `../broken/simple_plate_fixed.aicad` — one valid fix (not the only
  correct one).

## Verify

```
cad build project/benchmarks/stage3_core_skill/broken/simple_plate_broken.aicad --json
```

Expect `"status": "failed"`, one diagnostic with code `UNIT-E110`
(`DIMENSIONAL_ARITHMETIC_ERROR`, the "computed dimension does not match
the expected type" case) naming the `thickness * thickness` sub-
expression.

```
cad build project/benchmarks/stage3_core_skill/broken/simple_plate_fixed.aicad --output plate.step --name SimplePlate.body --json
```

Expect `"status": "ok"`, empty `"diagnostics"`, one artifact — proving the
reference fix actually builds, not merely that it "looks" fixed.

Proven by `crates/cad-cli/tests/stage3_ordinary_parts.rs`'s
`fix_compiler_error_task_broken_fixture_fails_with_the_expected_diagnostic`
and `..._fixed_fixture_builds_cleanly`.
