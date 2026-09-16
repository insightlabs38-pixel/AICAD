# AICAD-092: Implement geometry-fingerprint evidence/ranking/benchmark support without automatic recovery

## Status

Done. Second task of Batch S4-04.

## Objective

Per `project/DECISION_LOG.md#DL-8` (D7): "Fingerprinting may initially be
used only for diagnostics, ranking candidates shown to a user, and
experiments/benchmarking." Before this task, `FingerprintEvidence`
(`AICAD-080`) was pure static data — nothing computed it from real
geometry, compared two fingerprints, or ranked candidates by similarity;
`crate::resolve::resolve_reference` (`AICAD-088`) already unconditionally
refuses to auto-resolve a `ConstructionStrategy::GeometricFingerprint`
recipe (`BrokenReason::FingerprintAutoResolutionDisabled`), and that
refusal is explicitly **not** touched by this task. This task builds the
three allowed uses themselves — computing real evidence, comparing/ranking
it, and surfacing that as diagnostic/benchmark data — while proving, with a
dedicated end-to-end test, that doing so never changes the resolver's own
fail-closed outcome.

## Base / resulting commit

- Base: this invocation's own `AICAD-091` commit (same session).
- This task's commit: see `git log` (`AICAD-092` commit).

## What was implemented

New module `crates/cad-query/src/fingerprint.rs`:

- **`candidate_fingerprint(candidate)`** — computes a real
  `FingerprintEvidence` from a live `Candidate`, reusing the exact same
  `cad-occt-bridge` accessors `crate::eval`/`crate::resolve`/
  `crate::diagnostics` already use (`center_of_mass`/`vertex_point` for
  position, `area()`, `face_radius()`/`edge_radius()`, `face_normal()`).
  Only the position lookup is mandatory (`Result<_, KernelError>`); area/
  radius/normal are each independently best-effort and simply omitted when
  they do not apply to the candidate's kind — the same "omit rather than
  fabricate" discipline `crate::diagnostics::candidate_summary` already
  established.
- **`fingerprint_distance(a, b)`** — a partial-match dissimilarity score:
  Euclidean distance between `position`s (always compared), plus Euclidean
  distance between `normal`s and absolute difference between `area`s/
  `radius`es *only when both sides supply that field*. A field present on
  only one side contributes nothing to the score in either direction —
  documented explicitly as a deliberate honesty choice, not an oversight.
- **`RankedCandidate`**/**`rank_by_fingerprint(target, candidates)`** —
  orders a candidate set by `fingerprint_distance` ascending (most similar
  first). Never filters by a threshold, never drops a computable
  candidate, never designates a single "answer" — every caller still sees
  the full ordered list. A candidate whose fingerprint cannot be computed
  at all (a real kernel error) is dropped rather than assigned an
  arbitrary score.

`crates/cad-query/src/diagnostics.rs`:

- **`with_fingerprint_ranking(diagnostic, ranked)`** — a new, separate,
  explicitly opt-in combinator that attaches a `rank_by_fingerprint`
  result to an already-built `Diagnostic`'s `backend_details` under a
  `"fingerprint_ranking"` key (an array of `{"distance", "candidate"}`
  objects, most similar first), preserving any existing `"durability"`/
  `"fingerprint"` fields `AICAD-091`/`090` already put there. Neither
  `ambiguous_reference_diagnostic` nor `broken_reference_diagnostic` calls
  it automatically — a caller must ask for ranking explicitly, keeping the
  "never automatic" boundary visible at the call site, not just in the
  resolver.

`crates/cad-query/src/lib.rs` — `pub mod fingerprint;` plus re-exports;
module doc comment updated.

No change to `crate::resolve` at all — `ConstructionStrategy::
GeometricFingerprint` handling, `BrokenReason::
FingerprintAutoResolutionDisabled`, and D7's fail-closed contract are
exactly as `AICAD-088` left them.

## Proving "never automatic recovery"

`fingerprint::tests::rank_candidates_never_narrows_or_selects_
automatically` is the task's own central regression: it ranks a real
ambiguous cube's six tied faces by fingerprint similarity to one of them
(getting a fully populated, correctly-ordered ranking back), then builds a
`ConstructionStrategy::GeometricFingerprint` reference for that same target
evidence and calls `crate::resolve::resolve_reference` on it directly —
asserting the outcome is still `Broken(FingerprintAutoResolutionDisabled)`,
completely unaffected by the ranking just computed from the same real
geometry. This is an explicit end-to-end proof, not an inference from
"the code doesn't call resolve.rs" — the resolver path and the ranking path
are exercised against the identical shape in the same test.

## Tests / verification

9 new tests:

`crates/cad-query/src/fingerprint.rs` (7):
- `candidate_fingerprint_reports_real_area_and_normal_for_a_planar_face` /
  `candidate_fingerprint_reports_real_radius_for_a_cylindrical_face` — real
  geometry, real values, honest omission of the field that does not apply.
- `fingerprint_distance_is_zero_for_identical_evidence` /
  `fingerprint_distance_ignores_a_field_present_on_only_one_side` /
  `fingerprint_distance_accumulates_position_and_area_difference` — the
  distance metric's own contract, including the deliberate "absent field
  never counted" rule.
- `rank_by_fingerprint_orders_a_boxs_faces_nearest_first` — a real
  non-cube box's six faces ranked against a real target face's own
  fingerprint; asserts the list is fully populated, sorted, and the
  target's own face ranks itself first at distance ~0.
- `rank_candidates_never_narrows_or_selects_automatically` — see above.

`crates/cad-query/src/diagnostics.rs` (2):
- `with_fingerprint_ranking_adds_a_ranking_array_alongside_existing_
  backend_details` — durability + fingerprint evidence (both set by
  `broken_reference_diagnostic`) survive alongside a newly attached
  ranking; the ranking itself is fully populated (6 entries) and sorted.
- `with_fingerprint_ranking_on_a_diagnostic_with_no_prior_backend_
  details` — the combinator also works cleanly when there was nothing to
  preserve.

Commands run:

- `cargo fmt --all -- --check` -> clean (whole workspace, after `cargo fmt
  --all`).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  -> zero warnings (whole workspace; one `collapsible_if` finding in
  `fingerprint.rs` fixed during development using an `if let` chain).
- `cargo test -p cad-query` -> 74/74 passed.
- `cargo test --workspace` -> 0 failed, 1,186 total passing tests (1,177
  baseline + 9 `AICAD-092`).
- `python3 scripts/ci/semantic_ref_harness.py validate`/`self-test` -> both
  `"status": "ok"`, unchanged (frozen `AICAD-079A` corpus untouched; this
  task adds reusable ranking primitives a future benchmark harness may
  call, but does not itself build or modify one — see Limitations).
- `python3 scripts/ci/stage4_task_audit.py --check` -> `Stage-4 task
  metadata audit OK`.

## Limitations

- **No benchmark harness wiring** — `rank_by_fingerprint`/
  `candidate_fingerprint` are public, reusable primitives a future
  benchmark/experiment tool can call, but this task does not itself modify
  `scripts/ci/semantic_ref_harness.py` or the frozen `AICAD-079A` corpus.
  That remains `AICAD-096`'s own scope (per `project/SESSION_HANDOFF.md`'s
  own queue-correction note: "AICAD-096 extends/consumes the frozen
  AICAD-079A corpus rather than recreating its baseline").
- **`with_fingerprint_ranking` is not wired into any production call
  site** — matching the "evaluator/evidence-hook plumbing proven, not yet
  production-wired" precedent every prior Stage-4 task in this crate has
  established; a caller (CLI diagnostics output, a future health report)
  must invoke it explicitly.
- **`fingerprint_distance` is an unweighted, unnormalized heuristic** — the
  plan itself calls fingerprinting "approximate ... as a fallback
  discriminator," not a rigorous metric; weighting/normalizing across
  position/normal/area/radius units is left to a future task if evidence
  from real usage shows it matters, rather than guessed now.

## Regressions

None. The one new regression-shaped test this task adds
(`rank_candidates_never_narrows_or_selects_automatically`) passes cleanly;
it exists to keep passing on every future change to this module or to
`crate::resolve`.

## Next dependency

`AICAD-093` (Batch S4-04, next: raw topology handle epochs/stale-handle
rejection) `depends_on: AICAD-092` (satisfied).
