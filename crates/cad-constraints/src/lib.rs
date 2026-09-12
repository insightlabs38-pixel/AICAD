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
//!   covered).
//! - [`sketch_solver`][]: [`sketch_solver::RelaxationSolver`] (`AICAD-074`)
//!   — the first concrete [`sketch_constraint::SketchSolver`]
//!   implementation `DL-20` permits ("exactly one initial solver
//!   implementation... behind this interface"), a direct-projection
//!   Gauss-Seidel relaxation solver covering every baseline constraint
//!   kind. See that module's own doc comment for the algorithm,
//!   classification rules, versioned tolerance profile, and scope cuts.
//!
//! - [`apply`][]: [`apply::apply_solved_values`] (`AICAD-075`) — applies a
//!   [`SolveReport`]'s solved magnitudes back onto a concrete [`Sketch`]'s
//!   entities, producing the new (never-mutated-in-place, `D2`) [`Sketch`]
//!   value `cad-geometry-runtime::sketch_lowering` needs before it can
//!   lower a solved [`sketch_constraint::ConstraintSet`]'s target profile
//!   into kernel geometry. See that module's own doc comment for the
//!   partial-solution/functional-semantics rationale.
//!
//! Not yet implemented (future work, per this crate's own `README.md`):
//! 3D geometric constraints, assembly mate/joint constraints, and
//! algebraic parameter constraints.

pub mod apply;
pub mod sketch_constraint;
pub mod sketch_solver;

pub use apply::apply_solved_values;
pub use sketch_constraint::{
    Constraint, ConstraintId, ConstraintIrError, ConstraintKind, ConstraintSet, PointRef,
    SketchSolver, SketchVariable, SolveReport, SolveStatus, SolvedValues, resolve_point,
    sketch_variables,
};
pub use sketch_solver::{RelaxationSolver, SketchSolverProfile};
