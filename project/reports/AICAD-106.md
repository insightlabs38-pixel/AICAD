# AICAD-106: Establish typed Stage-5 modeling/construction and approximation tolerance primitives

## Status

Done.

## Objective

Give AICAD typed primitives for two of `project/DECISION_LOG.md#DL-26`'s
six independent numerical-tolerance domains — domain 2 (modeling/
construction) and domain 3 (approximation) — distinct from D5/D19's
comparison profile, the sketch solver's own tolerance, and each other, per
this task's own title scope.

## Base / resulting commit

Base: `f1bdfce` (`origin/claude/aicad-stage5-dev`, AICAD-104).

## What changed

### `crates/cad-units/src/tolerance.rs` (new)

- `ConstructionTolerance` (domain 2): a `Length`-dimensioned, canonical-unit
  (metres), strictly positive/finite magnitude. Grants exactly one
  evidenced default, `shell_offset_default()` — the literal `1e-6` the
  native bridge's own `aicad_occt_shell`/`aicad_occt_offset` OCCT join
  calls have hardcoded since Stage 1. Reusing an already-shipped value is
  not the same as inventing one, so this does not violate `DL-26`'s
  "no domain may invent a default without evidence" rule.
- `ApproximationTolerance` (domain 3): a `Length`-dimensioned linear
  component plus an `Angle`-dimensioned angular component (the same two
  quantities `GeometryQuery::Tessellate` already requires), both
  canonical-unit, strictly positive/finite. **No default constructor at
  all** — no shipped call site has ever hardcoded a tessellation deflection
  value (every existing `tessellate`/`Tessellate` call site supplies its
  own caller-chosen magnitude), so no evidenced default exists to reuse and
  none was invented.
- `ToleranceError` (`NonFinite` / `NonPositive`) — both constructors reject
  invalid input rather than clamping or silently accepting it.
- The two types share no constructor, no `From`/`Into` conversion between
  them, and no path to/from `cad_validation::profile::ComparisonProfile` or
  a sketch-solver tolerance — cross-domain reuse is a compile error, not
  merely a documented convention.

### Real integration (not merely unused new types)

- `crates/cad-query/src/eval.rs`: `Shape::classify_point`'s own tolerance
  (used by the real `contains`/`inside` query predicates,
  `AICAD-100A`) is a real, already-shipped instance of domain 2. It is now
  built through `ConstructionTolerance::new(...)` (a new
  `point_classify_tolerance()` helper) rather than a bare `const f64` —
  **the exact magnitude (`1e-7`) is unchanged**, deliberately distinct from
  `ConstructionTolerance::SHELL_OFFSET_DEFAULT_MAGNITUDE`'s own `1e-6`, per
  `DL-26`'s "no domain may silently inherit another domain's values" rule.
  `CONVEXITY_EPSILON`/`INTERSECTION_VOLUME_EPSILON` in the same file were
  deliberately **not** touched — that module's own pre-existing doc
  comment already reasons these are algorithm-internal numerical-robustness
  inputs, not a caller-overridable `DL-26` policy, and DL-26 does not
  require every internal epsilon to become a typed domain object.
- `crates/cad-geometry-api/src/ir.rs`: new test proving
  `ApproximationTolerance` composes directly into a real
  `GeometryQuery::Tessellate` node (AICAD-owned Geometry-IR state), not
  merely existing as a standalone unused type.

## Rationale for scope (why not touch `classify_point`'s own signature)

Changing `cad_occt_bridge::Shape::classify_point`'s public `tolerance: f64`
parameter to accept `ConstructionTolerance` directly would touch the
kernel-neutral FFI boundary and every call site across the workspace for a
change this task's acceptance criteria do not require. The typed value
flows to the identical `f64` call site via `.canonical_magnitude()` with no
behavior change — establishing the primitive and proving one real,
evidenced integration, not migrating every existing raw-`f64` tolerance in
the codebase.

## Tests added

- `cad-units::tolerance` (7 new): rejection of non-finite/non-positive
  input (both types), valid-magnitude round-trip, the evidenced default
  matches the shipped literal exactly, the two types are distinct at the
  `TypeId` level (a permanent "no such conversion exists" marker), and
  deterministic equality (same magnitude compares equal, different
  magnitude does not) — required for values entering AICAD-owned state to
  have stable, reproducible identity.
- `cad-geometry-api::ir` (1 new):
  `approximation_tolerance_composes_directly_into_a_tessellate_query`.
- `cad-query`'s existing 98 tests re-run unchanged (0 behavior change at
  the two touched call sites, confirmed by identical pass/fail results).

## Verification

```
cargo fmt --all -- --check                                                     # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings           # clean
cargo test -p cad-units -p cad-validation -p cad-geometry-api -p cad-query      # all ok
cargo test --workspace                                                        # 82 test-result blocks, 0 failed
```

## Limitations / follow-up

- Domains 1 (representation/validity), 4 (solver), and 5 (verification) are
  out of this task's own title scope and remain unaddressed — domain 4
  already has an owner (the Stage-3 sketch solver, `DL-20`); domains 1 and
  5 have no source-visible typed surface anywhere yet.
- `ApproximationTolerance` is not yet wired into a real `Tessellate` call
  site the way `ConstructionTolerance` is wired into `classify_point` —
  every existing tessellation call site already supplies its own
  caller-chosen magnitude directly as a `Quantity` pair, and none of them
  needed to change for this task's own scope; the composability proof
  (`cad-geometry-api::ir` test) demonstrates the integration path without
  altering existing call sites.

## Next dependency

`AICAD-108` (Batch S5-02) depends on `AICAD-106`.
