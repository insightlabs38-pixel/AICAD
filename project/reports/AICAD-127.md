# AICAD-127: build and freeze a difficult freeform exact-geometry corpus

## Status

Done. First task of batch `S5-09`.

## Objective

Per `docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md` §6 ("Low-level
completeness benchmark") and this task's own acceptance list: freeze a
versioned corpus of difficult/realistic freeform geometry (spline hook,
variable-section sweep, twisted loft, blade/impeller-like surface,
trimmed surfaces), each fixture with declared exact evidence and
tolerance domain, split into a `public` tuning subset and a `held_out`
adversarial subset, executable through the current `.aicad`/public API.

## Base / resulting commit

Base: `5085d02` (`origin/claude/aicad-stage5-dev`, `AICAD-126`).

## What changed

- `project/benchmarks/stage5_freeform_corpus/` — new frozen corpus: 4
  `public/` fixtures, 3 `held_out/` fixtures, each `case.aicad` +
  `case.md` (declared checked evidence + tolerance domain), plus a
  corpus `README.md`.
- `crates/cad-cli/tests/stage5_freeform_corpus.rs` — new: proves every
  fixture through the real production path (`ParametricBuildSession`),
  mirroring `stage4_reference_benchmark_fixtures.rs`'s own frozen-corpus
  discipline. 7 tests, all passing.
- `project/OWNER_DECISIONS.md` — new non-decision item recording the
  capability gap below (not an owner ruling — future implementation
  work).

## Central finding: a real topology-construction capability gap

Building this corpus's originally-planned positive-path fixtures (a real
spline-hook *face*, a real twisted-loft *solid*, a real trimmed *face*)
discovered that `make_edge`/`make_face_on_surface`
(`crates/cad-runtime/src/interp.rs`'s `curve_to_edge_op`/
`surface_to_surface_spec`, `AICAD-119`) only bridge the original Stage-2
analytic families (`Circle`/`Arc`/trimmed-`Line`;
`Plane`/`Cylinder`/`Cone`/`Sphere`/`Torus`) into real kernel B-rep
topology. A `Bezier`/`BSpline` curve (`AICAD-110`) or surface
(`AICAD-114`), or any `trim_surface` result (`AICAD-115`, even of an
analytic base), is rejected with an explicit
`UNSUPPORTED_TOPOLOGY_CONSTRUCTION` diagnostic — confirmed empirically
while building fixtures 01–03/06, not assumed in advance. This is never a
crash/hang/silent-wrong result (every rejection is immediate and
structured), but it does mean the docs/plan/16 §6 target categories that
fundamentally need a real face/solid are unreachable through the current
public surface for anything beyond the five analytic quadric families —
`AICAD-110`/`114`/`115` gave `.aicad` real freeform curve/surface
**values** (construction, evaluation, trim, intersection, projection,
distance — all already proven kernel-free by
`stage5_curves_checkpoint.rs`/`stage5_queries_checkpoint.rs`), but
`AICAD-119`'s topology dispatch was never extended to cover them.

Per this task's own acceptance criterion ("fixtures are executable
through current `.aicad`/public APIs or explicitly classified as
lower-level stress fixtures; no aspirational syntax is presented as
current usage"), every `public/` fixture below proves the freeform
geometry's own exact value-level math instead of a face/solid it cannot
yet construct; `held_out/06`/`07` turn the gap itself into adversarial
evidence that the rejection is explicit. Full detail:
`project/benchmarks/stage5_freeform_corpus/README.md`'s "Discovered
capability gap" section; recorded as a non-decision item in
`project/OWNER_DECISIONS.md` (closing it is ordinary future
implementation work, not an architecture ruling).

## Corpus contents

| Fixture | Category | Checked evidence |
| --- | --- | --- |
| `public/01_spline_hook` | spline/NURBS hook or bend | clamped-endpoint identity; `closest_point_on_curve` distance self-consistency |
| `public/02_twisted_variable_section_solid` | variable-section sweep + twisted loft | bilinear evaluation identities (center/edge-midpoint); real planar-cap topology, exact area |
| `public/03_twisted_blade_surface` | blade/impeller-like surface | clamped-corner + bilinear-center identities |
| `public/04_trimmed_freeform_patch` | trimmed surfaces | trim-delegation identity (bit-identical to untrimmed base) |
| `held_out/05_out_of_domain_trim_rejected` | numerical/domain adversarial | explicit `OutOfDomain` build failure |
| `held_out/06_near_degenerate_extreme_twist` | near-degenerate + capability gap (surface) | explicit `UNSUPPORTED_TOPOLOGY_CONSTRUCTION` |
| `held_out/07_spline_edge_construction_unsupported` | capability gap (curve) | explicit `UNSUPPORTED_TOPOLOGY_CONSTRUCTION` |

Every check is either an exact closed-form identity (clamped-endpoint/
corner/bilinear-center/edge-midpoint math, computed independently in the
Rust test) or an exact code-path delegation (trim evaluation), plus two
real kernel B-rep faces (`02`'s planar caps) with exact analytic area —
no fixture relies on an unexplained frozen "measured" number with no
independent derivation.

## Tolerance domains used (DL-26)

- Representation exactness (bit-level / `1e-9`–`1e-12` floating-point
  noise floor): all clamped-endpoint/corner/bilinear identities — these
  are closed-form math, not kernel measurements, so no modeling tolerance
  applies.
- D5/verification exactness: `02`'s planar-cap areas (`1e-12 m^2` against
  the exact square-area formula).
- No domain (bit-identical equality): `04`'s trim-delegation check, since
  it is a direct code-path identity, not a numerical approximation.

## No architecture boundary bypassed

No new `RuntimeBuiltin`, type, or kernel-adapter surface was added — this
task only adds fixtures and a proving test against the existing Stage-5
surface. No tolerance was widened to make a case pass; no ambiguity was
resolved arbitrarily; the two `held_out` capability-gap fixtures assert
the *existing* rejection behavior, they do not work around it.

## Verification

```
cargo fmt --all -- --check                                            # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings  # clean
cargo test -p cad-cli --test stage5_freeform_corpus                   # 7/7 passed
cargo test --workspace                                                # 1818 passed, 0 failed
```

(1811 pre-existing + 7 new fixture tests; no other suite regressed.)

## Limitations (honest, not hidden)

1. The capability gap above materially narrows what "difficult freeform"
   corpus fixtures can currently prove as real kernel topology — future
   work (tracked in `OWNER_DECISIONS.md`) should extend
   `curve_to_edge_op`/`surface_to_surface_spec` to dispatch Bezier/
   B-spline/trimmed geometry through OCCT's own matching constructors,
   after which `held_out/06`/`07` should be revisited to exercise their
   originally-intended near-degenerate/adversarial geometry questions
   rather than the interface-level rejection.
2. `01`'s hook profile and `03`'s blade surface are not (yet) real B-rep
   faces — only their value-level math is exact-checked, per the gap
   above.
3. Seven fixtures is a floor, not a claim of exhaustive category
   coverage; `AICAD-128`/`129` may add more as the adversarial campaign
   and example maintenance proceed.

## Next dependency

`AICAD-128` (numerical/robustness/determinism/resource adversarial
campaign) depends on this corpus (satisfied) and reuses its `public`/
`held_out` split as a starting point.
