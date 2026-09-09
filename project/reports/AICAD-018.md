# AICAD-018 — Create cad-occt-bridge safe Rust wrapper

## Objective
Wire `crates/cad-occt-bridge` (an AICAD-002/003 placeholder) up to
`native/occt_bridge`'s C ABI (AICAD-016) through Cargo, and provide a
safe Rust API over it in terms of `cad-kernel-api` (AICAD-017) types
only — no raw pointer, OCCT type, or native status code visible outside
this crate. Per `project/TASKS.yaml` (AICAD-018).

## Dependencies checked
- AICAD-016 (native C ABI) — complete, see `project/reports/AICAD-016.md`.
- AICAD-017 (`cad-kernel-api` handle/error types) — complete, see
  `project/reports/AICAD-017.md`; this task's error/handle conversions
  are exactly the "AICAD-018 is responsible for translating AicadStatus
  into KernelError" work AICAD-017's report already flagged as its own
  limitation.

## What was done

1. **`crates/cad-occt-bridge/build.rs`** (new): compiles
   `native/occt_bridge/src/occt_bridge.cpp` directly via the `cc` crate
   (a new `[build-dependencies]` entry) rather than invoking
   `native/occt_bridge/CMakeLists.txt` — that CMake project remains a
   native-only dev/test harness (AICAD-015/016's `occt_probe`/
   `abi_smoke_test`); this is the simpler, standard Cargo-side
   integration for a small native dependency, and avoids requiring a
   separate CMake configure step for `cargo build` itself. Locates OCCT
   headers by checking `AICAD_OCCT_INCLUDE_DIR` then a short list of
   known install paths (starting with `/usr/include/opencascade`, the
   path AICAD-015 already confirmed), failing with a clear message
   rather than a confusing compiler error if none match. Links the same
   OCCT toolkit list AICAD-016's `CMakeLists.txt` declares for the
   bridge library.
2. **`crates/cad-occt-bridge/Cargo.toml`**: added `cad-kernel-api` (path
   dependency) and `cc` (build-dependency). No other dependency was
   added.
3. **`crates/cad-occt-bridge/src/lib.rs`** (replacing the placeholder
   body):
   - a private `ffi` module with `#[repr(C)]`/`unsafe extern "C"`
     declarations matching `aicad/occt_bridge.h` field-for-field
     (`AicadOcctContext` opaque, `AicadStatusCode`/`AicadStatus`,
     `AicadShapeHandle`);
   - `status_to_error`/`check_status`: converts a non-`Ok` `AicadStatus`
     into the matching `cad_kernel_api::KernelError` variant, reading the
     fixed message buffer via `CStr::from_ptr` (safe because AICAD-016's
     native code null-terminates it on every return path — verified by
     re-reading `occt_bridge.cpp`, not assumed);
   - `OcctContext`: wraps `NonNull<ffi::AicadOcctContext>`, with
     `new()`, `create_box(dx, dy, dz) -> KernelResult<KernelSolid>`,
     `shape_volume(handle: KernelSolid) -> KernelResult<f64>`, and a
     `Drop` impl calling `aicad_occt_context_destroy`.
4. **`native/occt_bridge/src/occt_bridge.cpp`**: added one
   `static_assert(sizeof(AicadStatusCode) == sizeof(int32_t), ...)` —
   see "Implementation decisions" below. This is the only change to
   AICAD-016's file; its public header and behavior are unchanged.
5. Nine unit tests in `cad-occt-bridge`, run through the real compiled
   FFI (not mocked): round-trip create+volume, zero/negative/NaN
   dimension rejection, very-small/very-large box dimensional
   correctness, foreign-context handle rejection across two
   `OcctContext`s, out-of-range handle rejection, two contexts not
   interfering with each other's indices, and a plain drop not
   crashing.

## Implementation decisions
- **`#[repr(i32)]` for the mirrored `AicadStatusCode` enum, guarded by a
  new native `static_assert`.** Representing a C `enum` from Rust always
  assumes a specific underlying integer width, which the C/C++ standards
  leave implementation-defined. Rather than silently trust that GCC/Clang
  on this project's supported targets pick a 4-byte representation
  (true in practice for small non-negative enumerators, and confirmed by
  this task's own passing tests), added a `static_assert` in
  `occt_bridge.cpp` itself that fails the *native* build if that
  assumption ever stops holding — turning a possible silent ABI mismatch
  into a loud native-build failure instead of undefined behavior at the
  Rust/C++ boundary. This is a minor, backward-compatible addition to
  AICAD-016's file (its header/behavior are otherwise untouched).
- **No `unsafe impl Send`/`Sync` for `OcctContext`.** `NonNull<T>` (like
  any raw pointer) is `!Send`/`!Sync` by default in Rust, so
  `OcctContext` inherits that without any code — which is exactly
  `AGENTS.md` Stage-1 kernel policy #9 ("Treat KernelContext
  conservatively as single-thread-affine initially unless an approved
  architecture decision explicitly strengthens this"). Recorded here so
  a future task does not "fix" this by adding `Send`/`Sync` impls
  without that being a deliberate, approved decision.
- **`create_box`/`shape_volume` are inherent methods on `OcctContext`,
  not a trait implementation.** `cad-kernel-api` does not yet define a
  backend-operations trait (AICAD-017's own recorded scope decision);
  inventing one here, backed by only two operations, would be the same
  premature abstraction AICAD-017 declined to add. `OcctContext`'s
  method signatures already use only `cad-kernel-api` types
  (`KernelSolid`, `KernelResult`), so a future trait extraction is a
  pure refactor, not a breaking change to callers.
- **`AicadStatus::placeholder()` values are written but guaranteed to be
  overwritten** by every native call before being read; documented
  inline rather than using `MaybeUninit` (which would need `unsafe` to
  read back regardless, for no real benefit here since the struct is POD
  and cheap to zero-initialize).

## Files changed
- Added: `crates/cad-occt-bridge/build.rs`
- Modified: `crates/cad-occt-bridge/Cargo.toml` (added
  `cad-kernel-api` dependency, `cc` build-dependency)
- Modified: `crates/cad-occt-bridge/src/lib.rs` (placeholder body ->
  safe wrapper + tests)
- Modified: `native/occt_bridge/src/occt_bridge.cpp` (added one
  `static_assert`; no behavior change)
- Modified: `Cargo.lock` (records the new `cc`, `find-msvc-tools`,
  `shlex` dependencies pulled in by `cc`)

## Verification (exact commands/results)

```
$ cargo build -p cad-occt-bridge
    Compiling cad-occt-bridge v0.0.0 (.../crates/cad-occt-bridge)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.07s
(after fixing one clippy-adjacent dead_code warning during development —
 see below; final build is warning-free)

$ cargo test -p cad-occt-bridge
running 9 tests
test tests::create_box_and_volume_round_trip ... ok
test tests::dropping_a_context_does_not_panic_or_crash ... ok
test tests::handle_from_one_context_is_rejected_as_foreign_by_another ... ok
test tests::nan_dimension_is_rejected_as_invalid_argument ... ok
test tests::negative_dimension_is_rejected_as_invalid_argument ... ok
test tests::out_of_range_handle_is_rejected_as_invalid ... ok
test tests::two_contexts_do_not_interfere_with_each_others_shape_indices ... ok
test tests::very_small_and_very_large_boxes_have_dimensionally_correct_volume ... ok
test tests::zero_dimension_is_rejected_as_invalid_argument ... ok
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo fmt --all -- --check       # exit 0
$ cargo clippy --workspace --all-targets --all-features -- -D warnings   # exit 0, zero warnings
$ cargo test --workspace            # all crates pass, including this one's 9 new tests
```

A real lint was caught and fixed during development, not a false pass:
the first draft's FFI `AicadStatusCode` enum triggered
`#[warn(dead_code)]` ("variants ... are never constructed") because
Rust's dead-code analysis cannot see that every non-`Ok` variant is only
ever produced by reading a raw value across the FFI boundary. Added
`#[allow(dead_code)]` with an explanatory comment rather than an
artificial construction site, since the variants genuinely are complete
and correct despite never being literally written as
`AicadStatusCode::InvalidArgument` in Rust source.

FFI ownership/lifetime auditing (valgrind, run against the compiled test
binary):

```
$ valgrind --leak-check=full --show-leak-kinds=possible \
    target/debug/deps/cad_occt_bridge-<hash> --test-threads=1
LEAK SUMMARY:
   definitely lost: 0 bytes in 0 blocks
   indirectly lost: 0 bytes in 0 blocks
     possibly lost: 48 bytes in 1 blocks
   still reachable: 560 bytes in 2 blocks
```

Investigated the "possibly lost" block rather than dismissing it: its
full allocation backtrace (`--show-leak-kinds=possible`, full stack)
traces entirely through Rust's own `libtest` harness internals
(`std::sync::mpmc::context::Context`'s thread-local initialization used
by the test runner's result-collection channel — `<std::thread::Thread>::new`
called from `test::console::run_tests_console`), never through any
AICAD or OCCT code path. Confirmed this is not introduced by this task's
code by running the identical check against a trivial `fn main() {
println!("hello") }` Rust binary with no test harness involved: 0
possibly-lost bytes there. The "possibly lost" block is specific to
`libtest`'s own thread bookkeeping, not to `OcctContext`/`create_box`/
`shape_volume`; **0 definitely/indirectly lost bytes attributable to
this crate's own FFI code**, which is the property this audit actually
needed to establish.

## Artifacts
- `target/debug/deps/cad_occt_bridge-*` and the `libaicad_occt_bridge.a`
  (etc.) native objects `cc` produces under `target/debug/build/
  cad-occt-bridge-*/out/` — both are ordinary Cargo build outputs,
  already gitignored, reproducible from `cargo build -p cad-occt-bridge`.

## Regressions added
None — all nine tests are new coverage for a previously-empty crate; the
workspace's full test suite (`cargo test --workspace`) was re-run after
this task's changes and remains green end-to-end.

## Limitations
- `OcctContext` cannot detect a handle used after its issuing context
  has been dropped (documented on the type itself and already flagged
  as a known limitation in AICAD-016's report at the native level — this
  task does not add a checked-lifetime mechanism on top, since nothing
  in this task's scope requires one yet and doing so would be exactly
  the kind of unrequested abstraction `AGENTS.md` asks this agent to
  avoid).
- `create_box`/`shape_volume` remain the only two operations; no
  operations trait exists (see "Implementation decisions").
- The native compile step assumes GCC/Clang give `enum AicadStatusCode`
  an `int`-sized representation; guarded by a `static_assert` (see
  above) rather than proven for every conceivable compiler, since this
  project's supported toolchain is GCC/Clang on Linux (recorded
  environment versions, per AICAD-015/016's reports).

## Unresolved questions
None raised by this task. No `AICAD-018` escalation condition was
triggered: no public AICAD language syntax/semantics changed, no OCCT
type crosses out of this crate (all downstream signatures use
`cad-kernel-api` types only), no stage gate/benchmark/test needed
weakening, no reference-resolution ambiguity exists at this layer, no
unresolved architecture alternative needed selecting (the operations-
trait question is deferred, not resolved silently — see "Implementation
decisions"), and this task's scope was not expanded into AICAD-019's
shape-handle-table work.
