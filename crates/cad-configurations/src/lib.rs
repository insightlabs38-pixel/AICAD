//! `cad-configurations` — Stage-6 D29 configuration semantics: immutable
//! overlays over the `cad-assemblies` base model (`AICAD-149`), typed
//! declarative validity rules over a configuration's own values
//! (`AICAD-150`), and suppression as an inactive-but-traceable overlay
//! state (`AICAD-151`).
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
//! - [`suppression`] — inactive-but-traceable occurrence/mate/quantity
//!   views derived from a configuration's suppressed-occurrence set.

pub mod id;
pub mod overlay;
pub mod rule;
pub mod suppression;

pub use id::ConfigurationId;
pub use overlay::{Configuration, ResolvedConfiguration, resolve};
pub use rule::{
    Rule, RuleError, RuleEvaluation, RuleName, RuleSet, RuleSetError, RuleViolation, evaluate,
};
pub use suppression::{
    MateActivation, active_definition_counts, active_occurrences, classify_mates, is_active,
    suppressed_occurrences,
};
