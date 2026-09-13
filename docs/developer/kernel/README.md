# Kernel boundary

The kernel stack has three layers with deliberately different responsibilities:

1. **`cad-kernel-api`** — pure Rust, backend-neutral geometry values and operation contracts.
2. **`cad-occt-bridge`** — Rust adapter implementing those contracts against the native bridge and owning safe Rust-side kernel resources.
3. **`native/occt_bridge`** — C++/C ABI implementation permitted to include OCCT headers and store OCCT-specific objects.

Everything above this stack should reason in AICAD/geometry-domain terms, not OCCT classes.

See [occt-boundary.md](occt-boundary.md) for lifecycle/ABI rules and build integration.
