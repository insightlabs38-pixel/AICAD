//! `cad-validation` — shape validation/healing-report normalization
//! (`docs/plan/23_CROSS_SYSTEM_PARAMETER_CATALOG.md` §7-8, WP-01/WP-05;
//! not yet implemented — see this crate's `README.md`), plus (`AICAD-064A`,
//! Stage 3) the D5 v1 comparison-profile module `project/DECISION_LOG.md
//! #DL-12` assigned here: `crate::profile::ComparisonProfile`.
//!
//! This module is deliberately kernel-neutral: it compares plain `f64`
//! measurements against a versioned tolerance profile and contains no
//! `cad-occt-bridge`/`cad-kernel-api` dependency at all (see that crate's
//! own module doc comment). The calibration evidence that derived the v1
//! constants (`tests/calibration.rs`) exercises the real kernel bridge as
//! a *dev-dependency only* — never part of this crate's public API or
//! production dependency graph.

pub mod profile;

pub use profile::ComparisonProfile;
