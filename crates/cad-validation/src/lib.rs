//! `cad-validation` — shape validation/healing-report normalization
//! (`docs/plan/23_CROSS_SYSTEM_PARAMETER_CATALOG.md` §7-8, WP-01/WP-05),
//! plus (`AICAD-064A`, Stage 3) the D5 v1 comparison-profile module
//! `project/DECISION_LOG.md#DL-12` assigned here: `crate::profile::
//! ComparisonProfile`.
//!
//! `crate::healing::RepairPolicy` (`AICAD-120`, Stage 5) is this crate's
//! first `validate()`/`heal()`-family content: the explicit modeling/
//! construction tolerance policy `sew`/`heal` calls use (`project/
//! DECISION_LOG.md#DL-24` domain 2). The real native sew/heal calls and
//! their own raw structured evidence remain `cad-occt-bridge`'s
//! (`SewReport`/`HealReport`) — this crate stays kernel-neutral.
//!
//! This crate is deliberately kernel-neutral: production code contains no
//! `cad-occt-bridge`/`cad-kernel-api` dependency at all (see `profile`'s
//! own module doc comment for `ComparisonProfile`'s identical rationale).
//! The calibration evidence that derived `ComparisonProfile`'s v1
//! constants (`tests/calibration.rs`) exercises the real kernel bridge as
//! a *dev-dependency only* — never part of this crate's public API or
//! production dependency graph.

pub mod healing;
pub mod profile;

pub use healing::RepairPolicy;
pub use profile::ComparisonProfile;
