# 03 — Type System, Units, Tolerances, and General Programming Model

## 1. Type-system objective

The type system should catch engineering mistakes before geometry evaluation whenever possible. It must remain learnable enough that one compact skill can teach a general coding model normal usage.

## 2. Primitive types

Required ordinary types:

```text
Bool
Int
UInt
Float
Decimal (optional exact decimal for standards/data)
String
Bytes
Char (optional)
```

Use `Float` for numerical algorithms but avoid silently treating dimensional quantities as bare floats.

## 3. Dimensional quantity types

Minimum first-class dimensions:

```text
Length
Area
Volume
Angle
Time
Mass
Temperature
Force
Torque
Pressure
Stress
Energy
Power
Density
Velocity
Acceleration
AngularVelocity
Frequency
Current
Voltage
Resistance
```

Derived dimensional algebra must be checked:

```aicad
let area: Area = width * height;
let stress: Stress = force / area;
```

Invalid:

```aicad
let width: Length = 5kg;
```

## 4. Unit literals

Initial units should include common SI and common mechanical imperial units.

### Length

`nm`, `um`, `mm`, `cm`, `m`, `km`, `in`, `ft`

### Angle

`deg`, `rad`

### Mass

`mg`, `g`, `kg`, `lbm`

### Force

`N`, `kN`, `lbf`

### Pressure/stress

`Pa`, `kPa`, `MPa`, `GPa`, `psi`, `ksi`

### Temperature

`K`, `degC`, `degF` with affine-temperature semantics handled carefully.

The standard library can expand the set without changing grammar.

## 5. Quantity representation

Internally normalize quantities to canonical units plus dimension exponents. Preserve source/display units separately so diagnostics can use the engineer's chosen units.

A quantity should contain conceptually:

```text
canonical_value
unit_dimension
preferred_display_unit
precision/uncertainty metadata (optional)
```

## 6. Tolerances

Tolerance types should be first-class rather than annotations on strings.

```aicad
let shaft: Tolerance<Length> = 10mm +/- 0.01mm;
let slot: AsymmetricTolerance<Length> = 20mm +0.10mm -0.00mm;
let wall: Range<Length> = 1.5mm..3mm;
```

Core tolerance structures:

| Type | Fields |
|---|---|
| `Tolerance<T>` | nominal, plus, minus |
| `Range<T>` | min, max, inclusive flags |
| `Distribution<T>` | distribution family + parameters |
| `Fit` | standard/system, hole class, shaft class, nominal size |

## 7. Geometry types

Safe/reference-level types:

```text
Point2 Point3
Vector2 Vector3
Direction2 Direction3
Axis2 Axis3
Frame2 Frame3
Quaternion
Transform
Matrix3 Matrix4

Curve2 Curve3
Surface
Profile
Sketch
Wire
Face
Shell
Solid
Body
Part

VertexRef
EdgeRef
WireRef
FaceRef
ShellRef
SolidRef
AxisRef
FeatureRef
ComponentRef
```

Raw/ephemeral types:

```text
Vertex*
Edge*
Wire*
Face*
Shell*
Solid*
KernelShape*
```

## 8. Semantic engineering types

Built-in semantic interfaces/types should include:

```text
Hole
Thread
Fastener
Bearing
BearingSeat
Gear
Shaft
Boss
Rib
Pocket
Datum
Pattern
Material
ManufacturingProcess
SurfaceFinish
ToleranceFrame
Connector
MechanicalInterface
Joint
Mate
LoadCase
Requirement
TestResult
```

Not every semantic object needs a compiler intrinsic. Many should be standard-library structs/interfaces that lower to core geometry operations.

## 9. Collections

Required generic collections:

```text
Array<T, N>        // optionally fixed-size
List<T>
Set<T>
Map<K,V>
Optional<T>
Result<T,E>
Range<T>
Iterator<T>
Generator<T>
Graph<N,E>         // std library rather than compiler primitive if possible
```

## 10. Structs and enums

```aicad
struct HoleSpec {
    diameter: Length;
    depth: Optional<Length>;
}

enum MotorType {
    NEMA17,
    NEMA23,
}
```

Support destructuring and pattern matching.

## 11. Interfaces/traits

Interfaces make mechanical compatibility type-checkable:

```aicad
interface MotorMount {
    mounting_face: FaceRef;
    output_axis: AxisRef;
    mounting_holes: Pattern;
}
```

A component can implement one or more interfaces:

```aicad
part NEMA17 implements MotorMount { ... }
```

Use cases:

- interchangeable motors;
- bearing cartridges;
- PCB mount conventions;
- pneumatic ports;
- optical mounts;
- standardized flange interfaces.

## 12. Generics

Support ordinary generics:

```aicad
fn mount<T: MotorMount>(motor: T, chassis: Part) -> Assembly { ... }
```

Avoid template-metaprogramming complexity in v1. Favor readable bounds and explicit specialization only when needed.

## 13. Ownership/value semantics

Normal geometry objects should behave as immutable/shared value handles internally even if syntax permits ergonomic mutation. The runtime can use copy-on-write and feature-DAG nodes rather than copying geometry eagerly.

Raw topology handles are non-owning epoch-scoped pointers.

## 14. Nullability

Do not use implicit null.

```aicad
Optional<FaceRef>
```

must be handled explicitly.

## 15. Control flow

All standard control flow is required:

```aicad
if condition { ... } else { ... }
for item in items { ... }
while condition { ... }
loop { ... }
match value { ... }
break;
continue;
return value;
yield value;
```

## 16. Recursion

Allowed. Runtime budgets protect against accidental nontermination.

Useful cases:

- recursive branching channels;
- space frames;
- fractal/lattice construction;
- hierarchical assembly generation;
- recursive search over topology/assemblies.

## 17. Closures and functional operations

```aicad
let vertical = body.edges().filter(|e| e.direction ~= +Z);
let areas = body.faces().map(|f| f.area);
```

This is particularly useful for query composition and AI-generated geometry code.

## 18. Generators

```aicad
generator bolt_circle(count: Int, r: Length) -> Point2 {
    for i in 0..count {
        yield polar(r, 360deg * i / count);
    }
}
```

Generators avoid large materialized pattern lists.

## 19. Pure functions

```aicad
pure fn ...
```

Compiler guarantees no nondeterministic capabilities. Pure functions are candidates for memoization, compile-time evaluation, and parallel execution.

## 20. Unsafe blocks

Two separate unsafe concepts may be useful:

```aicad
unsafe geometry { ... }  // raw topology/kernel-grade operations
unsafe native { ... }    // trusted native plugin capability; normally package-only
```

User code should rarely require `unsafe native`.

## 21. Numerical precision policy

Do not expose a false promise of exact arithmetic for general B-rep operations. Define:

- dimensional quantities use deterministic IEEE floating point or chosen scalar representation;
- user-configurable modeling tolerance exists at project/kernel boundary;
- exact integers/rationals may be used for symbolic parameter calculations;
- geometry comparisons use explicit tolerances (`~=`, `near`, `within`) rather than equality of floating coordinates.

Example:

```aicad
face.normal ~= +Z within 0.1deg
```

## 22. Parameter metadata

Any `param` may include UI/validation metadata:

```aicad
@param(
    label="Wall thickness",
    min=1mm,
    max=8mm,
    step=0.1mm,
    group="Housing"
)
param wall: Length = 2.4mm;
```

Recommended metadata keys:

| Key | Meaning |
|---|---|
| `label` | Human-readable name |
| `description` | Documentation/help |
| `min`, `max` | Validation/UI range |
| `step` | UI step suggestion |
| `choices` | Restricted enumerated set |
| `group` | Inspector grouping |
| `advanced` | Hide in basic UI |
| `readonly` | Derived/display-only parameter |
| `unit_display` | Preferred UI unit |
| `sensitivity` | Optional optimization/tuning hint |

## 23. Rationale metadata

Support preserving why a design choice exists:

```aicad
@rationale("3 mm chosen for impact requirement R-18 and molding stability")
param wall = 3mm;
```

This becomes visible to humans and AI and feeds provenance/review tooling.
