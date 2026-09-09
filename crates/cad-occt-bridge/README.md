# cad-occt-bridge

WP-01 (Kernel bridge). Safe Rust wrapper around `native/occt_bridge`,
implementing `cad-kernel-api` against Open CASCADE Technology. Exposes only
coarse, domain-shaped operations (`create_box`, `extrude`, `boolean_union`,
`fillet`, `export_step`, ...) — never raw OCCT class names — per the
kernel-independence contract.

Kernel context lifecycle, exact geometry handles, curve/surface
constructors, B-rep operations, topology traversal, validation/healing,
meshing, and STEP primitives are owned here; source syntax, semantic refs,
user-facing package abstractions, and AI logic are explicitly not owned
here.

OCCT is treated as an external dependency under
`project/DECISION_LOG.md#DL-6` (development-phase policy; a formal
distribution/license review remains a separate future gate — see
`project/OWNER_DECISIONS.md` D13).

Plan references: `docs/plan/01_SYSTEM_ARCHITECTURE.md` §4, §8;
`docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-01.

## Implementation status (AICAD-018)

`build.rs` configures, builds, and installs `native/occt_bridge` (via
`cmake -S`/`--build`/`--install`, no `cmake` Cargo crate dependency) into
this crate's `OUT_DIR`, then links against the resulting
`aicad_occt_bridge` shared library with an rpath so `cargo test`/`cargo
run` find it without manual `LD_LIBRARY_PATH` setup.

- `src/ffi.rs`: raw, private `unsafe extern "C"` declarations mirroring
  `native/occt_bridge/include/aicad_occt_bridge.h` field-for-field. Status
  codes are received as plain `c_int`, not a `#[repr]` Rust enum (matching
  an unenumerated `#[repr]` enum value from C is undefined behavior;
  matching an unenumerated plain integer is not).
- `src/lib.rs`: the safe wrapper — `OcctContext` (RAII-owns a native
  context; not `Send`/`Sync`, matching the native bridge's
  single-thread-affine contract at the type level) and `Shape<'ctx>`
  (RAII-releases its native handle on drop; borrows its owning context, so
  the borrow checker rejects dropping the context before all its shapes
  are dropped — a compile-time guarantee on top of the native bridge's own
  runtime checks, see `project/reports/AICAD-016.md`).

See `project/reports/AICAD-018.md` for design rationale and test
evidence, including an empirical confirmation that dropping the context
before a shape it owns fails to compile.
