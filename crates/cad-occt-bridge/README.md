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

OCCT's own license and redistribution terms are an open owner decision —
see `project/OWNER_DECISIONS.md` D13.

Plan references: `docs/plan/01_SYSTEM_ARCHITECTURE.md` §4, §8;
`docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-01.
