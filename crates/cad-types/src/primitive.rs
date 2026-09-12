//! `PrimitiveType` — RFC-0004 §2's frozen ordinary (non-dimensional) type
//! list, itself frozen from
//! `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §2.

use std::fmt;

/// One of AICAD's ordinary (non-dimensional) primitive types.
///
/// `Decimal` and `Char` are listed as "optional" by both the plan and the
/// RFC; they are included here as ordinary variants (not feature-gated)
/// because neither document states a condition under which they would be
/// omitted from a conforming implementation — "optional" describes them as
/// optional *additions to the language design*, not as a build-time
/// toggle within this representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    Bool,
    Int,
    UInt,
    Float,
    Decimal,
    String,
    Bytes,
    Char,
}

impl PrimitiveType {
    /// All primitive types, in RFC-0004 §2's own listed order. Used by
    /// this module's own exhaustiveness tests.
    pub const ALL: [PrimitiveType; 8] = [
        PrimitiveType::Bool,
        PrimitiveType::Int,
        PrimitiveType::UInt,
        PrimitiveType::Float,
        PrimitiveType::Decimal,
        PrimitiveType::String,
        PrimitiveType::Bytes,
        PrimitiveType::Char,
    ];

    /// The type's source-level spelling, exactly as RFC-0004 §2 names it
    /// (matching how a `cad_ast::item::Type::Named` value for one of these
    /// would read as source text, e.g. `let ok: Bool = true;`).
    pub const fn name(self) -> &'static str {
        match self {
            PrimitiveType::Bool => "Bool",
            PrimitiveType::Int => "Int",
            PrimitiveType::UInt => "UInt",
            PrimitiveType::Float => "Float",
            PrimitiveType::Decimal => "Decimal",
            PrimitiveType::String => "String",
            PrimitiveType::Bytes => "Bytes",
            PrimitiveType::Char => "Char",
        }
    }

    /// Resolve a source-level type name to its `PrimitiveType`, or `None`
    /// if `name` does not name one of RFC-0004 §2's primitives (e.g. it
    /// names a dimensional or user-defined type instead). Case-sensitive:
    /// AICAD source-level type names are capitalized exactly as listed
    /// above, matching every worked example in the frozen plan/RFC text.
    pub fn from_name(name: &str) -> Option<PrimitiveType> {
        Self::ALL.into_iter().find(|p| p.name() == name)
    }
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_covers_every_variant_exactly_once() {
        // Exhaustiveness guard: if a variant is added/removed without
        // updating `ALL`, this and the round-trip tests below would
        // silently drift; this test pins the count and rejects duplicates.
        assert_eq!(PrimitiveType::ALL.len(), 8);
        let mut names: Vec<&str> = PrimitiveType::ALL.iter().map(|p| p.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 8, "duplicate primitive type name in ALL");
    }

    #[test]
    fn name_matches_rfc_0004_section_2_spelling() {
        assert_eq!(PrimitiveType::Bool.name(), "Bool");
        assert_eq!(PrimitiveType::Int.name(), "Int");
        assert_eq!(PrimitiveType::UInt.name(), "UInt");
        assert_eq!(PrimitiveType::Float.name(), "Float");
        assert_eq!(PrimitiveType::Decimal.name(), "Decimal");
        assert_eq!(PrimitiveType::String.name(), "String");
        assert_eq!(PrimitiveType::Bytes.name(), "Bytes");
        assert_eq!(PrimitiveType::Char.name(), "Char");
    }

    #[test]
    fn from_name_round_trips_every_variant() {
        for p in PrimitiveType::ALL {
            assert_eq!(PrimitiveType::from_name(p.name()), Some(p));
        }
    }

    #[test]
    fn from_name_rejects_unknown_and_dimensional_names() {
        assert_eq!(PrimitiveType::from_name("Length"), None);
        assert_eq!(PrimitiveType::from_name("bool"), None); // case-sensitive
        assert_eq!(PrimitiveType::from_name(""), None);
        assert_eq!(PrimitiveType::from_name("Point3"), None);
    }

    #[test]
    fn display_matches_name() {
        for p in PrimitiveType::ALL {
            assert_eq!(p.to_string(), p.name());
        }
    }

    #[test]
    fn is_copy_and_eq() {
        // Primitive types are small closed-set tags; they must be cheap
        // Copy values usable as HashMap/HashSet keys by later type-checker
        // work without the caller having to clone/allocate.
        let a = PrimitiveType::Int;
        let b = a;
        assert_eq!(a, b);
    }
}
