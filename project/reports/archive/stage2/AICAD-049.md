# AICAD-049: Implement dimensional arithmetic type rules

## Objective

Implement dimensional arithmetic operator type rules per
`project/TASKS.yaml` AICAD-049 (Stage-2 batch S2-05, first of two tasks):
"correct derived dimensions infer; illegal mixed-dimension arithmetic
fails with stable diagnostics." Starts Batch S2-05 (`049 -> 050`).

## Base commit

`96ad008` (`branch/wonderful-thompson-fkbtu6`'s own head at session start
— the furthest-progressed Stage-2 lineage found; see
`project/SESSION_HANDOFF.md` for the branch-reconstruction record this
session performed before starting any task work).

## Plan references read

`rfcs/0004-units-type-system.md` §5 ("different physical dimensions never
implicitly convert... there is no numeric escape hatch"; "derived
dimensions are canonicalized structurally, by dimension exponents") and §7
(affine absolute/delta operation rules, including the Stage-0-review
patch's `absolute - absolute -> delta` / `absolute - delta -> absolute` /
`delta - delta -> delta` table); `project/reports/AICAD-047.md` and
`AICAD-048.md`'s own "Unresolved questions"/"Known limitations" sections
(both flag the Pressure/Stress, Torque/Energy vector-sharing multiplicity
as this task's to resolve); `crates/cad-units/src/dimension_vector.rs`'s
own module doc comment (states directly: "Deciding what an *unannotated*
expression like `force * length` type-checks as ... is `AICAD-049`'s
'dimensional arithmetic type rules'"); `AGENTS.md`'s non-negotiables
("Stable semantic references are preferred; ambiguity is an error, never
an arbitrary selection").

## Implementation

- `crates/cad-units/src/arithmetic.rs` (new): the whole task.
  - `ArithmeticOp` (`Add`/`Sub`/`Mul`/`Div`) and `OperandType`
    (`Scalar(PrimitiveType)` or `Dimensional { dimension, affine }`,
    enforcing at construction that `affine.is_some() ==
    dimension.is_affine()`) — a minimal type representation scoped to
    exactly what dimensional-arithmetic rules need, not a general `Type`
    (see "Decisions" #1).
  - `check_binary_arithmetic(op, lhs, rhs, expected: Option<Dimension>)`:
    `+`/`-` require both operands to be the same kind (two matching-
    dimension `Dimensional`s, or two same-`PrimitiveType` numeric
    `Scalar`s); `*`/`/` additionally allow one `Dimensional` operand
    scaled by one numeric `Scalar` (dimension preserved), and two
    `Dimensional` operands combine via `DimensionVector` algebra with
    ambiguity resolution against the optional `expected` target (see
    "Decisions" #2).
  - `check_comparison(op, lhs, rhs)`: same-dimension (or same-scalar-type)
    requirement, no new dimensional result (`Bool` is the caller's to
    attach); affine operands additionally require matching
    absolute/delta kind (see "Decisions" #3).
  - `check_unary_neg(operand)`: numeric scalars and any `Dimensional`
    operand (affine or not) negate to the same type — see "Decisions" #4
    for why this must include absolute affine quantities.
  - `DimensionalArithmeticError` (10 variants, each with a stable
    `UNIT-E1##` `code()` string) plus a `Display` impl — see "Decisions"
    #5 for why this is a plain error type, not a `cad_diagnostics::Diagnostic`.
- `crates/cad-units/src/lib.rs`: adds `mod arithmetic;` and re-exports its
  public items; module doc comment extended to mention `AICAD-049`.

No third-party dependency added; no new intra-workspace dependency added
(`arithmetic` only uses this crate's own `DimensionVector` plus
`cad-types`, already a dependency).

## Decisions made and why

1. **`OperandType` is a narrow, task-scoped type, not a general `Type`.**
   `cad-hir` (where a real unified type representation eventually belongs)
   is still an empty placeholder — typed HIR is `AICAD-051`, and the
   general type checker consuming it is `AICAD-052`/`053`. Building a
   general-purpose type enum now, before any consumer needs one, would be
   exactly the "Public APIs...owned by a later task must wait"
   overreach `AGENTS.md`'s "No speculative future work" section warns
   against. `OperandType` therefore covers only a numeric/non-numeric
   `PrimitiveType` scalar or a `Dimension`+optional-`AffineKind` quantity
   — the two shapes a dimensional operator rule can actually distinguish
   between — and is documented as such so a later task doesn't mistake it
   for the general type representation.
2. **Ambiguous derived dimensions (Pressure/Stress, Torque/Energy) require
   an explicit target annotation; no tie-break preference is chosen.**
   This was the exact open question `AICAD-047.md`/`048.md` flagged as
   "exactly the kind of 'ambiguous reference selects silently' question
   `AGENTS.md` says should not be resolved without evidence/deliberate
   design", suggesting `AICAD-049` treat it as a first-class decision and
   escalate to `project/OWNER_DECISIONS.md` if evidence didn't make one
   resolution obviously correct. It did: `AGENTS.md`'s own non-negotiable
   ("ambiguity is an error, never an arbitrary selection") already
   settles this without inventing new policy, so no escalation was
   needed. Concretely: `resolve_derived_dimension` computes every named
   `Dimension` whose `DimensionVector::of(_)` matches the operation's
   result vector; zero matches is `UnknownDerivedDimension` (the language
   has no generic/anonymous quantity type to fall back to), exactly one
   match resolves silently (unambiguous — e.g. `Length * Length ->
   Area`), and more than one match is `AmbiguousDerivedDimension` unless
   the caller supplies an `expected` target dimension that the computed
   vector actually matches (`DimensionMismatch` if it does not). A fully
   cancelled (dimensionless) result resolves to
   `OperandType::Scalar(PrimitiveType::Float)` — the one numeric-
   defaulting choice this module makes, since a dimensional ratio
   (`1500mm / 1000mm`) is not generally integral; every broader numeric-
   literal-typing question remains `AICAD-052`+'s.
3. **Affine (`Temperature`) rules are stricter than plain-dimension rules,
   in three ways, none of them guessed at:**
   - `+`/`-`: exactly RFC-0004 §7's own patch table
     (`absolute+absolute` illegal; `absolute+delta`/`delta+absolute`
     -> absolute; `delta+delta` -> delta; `absolute-absolute` -> delta;
     `absolute-delta` -> absolute; `delta-delta` -> delta). One
     combination the RFC's patch table does not define —
     `delta-absolute` — is rejected (`IllegalAffineOperation`) rather
     than guessed at; nothing in RFC-0004 §7 gives "delta minus absolute"
     a meaning, and inventing one would be new type-system semantics
     beyond an approved RFC (an `AGENTS.md` escalation trigger this
     report avoids by simply not doing it).
   - `*`/`/`: rejected outright for any affine operand
     (`AffineMultiplicativeOperand`). RFC-0004 §7 only ever defines
     `+`/`-` rules for affine-dimensioned quantities; "what does
     `20degC * 2` mean, given the unit's non-zero origin" has no RFC
     answer, so it is a type error rather than an assumed linear-scaling
     interpretation (RFC-0004 §7 itself: affine handling is "intentionally
     more conservative than 'unit conversion is always linear
     rescaling'").
   - Comparison: requires matching absolute/delta kind
     (`AffineComparisonMix` otherwise) — comparing an absolute point to a
     difference is not addressed by RFC-0004 §7 and is not obviously
     meaningful, so rejected rather than assumed comparable.
4. **Unary negation preserves an affine quantity's absolute/delta kind
   unchanged (including `Absolute`).** This looks at first like it should
   be as restricted as `*`/`/`, but it is not: `crates/cad-lexer`
   (`AICAD-040`) does not fold a leading sign into a number token, so a
   literal like `-40degC` parses as `Unary::Neg` applied to the literal
   `40degC` (an ordinary absolute quantity) — the *only* way a negative-
   magnitude absolute affine literal is representable in source at all.
   Rejecting `Neg` on `Absolute` would make every negative-Celsius/
   Fahrenheit literal a type error, which is not what RFC-0004 intends
   (§7's own worked concern is about *adding two* absolute quantities,
   never about negating one on its own). This is a narrow, evidence-based
   reading of existing frozen material, not new semantics.
5. **Errors are a plain local `DimensionalArithmeticError` type (with a
   stable `code()` string), not a `cad_diagnostics::Diagnostic`.** Mirrors
   `crates/cad-units/src/registry.rs`'s own established convention
   (`UnitConversionError`, plain `std::error::Error`, no
   `cad-diagnostics` dependency) rather than introducing a new one this
   task doesn't need: this module has no source span to attach (no
   AST/HIR consumer exists yet), and `cad_diagnostics::Diagnostic`'s
   `with_source` builder needs one to be useful. Each variant's `code()`
   returns a `UNIT-E1##` string (`UNIT` is an already-frozen RFC-0005 §2
   family; `101`-`110` were unused by any other crate at the time of
   writing — checked via `grep -rn "UNIT-" crates/ docs/ rfcs/ specs/
   project/`) so a later `cad-diagnostics`-aware caller (the general type
   checker, `AICAD-052`+) can build a real `Diagnostic` from
   `DiagnosticCode::parse(err.code())` plus its own source span, without
   this module needing to guess at one prematurely.
6. **No escalation triggered.** This task changes no already-approved
   public syntax/semantics (RFC-0004 §5/§7 are already resolved, DL-3);
   the one genuine open design question (derived-dimension ambiguity) was
   answered by an already-approved `AGENTS.md` non-negotiable, not a new
   architecture decision; and no existing gate/test was weakened.

## Files changed

- Added: `crates/cad-units/src/arithmetic.rs`.
- Modified: `crates/cad-units/src/lib.rs`.
- Modified: `project/TASKS.yaml` (AICAD-049 `status: done`).
- Added: `project/reports/AICAD-049.md` (this report).

## Verification (exact commands/results)

```
$ cargo test -p cad-units
running 75 tests
test arithmetic::tests::* (39 tests) ... ok
test dimension_vector::tests::* (18 tests) ... ok
test registry::tests::* (18 tests) ... ok
test result: ok. 75 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo fmt --all -- --check
(exit code 0, no output)

$ cargo clippy -p cad-units --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(zero warnings)

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo test --workspace
(61 test binaries; every one `test result: ok`; exit code 0)
```

## Tests/regressions

39 new tests in `arithmetic.rs` (75 total in `cad-units`, combined with
`AICAD-047`'s 18 and `AICAD-048`'s 18): same-dimension add/sub type-checks
(the "mm + in" case, at the type level — unit-literal spelling is
`registry`'s concern, not this module's); mixed-dimension rejection (the
"mm + seconds" case) for both `+` and `-`; scalar-vs-dimensional mixing
rejection; multiplication/division derived-dimension inference (`Length *
Length -> Area`, three-way `-> Volume`, `Mass * Acceleration -> Force`,
`Length / Time -> Velocity`, `Length / Length -> Scalar(Float)`, the
reciprocal case `Scalar / Time -> Frequency`, and scalar-factor scaling
both directions); the full Pressure/Stress and Torque/Energy ambiguity
matrix (unannotated rejection with both named candidates listed, and
correct resolution under each of the two possible annotations, plus
rejection of a *mismatched* annotation and of a vector matching no named
dimension at all); comparison across compatible/incompatible dimensions
and across scalar/dimensional mixing; the complete RFC-0004 §7 affine
absolute/delta `+`/`-` result table (six legal combinations) plus both
illegal combinations (`absolute+absolute`, `delta-absolute`);
affine-times-scalar and affine-divided-by-affine rejection; affine
comparison kind-mismatch rejection and same-kind acceptance; unary
negation preserving both a plain dimension and an absolute affine
quantity, and rejecting a non-numeric scalar; scalar type-mismatch/
non-numeric-operand rejection; `Display` formatting for `OperandType`; and
an exhaustiveness/uniqueness check over every error variant's `code()`
shape and distinctness. No regressions found or introduced (this module is
new; `dimension_vector`/`registry`'s own pre-existing 36 tests are
unaffected and still pass).

## Known limitations

- Not wired into `cad-ast`/HIR/a real type checker yet — this module only
  exposes the type *rule*; `AICAD-050` (name binding) does not need it,
  and `AICAD-052`/`053` (the type checker) is the actual consumer. No
  `Expr`/`Stmt` AST node currently calls into this module.
- Plain scalar-vs-scalar arithmetic/comparison only accepts identical
  `PrimitiveType`s (no `Int + Float` promotion, no literal-type
  defaulting beyond the one dimensionless-ratio case documented in
  Decisions #2) — deliberately deferred to the general type checker
  (`AICAD-052`), which owns numeric-type rules generally, not just where
  a dimension happens to be involved.
- `Tolerance<T>`/`Range<T>`/etc. arithmetic (RFC-0004 §6) is untouched —
  still not owned by any named task, per `AICAD-046`'s own prior note.
- The `UNIT-E1##` codes this module documents are provisional, like every
  other `cad-diagnostics` code, pending D10 (diagnostic code/schema
  stability policy, still open per `project/OWNER_DECISIONS.md`).

## Unresolved questions

None requiring further owner escalation. `AICAD-050` ("name binding/
scopes/symbol table") is next in Batch S2-05's fixed order; it operates on
`cad-ast` (via `crates/cad-compiler`, which already owns the module-loader
phase immediately before it in `docs/plan/02_LANGUAGE_AND_COMPILER.md`
§17's phase list) and does not itself need this task's arithmetic rules —
those are consumed later, by the type checker (`AICAD-052`/`053`).
