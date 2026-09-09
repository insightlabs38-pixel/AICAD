# 08 — Unified Constraints, Executable Requirements, Contracts, and CAD Tests

## 1. Core idea

Traditional CAD often separates sketch constraints, assembly mates, and engineering checks into unrelated systems. This platform should use a common conceptual constraint layer, while allowing specialized solvers underneath.

## 2. Constraint domains

1. **Algebraic parameter constraints** — equations/inequalities among parameters.
2. **2D sketch constraints** — geometric relations among sketch entities.
3. **3D geometric constraints** — relations among faces/axes/frames.
4. **Assembly constraints** — mates/joints/DOF.
5. **Engineering requirements** — measurable pass/fail conditions.
6. **Optimization constraints** — feasible-region constraints.

They share syntax and diagnostics but may use different numerical solving strategies.

## 3. Constraint syntax

```aicad
constraint {
    width >= 20mm;
    hole_spacing == width - 2 * margin;
    parallel(faceA, faceB);
    clearance(pcb, housing) >= 1.5mm;
}
```

## 4. Constraint representation

Normalize into a constraint IR containing:

```text
id
source span
domain
variables/references
relation type
expression/evaluator
tolerance
priority/strength
scope
status
solver metadata
```

## 5. Hard vs soft constraints

Support:

```text
hard    must be satisfied
soft    optimization preference
```

Example:

```aicad
hard clearance(pcb, housing) >= 1.5mm;
soft mass(housing) <= 250g weight=0.5;
```

Avoid silently violating hard engineering requirements.

## 6. Solver diagnostics

The solver must provide minimal or approximate conflict sets when possible:

```text
CONSTRAINT-E041 UNSATISFIABLE

Conflicting constraints:
  C12: distance(A,B) == 20mm
  C14: distance(A,B) == 25mm

No solution exists within tolerance 1um.
```

For underconstrained geometry:

```text
Sketch has 3 remaining DOF:
  endpoint P4: X translation
  endpoint P4: Y translation
  line L7: rotation
```

## 7. Executable requirements

Requirements have identity, description, rationale, severity, and executable assertions.

```aicad
requirement PCB_CLEARANCE {
    description: "Maintain PCB-to-wall clearance";
    severity: error;
    assert clearance(pcb, housing) >= 1.5mm;
}
```

Recommended fields:

| Field | Type | Meaning |
|---|---|---|
| `id` | String | Stable requirement ID |
| `description` | String | Human requirement |
| `rationale` | String? | Why it exists |
| `source` | URI/String? | External spec link/reference |
| `severity` | enum | info/warn/error/blocker |
| `owner` | String? | Team/discipline, not necessarily person |
| `assert` | expression | Executable condition |
| `applies_to` | query/config set | Scope |
| `verification` | enum | geometry/test/simulation/inspection/manual |
| `status` | derived | pass/fail/unknown/not-run |

## 8. CAD unit tests

```aicad
test "housing is manifold" {
    assert manifold(housing);
}

test "four mounting holes" {
    expect mounting_holes.count() == 4;
}

test "USB clearance" {
    assert clearance(usb_connector, cutout) >= 0.4mm;
}
```

## 9. Test categories

- syntax/type tests;
- geometry validity;
- semantic reference health;
- dimensional properties;
- topology counts/relations;
- collision/clearance;
- configuration matrix;
- assembly DOF;
- manufacturing rules;
- solver/simulation output;
- drawing/PMI completeness;
- regression/snapshot tests.

## 10. Geometry assertions

Baseline assertion library:

```text
valid(shape)
manifold(shape)
closed(shape)
volume(shape) relation value
mass(shape) relation value
area(ref) relation value
length(ref) relation value
min_wall(shape) relation value
clearance(a,b) relation value
interference(a,b).is_empty()
count(query) relation integer
parallel(a,b)
perpendicular(a,b)
concentric(a,b)
inside(a,b)
contains(a,point)
connected(a,b)
```

## 11. Approximate assertions

```aicad
expect measured ~= 20mm within 0.01mm;
```

Never require users to compare floating geometry with exact `==` unless the values are symbolic/constructed exactly enough for the operation.

## 12. Parameterized tests

```aicad
for cfg in Product.valid_configurations() {
    test_config(cfg) {
        assert valid(build(cfg));
        assert mass(build(cfg)) < 2kg;
    }
}
```

## 13. Requirement traceability

Every requirement result should link to:

```text
requirement source
source-code assertion
semantic geometry referenced
build configuration
analysis/test result
produced evidence artifacts
commit/build ID
```

This enables model-based engineering traceability.

## 14. Contracts

Functions/features can declare pre/postconditions:

```aicad
fn bearing_mount(b: Bearing) -> Part
requires b.outer_diameter > 0mm
ensures concentric(result.bore_axis, b.axis)
ensures valid(result)
{
    ...
}
```

Contracts are checked at build/test levels chosen by profile.

## 15. Invariants

Parts/types can define invariants:

```aicad
part Housing {
    invariant min_wall(self) >= 1.5mm;
    invariant valid(self);
}
```

Useful for reusable package components.

## 16. Build profiles

Not every build should run expensive analysis.

Profiles:

```text
fast       parse/type/geometry/basic validity
standard   + tests + reference health + collision basics
release    + all required DFM/PMI/analysis gates
audit      + deterministic evidence/provenance bundle
```

## 17. Test result schema

Each result includes:

```text
id
status: pass/fail/error/skipped/unknown
expected
observed
units
tolerance
entities involved
source span
diagnostic code
evidence artifact refs
duration/resource usage
```

## 18. AI repair loop

Tests are a primary AI control mechanism:

```text
spec -> source -> build -> tests -> structured failures -> patch -> rebuild
```

The agent should not declare success solely because geometry renders.

## 19. New feature: verification coverage

Add a coverage report analogous to software test coverage, but semantic rather than line-based:

```text
critical parameters covered by requirements: 92%
semantic interfaces covered: 100%
safety-critical parts with release assertions: 100%
AI-created features with at least one verification path: 96%
```

This is not a mathematical proof of correctness, but it helps organizations see unverified design areas.
