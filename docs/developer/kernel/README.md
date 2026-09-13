# Kernel boundary

`cad-kernel-api` is the kernel-neutral contract. `cad-occt-bridge` is the OpenCascade implementation boundary; direct OCCT/native calls must not leak upward into compiler/runtime/source semantics.

Spatial types validate concepts such as directions, axes, frames, planes, and rigid transforms at the kernel-neutral boundary. Native handles are implementation details. Determinism/validation policy is layered: source/compiler determinism, locked-kernel semantic/numerical behavior, and cross-environment comparison are distinct concerns.

When adding geometry capability, first decide whether the change belongs in source API, internal Geometry IR, kernel-neutral API, or OCCT implementation; do not automatically mirror one layer into another.
