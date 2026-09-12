# AICAD-047: Create cad-units dimensional vector/canonicalization per RFC-0004

## Objective

Implement RFC-0004 §5's structural dimension-exponent canonicalization
("derived dimensions are canonicalized structurally, by dimension
exponents... not by how a unit is spelled in source") as `cad-units`'s
`DimensionVector`, per `project/TASKS.yaml` AICAD-047 (Stage-2 batch
S2-04, second of three tasks).

## Base commit

`45d8616` (this session's own `AICAD-046` commit).

## Plan references read

`rfcs/0004-units-type-system.md` §3 (dimension list, re-read against the
concrete derivations below), §5 (structural canonicalization), §4 (unit
literal groupings — specifically the shared "Pressure/stress" heading,
which turned out to be load-bearing evidence, see Decisions §1);
`docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §3-5 (same content,
un-patched source).

## Implementation

- `crates/cad-units/Cargo.toml`: adds `cad-types = { path = "../cad-types" }`
  (matches the existing intra-workspace path-dependency convention, e.g.
  `crates/cad-parser/Cargo.toml`).
- `crates/cad-units/src/dimension_vector.rs` (new): `DimensionVector` — a
  `Copy` struct of six `i8` exponents (`length`, `mass`, `time`,
  `temperature`, `angle`, `current`) covering every one of `cad-types`'s 21
  named dimensions structurally. `DimensionVector::of(Dimension) ->
  DimensionVector` is the total, one-way canonicalization function;
  `Mul`/`Div` implement vector addition/subtraction (quantity
  multiplication/division); `is_dimensionless()`, `DIMENSIONLESS`, and a
  private `powi` complete the algebra; `Display` renders e.g. `L^1 M^1
  T^-2` for debugging.
- `crates/cad-units/src/lib.rs`: re-exports `DimensionVector`; crate-level
  doc comment states the `AICAD-047`/`048`/`049` split.
- `crates/cad-units/README.md`: replaced the stale "open owner decision D4"
  text (D4 was resolved by RFC-0004/`DECISION_LOG.md#DL-3` back in Stage 0
  planning, before this crate existed — the README had never been updated
  to say so) with a "Status" section.

No third-party dependency added.

## Decisions made and why

1. **Found and documented a real, intentional dimensional collision:
   `Pressure`/`Stress` and `Torque`/`Energy` share identical base-exponent
   vectors.** Deriving each of the 21 dimensions' vectors from first
   principles (`Force = Mass * Length / Time^2`, `Pressure = Force /
   Area`, `Torque = Energy = Force * Length`, etc.) surfaces that
   `Pressure`/`Stress` (both `Force/Area`) and `Torque`/`Energy` (both
   `Force * Length`) are physically identical in plain SI base exponents,
   even though RFC-0004 §3 lists all four as distinct named dimensions.
   This is not a defect in the vector encoding — it is confirmed
   *intentional* by RFC-0004 §4 itself, which lists one shared unit set
   ("Pa kPa MPa GPa psi ksi") under one heading, "Pressure/stress", for
   both dimensions. Given `AGENTS.md`'s "stable semantic references are
   preferred; ambiguity is an error, never an arbitrary selection," the
   correct response was *not* to invent a disambiguation rule (e.g.
   picking Energy over Torque when both match) inside this task. Instead:
   - `DimensionVector` provides no `DimensionVector -> Dimension` reverse
     lookup at all — only the forward, unambiguous `Dimension ->
     DimensionVector` direction (`of`). A caller can always check
     compatibility against an *expected* dimension
     (`DimensionVector::of(expected) == computed`), which stays
     well-defined even when multiple named dimensions would match the same
     bare vector, because RFC-0004's own worked example
     (`let area: Area = width * height;`) always supplies that expected
     dimension from an annotation, parameter type, or return type.
   - The module doc comment documents this explicitly (with the specific
     colliding pairs and the RFC-0004 §4 evidence) so `AICAD-049`
     ("dimensional arithmetic type rules") — which will decide what an
     *unannotated* expression like `force * length` type-checks as, if
     anything — starts from this finding instead of rediscovering it.
   - Added dedicated tests (`pressure_and_stress_intentionally_share_a_vector`,
     `torque_and_energy_intentionally_share_a_vector`) asserting the
     collision as *expected* behavior, not treating it as a bug to
     "fix" later by accident.
   This did not rise to an `OWNER_DECISIONS.md` escalation: no ambiguity
   was actually *resolved* here (silently or otherwise) — the task's own
   scope (forward canonicalization + vector algebra) never required
   picking one of Torque/Energy or Pressure/Stress over the other, and I
   deliberately kept the API from being able to make that choice.
   `AICAD-049` is exactly where this will need a real decision (how
   operator type rules resolve/reject the ambiguity), flagged here so that
   task starts informed rather than from scratch.
2. **Angle is kept as an explicit base axis, not folded into
   "dimensionless" (contrary to plain SI convention, where radians are
   dimensionless).** Verified this is necessary, not a style choice: without
   it, `AngularVelocity` (`Angle^1 * Time^-1`) and `Frequency` (`Time^-1`)
   would reduce to the identical vector `Time^-1`, even though RFC-0004 §3
   lists them as separate first-class dimensions and no possible target
   type could distinguish them from raw exponents alone (unlike the
   Pressure/Stress case, which at least the RFC groups by shared units —
   nothing in the RFC groups Frequency and AngularVelocity as
   interchangeable). Test `angle_axis_distinguishes_angular_velocity_from_frequency`
   pins this.
3. **Six base axes (length, mass, time, temperature, angle, current), not
   the full SI seven (omitting luminous intensity and amount of
   substance).** RFC-0004 §3's 21-dimension list never needs either —
   verified by deriving all 21 vectors and finding both axes stay at zero
   throughout. Adding unused axes would be speculative (no RFC-0004
   dimension currently needs them) and would cost every future vector
   comparison two always-zero fields for no benefit; adding a seventh axis
   later, if a future dimension needs it, is a small additive change, not
   a redesign (`DimensionVector`'s fields would simply grow).
4. **`i8` exponents**, matching `AICAD-046`'s report's own reasoning
   pattern: RFC-0004 §3's dimensions all have small integer exponents (the
   largest magnitude present is `Volume`'s `length = 3`); `i8` is exact and
   avoids the false generality of, say, `i32` for values that will never
   exceed single digits.
5. **`powi` stays private (`#[allow(dead_code)]`, used only by this
   crate's own tests) rather than a public API.** No dimension in
   RFC-0004 §3's list needs an exponent power other than what repeated
   `Mul`/`Div` already produce via the tests (`Area = Length^2`, `Volume =
   Length^3`); making it public now would be speculative surface area with
   no current consumer, contrary to `AGENTS.md`'s "smallest correct
   change." Kept implemented (not deleted) since the tests already use it
   to cross-check `Mul`/`Div` consistency, and it costs nothing to leave
   as a private, tested implementation detail.
6. **No escalation triggered.** Finding 1 (Pressure/Stress, Torque/Energy)
   is exactly the kind of ambiguity `AGENTS.md` says must never be resolved
   silently — the response here was to *not* resolve it (no reverse
   lookup, explicit documentation, explicit tests asserting the collision
   as expected), which is the safe default that leaves the actual
   type-checking decision to `AICAD-049` where it belongs, rather than an
   architecture change I made unilaterally.

## Files changed

- Added: `crates/cad-units/src/dimension_vector.rs`.
- Modified: `crates/cad-units/Cargo.toml`, `crates/cad-units/src/lib.rs`,
  `crates/cad-units/README.md`, `project/TASKS.yaml` (AICAD-047
  `status: done`).
- Added: `project/reports/AICAD-047.md` (this report).

## Verification (exact commands/results)

```
$ cargo test -p cad-units
running 18 tests
test dimension_vector::tests::acceleration_is_velocity_over_time ... ok
test dimension_vector::tests::density_is_mass_over_volume ... ok
test dimension_vector::tests::angular_velocity_is_angle_over_time ... ok
test dimension_vector::tests::angle_axis_distinguishes_angular_velocity_from_frequency ... ok
test dimension_vector::tests::distinct_named_dimensions_can_still_compare_unequal ... ok
test dimension_vector::tests::display_omits_zero_exponents ... ok
test dimension_vector::tests::force_is_mass_times_acceleration ... ok
test dimension_vector::tests::length_area_volume_are_powers_of_length ... ok
test dimension_vector::tests::mul_and_div_are_inverse ... ok
test dimension_vector::tests::of_is_total_over_every_named_dimension ... ok
test dimension_vector::tests::power_is_energy_over_time ... ok
test dimension_vector::tests::powi_matches_repeated_multiplication ... ok
test dimension_vector::tests::pressure_and_stress_intentionally_share_a_vector ... ok
test dimension_vector::tests::pressure_is_force_over_area ... ok
test dimension_vector::tests::torque_and_energy_intentionally_share_a_vector ... ok
test dimension_vector::tests::velocity_is_length_over_time ... ok
test dimension_vector::tests::voltage_and_resistance_derive_from_power_and_current ... ok
test dimension_vector::tests::division_by_self_is_dimensionless ... ok
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo fmt -p cad-units -- --check
(exit code 0, no output)

$ cargo clippy -p cad-units --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
```

Full-workspace build/test/lint re-run before this task's commit; results
recorded in `project/SESSION_HANDOFF.md`.

## Tests/regressions

18 new tests: every derived dimension's algebraic derivation from base
quantities (`Force = Mass * Acceleration`, `Pressure = Force / Area`,
`Power = Energy / Time`, `Voltage`/`Resistance` from `Power`/`Current`,
etc.), the Angle-axis distinguishing test, the two intentional-collision
tests, algebra identities (`mul_and_div_are_inverse`,
`division_by_self_is_dimensionless`), `Display`, and `powi` cross-checked
against repeated multiplication. No regressions found or introduced.

## Known limitations

- No `DimensionVector -> Dimension` reverse lookup exists, by design (see
  Decisions §1) — `AICAD-049` needs to decide how (or whether) to resolve
  the Pressure/Stress and Torque/Energy multiplicity when type-checking an
  expression against an expected type.
- No unit literals/conversions yet (`AICAD-048`).
- `DimensionVector` is not yet wired into `cad-ast`/binding/type-checking —
  that starts at `AICAD-050`/`052`.

## Unresolved questions

None requiring owner escalation right now. Flagging for `AICAD-049`'s own
task loop (not this report's escalation list, since nothing here actually
needed resolving to complete `AICAD-047`): how should dimensional
arithmetic type-checking handle an unannotated expression whose computed
vector matches more than one named dimension (e.g. `force * length` with
no `: Torque`/`: Energy` target)? Options include (a) requiring an
explicit target type whenever the vector is ambiguous and erroring
otherwise, (b) a documented tie-break preference, or (c) treating it as
always requiring annotation for every derived (non-primitive) dimension,
consistent-explicit rather than case-by-case. This is exactly the kind of
"ambiguous reference selects silently" question `AGENTS.md` says should not
be resolved without evidence/deliberate design — recommend `AICAD-049`
treat it as a first-class design decision (escalating to
`project/OWNER_DECISIONS.md` if the evidence at that point doesn't make
one option clearly correct), not an incidental implementation detail.
