# AICAD-093: Implement raw topology handle epochs/stale-handle rejection

## Status

Done. Third and final task of Batch S4-04.

## Objective

Per `project/DECISION_LOG.md#DL-24` (D22) and `AGENTS.md`'s own
non-negotiable "Raw topology is ephemeral/unsafe and epoch-bound": a raw
handle must be "scoped to an appropriate build/context/epoch lifetime, and
explicitly invalid once its owning context/epoch is invalid," never
promoted into a persistent semantic reference. Before this task, that
property was documented in many places (`cad-kernel-api`,
`cad-occt-bridge`, `cad-geometry-api::ir`, `cad-references`'s own crate doc
comment) but enforced only by Rust's borrow checker: a
`cad_occt_bridge::Shape<'ctx>` cannot outlive its owning `OcctContext`. That
covers a *compile-time-dead* handle, but not a handle that is still a
perfectly valid, alive Rust value while its owning build/regeneration
*generation* has moved on (e.g. captured before an incremental rebuild
regenerated the topology it pointed into) — the borrow checker cannot see
that. This task adds the missing runtime mechanism: an explicit epoch
counter and a raw-handle wrapper that rejects stale access outright.

## Base / resulting commit

- Base: this invocation's own `AICAD-092` commit (same session).
- This task's commit: see `git log` (`AICAD-093` commit).

## Design decisions

- **Generic `RawHandle<T>`, not tied to a concrete kernel type.** D22
  itself explicitly defers "exact raw-handle representation" and "exact
  epoch encoding" to later, closer-to-Stage-5 work; the campaign brief's
  own carve-out names only "the Stage-4 raw-handle epoch work explicitly
  required by AICAD-093," not a frozen raw-topology API. A generic wrapper
  proves the epoch mechanism itself without prematurely committing to that
  surface, and keeps `cad-references` kernel-neutral (it depends on
  nothing but `cad-diagnostics`).
- **`Epoch` is tagged with its own `EpochCounter`'s identity, not just a
  bare generation number.** An early draft used a bare `u64` generation;
  self-review caught that two independent counters (e.g. two independent
  kernel contexts) both starting at generation `0` would then spuriously
  validate each other's handles — a real correctness gap for a mechanism
  whose entire job is precise staleness checking. Fixed by giving every
  `EpochCounter` a process-wide, never-reused identity
  (`NEXT_COUNTER_ID: AtomicU64`) and folding it into `Epoch`'s own
  equality; `raw_handle::tests::independent_counters_never_accept_each_
  others_handles` is the regression proving this.
- **Rejection is an explicit `Result`, never a panic or a silent stale
  read.** `RawHandle::get(&self, &EpochCounter) -> Result<&T, StaleHandle>`
  — matching every other Stage-4 fail-closed pattern in this crate
  (`ResolutionOutcome`, `EvalError`): a caller must handle the stale case,
  it can never be accidentally ignored.
- **No conversion to/from `ReferenceRecipe`/`AnyRef`.** Per the campaign
  brief: "Do not make a raw handle durable by quietly attaching enough
  metadata to turn it into a second semantic-reference system." A
  `RawHandle` carries only its payload and its `Epoch` — no feature name,
  construction strategy, or durability classification. This is documented
  explicitly in the module's own doc comment rather than left implicit.

## What was implemented

New module `crates/cad-references/src/raw_handle.rs` (kernel-neutral):

- **`Epoch`** — an opaque `(counter_id, generation)` pair; equality only
  meaningful relative to the `EpochCounter` that produced it.
- **`EpochCounter`** — owns one build/regeneration context's current
  generation. `new()` (starts at generation 0 with a fresh, unique
  identity), `current()`, `advance()` (invalidates every handle minted so
  far), `mint(payload)`.
- **`RawHandle<T>`** — `payload` + minting `Epoch`. `get(&EpochCounter) ->
  Result<&T, StaleHandle>`, `is_current(&EpochCounter) -> bool`,
  `minted_epoch()`.
- **`StaleHandle`** — `{ minted_epoch, current_epoch }`, `Display`/`Error`.

`crates/cad-references/src/lib.rs` — `pub mod raw_handle;` + re-exports;
crate doc comment and `DurabilityLevel::Raw`'s own doc comment
(`durability.rs`) updated to point at this module as the concrete
implementation of the previously-documentation-only "raw handle, no
recipe" case.

New module `crates/cad-query/src/raw_handle.rs` (test-only, not `pub`):
supplies the "topology" half of "raw topology handle epochs" —
`cad-references` being kernel-neutral means its own tests can only use
generic payloads (`i32`, `&str`), so this module wraps a real, live
`crate::eval::Candidate` (backed by real `cad-occt-bridge` geometry) in a
`RawHandle` and proves stale rejection against it.

## Tests / verification

8 new tests:

`crates/cad-references/src/raw_handle.rs` (6):
- `a_handle_is_readable_before_the_counter_advances`
- `advancing_the_counter_makes_every_prior_handle_stale`
- `a_handle_minted_after_advance_is_readable_again`
- `repeated_advances_keep_every_earlier_handle_permanently_stale`
- `independent_counters_never_accept_each_others_handles` — the
  counter-identity regression described above.
- `stale_handle_display_names_both_epochs`

`crates/cad-query/src/raw_handle.rs` (2, real geometry):
- `a_raw_handle_to_a_real_face_is_rejected_once_its_epoch_counter_
  advances` — mints a `RawHandle` around a real cube face `Candidate`,
  reads real area (4.0) through it while current, advances the epoch
  counter (simulating a regeneration with nothing about the `Candidate`
  Rust value itself changing or being dropped — it stays alive and
  borrow-check-valid for the whole test), then proves the same handle is
  rejected.
- `a_raw_handle_minted_after_regeneration_reaches_the_new_topology` — a
  stale pre-regeneration handle stays rejected while a handle minted after
  a real second shape is built and the epoch advances reaches the real new
  geometry (area 9.0, not the old 4.0).

Commands run:

- `cargo fmt --all -- --check` -> clean (whole workspace, after `cargo fmt
  --all`).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  -> zero warnings (whole workspace).
- `cargo test -p cad-query -p cad-references` -> 76/76 + 32/32 passed.
- `cargo test --workspace` -> 0 failed, 1,194 total passing tests (1,186
  baseline + 8 `AICAD-093`).
- `python3 scripts/ci/semantic_ref_harness.py validate`/`self-test` -> both
  `"status": "ok"`, unchanged (this task does not touch the frozen
  `AICAD-079A` corpus or resolver behavior at all).
- `python3 scripts/ci/stage4_task_audit.py --check` -> `Stage-4 task
  metadata audit OK`.

## Limitations

- **Not wired to a real `FeatureGraph`/`ParametricBuildSession` build** —
  matching every earlier Stage-4 task in this crate's own established
  precedent. `EpochCounter`/`RawHandle` are a standalone, real, tested
  mechanism; connecting one `EpochCounter` per build session and calling
  `advance()` exactly on regeneration/dirty-subgraph rebuild is
  `AICAD-094`'s own integration job (per that task's own
  `depends_on: AICAD-093`), not built here.
- **No concrete raw-topology construction/inspection API** — this task
  implements the epoch primitive only, not a `Vertex*`/`Edge*`/`Face*`-style
  raw handle surface, a source-language `unsafe`/`raw` block, or any other
  Stage-5 raw-geometry capability. D22 itself defers exact raw-handle
  representation and source syntax; the campaign brief names only "the
  Stage-4 raw-handle epoch work explicitly required by AICAD-093" as
  authorized here.
- **`Epoch`/`StaleHandle` carry no human-readable session identity** (only
  an opaque counter id + generation) — sufficient for the mechanism's own
  correctness and for a machine to detect staleness, but a future
  production integration may want to attach a session/context label for
  diagnostics; left to that integration rather than guessed now.

## Regressions

None. `independent_counters_never_accept_each_others_handles` is a new
permanent regression guarding the counter-identity fix described above.

## Batch S4-04 complete

`AICAD-091`/`092`/`093` are all done. Per `project/CURRENT_STAGE.md`'s
fixed batch list, the next batch is S4-05 (`AICAD-094`/`095` — replaying
semantic references during incremental parameter regeneration, and the
`cad refs check` reference-health report), which `depends_on: AICAD-093`
(satisfied). Per the campaign brief ("Each invocation works on exactly ONE
fixed batch"), this invocation stops here at the end of S4-04 rather than
continuing into S4-05.
