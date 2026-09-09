# cad-kernel-api

WP-01 (Kernel bridge). Backend-independent handle/error types
(`KernelCurve`, `KernelSurface`, `KernelShape`, `KernelVertex`, `KernelEdge`,
`KernelWire`, `KernelFace`, `KernelShell`, `KernelSolid`). Must not own
source syntax, semantic references, user-facing package abstractions, or
AI logic.

## Implementation status (AICAD-017)

Implemented: `KernelId` (context/slot/generation identity) plus the nine
handle types above, each a distinct newtype wrapping a `KernelId`, and
`KernelError`/`KernelResult<T>` mirroring
`native/occt_bridge/include/aicad_occt_bridge.h`'s status codes. No FFI or
OCCT dependency — `cad-occt-bridge` (AICAD-018) is the crate that
constructs these types from the native C ABI. See `src/lib.rs`'s doc
comments and `project/reports/AICAD-017.md` for the design rationale.

Plan references: `docs/plan/01_SYSTEM_ARCHITECTURE.md` §2.6;
`docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-01; RFC-0002 §3-4.
