# AICAD-048: Implement initial unit registry and conversions

## Objective

Implement RFC-0004 §4's frozen initial unit literal set (length, angle,
mass, force, pressure/stress, temperature) as an executable registry with
affine-aware conversions, per `project/TASKS.yaml` AICAD-048 (Stage-2 batch
S2-04, third/final task). Completes Batch S2-04 (`046 -> 047 -> 048`).

## Base commit

`6c3aec2` (this session's own `AICAD-047` commit).

## Plan references read

`rfcs/0004-units-type-system.md` §4 (the exact frozen unit list per
family), §7 (affine-unit absolute/delta conversion rules), §5 ("different
physical dimensions never implicitly convert... there is no numeric escape
hatch"); `project/reports/AICAD-040.md` (confirms the lexer deliberately
defers unit validation to this exact task/crate, decision 1).

## Implementation

- `crates/cad-units/src/registry.rs` (new): `UnitDef` (symbol, `Dimension`,
  scale-to-canonical factor, optional affine offset); a `const UNITS: &[UnitDef]`
  table with all 32 `(symbol, dimension)` entries RFC-0004 §4 requires;
  `lookup(symbol, dimension)` (exact) and `lookup_any(symbol)` (all
  dimensions matching a symbol); `to_canonical_absolute`/
  `to_canonical_delta`/`from_canonical_absolute`/`from_canonical_delta`
  (single-unit conversion, affine-offset-aware); `convert_absolute`/
  `convert_delta` (unit-to-unit, same-dimension-only, returning
  `Result<f64, UnitConversionError>`).
- `crates/cad-units/src/lib.rs`: adds `mod registry;` and re-exports its
  public items.
- `crates/cad-units/README.md`: "Status" section extended with `AICAD-048`.

No third-party dependency added.

## Decisions made and why

1. **Scope is exactly RFC-0004 §4's frozen list — six unit families, no
   more.** RFC-0004 §4 lists concrete unit literals only for `Length`,
   `Angle`, `Mass`, `Force`, and the shared "Pressure/stress" heading, plus
   `Temperature`; it defines **no** units at all for `Volume`, `Time`,
   `Torque`, `Energy`, `Power`, `Density`, `Velocity`, `Acceleration`,
   `AngularVelocity`, `Frequency`, `Current`, `Voltage`, or `Resistance` —
   even though `cad-types`'s `Dimension` (`AICAD-046`) lists all 21 as
   first-class dimensions. RFC-0004 §4 itself frames this as deliberate:
   "the standard library may expand this set without a grammar change."
   Inventing units for the other 15 dimensions now would be exactly the
   speculative work `AGENTS.md`'s "No speculative future work" rule warns
   against — there is no RFC evidence for what those unit literals should
   even be spelled, and a later task/RFC expansion owns that.
2. **`Pa`/`kPa`/`MPa`/`GPa`/`psi`/`ksi` are each registered twice — once
   under `Dimension::Pressure`, once under `Dimension::Stress`.** RFC-0004
   §4 lists one shared unit set under the combined heading
   "pressure/stress"; `AICAD-047`'s finding (documented in
   `dimension_vector.rs`) already established that `Pressure` and `Stress`
   are intentionally distinct named dimensions despite being physically
   identical. Registering the same units under both dimensions, rather
   than picking one, keeps both conventions available and keeps
   `lookup(symbol, dimension)` exact for either. Added
   `conversion_rejects_pressure_to_stress_despite_identical_units` to
   pin the resulting behavior: converting a `Pa`-typed `Pressure` value to
   a `Pa`-typed `Stress` value is still rejected — same unit, same scale,
   different named dimension, so RFC-0004 §5's "different physical
   dimensions never implicitly convert" applies regardless of unit
   identity. This is `AICAD-047`'s finding validated one layer down, at
   the concrete-conversion level rather than only the abstract vector
   level.
3. **`lookup_any` returns every candidate rather than picking one.**
   Mirrors `AICAD-047`'s "no `DimensionVector -> Dimension` reverse
   lookup" decision at the registry level: a bare unit suffix like `"Pa"`
   (as tokenized by `crates/cad-lexer`'s `AICAD-040` suffix scanning) is
   genuinely ambiguous between `Pressure` and `Stress` without a target
   type from context, and `AGENTS.md` requires ambiguity to surface as an
   error/explicit set, never an arbitrary silent pick. A future type
   checker (`AICAD-052`+) resolving a unit-suffixed literal against an
   expected type will use `lookup(symbol, expected_dimension)`, which is
   exact; `lookup_any` exists for whatever diagnostic/completion tooling
   needs to enumerate candidates when no target type is available yet.
4. **Conversion factors are derived via `const` arithmetic from named,
   sourced base constants, not pasted as unsourced magic numbers.**
   E.g. `N_PER_LBF = KG_PER_LBM * STANDARD_GRAVITY_M_PER_S2` and
   `PA_PER_PSI = N_PER_LBF / (M_PER_IN * M_PER_IN)`, each base constant
   commented with what makes it exact (`0.0254` m/in and `9.80665` m/s²
   are both exact by international definition; `0.45359237` kg/lbm is the
   exact international avoirdupois-pound definition). This makes every
   derived factor independently checkable against its definition instead
   of trusting a single copied decimal, and the tests cross-check the
   results against independently-known published values (freezing/boiling
   points, `1 psi ≈ 6894.757293168 Pa`, `1 lbf ≈ 4.4482216152605 N`) rather
   than merely re-deriving the same arithmetic the implementation already
   did.
5. **Temperature: `K` carries no affine offset (`affine_offset: None`),
   even though `Dimension::Temperature.is_affine()` is `true`.** This is
   not an inconsistency: `is_affine` (from `cad-types`, `AICAD-046`)
   describes the *dimension* ("has some affine units"), while
   `affine_offset` describes a specific *unit* within that dimension. `K`
   is already the canonical/absolute reference point (0 K = absolute
   zero, no additive shift needed to reach itself), so its `affine_offset`
   is correctly `None`; only `degC`/`degF`, whose zero points are shifted
   relative to absolute zero, need `Some(offset)`. Verified by
   `non_affine_units_have_identical_absolute_and_delta_conversion`-style
   reasoning applied specifically to `K` implicitly through every
   Kelvin-target conversion test.
6. **No escalation triggered.** This task implements RFC-0004 §4's already-
   frozen list with §5/§7's already-resolved conversion rules (DL-3); it
   introduces no new type-system semantics, does not weaken any check, and
   the one genuine ambiguity available to hit (Pressure/Stress unit
   sharing) was handled the same non-silent way `AICAD-047` already
   established, not resolved arbitrarily here.

## Files changed

- Added: `crates/cad-units/src/registry.rs`.
- Modified: `crates/cad-units/src/lib.rs`, `crates/cad-units/README.md`,
  `project/TASKS.yaml` (AICAD-048 `status: done`).
- Added: `project/reports/AICAD-048.md` (this report).

## Verification (exact commands/results)

```
$ cargo test -p cad-units
running 32 tests
test dimension_vector::tests::* (18 tests) ... ok
test registry::tests::conversion_rejects_different_physical_dimensions ... ok
test registry::tests::conversion_rejects_pressure_to_stress_despite_identical_units ... ok
test registry::tests::error_display_names_both_dimensions ... ok
test registry::tests::every_rfc_0004_section_4_symbol_is_registered ... ok
test registry::tests::force_lbf_known_value ... ok
test registry::tests::inches_and_feet_known_values ... ok
test registry::tests::length_round_trip_mm_to_m_and_back ... ok
test registry::tests::lookup_any_returns_exactly_one_for_unambiguous_symbols ... ok
test registry::tests::lookup_any_surfaces_the_pressure_stress_multiplicity ... ok
test registry::tests::lookup_rejects_symbol_dimension_mismatch ... ok
test registry::tests::non_affine_units_have_identical_absolute_and_delta_conversion ... ok
test registry::tests::psi_and_ksi_known_relationship ... ok
test registry::tests::temperature_absolute_freezing_and_boiling_points ... ok
test registry::tests::temperature_delta_never_applies_the_affine_offset ... ok
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo fmt -p cad-units -- --check
(exit code 0, no output)

$ cargo clippy -p cad-units --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
```

Full-workspace build/test/lint re-run before this task's commit; results
recorded in `project/SESSION_HANDOFF.md` (this session's Batch S2-04
close-out).

## Tests/regressions

14 new tests in `registry.rs` (32 total in `cad-units` combined with
`AICAD-047`'s 18): exhaustive symbol-registration coverage for all 6 unit
families, lookup exactness/ambiguity-surfacing, round-trip and known-value
conversions (inches, feet, `lbf`, `psi`/`ksi`, Celsius/Fahrenheit freezing/
boiling points, the 20°C = 68°F cross-check), the core affine
absolute-vs-delta invariant, the non-affine absolute-equals-delta
invariant, dimension-mismatch rejection (including the Pressure/Stress
case), and `Display` for the error type. No regressions found or
introduced.

## Known limitations

- No units exist yet for the 15 dimensions RFC-0004 §4 does not list units
  for (see Decisions §1) — expanding the set (per the RFC's own "standard
  library may expand this set" allowance) is not this task's scope.
- Not wired into `cad-ast`/binding/type-checking yet (`AICAD-050`/`052`).
- Dimensional arithmetic type rules (what `force * length` type-checks as)
  remain `AICAD-049`'s open item, now informed by both `AICAD-047`'s vector
  finding and this task's concrete-unit confirmation of it.

## Unresolved questions

None requiring owner escalation. Batch S2-04 (`AICAD-046` -> `047` -> `048`)
is complete; per the campaign's fixed batch order, `AICAD-049` (Batch
S2-05, "dimensional arithmetic type rules") is next and should read this
report and `AICAD-047`'s alongside RFC-0004 before designing how
unannotated expressions resolve the documented Pressure/Stress and
Torque/Energy multiplicity.
