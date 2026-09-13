# RFC-0004: Units and Type System

- Status: **Accepted Stage-0 baseline** (`project/DECISION_LOG.md#DL-10`).
- Owner rulings incorporated/clarifying this RFC: DL-3 (D4 type/unit semantics), DL-12 and DL-17+AICAD-064A (D5/D19 equivalence profile), DL-13 (D16 collection/iteration minimum), DL-14 (D17 general generics/data-carrying enums/Result/Optional), DL-20 (D11 solver-independent constraint semantics), DL-21 (D20 always-seeded nominal RuntimeBuiltin types), DL-24 (D22 safe/raw geometry tiers), DL-26 (D24 tolerance domains), DL-29 (D27 general nominal interfaces/protocols), and DL-30 (D28 solver-neutral assembly relations), plus the accepted AICAD-075A spatial model.
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

## 4. Generic algebraic data types and future interfaces — D17/D27

Current source semantics support ordinary generic structs, enums, and functions using bare type parameters. Enums support unit, tuple-payload, and record-payload variants. Pattern matching supports corresponding destructuring and nominal-enum exhaustiveness diagnostics.

`Result<T,E>` and `Optional<T>` are ordinary generic prelude enum types built from that same language machinery. They are not hard-coded compiler semantic types. Result propagation is explicit through ordinary control flow (typically `match`); no `?`-style propagation syntax is current.

Current Stage-3 generics do **not** yet include interface/protocol bounds. D27/DL-29 now approves the future Stage-6 semantic baseline: a restrained general nominal interface/protocol concept supporting contract declaration, explicit nominal conformance/implementation, static conformance checking, and interface-constrained generic functions/types. Exact syntax remains subject to the normal language RFC process and is not current grammar. D27 explicitly does not approve state/class inheritance, runtime monkey-patching, mandatory dynamic dispatch, trait objects, higher-kinded types, specialization, associated-type machinery, complex variance rules, or negative bounds.

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

The current always-available source spatial environment contains `Vector2<T>`, `Vector3<T>`, `Point2`, `Point3`, `Axis3`, `Frame3`, and `Plane`. AICAD-075A establishes the shared kernel-neutral semantics below these source values:

- position (`Point3`) and displacement/vector (`Vector3`) are distinct semantic concepts;
- directions are validated normalized vectors and reject degenerate/non-finite input;
- `Axis3` is an origin + direction;
- `Frame3` is orthonormal and right-handed;
- a plane is origin + normal and is not identical to a full in-plane frame;
- proper rigid `Transform` semantics use translation + right-hand-rule rotation and exclude reflection/scale.

Internal kernel-neutral concepts such as validated `Direction3`, `Plane3`, or `Transform` do not automatically imply a separate direct source constructor/type spelling. Internal semantic capability and source-language exposure are different layers.

## 8. RuntimeBuiltin type closure — D20/DL-21

D20 is **resolved**, not open. DL-21 is authoritative and AICAD-076A implements the ruling. The closed always-seeded RuntimeBuiltin catalogue may use approved AICAD standard nominal types such as `Point3`, `Axis3`, `Frame3`, and `Plane` in signatures.

The always-seeded builtin environment must be type-closed: all nominal types required by automatically seeded builtin signatures are automatically available to signature checking. Eager signature collection/type checking remains authoritative. The builtin catalogue together with its required standard type declarations must be independently type-validatable with zero diagnostics against an otherwise-empty program.

`with_geometry_types` may remain as an idempotent compatibility/composition helper; callers do not need to invoke it to make automatically seeded builtin signatures type-correct. AICAD-076's scalar-decomposition signatures were temporary compatibility workarounds, not the intended long-term Safe CAD API architecture.

This remains a closed first-party standard environment. It does not authorize arbitrary plugin/runtime type injection, host callbacks, OCCT types in public/HIR signatures, or a dynamic extension ABI. D21/DL-23 permits future declarative/single-source scaling of this closed catalogue while preserving deterministic stable builtin identity; exact generation mechanics remain deferred.

## 9. Persistent references and raw topology — D22/DL-24

Stage-3 named outputs, `GeomId`s, feature IDs, sketch/entity IDs, and raw face/edge integer indices are not persistent Stage-4 topology-reference types. The current source language exposes no durable `VertexRef`/`EdgeRef`/`WireRef`/`FaceRef`/`ShellRef`/`SolidRef` resolution capability.

D22 freezes the future type/identity boundary: safe semantic geometry, raw/unsafe AICAD-owned handles, persistent semantic topology references, and kernel-native objects are distinct. A raw handle is kernel-neutral in public semantics, opaque at the source/HIR boundary, and context/epoch-scoped; it is neither an OCCT pointer nor durable semantic identity. Crossing from raw to safe requires explicit validation/adoption producing a new safe value with validation/provenance evidence. Exact Stage-5 type spelling, handle representation, and epoch encoding remain deferred.

## 10. Tolerance taxonomy — D24/DL-26

The type system must not be read as defining one global tolerance parameter for unrelated numerical domains. D24 recognizes at least six distinct domains:

- **representation/validity tolerance** for internal mathematical/kernel representation validity;
- **modeling/construction tolerance** for geometric construction behavior;
- **approximation tolerance** for fitting/interpolation/approximation error bounds;
- **solver tolerance** for numerical convergence/satisfaction;
- **verification tolerance** for explicit engineering acceptance/assertion semantics;
- **D5 equivalence/comparison tolerance**, whose calibrated v1 profile remains `linear_abs = 1e-4 mm`, `linear_rel = 0`, `area_abs = 1e-6`, `area_rel = 1e-3`, `volume_abs = 1e-6`, `volume_rel = 1e-3`, `center_of_mass_abs = 1e-4 mm`, with DL-12 scale-aware comparison formulas.

These policies may interact but are not aliases. No domain may silently inherit another domain's values, and no tolerance may be silently widened merely to make a failed operation/test pass. D24 does not invent numerical defaults for future Stage-5/6/7 domains; those remain separately specified work.

## 11. Constraint and future assembly-relation semantics — D11/D28

AICAD's constraint IR owns typed/dimensioned variables, semantic IDs, constraint-kind meaning/parameters, provenance, solve-status vocabulary, and structured evidence. Numerical solvers are adapters/backends: they choose algorithms and produce candidate numerical results/status but may not redefine dimensional semantics, constraint meaning, or silently promote an arbitrary solution branch into public semantics.

D28/DL-30 extends this principle to future Stage-6 assembly mates/joints and observable pose. The AICAD semantic layer owns relation meaning, satisfaction/DOF/conflict/redundancy semantics, and deterministic observable pose. Underconstrained or multiple-solution assemblies may not silently inherit an arbitrary backend pose; deterministic grounding/gauge handling and any branch-selection policy belong to AICAD semantics above the solver. The exact canonicalization algorithm and Stage-6 solver implementation remain deferred.

Stage 3's sketch constraint subsystem implements the D11 boundary internally. It does not imply source-level `sketch { ... }` syntax or implement the Stage-6 assembly model approved by D26-D30.

## 12. Historical rationale and remaining future work

Stage-0 froze typed quantities because engineering software cannot safely treat units, affine temperatures, or tolerances as display metadata. Stage-2 later generalized the type system rather than hard-coding `Result`, and deliberately bounded collections/iteration rather than silently implementing the entire future standard library. Stage 3 reused one kernel-neutral spatial model rather than creating per-feature axis/frame conventions.

D27 now resolves the semantic direction for future interfaces/bounded generics, and D22/D24/D28 resolve the corresponding raw-geometry, tolerance-domain, and assembly-solver baselines, but their implementation and explicitly deferred syntax/representation/default details remain future work. Still future and not current Stage-3 syntax: interfaces/`implements`/generic bounds, `Set`/`Map`, comprehensions, closures/generators, general user iterators, raw topology types/syntax, persistent topology refs, assemblies/configurations, and verification-language types. Owner approval of their future semantic baseline does not itself authorize implementation.
