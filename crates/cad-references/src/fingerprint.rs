//! [`FingerprintEvidence`] — approximate geometric evidence for the
//! `ConstructionStrategy::GeometricFingerprint` recipe strategy
//! (`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §3 item 6).
//!
//! Deliberately plain, untyped `f64` fields, not `cad_units` quantities:
//! the plan itself calls this "approximate position/normal/area/radius as
//! a fallback discriminator" — it is evidence for ranking/diagnostics
//! (`project/DECISION_LOG.md#DL-8`), never a value participating in exact
//! typed-unit arithmetic. `AICAD-092` (a later, separately-gated task) may
//! extend this shape once it implements fingerprint-fallback policy
//! itself; this task only represents the data a recipe may carry today.

/// Approximate, non-authoritative geometric evidence. Per D7/`DL-8`, a
/// [`crate::recipe::ConstructionStrategy::GeometricFingerprint`] recipe's
/// durability is unconditionally
/// [`crate::durability::DurabilityLevel::QueryGeometric`] — this struct
/// cannot upgrade that classification no matter which fields are set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FingerprintEvidence {
    pub position: [f64; 3],
    pub normal: Option<[f64; 3]>,
    pub area: Option<f64>,
    pub radius: Option<f64>,
}

impl FingerprintEvidence {
    pub fn at_position(position: [f64; 3]) -> FingerprintEvidence {
        FingerprintEvidence {
            position,
            normal: None,
            area: None,
            radius: None,
        }
    }

    pub fn with_normal(mut self, normal: [f64; 3]) -> FingerprintEvidence {
        self.normal = Some(normal);
        self
    }

    pub fn with_area(mut self, area: f64) -> FingerprintEvidence {
        self.area = Some(area);
        self
    }

    pub fn with_radius(mut self, radius: f64) -> FingerprintEvidence {
        self.radius = Some(radius);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_sets_only_requested_fields() {
        let evidence = FingerprintEvidence::at_position([1.0, 2.0, 3.0]).with_area(42.0);
        assert_eq!(evidence.position, [1.0, 2.0, 3.0]);
        assert_eq!(evidence.area, Some(42.0));
        assert_eq!(evidence.normal, None);
        assert_eq!(evidence.radius, None);
    }
}
