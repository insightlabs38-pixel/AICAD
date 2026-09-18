//! Kernel-neutral raw-to-safe adoption outcome (`AICAD-108`, `project/
//! DECISION_LOG.md#DL-24` (D22): "Promotion from raw geometry into normal
//! safe semantic geometry requires an **explicit validation/adoption
//! operation** that produces a new safe value and retains appropriate
//! validation/provenance evidence").
//!
//! # The raw-handle tier already exists — this module is the missing other
//! half
//!
//! `cad_references::raw_handle::RawHandle<T>` (`AICAD-093`) already
//! implements D22's raw/unsafe tier in full: an epoch-bound, AICAD-owned,
//! kernel-neutral wrapper generic over its own payload — Stage 5's raw
//! topology tier can (and, per this task's finding, should) reuse it
//! directly (e.g. `RawHandle<cad_kernel_api::KernelShape>`) rather than
//! this task minting a second, parallel raw-handle type; see `project/
//! reports/AICAD-108.md` for the explicit "no new type needed here" finding.
//! What D22 requires and no existing type yet provides is the *adoption*
//! half — the explicit operation's own outcome shape, which this module
//! defines.
//!
//! [`AdoptionOutcome`] is generic over the safe value a successful adoption
//! produces (a future caller instantiates it with, e.g., a `GeomId`) so
//! this module fixes only the adoption *contract* — validated evidence on
//! success, an explicit, never-guessed reason on failure — not any
//! particular raw payload's own validation logic (`AICAD-122`-`124`'s own
//! job, per the fixed Stage-5 batch order).

/// The outcome of one explicit raw-to-safe adoption operation — see module
/// doc comment. Never constructed by silently reinterpreting a raw value as
/// validated (D22's own explicit prohibition) — a caller only ever obtains
/// one of these from running the actual validation an adoption operation
/// performs.
#[derive(Debug, Clone, PartialEq)]
pub enum AdoptionOutcome<T> {
    /// The raw value was validated and promoted into `value`, a new safe
    /// semantic value — `evidence` records what was actually checked, so a
    /// caller (or a later audit) can inspect *why* this adoption was
    /// accepted, not merely that it was.
    Adopted {
        value: T,
        evidence: AdoptionEvidence,
    },
    /// Adoption was refused — see [`AdoptionRejection`]. No safe value is
    /// ever produced alongside a rejection.
    Rejected(AdoptionRejection),
}

impl<T> AdoptionOutcome<T> {
    pub fn is_adopted(&self) -> bool {
        matches!(self, AdoptionOutcome::Adopted { .. })
    }

    /// The adopted value, if adoption succeeded.
    pub fn value(&self) -> Option<&T> {
        match self {
            AdoptionOutcome::Adopted { value, .. } => Some(value),
            AdoptionOutcome::Rejected(_) => None,
        }
    }
}

/// What a successful adoption actually checked — an honest record, not a
/// bare "trust me" flag. `checks_performed` names each validation step run
/// (e.g. `"is_valid"`, `"closed_shell"`); a future task may extend this
/// with richer structured evidence without breaking this shape (new fields
/// added, not existing ones repurposed — `AGENTS.md`'s diagnostic-stability
/// precedent applied here to evidence records).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AdoptionEvidence {
    pub checks_performed: Vec<String>,
}

impl AdoptionEvidence {
    pub fn new(checks_performed: Vec<String>) -> AdoptionEvidence {
        AdoptionEvidence { checks_performed }
    }
}

/// Why an adoption was refused — never guessed at, never silently
/// defaulted to "invalid".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdoptionRejection {
    /// The raw handle was presented after its own owning epoch advanced
    /// (`cad_references::raw_handle::StaleHandle`'s own condition) —
    /// adoption never accepts a handle it cannot first prove is still
    /// current.
    StaleHandle,
    /// The raw geometry failed an explicit validation check; `reason`
    /// names which one (e.g. `"not manifold"`, `"self-intersecting"`).
    ValidationFailed { reason: String },
    /// This raw payload's own shape/kind is not yet supported by adoption
    /// — an honest "not implemented", never a silent pass-through.
    Unsupported { reason: String },
}

impl std::fmt::Display for AdoptionRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdoptionRejection::StaleHandle => {
                write!(f, "adoption refused: the raw handle is stale")
            }
            AdoptionRejection::ValidationFailed { reason } => {
                write!(f, "adoption refused: validation failed ({reason})")
            }
            AdoptionRejection::Unsupported { reason } => {
                write!(f, "adoption refused: unsupported ({reason})")
            }
        }
    }
}

impl std::error::Error for AdoptionRejection {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adopted_outcome_carries_the_value_and_its_own_evidence() {
        let outcome = AdoptionOutcome::Adopted {
            value: 42,
            evidence: AdoptionEvidence::new(vec!["is_valid".to_string()]),
        };
        assert!(outcome.is_adopted());
        assert_eq!(outcome.value(), Some(&42));
    }

    #[test]
    fn rejected_outcome_carries_no_value() {
        let outcome: AdoptionOutcome<i32> =
            AdoptionOutcome::Rejected(AdoptionRejection::StaleHandle);
        assert!(!outcome.is_adopted());
        assert_eq!(outcome.value(), None);
    }

    #[test]
    fn every_rejection_reason_has_a_non_empty_message() {
        for rejection in [
            AdoptionRejection::StaleHandle,
            AdoptionRejection::ValidationFailed {
                reason: "not manifold".to_string(),
            },
            AdoptionRejection::Unsupported {
                reason: "raw wire adoption".to_string(),
            },
        ] {
            assert!(!rejection.to_string().is_empty());
        }
    }

    #[test]
    fn rejection_implements_std_error() {
        fn assert_error<E: std::error::Error>(_: &E) {}
        assert_error(&AdoptionRejection::StaleHandle);
    }

    #[test]
    fn default_evidence_records_no_checks() {
        let evidence = AdoptionEvidence::default();
        assert!(evidence.checks_performed.is_empty());
    }
}
