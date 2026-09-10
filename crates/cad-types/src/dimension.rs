//! `Dimension` — RFC-0004 §3's frozen minimum first-class dimensional
//! quantity types, plus the `AffineKind` discriminant RFC-0004 §5's
//! Stage-0-independent-review patch adds for affine-dimensioned
//! quantities.

use std::fmt;

/// One of RFC-0004 §3's minimum first-class dimensions.
///
/// This is the *named* identity of a dimension — what a source-level
/// `Length`/`Force`/etc. type reference resolves to (see
/// `crates/cad-ast/src/item.rs`'s `Type::Named`, whose own doc comment
/// says this binding "happens starting `AICAD-046`") — not its structural
/// exponent-vector decomposition (`Velocity = Length^1 * Time^-1`), which
/// is `cad-units`'s `AICAD-047` concern: RFC-0004 §5 says derived
/// dimensions "are canonicalized structurally, by dimension exponents...
/// not by how a unit is spelled in source," and that is a distinct
/// question from *which named dimensions exist in the first place*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dimension {
    Length,
    Area,
    Volume,
    Angle,
    Time,
    Mass,
    Temperature,
    Force,
    Torque,
    Pressure,
    Stress,
    Energy,
    Power,
    Density,
    Velocity,
    Acceleration,
    AngularVelocity,
    Frequency,
    Current,
    Voltage,
    Resistance,
}

impl Dimension {
    /// All 21 dimensions, in RFC-0004 §3's own listed order. Used by this
    /// module's own exhaustiveness tests.
    pub const ALL: [Dimension; 21] = [
        Dimension::Length,
        Dimension::Area,
        Dimension::Volume,
        Dimension::Angle,
        Dimension::Time,
        Dimension::Mass,
        Dimension::Temperature,
        Dimension::Force,
        Dimension::Torque,
        Dimension::Pressure,
        Dimension::Stress,
        Dimension::Energy,
        Dimension::Power,
        Dimension::Density,
        Dimension::Velocity,
        Dimension::Acceleration,
        Dimension::AngularVelocity,
        Dimension::Frequency,
        Dimension::Current,
        Dimension::Voltage,
        Dimension::Resistance,
    ];

    /// The dimension's source-level type name, exactly as RFC-0004 §3
    /// spells it.
    pub const fn name(self) -> &'static str {
        match self {
            Dimension::Length => "Length",
            Dimension::Area => "Area",
            Dimension::Volume => "Volume",
            Dimension::Angle => "Angle",
            Dimension::Time => "Time",
            Dimension::Mass => "Mass",
            Dimension::Temperature => "Temperature",
            Dimension::Force => "Force",
            Dimension::Torque => "Torque",
            Dimension::Pressure => "Pressure",
            Dimension::Stress => "Stress",
            Dimension::Energy => "Energy",
            Dimension::Power => "Power",
            Dimension::Density => "Density",
            Dimension::Velocity => "Velocity",
            Dimension::Acceleration => "Acceleration",
            Dimension::AngularVelocity => "AngularVelocity",
            Dimension::Frequency => "Frequency",
            Dimension::Current => "Current",
            Dimension::Voltage => "Voltage",
            Dimension::Resistance => "Resistance",
        }
    }

    /// Resolve a source-level type name to its `Dimension`, or `None` if
    /// `name` does not name one of RFC-0004 §3's dimensions.
    pub fn from_name(name: &str) -> Option<Dimension> {
        Self::ALL.into_iter().find(|d| d.name() == name)
    }

    /// Whether this dimension has affine (offset, not purely scaled) units
    /// (RFC-0004 §7: "initially Celsius and Fahrenheit"). Only affine
    /// dimensions ever carry a meaningful `AffineKind` — RFC-0004 §5's
    /// patch is explicit that the discriminant applies to
    /// "affine-dimensioned quantities", not every quantity, so a
    /// `Length`/`Force`/etc. value never carries one (modeled at call
    /// sites as `Option<AffineKind>`, always `None` when `is_affine` is
    /// `false`).
    pub const fn is_affine(self) -> bool {
        matches!(self, Dimension::Temperature)
    }
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The absolute-vs-delta discriminant RFC-0004 §5's Stage-0-review patch
/// adds to affine-dimensioned quantities (`Dimension::is_affine`), carried
/// explicitly rather than inferred from unit spelling or surrounding
/// context (§5, §7 both require this).
///
/// Only meaningful for affine dimensions. A non-affine quantity is
/// modeled as having no `AffineKind` at all (`Option<AffineKind>` at the
/// call site) rather than an implicit third variant here — a `Length` or
/// `Force` value is not "absolute" in the sense this discriminant means,
/// it simply does not carry the affine concept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AffineKind {
    /// An absolute quantity (`20degC`). RFC-0004 §7: two absolutes may
    /// never be added together; subtracting two absolutes yields a
    /// `Delta` (§7's own patch).
    Absolute,
    /// A delta/difference quantity (e.g. a temperature *change* of
    /// `5degC`). Does not carry the affine offset on conversion (§7): a
    /// 5°C difference converts to a 5K difference, never a 278.15K one.
    Delta,
}

impl fmt::Display for AffineKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            AffineKind::Absolute => "absolute",
            AffineKind::Delta => "delta",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_covers_every_variant_exactly_once() {
        assert_eq!(Dimension::ALL.len(), 21);
        let mut names: Vec<&str> = Dimension::ALL.iter().map(|d| d.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 21, "duplicate dimension name in ALL");
    }

    #[test]
    fn matches_rfc_0004_section_3_minimum_list_exactly() {
        // Pinned directly against RFC-0004 §3's own listed order so a
        // silent drop/rename/reorder is caught immediately.
        let expected = [
            "Length",
            "Area",
            "Volume",
            "Angle",
            "Time",
            "Mass",
            "Temperature",
            "Force",
            "Torque",
            "Pressure",
            "Stress",
            "Energy",
            "Power",
            "Density",
            "Velocity",
            "Acceleration",
            "AngularVelocity",
            "Frequency",
            "Current",
            "Voltage",
            "Resistance",
        ];
        let actual: Vec<&str> = Dimension::ALL.iter().map(|d| d.name()).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn from_name_round_trips_every_variant() {
        for d in Dimension::ALL {
            assert_eq!(Dimension::from_name(d.name()), Some(d));
        }
    }

    #[test]
    fn from_name_rejects_unknown_and_primitive_names() {
        assert_eq!(Dimension::from_name("Speed"), None);
        assert_eq!(Dimension::from_name("length"), None); // case-sensitive
        assert_eq!(Dimension::from_name("Int"), None);
    }

    #[test]
    fn only_temperature_is_affine() {
        for d in Dimension::ALL {
            assert_eq!(d.is_affine(), d == Dimension::Temperature, "{d:?}");
        }
    }

    #[test]
    fn display_matches_name() {
        for d in Dimension::ALL {
            assert_eq!(d.to_string(), d.name());
        }
    }

    #[test]
    fn affine_kind_display() {
        assert_eq!(AffineKind::Absolute.to_string(), "absolute");
        assert_eq!(AffineKind::Delta.to_string(), "delta");
    }

    #[test]
    fn affine_kind_variants_are_distinct() {
        assert_ne!(AffineKind::Absolute, AffineKind::Delta);
    }
}
