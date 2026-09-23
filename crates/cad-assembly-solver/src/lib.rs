//! `cad-assembly-solver` -- the numerical assembly-solver adapter for
//! Stage-6 assemblies (`AICAD-142`, batch `S6-05`,
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
//!
//! `AICAD-143` (deterministic grounding/lowering) and `AICAD-144` (the
//! baseline backend) are later commits in this same batch, adding
//! `grounding`/`baseline`/`linalg` on top of this module.

pub mod adapter;

pub use adapter::{
    AdapterOutcome, AdapterSolution, AssemblyProblem, AssemblySolverAdapter, CONVERGENCE_TOLERANCE,
    EchoAdapter, ProblemError, RejectingAdapter, Residual, ZeroStartAdapter,
};
