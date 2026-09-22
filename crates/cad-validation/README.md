# cad-validation

Shape validation and healing-report normalization
(`validate()`/`heal()` per `docs/plan/23_CROSS_SYSTEM_PARAMETER_CATALOG.md`
§7-8), supporting WP-01/WP-05. `validate()` must never silently heal unless
explicitly requested.

## Status (`AICAD-120`, Stage 5)

`src/healing.rs`: `RepairPolicy` — the explicit modeling/construction
tolerance policy (`project/DECISION_LOG.md#DL-24` domain 2) a `sew`/`heal`
call uses. Kernel-neutral: no `cad-occt-bridge`/`cad-kernel-api` dependency.
The real native sewing (`BRepBuilderAPI_Sewing`)/healing (`ShapeFix_Shape`)
calls and their own raw structured evidence (`SewReport`/`HealReport`,
including the empirically-discovered `HealReport::kind_changed` — an
unclosable Solid silently demoted to a valid-looking Shell) live in
`cad-occt-bridge`; `cad_geometry_api::ir::GeometryOp::{Sew,Heal}` and
`cad-geometry-runtime`'s dispatch of them carry a plain tolerance
`Quantity`, matching every other dimensioned `GeometryOp` field.

## Status (`AICAD-064A`, Stage 3)

`src/profile.rs`: `ComparisonProfile` — the D5 v1 comparison-profile
module `project/DECISION_LOG.md#DL-12` assigned to this crate. Kernel-
neutral (no `cad-occt-bridge`/`cad-kernel-api` dependency in production
code — those are dev-dependencies, used only by the calibration evidence
below). Implements `DL-12`'s versioned, dimension-aware equivalence shape
(`max(abs, rel * S^k)` for linear/area/volume, a plain linear tolerance for
center-of-mass) with the complete v1 constant set:

```text
linear_abs         = 0.0001   (owner-accepted, DECISION_LOG.md#DL-17)
linear_rel         = 0.0      (AICAD-064A: no scale-dependent growth observed, 1mm-1000mm)
area_abs           = 0.000001 (AICAD-064A)
area_rel           = 0.001    (AICAD-064A)
volume_abs         = 0.000001 (AICAD-064A)
volume_rel         = 0.001    (owner-accepted, DECISION_LOG.md#DL-17)
center_of_mass_abs = linear_abs (owner-accepted, DECISION_LOG.md#DL-17)
```

`tests/calibration.rs`: the bounded multi-scale calibration corpus
(`project/DECISION_LOG.md#DL-17` requires it before `AICAD-064A` may set
the remaining four constants) — primitives (box, cylinder), a rigid
transform, booleans (union/intersect/cut), fillet, chamfer, a near-zero-
volume fixture, and repeated-rebuild drift, each at four characteristic
scales (1mm/10mm/100mm/1000mm). Every fixture matched its closed-form
expected value to within double-precision floating-point rounding
(volume/area relative error ≤ ~5e-16); linear quantities (bounding box,
center of mass) showed a small, scale-*independent* ~1e-7 absolute
deviation (a fixed OCCT geometric-tolerance margin, not growing with
scale); repeated rebuilds were bit-for-bit identical across 5 repeats at
every scale. See `project/reports/AICAD-064A.md` for the full measured
table and derivation reasoning. 7 tests (5 in `src/profile.rs`, 2 in
`tests/calibration.rs`).

Plan references: `docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`
(`validate`, `heal`); `docs/plan/23_CROSS_SYSTEM_PARAMETER_CATALOG.md` §7-8;
`docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md`; `project/DECISION_LOG.md`
`#DL-12`/`#DL-17`; `project/OWNER_DECISIONS.md#D19`.
