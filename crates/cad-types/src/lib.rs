//! `cad-types` — AICAD's primitive and dimensional type representation
//! (RFC-0004 §2-3, §5's affine-kind patch, §7).
//!
//! This is the *semantic* type-system counterpart to
//! `cad_ast::item::Type` (`crates/cad-ast/src/item.rs`'s `Type::Named`/
//! `Type::Generic`, which are purely syntactic — see that type's own doc
//! comment: "no relation yet to any `cad-types`/`cad-units` concept; that
//! binding happens starting `AICAD-046`"). Resolving a syntactic `Type` to
//! one of these is later binding/type-checking work (`AICAD-050`/`052`);
//! this crate only defines what the resolved types *are*.
//!
//! ## Scope (`AICAD-046`; see `project/reports/AICAD-046.md` for the full
//! accounting)
//! - `PrimitiveType`: RFC-0004 §2's ordinary (non-dimensional) types.
//! - `Dimension`: RFC-0004 §3's minimum first-class dimensions, as a
//!   closed *named* set — not their structural exponent decomposition.
//! - `AffineKind`: the absolute/delta discriminant RFC-0004 §5's
//!   Stage-0-review patch adds for affine-dimensioned quantities.
//!
//! Deliberately **out of scope** here:
//! - Derived-dimension exponent algebra/structural canonicalization
//!   (RFC-0004 §5's "canonicalized structurally, by dimension exponents")
//!   is `cad-units`'s `AICAD-047`.
//! - Concrete unit literals and conversions (RFC-0004 §4) are
//!   `cad-units`'s `AICAD-048`.
//! - `Tolerance<T>`/`Range<T>`/`Distribution<T>`/`Fit` (RFC-0004 §6) —
//!   no Stage-2 batch task through S2-05 names them; guessing their
//!   type-level representation ahead of a task that actually owns them
//!   would be exactly the speculative work `AGENTS.md`'s "No speculative
//!   future work" rule warns against.

mod dimension;
mod primitive;

pub use dimension::{AffineKind, Dimension};
pub use primitive::PrimitiveType;
