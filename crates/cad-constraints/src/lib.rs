//! `cad-constraints` — WP-08 constraint system, per `docs/plan/
//! 08_CONSTRAINTS_REQUIREMENTS_TESTS.md` §2-6 and `docs/plan/
//! 22_REPOSITORY_WORK_PACKAGES.md` WP-08.
//!
//! - [`sketch_constraint`][]: [`sketch_constraint::ConstraintSet`]/
//!   [`sketch_constraint::ConstraintKind`]/[`sketch_constraint::SketchSolver`]
//!   (`AICAD-073`, Batch S3-05, `project/DECISION_LOG.md#DL-20`) — the
//!   solver-independent 2D sketch constraint IR and its solver-adapter
//!   boundary. See that module's own doc comment for the full identity
//!   model and deliberate scope cuts (which baseline constraint kinds are
//!   covered, and why no concrete `SketchSolver` implementation ships
//!   with this task).
//!
//! Not yet implemented (future work, per this crate's own `README.md`):
//! 3D geometric constraints, assembly mate/joint constraints, algebraic
//! parameter constraints, and any concrete `SketchSolver`
//! implementation (`AICAD-074`).

pub mod sketch_constraint;

pub use sketch_constraint::{
    Constraint, ConstraintId, ConstraintIrError, ConstraintKind, ConstraintSet, PointRef,
    SketchSolver, SketchVariable, SolveReport, SolveStatus, SolvedValues, resolve_point,
    sketch_variables,
};
