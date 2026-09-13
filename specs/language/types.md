# Type system

Status: current canonical Stage-3 type-system boundary. This document records approved semantics; it does not promote future planning types into the current source language.

## Primitive and dimensional values

AICAD distinguishes ordinary primitive values from dimensioned engineering quantities. Current primitive type identity includes `Bool`, `Int`, `UInt`, `Float`, `Decimal`, `String`, `Bytes`, and `Char` in the semantic type layer. Dimensioned quantities are typed values, not untyped floats.

The established named dimensions are `Length`, `Area`, `Volume`, `Angle`, `Time`, `Mass`, `Temperature`, `Force`, `Torque`, `Pressure`, `Stress`, `Energy`, `Power`, `Density`, `Velocity`, `Acceleration`, `AngularVelocity`, `Frequency`, `Current`, `Voltage`, and `Resistance`. Derived-dimension arithmetic is structurally canonicalized. Distinct named dimensions that share the same exponent vector or unit spellings are not silently conflated; ambiguity is diagnosed rather than arbitrarily resolved.

Implicit unit conversion is permitted only within compatible dimensions under the approved unit/type rules. Current registered literal families include length, angle, mass, force, pressure/stress, and affine temperature units; the unit registry may expand independently of grammar because a numeric token's unit suffix is validated semantically rather than fixed by lexical grammar.

## Affine quantities

Affine dimensions distinguish **absolute** from **delta** quantities (D4/DL-3). Temperature is the current affine dimension. Unit offsets apply to absolute conversions but not to deltas: a temperature difference converts by scale only. Arithmetic must preserve the absolute/delta distinction; for example, subtracting two absolute affine quantities yields a delta rather than another absolute value.

## User-defined and generic types

Current source semantics support structs, enums, and ordinary generic type parameters on structs, enums, and functions under D17. Enum variants may be unit variants, positional tuple variants, or named record variants. Generic parameters are compile-time type parameters; current syntax has no interface/trait bounds, defaults, variance, higher-kinded types, specialization, dependent types, or lifetime system.

`Result<T,E>` and `Optional<T>` are ordinary generic prelude enums built from the same language machinery available to user code. They do not receive hidden compiler-only type semantics.

## Collections and iteration

D16 authorizes the Stage-2 minimum:

- immutable `List<T>` values constructible with list literals;
- `Range<Int>` and `Range<UInt>` values from half-open `start..end` and inclusive `start..=end` expressions, automatically iterable in deterministic ascending-by-one order;
- `Iterator<T>` as an internal/runtime iteration abstraction, not a user-visible compiler-magic protocol.

`Set<T>`, `Map<K,V>`, comprehensions, arbitrary user-defined iterator protocols, async/parallel iteration, and automatic dimensional-range stepping are not current canonical language capabilities.

## Geometry and spatial types

`Geometry` is an opaque AICAD source value identifying geometry construction in the backend-neutral execution model. It is not an OCCT shape, topology pointer, or persistent topology reference.

The current standard nominal geometry/spatial source environment contains `Vector2<T>`, `Vector3<T>`, `Point2`, `Point3`, `Axis3`, `Frame3`, and `Plane`. These types are always available with the current RuntimeBuiltin environment under D20/DL-21; AICAD-076's temporary scalar-decomposition workaround is not the canonical long-term pattern.

The accepted AICAD-075A spatial semantics remain kernel-neutral:

- a point represents position; a vector represents displacement/direction magnitude and the two are not interchangeable merely because both have three coordinates;
- a direction is a validated normalized vector in the kernel-neutral semantic layer and rejects degenerate/non-finite input rather than inventing an axis;
- `Axis3` combines an origin point and direction;
- `Frame3` is an orthonormal, right-handed frame;
- `Plane`/kernel-neutral plane semantics represent an origin plus normal and are not the same identity as a frame, because multiple in-plane frame orientations describe the same plane;
- `Transform` semantics are proper rigid rotation+translation, use the right-hand rule for positive rotation, and do not represent reflection/scale.

Not every internal kernel-neutral value necessarily has a separate direct source constructor. Internal semantic/IR capability and source-language constructibility are separate questions.

## Sketch/reference identity is not topology identity

Stage-3 sketch/entity IDs, constraint IDs, parameter/binding IDs, feature IDs, provenance, named model outputs, GeometryGraph IDs, and raw face/edge indices each serve local semantic/execution roles. None is automatically a persistent Stage-4 `VertexRef`/`EdgeRef`/`WireRef`/`FaceRef`/`ShellRef`/`SolidRef`.

The current source language exposes no direct `sketch { ... }` construct and no persistent semantic topology-reference types. Stage-4 durable reference resolution remains future implementation governed by D7's fail-closed policy.

## RuntimeBuiltin type closure — D20/DL-21

D20 is **resolved** by DL-21 and implemented by AICAD-076A. Approved AICAD standard nominal types such as `Point3`, `Axis3`, `Frame3`, and `Plane` may appear in always-seeded RuntimeBuiltin signatures.

The always-seeded builtin environment must be type-closed: every nominal type required by an automatically seeded builtin signature is automatically available to signature collection and type checking. Eager signature collection/type checking remains authoritative, and the builtin catalogue together with its required standard type declarations must be independently type-validatable with zero diagnostics against an otherwise-empty program.

`with_geometry_types` may remain as an idempotent compatibility/composition helper; callers are not required to invoke it to make automatically seeded builtin signatures type-correct. AICAD-076's scalar-decomposition signatures were temporary compatibility workarounds, not the intended long-term Safe CAD API architecture.

This is a closed first-party standard environment. It does not authorize arbitrary plugin/runtime type injection, host callbacks, OCCT classes in public/HIR signatures, or dynamic extension of `BuiltinFnId` from AICAD source.

## Tolerance terminology

Do not treat every numerical threshold as one global epsilon.

- **D5 equivalence comparison profile** — versioned geometry-comparison policy. The current v1 defaults are: `linear_abs = 1e-4 mm`, `linear_rel = 0`, `area_abs = 1e-6`, `area_rel = 1e-3`, `volume_abs = 1e-6`, `volume_rel = 1e-3`, `center_of_mass_abs = 1e-4 mm`; effective linear/area/volume tolerances scale according to DL-12's `max(abs, rel*S^n)` rules.
- **Solver convergence tolerance** — numerical convergence/control for a solver implementation; the Stage-3 sketch solver owns its own profile and it is not D5.
- **Modeling/construction tolerance** — operation/kernel construction policy where required; no new general source default is defined by this specification cleanup.
- **Approximation tolerance** — operation-specific approximation/discretization policy; future defaults remain unresolved where not already defined.
- **Verification/assertion tolerance** — tolerance explicitly attached to an engineering assertion/test/evidence context; it does not inherit D5 automatically.
- **Private representation-validity thresholds** — local implementation guards such as normalization/frame validity thresholds; they are not public modeling/equivalence defaults merely because they exist in code.

A value approved for one category must not silently become the default for another category.
