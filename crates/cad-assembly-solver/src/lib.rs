//! `cad-assembly-solver` -- the numerical assembly-solver adapter for
//! Stage-6 assemblies (`AICAD-142`/`143`/`144`, batch `S6-05`,
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
//! - [`baseline`] (`AICAD-144`) -- the one bounded deterministic
//!   Levenberg-Marquardt backend behind the adapter contract, plus
//!   [`baseline::solve_assembly`], the AICAD-owned entry point most
//!   callers use.
//! - [`dof`] (`AICAD-145`) -- semantic degrees-of-freedom analysis over
//!   [`grounding::lower_for_dof_analysis`]'s all-non-ground lowering,
//!   attributed back to the occurrences/relations that produced it.
//! - [`conflict`] (`AICAD-146`) -- structural redundancy/conflict
//!   diagnostics over [`grounding::lower`]'s solver-facing lowering,
//!   distinguishing dependent-but-consistent relations from mutually
//!   inconsistent ones without requiring an iterative solve.
//! - [`linalg`] -- small dense-matrix utilities [`grounding`] and
//!   [`baseline`] share.

pub mod adapter;
pub mod baseline;
pub mod conflict;
pub mod dof;
pub mod grounding;
mod linalg;

pub use adapter::{
    AdapterOutcome, AdapterSolution, AssemblyProblem, AssemblySolverAdapter, CONVERGENCE_TOLERANCE,
    EchoAdapter, ProblemError, RejectingAdapter, Residual, ZeroStartAdapter,
};
pub use baseline::{BaselineSolver, BaselineSolverProfile, SolveOutcome, solve_assembly};
pub use conflict::{AssemblyDiagnosis, ConstraintDiagnosis, diagnose};
pub use dof::{AssemblyDof, OccurrenceDof, analyze as analyze_dof};
pub use grounding::{
    GroundingProfile, Lowering, LoweringError, StructuralDof, lower, lower_for_dof_analysis,
    structural_dof,
};
