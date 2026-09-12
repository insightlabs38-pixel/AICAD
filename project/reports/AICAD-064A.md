# AICAD-064A — Implement/calibrate the D5 v1 cad-validation comparison profile

## Result
PASS

## Objective
Implement the D5 v1 comparison-profile module in `crates/cad-validation`
(`project/DECISION_LOG.md#DL-12` assigned this crate; `#DL-17` accepted
three constants directly evidenced by `AICAD-034` and required this task
to derive the remaining four — `linear_rel`, `area_abs`, `area_rel`,
`volume_abs` — from a bounded multi-scale calibration corpus per
`project/OWNER_DECISIONS.md#D19`). First task of Stage-3 Batch S3-00, per
`project/TASKS.yaml`.

## Base / resulting commit
Base: `0a5202d` (this branch's own Stage-3 setup commit, off
`origin/main` at `0b6b0b3`). Resulting commit: this task's own commit on
`claude/aicad-stage3-dev`.

## Changes
- `crates/cad-validation/src/lib.rs` — module doc comment, `pub mod
  profile;`.
- `crates/cad-validation/src/profile.rs` (new) — `ComparisonProfile`:
  `DL-12`'s versioned, dimension-aware equivalence shape
  (`linear_tolerance`/`area_tolerance`/`volume_tolerance`/
  `center_of_mass_tolerance`, each `max(abs, rel * S^k)` except
  center-of-mass, a plain linear tolerance) plus `*_matches` comparison
  helpers and `ComparisonProfile::v1()`. Kernel-neutral: zero dependency
  on `cad-occt-bridge`/`cad-kernel-api` in this file. 5 unit tests.
- `crates/cad-validation/Cargo.toml` — `cad-kernel-api`/`cad-occt-bridge`
  as **dev-dependencies only** (never in `[dependencies]`), for the
  calibration test's own real-kernel evidence gathering.
- `crates/cad-validation/tests/calibration.rs` (new) — the bounded
  multi-scale calibration corpus: 2 tests, one measuring absolute/
  relative error across every required category at four characteristic
  scales, one measuring repeated-rebuild drift.
- `crates/cad-validation/README.md` — status section recording the
  complete v1 constant set and a summary of the calibration evidence.

## Material decisions

### Where the comparison profile lives, and why it stays kernel-neutral
`crates/cad-validation`'s existing README already assigns it "shape
validation and healing-report normalization"; `DECISION_LOG.md#DL-12`
independently assigns it the D5 comparison-profile module. Both fit the
same crate (WP-01/WP-05's own validation/verification layer). The
comparison-profile logic itself (`src/profile.rs`) is pure `f64`
arithmetic with zero kernel dependency — it never needs to know how a
measurement was produced, only how to compare it against a tolerance.
`cad-kernel-api`/`cad-occt-bridge` are added as **dev-dependencies only**
(used exclusively by `tests/calibration.rs`, the one-time evidence-
gathering harness this task's own acceptance criteria require) — this
preserves `AGENTS.md`'s "public language/API must not expose OCCT-specific
classes" for this crate's actual public API while still letting the
calibration corpus exercise the real kernel bridge for genuine evidence
(dev-dependencies never appear in a published crate's dependency graph or
public API surface).

### Calibration corpus design
Four characteristic scales (1mm/10mm/100mm/1000mm, matching `DL-17`'s
explicit span) × the eight required categories: primitives (box,
cylinder), a rigid transform (translate + rotate, which must preserve
volume exactly), booleans (union, intersect, a clean through-cut against a
cylinder), fillet (a single convex vertical edge), chamfer (same), an
added near-zero-volume fixture (a very thin flat box — `project/
OWNER_DECISIONS.md#D19` explicitly flagged this as untested by any prior
evidence), analytically checkable area (box/cylinder total surface area),
center of mass (box symmetry), and repeated-rebuild drift (5 fresh
rebuilds of a combined fillet+chamfer fixture per scale). Every closed-form
expected value is derived independently of the kernel, reusing the exact
convex-fillet/chamfer removed-volume formulas `AICAD-027`/`AICAD-034`
already established (`r^2 * (1 - pi/4) * L` for a convex fillet, `d^2/2 *
L` for a chamfer). Edge selection for fillet/chamfer uses geometric
bounding-box matching, never `get_edge`'s raw enumeration index, mirroring
`crates/cad-occt-bridge/tests/stage1_bracket.rs`'s own established
pattern. Absolute error, relative error, and repeat-run drift are recorded
as three separate measurements per `DL-17`'s explicit instruction, never
conflated into one number; no cross-platform claim is made from these
same-machine repeated runs.

### A genuine finding during test-writing: OCCT's own edge-bounding-box tolerance
The fillet/chamfer edge-selection helper initially used an exact-zero
width/position comparison (mirroring `stage1_bracket.rs`'s own looser
predicate) and failed to find any edge at all. Direct inspection (a
throwaway debug binary printing every edge's bounding box on a bare cube)
found OCCT's own edge bounding box carries a small, **fixed** geometric
tolerance margin of exactly `1e-7` (length units), constant across all
four tested scales (1mm through 1000mm) — not a scale-relative fuzz. This
is itself calibration evidence (see "linear_rel derivation" below), and
the edge-selection helper's tolerance was widened to `1e-5` (a ~50x margin
over that observed floor) to accommodate it.

A second, related finding: the repeated-rebuild fixture initially tried to
fillet **and** chamfer the same box edge (the origin corner) in sequence.
This fails structurally, not numerically: once an edge is filleted, that
edge's own straight geometry no longer exists (it is replaced by a curved
fillet surface), so no vertical edge remains at that exact corner to
chamfer. The fixture was corrected to fillet the origin corner and chamfer
the *opposite* corner instead — a real modeling-order lesson (`AGENTS.md`'s
"raw topology is ephemeral" applies to *feature-order edge selection*
inside one build, not only to persistent identity across rebuilds), not a
kernel defect.

### Derivation reasoning for each of the four newly-derived constants
Full measured table: `project/reports/AICAD-064A.md` §"Measured evidence"
below (reproducible via `cargo test -p cad-validation --test calibration
-- --nocapture --test-threads=1`).

- **`linear_rel = 0.0`.** The only linear-quantity errors observed
  (bounding-box corners, center of mass) were exactly `1e-7` in absolute
  terms at *every* scale tested (1mm through 1000mm) — the same fixed
  OCCT edge/bbox tolerance margin found above, which does not grow with
  scale. This is real evidence (not the "no evidence either way" gap
  `D19` originally flagged) directly supporting `D19`'s own suggested
  conservative fallback: "set `linear_rel = 0` (pure absolute floor) until
  a multi-scale fixture produces real evidence." That fixture now exists,
  and it shows no scale-dependent growth up to 1000mm — this constant
  should be revisited if a future task exercises characteristic scales
  well beyond 1000mm (e.g. large assemblies) and finds growth this
  corpus's own tested range could not observe.
- **`area_rel = 0.001` (1e-3).** Measured area relative error across every
  box/cylinder fixture and all four scales topped out at `1.895e-16` —
  essentially double-precision floating-point rounding, since OCCT's
  B-rep area computation is exact analytic integration, not a mesh
  approximation. This is now *directly* measured evidence (D19's original
  gap: "no report measured an area comparison directly"), not a
  dimensional-analogy guess. `1e-3` mirrors the already owner-accepted
  `volume_rel`'s own order of magnitude for the identical class of exact
  B-rep property computation (area and volume are computed by the same
  kind of analytic integration over the same B-rep, and both showed the
  same ~1e-16 same-machine noise floor here) — a ~1e12x safety margin over
  the observed same-machine floor, consistent with `D19`'s own caution
  against inferring cross-platform behavior from same-machine runs (a much
  larger, kernel-version/platform-tolerant margin than the observed noise
  would justify on its own).
- **`area_abs = 1e-6` (mm²).** The largest absolute area error observed
  across every scale was `9.313e-10` mm² (at the 1000mm scale). `1e-6` is
  a ~1000x margin over that floor, matching `DL-17`'s own established
  margin philosophy for `linear_abs` (roughly 100x over its own observed
  floor).
- **`volume_abs = 1e-6` (mm³).** The dedicated near-zero-volume fixture (a
  box `S x S x (S * 1e-6)`, volumes ranging from `1e-6` mm³ at the 1mm
  scale to `1000` mm³ at the 1000mm scale) matched its closed-form volume
  with **zero** measured absolute error at every scale — this directly
  closes `D19`'s explicit gap ("no fixture ever compared a near-zero
  volume, so no floor value has evidence either way"). `1e-6` is chosen as
  a small, consistent-order-of-magnitude floor alongside `area_abs`,
  comfortably above the (unmeasurable-as-nonzero) observed floor.

No constant required escalation back to `project/OWNER_DECISIONS.md` — the
calibration evidence clearly justified all four (per `AGENTS.md`'s
"escalate rather than guess if measurements do not justify a sane
default"; here they did).

## Measured evidence (summary; full table in `tests/calibration.rs`'s own
`--nocapture` output, reproduced in this task's session log)

| Category | Max relative error | Max absolute error | Notes |
|---|---|---|---|
| Volume (box, cylinder, transform, union, intersect, cut, fillet, chamfer) | 4.547e-16 | — | across all 4 scales |
| Area (box, cylinder) | 1.895e-16 | — | across all 4 scales |
| Linear (bbox corners, center of mass) | — | 1.000e-7 | constant across all 4 scales (fixed OCCT tolerance, not scale-relative) |
| Near-zero volume | 0 (exact) | 0 (exact) | 1mm-1000mm scale, thickness = scale * 1e-6 |
| Repeated rebuild (5x, fillet+chamfer) | 0 (exact) | 0 (exact) | bit-for-bit identical at every scale, same machine |

## Verification
- `cargo fmt --all -- --check` → clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings, 27 crates (unchanged count — no new crate added).
- `cargo test -p cad-validation` → 7 passed (5 `src/profile.rs`, 2
  `tests/calibration.rs`), 0 failed.
- `cargo test -p cad-validation --test calibration -- --nocapture
  --test-threads=1` → both tests pass; full measured table printed (see
  "Measured evidence" above for the summary this table was derived from).
- `cargo test --workspace` → 0 failures across every crate (no regression
  in any Stage-1/Stage-2 test).
- `cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1` →
  3/3 (Stage-2 gate proof unaffected).

## Regressions/tests
None found. 7 new tests added (5 profile unit tests + 2 calibration
integration tests); no existing test modified or weakened.

## Findings / limitations
- The calibration corpus is same-machine, single-kernel-version evidence
  only (Rust 1.98.1, OCCT 7.6.3, this session's Ubuntu 24.04 container) —
  per `D19`'s own explicit caution, no cross-platform claim is made or
  should be inferred from it. A future cross-platform/cross-kernel-version
  D5 Level-3/4 benchmark (`DL-12`) remains separate, later work.
- `linear_rel = 0.0` is evidenced only up to the 1000mm characteristic
  scale this corpus tested; a future task working at much larger
  characteristic scales (e.g. large assemblies, Stage 6+) should re-check
  this assumption against evidence at that scale before relying on it
  unmodified.
- `crates/cad-validation`'s own original scope (shape validation/healing-
  report normalization, `validate()`/`heal()`) remains entirely
  unimplemented — this task added only the comparison-profile module
  `DL-12` assigned here; the validation/healing work is a separate,
  not-yet-scheduled task.

## Owner blockers
None. `D19` is now fully resolved (all seven v1 constants set); no new
escalation filed.

## Next dependency
`AICAD-065` ("Implement first-class param declarations and derived
expressions"), the second and final task of Batch S3-00.
