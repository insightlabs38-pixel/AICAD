//! `cad-assembly-solver` -- the numerical assembly-solver adapter for
//! Stage-6 assemblies (`AICAD-142`/`143`, batch `S6-05`,
//! `project/OWNER_DECISIONS.md#D28`, `project/DECISION_LOG.md#DL-30`).
//!
//! `cad-assemblies` (Stage-6's own semantic mate/joint IR, `AICAD-139`/
//! `140`) never depends on this crate -- Checkpoint A (`AICAD-141`)
//! already proved the semantic assembly model works with no solver/
//! adapter crate anywhere in `cad-assemblies`'s dependency tree, and
//! nothing in this crate reverses that: it is *this* crate that depends
//! on `cad-assemblies`, one direction only.
//!
//! - [`adapter`] (`AICAD-142`) -- the semantic-neutral numeric adapter
//!   contract ([`adapter::AssemblySolverAdapter`]) plus reference/mock
//!   backends.
//! - [`grounding`] (`AICAD-143`) -- the deterministic grounding/initial-
//!   pose/representative-pose policy, and the lowering from
//!   `cad_assemblies::{Mate, Joint}` into [`adapter::AssemblyProblem`].
//! - [`linalg`] -- small dense-matrix utilities [`grounding`] uses for its
//!   own structural DOF analysis.
//!
//! `AICAD-144` (the baseline backend) is a later commit in this same
//! batch, adding `baseline` on top of this module.

pub mod adapter;
pub mod grounding;
mod linalg;

pub use adapter::{
    AdapterOutcome, AdapterSolution, AssemblyProblem, AssemblySolverAdapter, CONVERGENCE_TOLERANCE,
    EchoAdapter, ProblemError, RejectingAdapter, Residual, ZeroStartAdapter,
};
pub use grounding::{
    GroundingProfile, Lowering, LoweringError, StructuralDof, lower, structural_dof,
};
