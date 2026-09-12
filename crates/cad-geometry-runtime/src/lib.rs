//! `cad-geometry-runtime` (`AICAD-060`) — the Geometry-IR-to-kernel half of
//! the dispatch boundary `cad-geometry-api::ir`'s own module doc comment
//! named as "`AICAD-060`'s job": walking an already-validated
//! [`cad_geometry_api::GeometryGraph`] and actually calling
//! `cad-occt-bridge`'s kernel-neutral operations to realize it, per RFC-0002
//! §2's architecture-layer diagram (`Geometry IR -> Kernel call graph ->
//! B-rep`).
//!
//! # Scope note (this task's own escalation)
//!
//! `AICAD-060`'s title is "Implement HIR/runtime geometry dispatch into
//! Geometry IR/kernel API" — two halves:
//!
//! 1. **Geometry IR -> kernel API** ([`dispatch`]): given an already-built
//!    `GeometryGraph`, execute it against a real `cad_occt_bridge::OcctContext`
//!    and report results/errors. This half needs no new language syntax or
//!    binding mechanism at all — it is implemented completely in this crate.
//! 2. **HIR/runtime -> Geometry IR** ([`bridge`]): converting an evaluated
//!    `cad_runtime::value::NumberValue` into a `cad_geometry_api::Quantity`
//!    (a direct, representation-preserving field copy — implemented here)
//!    is *not* the open question. The open question is what actually
//!    *triggers* building `GeometryGraph` nodes while an ordinary `.aicad`
//!    program runs: `cad_runtime::interp::Interpreter::call` today
//!    dispatches exactly two callee kinds (`BindingKind::Fn`, requiring a
//!    real AICAD-source `HirBlock` body, and `BindingKind::EnumVariant`),
//!    and no `.aicad` source program can construct a `List`/`Range`-style
//!    "compiler-builtin construct with no source body" for a *function*
//!    today (only `cad_hir::typeck`'s `CheckedType::List`/`CheckedType::
//!    Range` special-case that pattern for *types*). Giving a program a way
//!    to actually invoke `box(...)`/`cylinder(...)`/etc. therefore needs
//!    either a new `BindingKind` (a geometry-intrinsic callee dispatched
//!    specially, mirroring `BindingKind::EnumVariant`'s own precedent) or
//!    some other new binding/dispatch mechanism `cad-hir`/`cad-runtime`
//!    does not have yet — an `AGENTS.md` "change public language syntax or
//!    semantics beyond an approved RFC" / "select between major unresolved
//!    architecture alternatives" escalation trigger this task does not
//!    silently resolve. See `project/OWNER_DECISIONS.md#D18` (escalated by
//!    this task) and this task's own `project/reports/AICAD-060.md`.
//!
//! This crate therefore fully implements everything decidable without that
//! ruling — the complete `GeometryGraph -> kernel` executor, proven against
//! a real `OcctContext` with B-rep validity/volume/bounding-box/STEP-export
//! evidence (never a render-only check, per `AGENTS.md`'s evidence rule) —
//! and leaves wiring it to actual `.aicad` call syntax for after `D18` is
//! ruled on, exactly as `AICAD-056`/`AICAD-057` left `D16`/`D17`.

pub mod bridge;
pub mod dispatch;
pub mod sketch_lowering;

pub use bridge::number_value_to_quantity;
pub use dispatch::{DispatchError, GraphResults, NodeResult, dispatch_graph};
pub use sketch_lowering::{LoweredFace, SketchLoweringError, lower_profile_to_face};
