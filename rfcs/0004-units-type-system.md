# RFC-0004: Units and Type System

- Status: **Accepted Stage-0 baseline** (`project/DECISION_LOG.md#DL-10`).
- Owner rulings incorporated/clarifying this RFC: DL-3 (D4 type/unit semantics), DL-12 and DL-17+AICAD-064A (D5/D19 equivalence profile), DL-13 (D16 collection/iteration minimum), DL-14 (D17 general generics/data-carrying enums/Result/Optional), DL-20 (D11 solver-independent constraint semantics), plus the accepted AICAD-075A spatial model. The live repository also records DL-21 for D20; the active transition directive conflicts with that status, so this RFC records the conflict rather than silently resolving it.
- Canonical current detail: `specs/language/types.md`.

## 1. Summary

AICAD treats engineering quantities and semantic data types as first-class language types. Dimensional analysis, affine quantities, generic algebraic data types, and kernel-neutral spatial values belong above the geometry kernel and must not degrade into untyped numeric conventions or backend-native identity.

The original Stage-0 type-system baseline remains accepted, but later Stage-2/3 decisions narrowed which planned capabilities are current. This RFC therefore distinguishes current approved language semantics from future collection/interface/metaprogramming examples in the foundation plan.

## 2. Primitive and engineering quantity baseline

AICAD's semantic type layer includes ordinary primitives (`Bool`, integer/float/string/bytes/character families) plus dimensioned engineering quantities. Named dimensions include length/area/volume/angle/time/mass/temperature/force/torque/pressure/stress/energy/power/density/velocity/acceleration/angular velocity/frequency/current/voltage/resistance.

Quantities are not bare floats with a display suffix. Dimensional arithmetic is checked structurally. Same-vector named dimensions are not silently chosen by arbitrary tie-breaking; an explicit expected/annotated type resolves genuine semantic ambiguity.

## 3. Units and conversions — D4/DL-3

Unit conversion is allowed only within compatible dimensions. The unit registry, not the grammar, decides whether a numeric suffix names a known unit.

Affine quantities distinguish absolute and delta values. Temperature is the current affine dimension. Offset conversions apply only to absolute quantities; delta conversion uses scale only. Arithmetic preserves the semantic distinction—for example, absolute minus absolute yields a delta.

`Tolerance<T>`-style uncertainty semantics must remain conservative/typed where implemented; this RFC does not authorize statistical/RSS combination as the default merely because it is useful in another API.

## 4. Generic algebraic data types — D17/DL-14

Current source semantics support ordinary generic structs, enums, and functions using bare type parameters. Enums support unit, tuple-payload, and record-payload variants. Pattern matching supports corresponding destructuring and nominal-enum exhaustiveness diagnostics.

`Result<T,E>` and `Optional<T>` are ordinary generic prelude enum types built from that same language machinery. They are not hard-coded compiler semantic types. Result propagation is explicit through ordinary control flow (typically `match`); no `?`-style propagation syntax is current.

Current generics intentionally do **not** include interface/trait bounds, higher-kinded types, variance, specialization, generic associated types, dependent types, variadic generics, or lifetime parameters. Interface/bounded-generic semantics remain future Stage-6 language architecture.

## 5. Collections and iteration — D16/DL-13

The current minimum is deliberately bounded:

- immutable `List<T>` values constructed with list literals;
- `Range<Int>` and `Range<UInt>` from `start..end` / `start..=end`, ascending by one;
- an internal/runtime `Iterator<T>` abstraction sufficient to implement `for`, not a public general compiler-intrinsic protocol.

`Set<T>`, `Map<K,V>`, comprehensions, arbitrary source-defined iterators, async/parallel iteration, and implicit dimensional-range stepping are not current language semantics. Their appearance in future planning examples is illustrative until separately approved.

## 6. Control flow and recursion

Current language control flow includes `if`, `match`, `for`, `while`, `loop`, `break`, `continue`, and `return`; `if` and `match` may produce values in expression position where their branches/arms satisfy type requirements.

Recursion is supported, but the current evaluator's concrete call-depth ceiling is an implementation safety/resource limit rather than a permanent language-level number. Future runtimes may change the evaluator architecture without changing the language's recursion semantics.

Closures/lambdas, generators/yield, comprehensions, and async/parallel iteration are not current Stage-3 language capabilities.

## 7. Geometry and spatial nominal types

`Geometry` is an opaque AICAD value referring to backend-neutral geometry construction, not an OCCT object or persistent topology handle.

The current Stage-3 implementation always makes `Vector2<T>`, `Vector3<T>`, `Point2`, `Point3`, `Axis3`, `Frame3`, and `Plane` available in the standard type environment. AICAD-075A establishes the shared kernel-neutral semantics below these source values:

- position (`Point3`) and displacement/vector (`Vector3`) are distinct semantic concepts;
- directions are validated normalized vectors and reject degenerate/non-finite input;
- `Axis3` is an origin + direction;
- `Frame3` is orthonormal and right-handed;
- a plane is origin + normal and is not identical to a full in-plane frame;
- proper rigid `Transform` semantics use translation + right-hand-rule rotation and exclude reflection/scale.

Internal kernel-neutral concepts such as validated `Direction3`, `Plane3`, or `Transform` do not automatically imply a separate direct source constructor/type spelling. Internal semantic capability and source-language exposure are different layers.

## 8. RuntimeBuiltin type closure — D20 authority conflict

The live repository records D20 as **RESOLVED — DL-21**, and the Stage-3 implementation follows that record: the always-seeded RuntimeBuiltin environment is type-closed for its current nominal signatures, eager signature collection remains authoritative, and AICAD-076's scalar-flattening workaround is recorded as temporary.

The owner-provided Stage-3 -> Stage-4 normative-cleanup directive, however, explicitly states that D20 is currently open and must not be silently resolved. Those two authoritative inputs conflict. This cleanup therefore does **not** change `OWNER_DECISIONS.md`, reopen D20, supersede DL-21, or convert the existing implementation into a fresh permanent architecture ruling. It records current behavior and treats D20's governance status as an explicit owner-reconciliation blocker before final Stage-4 initialization.

Regardless of the status conflict, neither source authorizes arbitrary plugin/native type registration, host callbacks, OCCT types in public/HIR signatures, or an unreviewed dynamic extension ABI.

## 9. Persistent references and raw topology are not current type claims

Stage-3 named outputs, `GeomId`s, feature IDs, sketch/entity IDs, and raw face/edge integer indices are not persistent Stage-4 topology-reference types. The current source language exposes no durable `VertexRef`/`EdgeRef`/`WireRef`/`FaceRef`/`ShellRef`/`SolidRef` resolution capability.

Future low-level/raw topology may use opaque AICAD-owned epoch-local handles, but exact Stage-5 type/syntax remains unresolved. A raw handle is neither an OCCT pointer nor durable semantic identity.

## 10. Tolerance taxonomy

The original type-system material must not be read as one global tolerance parameter for unrelated numerical domains.

- **D5 equivalence comparison** has a versioned calibrated v1 profile: `linear_abs = 1e-4 mm`, `linear_rel = 0`, `area_abs = 1e-6`, `area_rel = 1e-3`, `volume_abs = 1e-6`, `volume_rel = 1e-3`, `center_of_mass_abs = 1e-4 mm`, with DL-12 scale-aware comparison formulas.
- **Solver convergence** is solver-specific numerical control; the Stage-3 sketch solver owns a separate profile.
- **Modeling/construction tolerance**, **approximation tolerance**, **verification/assertion tolerance**, and **private representation-validity thresholds** are distinct categories. No new global default for those categories is created here.

D5 constants must not be copied into another category merely to fill an unspecified default.

## 11. Constraint semantics — D11/DL-20

AICAD's constraint IR owns typed/dimensioned variables, semantic IDs, constraint-kind meaning/parameters, provenance, solve-status vocabulary, and structured evidence. Numerical solvers are adapters/backends: they choose algorithms and produce candidate numerical results/status but may not redefine dimensional semantics, constraint meaning, or silently promote an arbitrary solution branch into public semantics.

Stage 3's sketch constraint subsystem implements this boundary internally. It does not imply source-level `sketch { ... }` syntax, and it does not decide Stage-6 assembly solver/relation policy or require one universal concrete constraint struct for all future domains.

## 12. Historical rationale and remaining future work

Stage-0 froze typed quantities because engineering software cannot safely treat units, affine temperatures, or tolerances as display metadata. Stage-2 later generalized the type system rather than hard-coding `Result`, and deliberately bounded collections/iteration rather than silently implementing the entire future standard library. Stage 3 reused one kernel-neutral spatial model rather than creating per-feature axis/frame conventions.

Still future: interfaces/`implements`/generic bounds, `Set`/`Map`, comprehensions, closures/generators, general user iterators, raw topology types/syntax, persistent topology refs, assemblies/configurations, and verification-language types. D20's governance status itself also requires explicit owner reconciliation because the current directive conflicts with the live DL-21 record.
