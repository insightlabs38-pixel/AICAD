# Type system

Status: current canonical Stage-3 type-system boundary. This document records approved semantics; it does not promote future planning types into the current source language.

## Primitive and dimensional values

AICAD distinguishes ordinary primitive values from dimensioned engineering quantities. Current primitive type identity includes `Bool`, `Int`, `UInt`, `Float`, `Decimal`, `String`, `Bytes`, and `Char` in the semantic type layer. Dimensioned quantities are typed values, not untyped floats.

The established named dimensions are `Length`, `Area`, `Volume`, `Angle`, `Time`, `Mass`, `Temperature`, `Force`, `Torque`, `Pressure`, `Stress`, `Energy`, `Power`, `Density`, `Velocity`, `Acceleration`, `AngularVelocity`, `Frequency`, `Current`, `Voltage`, and `Resistance`. Derived-dimension arithmetic is structurally canonicalized. Distinct named dimensions that share the same exponent vector or unit spellings are not silently conflated; ambiguity is diagnosed rather than arbitrarily resolved.

Implicit unit conversion is permitted only within compatible dimensions under the approved unit/type rules. Current registered literal families include length, angle, mass, force, pressure/stress, and affine temperature units; the unit registry may expand independently of grammar because a numeric token's unit suffix is validated semantically rather than fixed by lexical grammar.

## Affine quantities

Affine dimensions distinguish **absolute** from **delta** quantities (D4/DL-3). Temperature is the current affine dimension. Unit offsets apply to absolute conversions but not to deltas: a temperature difference converts by scale only. Arithmetic must preserve the absolute/delta distinction; for example, subtracting two absolute affine quantities yields a delta rather than another absolute value.

## User-defined and generic types

Current source semantics support structs, enums, and ordinary generic type parameters on structs, enums, and functions under D17. Enum variants may be unit variants, positional tuple variants, or named record variants. Generic parameters are compile-time type parameters; current syntax has no defaults, variance, higher-kinded types, specialization, dependent types, or lifetime system.

## Interfaces/protocols and bounded generics — D27/DL-29 (AICAD-132)

D27/DL-29's restrained general nominal interface/protocol mechanism is implemented (`AICAD-132`), on top of D17's generic foundation rather than replacing it:

- `interface Name { field: Type, field: Type }` declares a nominal contract: a named, closed list of required fields (the same `name: Type` shape a `struct`'s own fields use). An interface is never itself a usable value type — it cannot appear as an ordinary parameter/field/return type, only as a generic bound or an `implements` target — so no trait-object/dynamic-dispatch capability exists.
- `struct Name implements Interface1, Interface2 { ... }` and `part Name implements Interface1, ... { ... }` declare explicit, nominal conformance. Conformance is statically verified: the implementer must carry, for every interface field, a field of the same name and a compatible type — a `struct`'s own declared fields, or a `part`'s own top-level `param` declarations (the only part-body items with a mandatory explicit type). A structurally matching type that never declares `implements` does not conform.
- `T: Interface1 + Interface2` on a `fn`/`struct`/`enum` type parameter constrains that parameter to types verified to conform to every listed interface. A bound is checked wherever the parameter is instantiated: an ordinary generic-function call site (reusing D17's existing call-site type inference) and a generic-struct/enum `Name<Args>` type-application site alike.

This mechanism does not add class/state inheritance, runtime monkey-patching, mandatory dynamic dispatch, trait objects, higher-kinded types, specialization, associated-type machinery, complex variance, or negative bounds — none of those are part of the implemented baseline.

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

D22/DL-24 establishes the future raw/safe identity boundary: a raw/unsafe geometry handle is an opaque AICAD-owned, kernel-neutral, epoch/context-scoped value, not an OCCT pointer and not a persistent semantic topology reference. Explicit validation/adoption must produce a new safe semantic geometry value when crossing from the raw tier. Exact Stage-5 source spelling, raw-handle representation, and epoch encoding remain deferred.

## RuntimeBuiltin type closure — D20/DL-21

D20 is **resolved** by DL-21 and implemented by AICAD-076A. Approved AICAD standard nominal types such as `Point3`, `Axis3`, `Frame3`, and `Plane` may appear in always-seeded RuntimeBuiltin signatures.

The always-seeded builtin environment must be type-closed: every nominal type required by an automatically seeded builtin signature is automatically available to signature collection and type checking. Eager signature collection/type checking remains authoritative, and the builtin catalogue together with its required standard type declarations must be independently type-validatable with zero diagnostics against an otherwise-empty program.

`with_geometry_types` may remain as an idempotent compatibility/composition helper; callers are not required to invoke it to make automatically seeded builtin signatures type-correct. AICAD-076's scalar-decomposition signatures were temporary compatibility workarounds, not the intended long-term Safe CAD API architecture.

This is a closed first-party standard environment. It does not authorize arbitrary plugin/runtime type injection, host callbacks, OCCT classes in public/HIR signatures, or dynamic extension of `BuiltinFnId` from AICAD source.

D21/DL-23 permits future scaling of this **closed** catalogue through a declarative/single-source description that may derive IDs, signatures, required standard-type dependencies, dispatch/effect/validation metadata, and documentation/test metadata. Stable builtin identity must be deterministic and deliberately managed rather than accidentally tied to enum/source ordering. Exact catalogue-generation mechanics are deferred to Stage 5 and do not create an open plugin/native registration mechanism.

## Tolerance terminology — D24/DL-26

AICAD does not define unrelated numerical policies through one global epsilon. D24 recognizes at least six distinct tolerance domains:

- **Representation/validity tolerance** — internal validity thresholds for mathematical/kernel representations.
- **Modeling/construction tolerance** — operation/kernel construction policy such as intersection, trimming, sewing, healing, booleans, and related construction behavior.
- **Approximation tolerance** — operation-specific fitting/interpolation/discretization error policy.
- **Solver tolerance** — numerical convergence/satisfaction control for a solver implementation; the Stage-3 sketch solver owns its own profile and it is not D5.
- **Verification tolerance** — tolerance explicitly attached to an engineering acceptance/assertion/test/evidence context.
- **D5 equivalence/comparison tolerance** — the versioned geometry-comparison contract. The current v1 defaults remain: `linear_abs = 1e-4 mm`, `linear_rel = 0`, `area_abs = 1e-6`, `area_rel = 1e-3`, `volume_abs = 1e-6`, `volume_rel = 1e-3`, `center_of_mass_abs = 1e-4 mm`; effective linear/area/volume tolerances scale according to DL-12's `max(abs, rel*S^n)` rules.

These domains may interact but are not aliases. A value approved for one domain must not silently become another domain's default merely because both use floating-point quantities, and no tolerance may be silently widened merely to make a failed operation or test pass. Future Stage-5/6/7 defaults and override mechanics remain separately deferred; D24 does not invent them.
