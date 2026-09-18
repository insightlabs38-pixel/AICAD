# AICAD-102: Complete dimensional and spatial source-value construction required by Stage-5 queries

## Status

Done. Batch S5-00, second task (depends only on `AICAD-100A`, independent of `AICAD-101`).

## Objective

`AICAD-100A` found that Stage-4 source-query lowering could not honestly
expose Area-dimension comparisons because the frozen source unit/value
surface lacked an `Area` spelling. This task closes that gap and confirms
the Point3/Vector3/Direction/Axis3/Frame3 construction Stage-5 queries
need.

## Base / resulting commit

Base: `cf67a59` (`claude/aicad-stage5-dev`, `AICAD-104A`). This task's
commits are the remainder of `git log cf67a59..HEAD` on that branch.

## Investigation finding: most of this was already done

Before writing any code, a full read of `cad-units`/`cad-hir`/`cad-runtime`
found:

- `cad_types::Dimension::Area` already existed, and `Length * Length ->
  Area` dimensional-arithmetic derivation (`crates/cad-units/src/
  arithmetic.rs::check_binary_arithmetic`) already worked — a source
  program could already write `let a = 10mm * 10mm;` and get a real,
  correctly-typed `Area` value. `rfcs/history/stage0/0004-units-type-
  system.md` §3 documents this exact idiom as RFC-0004's own anticipated
  construction path (`let area: Area = width * height;`).
- `Point3`, `Vector3<T>`, `Point2`, `Vector2<T>`, `Axis3`, `Frame3`, `Plane`
  are already real, always-seeded standard source-level `struct` types
  (`crates/cad-hir/src/geometry_types.rs`'s `GEOMETRY_TYPES_SOURCE`, seeded
  unconditionally by `crates/cad-hir/src/lower.rs::seed_standard_types`,
  `DL-21`'s type-closure rule). Real source construction syntax
  (`Point3(x = ..., y = ..., z = ...)`) already works and is already
  exercised through genuine `.aicad` source text by `crates/cad-runtime/
  src/spatial.rs`'s own existing test suite (`point3_from_value`,
  `axis3_from_value`, `direction3_from_value`, `frame3_from_value`,
  `plane3_from_value`). There is no standalone `Direction` struct type by
  design — RFC-0004 §7 rules that a kernel-neutral validated concept does
  not automatically need its own separate source spelling; a direction is
  an ordinary `Vector3<Float>`, normalized where a kernel conversion needs
  a unit vector (`direction3_from_value`).

The genuine, sole missing piece: **no `Area`-dimensioned unit-literal
spelling** (`mm2`, `m2`, ...) existed in `crates/cad-units/src/
registry.rs`'s frozen unit table, so a program could not write `500mm2`
directly (only the derived-multiplication form worked).

## What was implemented

`crates/cad-units/src/registry.rs`'s `UNITS` table gained one squared
`Area` unit per already-frozen `Length` unit (`nm2 um2 mm2 cm2 m2 km2 in2
ft2`), each scaled by that length unit's own factor squared. This is not
an escalation-worthy new architecture decision: RFC-0004 §4 itself is
explicit that "the standard library may expand this set without a grammar
change," and `cad_types::Dimension::Area` was already a first-class named
dimension in RFC-0004 §3's own frozen list — only its literal spelling was
deferred. No lexer, parser, or type-checker code changed: `crates/cad-
lexer` already fuses any identifier-shaped suffix into a candidate unit
with zero validation, and both `cad_hir::lower`/`typeck` resolve a
literal's dimension purely via `cad_units::lookup_any(symbol)`, which is
registry-data-driven — adding rows to `UNITS` was sufficient for `500mm2`
to type-check and evaluate correctly through the entire existing pipeline.

`cad_query::value::Magnitude::new(value, ty)` was already fully
dimension-generic (it wraps `OperandType` as-is); no code change was
needed there either for an `Area`-tagged `Magnitude` to be constructible.

No public syntax changed beyond the new unit-literal spellings themselves
(an RFC-authorized standard-library expansion, not a grammar change). No
bare-`f64`/display-unit shortcut was introduced — `500mm2` carries the
same `OperandType::dimensional(Area, None)` tag and canonical-square-metre
storage every other unit literal already uses.

## Tests added

`crates/cad-units/src/registry.rs`: `area_unit_literals_are_registered_
as_the_standard_librarys_own_rfc_0004_4_expansion` (positive, all 8
symbols), `area_round_trip_mm2_to_m2_and_back` (round-trip), `area_unit_
conversion_rejects_length` (dimension-mismatch), `square_feet_known_value`
(known-value cross-check).

`crates/cad-hir/src/lower.rs`: `area_literal_resolves_to_dimensional_area`
(positive, HIR literal type).

`crates/cad-hir/src/typeck.rs`: `area_unit_literal_type_checks_to_
dimensional_area` (positive), `length_times_length_infers_area_with_no_
annotation_needed` (positive, pre-existing derived path pinned as part of
this task's own coverage), `area_plus_length_is_rejected_not_coerced`
(dimension-mismatch, `UNIT-E104`), `area_literal_and_derived_area_are_the_
same_dimension` (positive: literal and derived Area freely add).

`crates/cad-runtime/src/interp.rs`: `area_literal_evaluates_to_canonical_
square_metres` (round-trip source-to-runtime, checks both the canonical
magnitude and the runtime `OperandType`), `area_literal_and_derived_area_
add_to_the_same_canonical_value` (round-trip).

`crates/cad-query/src/predicate.rs`: `geometry_predicate_area_comparison_
can_carry_an_area_dimensioned_threshold` (positive: a real `Area`-tagged
`Magnitude`, not the pre-existing test's `Length`-tagged placeholder, can
now be constructed and carried by `GeometryPredicate::Area`).

## Verification

```
cargo fmt --all -- --check                                                          # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings                # clean
cargo test -p cad-units -p cad-types -p cad-hir -p cad-runtime -p cad-query -p cad-cli
                                                                                       # all green (cad-units: 79, cad-hir: 242, cad-runtime: 148, cad-query: 79)
cargo test --workspace                                                              # 80 test-result blocks, all ok, 0 failed
python3 scripts/ci/semantic_ref_harness.py validate                                 # {"cases": 10, "splits": {"held_out": 3, "public": 7}, "status": "ok"}
```

## Limitations / follow-ups (for AICAD-103/104)

- `.aicad` query-clause arguments (`HirQueryArg`) only accept `Name`/
  `Number` tokens today — there is no clause-grammar syntax yet to spell a
  nested struct-literal/`Point3`/`Frame3` argument for `nearest_to(...)`/
  `farthest_from(...)`/`relative_to(...)`. `AICAD-103`'s own job.
- `cad_query::predicate::SpatialTarget`/`Point3`/`Frame3` are a distinct,
  simpler plain-data type from `cad_hir::geometry_types`'s source-level
  struct; a lowering/bridging layer between the two is needed regardless
  of clause-grammar extension — `AICAD-103`'s own job.
- Found but deliberately deferred: `cad_query::eval::compare_magnitude`
  compares a `Magnitude`'s raw `.value` against the kernel-computed actual
  value without independently re-validating `.ty`'s dimension against the
  predicate it is attached to (e.g. nothing currently stops constructing
  `GeometryPredicate::Area(Comparison::Gt(<a Length-tagged Magnitude>))`
  in Rust, as the pre-existing `geometry_predicate_area_comparison_
  carries_a_typed_threshold` test happens to do). Not fixed here: this
  task only adds source-value *construction*, and the type checker already
  prevents a real `.aicad` program from ever producing a mistagged
  `Magnitude` in the first place once `AICAD-103` wires a real `area(...)`
  clause through the type-checked literal path — so the actual
  reachable-from-source risk is nil, but the defensive check is worth
  adding in `AICAD-103` when that lowering code is written, since that is
  where a real implementation mistake could otherwise slip through
  silently.
