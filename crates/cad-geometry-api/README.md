# cad-geometry-api

WP-05 (Geometry language API). Safe language-facing geometry types,
high-level and low-level API signatures, Geometry IR, lowering to the
kernel bridge, normalized properties/errors.

Rule: no public user type exposes OCCT-specific C++ classes.

## Status (`AICAD-059`)

The `ir` module implements the backend-independent Geometry IR:
[`GeometryGraph`], an append-only, functional/SSA node graph
([`GeometryOp`]/[`GeometryQuery`] nodes addressed by [`GeomId`]) sitting
between the (not-yet-built) Feature IR/dependency DAG and the Stage-1
kernel-neutral API (`cad-kernel-api`), per `docs/plan/
01_SYSTEM_ARCHITECTURE.md` §5's IR-layering diagram. See the module's own
doc comment for the full design (backend-independence boundary, typed
`Quantity` parameters, raw index-based edge/face selection as a documented
known limitation, and the deliberate scope cut against low-level topology
exploration).

Dispatching a `GeometryGraph` into actual kernel calls
(`cad-kernel-api`/`cad-occt-bridge`) is `AICAD-060`'s job, not this task's —
this crate builds and validates the IR only; it never constructs a kernel
context or calls into `cad-occt-bridge`.

[`GeometryGraph`]: src/ir.rs
[`GeometryOp`]: src/ir.rs
[`GeometryQuery`]: src/ir.rs
[`GeomId`]: src/ir.rs

Plan references: `docs/plan/02_LANGUAGE_AND_COMPILER.md` §5 (IR layering);
`docs/plan/04_HIGH_LEVEL_MODELING_API.md`;
`docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`;
`docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-05.
