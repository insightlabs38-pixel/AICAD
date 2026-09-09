# RFC-0004: Units/Type System

- Status: Draft (Stage 0)
- Stage-0 build item covered (`docs/plan/15_IMPLEMENTATION_ROADMAP.md`
  Stage 0): "type/units model".
- Owner ruling incorporated: DL-3 (type/units semantics specifics). See
  `project/DECISION_LOG.md`.
- Depends on: RFC-0001 (surface syntax for literals/types).

## 1. Summary

This RFC freezes AICAD's type system baseline from
`docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` and resolves the
previously open dimension-canonicalization, implicit-conversion, and
tolerance-arithmetic questions (`project/OWNER_DECISIONS.md` D4) via DL-3.
This directly implements non-negotiable invariant 5
(`docs/plan/00_PRINCIPLES_AND_SCOPE.md` §3: "Units are in the type system,
not plain untyped floats").

## 2. Primitive types (frozen, from `03` §2)

`Bool`, `Int`, `UInt`, `Float`, `String`, `Bytes`, with `Decimal` and `Char`
as optional additions. `Float` is for numerical algorithms; dimensional
quantities are never bare `Float` values (§3).

## 3. Dimensional quantity types (frozen, from `03` §3)

Minimum first-class dimensions: `Length`, `Area`, `Volume`, `Angle`,
`Time`, `Mass`, `Temperature`, `Force`, `Torque`, `Pressure`, `Stress`,
`Energy`, `Power`, `Density`, `Velocity`, `Acceleration`,
`AngularVelocity`, `Frequency`, `Current`, `Voltage`, `Resistance`.
Derived dimensional algebra is checked (`let area: Area = width * height;`
type-checks; `let width: Length = 5kg;` is a type error).

## 4. Unit literals (frozen, from `03` §4)

Initial unit set: length (`nm um mm cm m km in ft`), angle (`deg rad`),
mass (`mg g kg lbm`), force (`N kN lbf`), pressure/stress
(`Pa kPa MPa GPa psi ksi`), temperature (`K degC degF`, affine — see §7).
The standard library may expand this set without a grammar change.

## 5. Canonicalization and implicit conversion (DL-3, resolved)

- **Dimensions have a unique canonical internal representation independent
  of user-selected display units.** A quantity's runtime/comparison
  identity is its canonical value plus dimension, never its
  source-literal unit.
- **Quantities of the same dimension may be implicitly converted** for
  arithmetic and comparison (`5mm + 2cm` type-checks and evaluates
  correctly without an explicit conversion call).
- **Different physical dimensions never implicitly convert.** `Length +
  Time` is a type error under every circumstance; there is no numeric
  "escape hatch."
- **Derived dimensions are canonicalized structurally**, by dimension
  exponents (e.g. `Velocity = Length^1 * Time^-1`), not by how a unit is
  spelled in source. Two derived quantities with the same exponent vector
  are the same dimension regardless of which unit literals produced them.
- A quantity's conceptual shape remains as described in `03` §5:
  canonical value, unit dimension, preferred display unit, optional
  precision/uncertainty metadata. Display unit is presentation-only and
  never affects type-checking or comparison.
- **Patch (independent Stage-0 review):** `03` §5's quantity shape, as
  written, has no field distinguishing an absolute quantity from a delta
  quantity, yet §7 below requires the type checker to reject
  `absolute + absolute` for affine dimensions. To keep this RFC internally
  consistent, affine-dimensioned quantities (§7) carry one additional
  discriminant beyond the general shape above — `affine_kind: absolute |
  delta` — and it is this discriminant, never the source unit spelling,
  that the type checker uses to admit or reject an operation. This patch
  only makes explicit a mechanism §7's invariant already requires; it does
  not select the discriminant's concrete surface syntax or type-name
  spelling (e.g. whether a delta is its own named type or a tagged
  `Temperature` value) — that concrete encoding remains Stage-2 work, as
  §7 already states.

## 6. Tolerances (DL-3, resolved)

- `Tolerance<T>`, `AsymmetricTolerance<T>`, `Range<T>`, `Distribution<T>`,
  and `Fit` remain first-class structured types, not string annotations
  (`03` §6).
- **`Tolerance<T>` is initially defined by conservative interval
  semantics.** Tolerance arithmetic propagates interval bounds (e.g. `(a
  +/- da) + (b +/- db)` yields a result whose bound is `da + db`, the
  worst-case sum, not a statistically reduced figure).
- **Statistical/RSS tolerance-stack semantics require an explicit, later,
  separate API** (`docs/plan/13_ENGINEERING_MODULES.md` §28's `RSS`/
  `Monte Carlo` methods) and are **never assumed by default**. A design
  that wants RSS composition must ask for it explicitly; conservative
  interval bounds are what plain `Tolerance<T>` arithmetic always
  produces.

## 7. Affine units (DL-3, resolved)

- Affine units — initially Celsius and Fahrenheit — **distinguish absolute
  quantities from delta quantities** and do not use ordinary scale-only
  conversion rules. The absolute-vs-delta distinction is carried by the
  `affine_kind` discriminant added to the quantity shape in §5 above; it is
  not inferred from unit spelling or context.
- Concretely: converting an absolute temperature (`20degC` -> Kelvin)
  requires the affine offset (`+273.15`); converting a temperature
  *difference* (`a delta of 5degC` -> Kelvin) does not apply that offset
  (a 5°C difference is a 5K difference, not a 278.15K one). The type
  system must make it a type error to add two *absolute* affine
  quantities together (`20degC + 20degC` is meaningless), while adding an
  absolute quantity and a delta quantity of the same unit family is
  well-defined.
- **Patch (independent Stage-0 review):** the RFC as originally drafted
  forbade `absolute + absolute` but never stated what `absolute - absolute`
  produces, leaving no defined way to construct a delta value at all.
  Subtracting two *absolute* quantities of the same affine unit family
  produces a *delta* quantity (`affine_kind: delta`, §5); subtracting a
  delta from an absolute produces an absolute; subtracting two deltas
  produces a delta. This is the minimal rule consistent with the
  already-approved absolute/delta distinction and does not introduce a new
  architecture decision.
- This is intentionally more conservative than "unit conversion is always
  linear rescaling," because affine-temperature bugs are a well-known,
  easy-to-introduce class of engineering error
  (`docs/plan/19_RESEARCH_NOTES_AND_SOURCES.md` §10 flags "formal
  dimensional type implementation and affine temperature handling" as a
  pre-implementation research item this ruling now answers at the policy
  level, leaving only the concrete type-system encoding to Stage 2).

## 8. Geometry, semantic, and structural types (frozen, from `03` §7-12)

Adopted as-is: safe geometry types (`Point2/3`, `Vector2/3`, `Frame2/3`,
`Curve2/3`, `Surface`, `Wire`, `Face`, `Shell`, `Solid`, `Part`, and the
`*Ref` family per RFC-0003); raw/ephemeral types (`Vertex*` ... `KernelShape*`,
per RFC-0002 §4); built-in semantic engineering interfaces (`Hole`,
`Thread`, `Fastener`, `Bearing`, `Gear`, `Requirement`, etc. — most as
standard-library types, not compiler intrinsics, per RFC-0001 §6);
generic collections (`Array<T,N>`, `List<T>`, `Set<T>`, `Map<K,V>`,
`Optional<T>`, `Result<T,E>`, `Range<T>`, `Iterator<T>`, `Generator<T>`);
structs/enums with destructuring; interfaces/traits (mechanical
compatibility, e.g. `interface MotorMount`); ordinary generics (no
template-metaprogramming in v1).

## 9. Ownership, control flow, and safety (frozen, from `03` §13-20)

Adopted as-is: geometry objects behave as immutable/shared value handles
(consistent with RFC-0001 §5's functional core); `Optional<T>` instead of
implicit null; full standard control flow (`if/for/while/loop/match/break/
continue/return/yield`); recursion allowed under runtime budgets
(RFC-0002 execution budgets, detailed further in a future runtime RFC);
closures and lazy query composition; generators for large pattern
sequences; `pure fn` functions guaranteed free of nondeterministic
capabilities; two unsafe forms, `unsafe geometry { }` (RFC-0002 §4) and
`unsafe native { }` (package-only, ordinary user code should rarely need
it).

## 10. Numerical precision policy (frozen, from `03` §21)

Dimensional quantities use deterministic IEEE floating point (or another
explicitly chosen scalar representation); a user-configurable modeling
tolerance exists at the project/kernel boundary; exact integers/rationals
may be used for symbolic parameter calculations; geometry comparisons use
explicit tolerance operators (`~=`, `near`, `within`), never bare equality
of floating coordinates. This RFC does not itself set the default project
tolerance value — that is implementation/config work, not a language
freeze.

## 11. Parameter and rationale metadata (frozen, from `03` §22-23)

`@param(label=..., min=..., max=..., step=..., choices=..., group=...,
advanced=..., readonly=..., unit_display=..., sensitivity=...)` and
`@rationale("...")` attributes are adopted as-is, feeding UI/inspector
tooling (Stage 9) and provenance/review tooling (Stage 13) respectively.

## 12. Alternatives considered

- **Explicit conversion required even within one dimension** (e.g.
  `mm(2cm)`) — rejected (DL-3); excessive ceremony for ordinary unit
  mixing that provides no additional type safety over structural
  same-dimension checking.
- **Defaulting `Tolerance<T>` arithmetic to RSS/statistical composition**
  — rejected (DL-3); would silently understate worst-case stacks for any
  design that did not deliberately opt into statistical treatment,
  conflicting with the "do not silently weaken validation" non-negotiable.
- **Treating affine units with ordinary scale-only conversion** (ignoring
  the absolute-vs-delta distinction) — rejected (DL-3); a well-documented
  source of real engineering bugs.
- **Canonicalizing derived dimensions by unit spelling rather than
  structurally** — rejected (DL-3); would make semantically identical
  dimensions (e.g. two different derivations of `Velocity`) fail to unify,
  breaking dimensional algebra checking.

## 13. Open questions (intentionally not resolved here)

- The default project-wide geometric modeling tolerance value (§10) is
  implementation/config, not part of this type-system freeze.
- `project/OWNER_DECISIONS.md` D11 (constraint IR/solver-independence
  rules) touches tolerance *usage* inside the constraint solver, not
  tolerance *arithmetic* itself (§6) — remains open, tracked separately.

## 14. Impact

- `crates/cad-types`, `crates/cad-units`: implement §2-7 starting Stage 2
  (AICAD-046-049).
- `crates/cad-hir`/type checker: enforce §5's same-dimension-only implicit
  conversion and structural canonicalization (AICAD-052).
- `crates/cad-requirements`: consumes §6's conservative interval
  `Tolerance<T>` semantics for geometry assertions (RFC yet to define
  requirements/tests in detail — Stage 7 scope, out of this RFC).
- `specs/language/types.md`: to be populated from this RFC starting Stage 2.
