# AICAD-090: Implement broken-reference diagnostic and non-guessing recovery hints

## Status

Done. Third and final task of Batch S4-03.

## Objective

Turn `crate::resolve::ResolutionOutcome::Broken` (`AICAD-088`) into a
structured `REF-E1##` diagnostic carrying non-guessing recovery hints
per each `BrokenReason` variant, matching the campaign brief's own
instruction: "Recovery hints may explain how a human/program could
repair a reference. A hint is not authorization for the resolver to
guess automatically."

## Base / resulting commit

- Base: this invocation's own `AICAD-089` commit (same session).
- This task's commit: see `git log` (`AICAD-090` commit).

## Choosing `REF-E101`

Unlike `AICAD-089`'s `REF-E102`, no plan/RFC document already names a
specific code for a broken reference — only the `REF` family and the
general `FAMILY-Exxx` shape are established
(`docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §10,
`specs/language/diagnostics.md`). A grep across the whole repository
(`.rs`/`.md`/`.yaml`) before implementing confirmed no other `REF-E1##`
code is committed anywhere. Per `project/DECISION_LOG.md#DL-18`, "adding
a new code within an already-reserved family ... remains ordinary task
work and needs no owner ruling" — this task mints `REF-E101` as its own
new code (lowest free number in the family; no ordering claim relative
to `REF-E102` is implied by the numbers).

## What was implemented

`crates/cad-query/src/diagnostics.rs` (same module `AICAD-089` added;
`src/lib.rs` gains a re-export):

- **`broken_reference_diagnostic(entity, expected_count, reason)`** —
  builds a `REF-E101 BROKEN_REFERENCE` `Diagnostic` from a real
  `&BrokenReason`. `expected_count: Option<usize>` is the caller's own
  cardinality expectation when one exists (`BrokenReason` itself does not
  always carry one, matching `ambiguous_reference_diagnostic`'s own
  identical `expected_count` parameter design from `AICAD-089`).
- **`broken_reason_detail(reason)`** — the private `(message,
  observed_count, hints)` mapping, one arm per `BrokenReason` variant:
  - `NoMatch` -> `observed: {"count": 0}`, three hints (check upstream
    deletion/suppression, loosen an overly narrow predicate, verify the
    feature/export name).
  - `TooFew { expected, found }` -> `observed: {"count": found}` (the
    real found count, never assumed), three hints (check upstream
    deletion, adjust `expect_count(...)` if the expectation itself
    changed, inspect the feature's lineage report).
  - `InsufficientEvidence(detail)` -> the static `detail` string
    (`crate::eval::EvalError::NoEvidence`'s own payload, or one of
    `crate::resolve`'s own `ExplicitExport`/`StructuralRole`/
    `UserConfirmed` messages) is quoted directly in both the message and
    the single hint — this task never re-derives or guesses *why*
    evidence is missing beyond what `AICAD-088` already recorded.
  - `FingerprintAutoResolutionDisabled(evidence)` -> the fingerprint
    evidence itself is attached via `Diagnostic::with_backend_details`
    (RFC-0005's own "backend-specific detail belongs only in
    `backend_details`" field — approximate/non-authoritative data is
    exactly that kind of detail here), **never** used to select or name a
    candidate; two hints (add stronger evidence instead of relying on the
    fingerprint alone, or record an explicit user confirmation after a
    human reviews the fingerprint evidence).
  - `QueryHandleNotRegistered(handle)` -> the real handle name is quoted
    in the message; two hints (register the query, check for a
    typo/stale rename).
- Every hint is `Suggestion::new(SuggestionConfidence::Informational)` —
  never `MachineApplicable`/`LikelyFix` — and every hint's own wording
  describes an *investigative or repair action a human/tool might take*,
  never a specific replacement entity. There is no field anywhere in this
  function's own output that could carry a guessed candidate identity: no
  code path here ever looks at live geometry to propose "candidate X",
  only at the already-decided `BrokenReason` and (for the fingerprint
  case) evidence `AICAD-088` already refused to use for selection.

## Tests / verification

6 new tests in `crates/cad-query/src/diagnostics.rs`:

- `no_match_produces_a_ref_e101_diagnostic_with_recovery_hints` —
  end-to-end: a real cube's cylindrical-face query against an all-planar
  box resolves `Broken(NoMatch)` via `AICAD-088`'s own `resolve_query`,
  then this test proves the diagnostic's `code`/`severity`/`title`/
  `expected`/`observed` match schema, every suggestion is
  `informational`, and every suggestion carries both an `action` and a
  `description`.
- `too_few_reports_the_real_found_count_and_expected_count` — a
  hand-built `TooFew { expected: 3, found: 1 }` reports the real `1`/`3`
  counts in both the structured fields and the message text.
- `fingerprint_auto_resolution_disabled_carries_evidence_as_backend_
  details_only` — proves the fingerprint evidence lands in
  `backend_details` (not `candidates`, not `expected`/`observed`, both
  `Null`), and that no suggestion text contains a fabricated entity
  identifier.
- `query_handle_not_registered_names_the_handle_in_the_message` /
  `insufficient_evidence_includes_the_reason_in_its_single_hint` — the
  remaining two `BrokenReason` variants each surface their own real
  payload in the diagnostic text, not a generic placeholder.
- `broken_reference_diagnostic_builds_from_a_real_resolver_outcome` —
  end-to-end: an unregistered `SemanticQuery` handle on a real `AnyRef`
  genuinely resolves `Broken` via `AICAD-088`'s own `resolve_reference`,
  and the diagnostic is built from that real outcome, not a hand-built
  `BrokenReason` (the same real-evidence discipline `AICAD-088`'s/`089`'s
  own tests already establish).

Commands run:

- `cargo fmt --all -- --check` -> clean (whole workspace).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  -> zero warnings (whole workspace).
- `cargo test -p cad-query` -> 58/58 passed (52 pre-existing + 6 new).
- `cargo test --workspace` -> 0 failed, 1,170 total passing tests
  (1,164 baseline + 6 `AICAD-090`).
- `python3 scripts/ci/semantic_ref_harness.py validate`/`self-test`,
  `python3 scripts/ci/stage4_task_audit.py --check` -> all pass, unchanged
  (frozen `AICAD-079A` corpus untouched; no benchmark/CI infrastructure
  changed by this task).

## Limitations

- **Hints are per-`BrokenReason`-variant, not per-case analysis** — same
  honesty boundary `AICAD-089`'s own suggestions have: none is computed
  from the specific query/candidate context beyond what the reason
  itself already names (e.g. the real handle name, the real found/
  expected counts). A richer, context-specific hint generator is future
  work, not required by this task's own acceptance criteria.
- **No `source` span**, same boundary as `AICAD-089` — no `.aicad` source
  location is available at this layer; a caller with one may still call
  `Diagnostic::with_source` on the returned value.
- **`expected_count` is caller-supplied**, not derived from `BrokenReason`
  — `NoMatch`/`InsufficientEvidence`/`FingerprintAutoResolutionDisabled`/
  `QueryHandleNotRegistered` do not themselves carry the original
  cardinality expectation (only `TooFew` does, via its own `expected`
  field), matching `AICAD-089`'s identical design choice for
  `ambiguous_reference_diagnostic`'s own `expected_count` parameter.

## Regressions

None.

## Batch S4-03 complete

`AICAD-088`/`089`/`090` are all done. Per `project/CURRENT_STAGE.md`'s
fixed batch list, the next batch is S4-04 (`AICAD-091`/`092`/`093` —
reference durability levels, geometry-fingerprint fallback under
approved policy only, and raw topology handle epochs), which
`depends_on: AICAD-090` (satisfied). Per the campaign brief ("Each
invocation works on exactly ONE fixed batch"), this invocation stops
here at the end of S4-03 rather than continuing into S4-04.
