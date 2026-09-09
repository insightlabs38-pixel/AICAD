# AICAD-017 — Create cad-kernel-api backend-independent handle/error types

## Objective
Populate `crates/cad-kernel-api` (previously an AICAD-002/003 placeholder
stub) with the backend-independent handle and error types its own
`README.md` calls for (`KernelCurve`, `KernelSurface`, `KernelShape`,
`KernelVertex`, `KernelEdge`, `KernelWire`, `KernelFace`, `KernelShell`,
`KernelSolid`), per `project/TASKS.yaml` (AICAD-017) and
`project/DECISION_LOG.md#DL-5`/`#DL-6` (kernel-neutral surface, no
backend type crosses it).

## Dependencies checked
AICAD-016 (native C ABI boundary) — complete, see
`project/reports/AICAD-016.md`. This task's `KernelHandle` field shape
(`context_id: u64, index: u32`) intentionally mirrors AICAD-016's
`AicadShapeHandle` one-to-one (see "Implementation decisions"), and
`KernelError`'s variants mirror `AicadStatusCode` one-to-one, but this
crate has **zero** dependency on `crates/cad-occt-bridge` or any native
code — the mirroring is a design choice for AICAD-018 to exploit when
translating between the two, not a compile-time coupling.

## Scope decision
This task's title is narrower than "the kernel abstraction": it is
specifically the handle/error *types*. It does not define the geometry
*operations* trait (`create_box`, `boolean_union`, ...) that a backend
will eventually implement against these types — with only one real
operation in existence so far (AICAD-016's `create_box`/`shape_volume`),
generalizing an operations trait now would be exactly the premature
abstraction `AGENTS.md` warns against ("Three similar lines is better
than a premature abstraction"). That trait is deferred to whichever task
first needs to generalize over more than one operation (Batch 1B,
AICAD-020+, or AICAD-018 itself if it turns out to need one sooner).

## What was done

`crates/cad-kernel-api/src/lib.rs` (replacing the AICAD-002/003
placeholder body) now defines:

1. **`kind` module**: nine zero-sized marker types (`Shape`, `Solid`,
   `Shell`, `Face`, `Wire`, `Edge`, `Vertex`, `Curve`, `Surface`) — never
   instantiated, used only to parameterize handles.
2. **`KernelHandle<Kind>`**: a generic `{context_id: u64, index: u32}`
   value type. `Clone`/`Copy`/`PartialEq`/`Eq`/`Hash`/`Debug` are
   implemented **manually**, not via `#[derive(..)]`, specifically so
   that `Kind` itself is never required to implement any of those traits
   — `#[derive]` naively adds a `Kind: Trait` bound per field type
   parameter even though `PhantomData<Kind>` needs none, which would
   have forced every marker type in `kind` to derive traits it has no
   use for.
3. **Nine public type aliases** (`KernelShape = KernelHandle<kind::Shape>`,
   ..., `KernelSurface = KernelHandle<kind::Surface>`) — exactly the nine
   names `crates/cad-kernel-api/README.md` already committed to.
4. **`KernelError`**: a five-variant enum (`InvalidArgument`,
   `InvalidHandle`, `ForeignContextHandle`, `Failure`, `Internal`),
   each carrying a `String` message, implementing `Display` and
   `std::error::Error`. Variant set mirrors AICAD-016's
   `AicadStatusCode` category-for-category (its `AICAD_STATUS_OK` has no
   counterpart here, since `KernelError` only exists to represent the
   failure side of a `Result`).
5. **`KernelResult<T> = Result<T, KernelError>`** convenience alias.
6. Six unit tests: handle equality/inequality on field values, that two
   handles of genuinely different `Kind` type parameters (e.g.
   `KernelFace`/`KernelSolid`) still expose the same readable fields
   (i.e. the distinction is compile-time-only, not a hidden runtime
   discriminant), `Copy`/`Clone` behavior, and `KernelError`'s `Display`
   output/`std::error::Error` conformance.

`Cargo.toml` was not modified — it already declared an empty
`[dependencies]` table (from AICAD-002/003 scaffolding), which this task
preserves: **no dependency was added**, including no dependency on
`crates/cad-occt-bridge`, confirming kernel-neutrality is enforced by the
build graph, not merely by convention.

## Implementation decisions
- Mirrored `AicadShapeHandle`'s exact field shape (`context_id: u64,
  index: u32`) in `KernelHandle` rather than inventing a different
  representation, since the epoch/context-scoping *concept* (RFC-0002 §5)
  is backend-independent even though the *field values* a given backend
  assigns are backend-specific — a future non-OCCT backend can still
  populate these same two fields however it needs to, as long as it
  upholds the same "valid only against the issuing context, only while
  the slot is live" contract.
- Used a generic `KernelHandle<Kind>` with phantom marker types rather
  than nine independent structs with duplicated fields/impls, since the
  nine kinds genuinely share identical runtime representation and
  validity rules — this is the "reuse over duplication" case, not the
  "premature abstraction" case, because the nine names already exist as
  a committed public surface (the crate's own README), not a speculative
  future need.
- Did not add a `KernelContext` type/trait in this crate. `AicadOcctContext`
  (AICAD-016) is backend-owned (its OCCT-specific lifetime/allocation
  behavior has no kernel-neutral generalization yet with only one
  backend in existence); this crate only needs `context_id: u64` as a
  plain comparable value inside a handle, which it already has.

## Files changed
- Modified: `crates/cad-kernel-api/src/lib.rs` (placeholder body ->
  handle/error types + tests)

## Verification (exact commands/results)

```
$ cargo test -p cad-kernel-api
running 6 tests
test tests::handle_kind_is_a_compile_time_distinction_not_a_runtime_field ... ok
test tests::handles_with_different_fields_are_not_equal ... ok
test tests::kernel_error_implements_std_error ... ok
test tests::handles_with_equal_fields_are_equal_regardless_of_kind_type_identity ... ok
test tests::kernel_error_display_includes_message_and_category ... ok
test tests::kernel_handle_is_copy_and_clone ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Clippy caught a real issue during this task, not a false pass: the
first draft of `kernel_handle_is_copy_and_clone` called `.clone()`
directly on a `Copy` value (`clippy::clone_on_copy`), which `cargo
clippy --workspace --all-targets --all-features -- -D warnings` failed
on (`error: using \`clone\` on type \`KernelHandle<Shape>\` which
implements the \`Copy\` trait`). Fixed by testing `Clone`/`Copy` through
generic helper functions (`fn assert_copy<T: Copy>`, `fn clone_it<T:
Clone>`) instead of calling `.clone()` on a value clippy can see is
`Copy` at the call site — this actually verifies both trait impls exist
generically, which is a more faithful test of "this type is Clone and
Copy" than the original code was. Re-ran after the fix:

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
(exit 0, zero warnings)

$ cargo test --workspace
(all crates' test suites pass, including cad-kernel-api's 6 new tests)
```

## Artifacts
None beyond the modified source file (no build/native artifacts for a
pure-Rust crate).

## Regressions added
None — all six tests are new coverage for a previously-empty crate.

## Limitations
- No geometry-operations trait exists yet in this crate (see "Scope
  decision"); `crates/cad-occt-bridge` (AICAD-018) cannot yet be written
  as "implements trait X from cad-kernel-api" — it will need its own
  concrete operation functions for now, using these handle/error types
  as their signatures' vocabulary.
- `KernelHandle::new` is a public constructor; nothing in the type system
  prevents a caller from constructing a handle with fabricated
  `context_id`/`index` values. The doc comment states the intended
  discipline (only backend crates construct these, from a real
  backend-issued value) but does not enforce it — enforcing it would
  require either sealing the crate boundary (impossible while
  `cad-occt-bridge` is a separate crate that must construct these) or a
  capability/token design not justified by anything in scope yet.

## Unresolved questions
None raised by this task. No `AICAD-017` escalation condition was
triggered: no public AICAD language syntax/semantics changed (this is an
internal Rust crate, not exposed to AICAD source), no OCCT/backend type
was introduced (verified by the crate's empty dependency list), no stage
gate/benchmark/test needed weakening, no reference-resolution ambiguity
exists at this layer, no unresolved architecture alternative needed
selecting, and this task's scope was not expanded into AICAD-018/019's
work.
