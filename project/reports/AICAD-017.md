# AICAD-017 — Create cad-kernel-api backend-independent handle/error types

## Objective

Populate `crates/cad-kernel-api` with the backend-independent kernel
handle and error types its own `README.md` commits it to
(`KernelCurve`, `KernelSurface`, `KernelShape`, `KernelVertex`,
`KernelEdge`, `KernelWire`, `KernelFace`, `KernelShell`, `KernelSolid`,
plus a structured error type) — kernel-neutral, with no dependency on
`cad-occt-bridge`, the native bridge, or OCCT. Per
`project/TASKS.yaml` (AICAD-017, Stage 1 / Batch 1A).

## Dependencies checked

AICAD-016 (native C ABI boundary) — complete,
`project/reports/AICAD-016.md`. This task's handle shape (three plain
integers: context id, slot, generation) mirrors
`native/occt_bridge/include/aicad_occt_bridge.h`'s `AicadShapeHandle`
intentionally, but `cad-kernel-api` itself has zero code dependency on
the native crate/header — the mirroring is a design choice recorded
here, not a build dependency.

## What was done

1. Added `crates/cad-kernel-api/src/handle.rs`:
   - `RawHandle { context_id: u64, slot: u32, generation: u32 }` — the
     shared identity shape every kernel handle newtype wraps.
   - A `kernel_handle!` macro declaring nine newtypes
     (`KernelShape`, `KernelVertex`, `KernelEdge`, `KernelWire`,
     `KernelFace`, `KernelShell`, `KernelSolid`, `KernelCurve`,
     `KernelSurface`) — exactly the list in
     `docs/plan/01_SYSTEM_ARCHITECTURE.md` §2.6 and this crate's own
     `README.md` — each with `from_raw`/`raw` and deriving
     `Debug, Clone, Copy, PartialEq, Eq, Hash`. Using distinct newtypes
     (rather than one generic `KernelHandle<Kind>` or a bare
     `RawHandle`) means the type system itself prevents passing a
     `KernelFace` where a `KernelSolid` is expected, once operations
     that produce/consume the other eight kinds exist (Batch 1B/1C).
2. Added `crates/cad-kernel-api/src/error.rs`: `KernelError` (six
   variants: `InvalidArgument`, `InvalidHandle`, `StaleHandle`,
   `ForeignContext`, `NativeFailure`, `Unknown`, each carrying a
   diagnostic string where relevant) implementing `Display` and
   `std::error::Error`, plus `pub type KernelResult<T> = Result<T,
   KernelError>`. The variant set matches `AicadStatus` in the native
   header one-for-one (minus `Ok`, which is `Result::Ok` here), so
   `cad-occt-bridge` (AICAD-018) has a lossless place to map every
   native status.
3. `crates/cad-kernel-api/src/lib.rs` re-exports both modules' public
   items; `handle`/`error` themselves are private modules.

## Implementation decisions

- Kept `RawHandle`'s fields `pub` (not accessed only through methods)
  since this crate's own consumer (`cad-occt-bridge`) needs to construct
  a `RawHandle` from a native `AicadShapeHandle` and read it back to
  reconstruct one — an accessor-only design would just add
  indirection with no safety benefit, since both crates are trusted,
  in-workspace code at the same trust boundary (the untrusted boundary
  is the native C ABI, already handled in AICAD-016/018).
- `KernelError::NativeFailure`/`Unknown` carry a `String` rather than a
  more structured payload — sufficient for Stage 1 (these are primarily
  diagnostic today; nothing yet needs to pattern-match on their
  contents), and adding a richer error-metadata scheme now would be
  speculative ahead of a task that actually needs it.
- Did not add a `KernelContext`/lifecycle type here — this crate's own
  `README.md` and `cad-occt-bridge`'s `README.md` both already state
  context lifecycle is owned by `cad-occt-bridge`, not
  `cad-kernel-api`; adding it here would contradict that pre-existing
  scope split.

## Verification (exact commands/results)

```
$ cargo test -p cad-kernel-api
running 4 tests
test error::tests::display_is_non_empty_for_every_variant ... ok
test handle::tests::handles_with_distinct_generations_are_never_equal ... ok
test handle::tests::round_trips_through_raw ... ok
test handle::tests::handles_from_distinct_contexts_are_never_equal ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo fmt --all -- --check          # exit 0
$ cargo clippy --workspace --all-targets --all-features -- -D warnings   # exit 0, zero warnings
$ cargo build --workspace --all-targets   # exit 0
$ cargo test --workspace                  # exit 0, all suites ok
```

## Files changed

- Added: `crates/cad-kernel-api/src/handle.rs`
- Added: `crates/cad-kernel-api/src/error.rs`
- Modified: `crates/cad-kernel-api/src/lib.rs`

## Regressions added

4 unit tests in `crates/cad-kernel-api` (handle equality across
contexts/generations, raw round-trip, error `Display` non-emptiness),
run under `cargo test --workspace` going forward.

## Limitations

- Only the nine handle newtypes and the error type exist; no trait
  abstracting "a kernel backend" (e.g. `trait KernelBackend`) was added,
  since Stage 1 has exactly one backend (OCCT) and no second
  implementation to justify a trait boundary yet — `cad-occt-bridge`
  implements against these concrete types directly (AICAD-018).

## Unresolved questions

None. No escalation condition in `project/TASKS.yaml` (AICAD-017) was
triggered.
