//! `cad-configurations` — Stage-6 D29 configuration semantics: immutable
//! overlays over the `cad-assemblies` base model (`AICAD-149`) and typed
//! declarative validity rules over a configuration's own values
//! (`AICAD-150`).
//!
//! Depends on `cad-assemblies` one direction only, mirroring `cad-assembly-
//! solver`'s own precedent: `cad-assemblies` has no dependency on this
//! crate, so Checkpoint A/B's "the semantic assembly model works without
//! solver/configuration authority" evidence is unaffected by anything here.
//!
//! - [`id::ConfigurationId`] — stable configuration identity.
//! - [`overlay::Configuration`]/[`overlay::resolve`] — the overlay itself
//!   and its read-only resolution against a base registry.
//! - [`rule`] — typed declarative validity rules over a configuration's
//!   named values.
//!
//! `AICAD-151` (suppression) builds on this module next.

pub mod id;
pub mod overlay;
pub mod rule;

pub use id::ConfigurationId;
pub use overlay::{Configuration, ResolvedConfiguration, resolve};
pub use rule::{
    Rule, RuleError, RuleEvaluation, RuleName, RuleSet, RuleSetError, RuleViolation, evaluate,
};
