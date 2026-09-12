//! Unit registry and conversions — RFC-0004 §4's frozen **initial** unit
//! literal set: "length (`nm um mm cm m km in ft`), angle (`deg rad`),
//! mass (`mg g kg lbm`), force (`N kN lbf`), pressure/stress (`Pa kPa MPa
//! GPa psi ksi`), temperature (`K degC degF`, affine)." RFC-0004 §4 itself
//! is explicit that "the standard library may expand this set without a
//! grammar change" — this registry covers exactly the six unit families
//! §4 freezes now (`AICAD-048`), nothing beyond it (no units exist yet for
//! `Volume`/`Time`/`Torque`/`Energy`/`Power`/etc. — RFC-0004 §4 defines
//! none for them either).
//!
//! `crates/cad-lexer` already recognizes *any* identifier-shaped suffix
//! immediately following a number as a candidate unit (`AICAD-040`,
//! deliberately deferring validation to "the unit registry (`AICAD-048`,
//! `crates/cad-units`), which is where RFC-0004 §4's actual table
//! belongs as executable data" — see `project/reports/AICAD-040.md`
//! decision 1). This module is that table.

use cad_types::Dimension;
use std::fmt;

/// One registered unit literal: its source spelling, the named dimension
/// it belongs to, and how to convert a value in this unit to/from that
/// dimension's canonical unit (documented per family below).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitDef {
    /// Exact source-level spelling (`"mm"`, `"degC"`, ...), matching
    /// `crates/cad-lexer`'s fused `Number { unit: Option<String>, .. }`
    /// suffix text.
    pub symbol: &'static str,
    pub dimension: Dimension,
    /// Multiply a value in this unit by this factor to reach the
    /// dimension's canonical unit (before any affine offset below).
    pub scale_to_canonical: f64,
    /// `Some(offset)` only for affine units (`Dimension::is_affine`,
    /// currently `degC`/`degF` — never `K`, which is already the affine
    /// *canonical* unit and therefore needs no offset of its own): the
    /// additive offset applied, after scaling, when converting an
    /// **absolute** quantity to canonical (RFC-0004 §7). Never applied for
    /// a **delta**/difference quantity — see `to_canonical_delta` below,
    /// and RFC-0004 §7's own example: "a 5°C difference is a 5K
    /// difference, not a 278.15K one."
    pub affine_offset: Option<f64>,
}

// --- Length: canonical unit = metre (SI base unit). ---
const M_PER_IN: f64 = 0.0254; // internationally defined exact value.
const M_PER_FT: f64 = 12.0 * M_PER_IN; // 1 ft = 12 in, exact.

// --- Angle: canonical unit = radian. ---
const RAD_PER_DEG: f64 = std::f64::consts::PI / 180.0;

// --- Mass: canonical unit = kilogram (SI base unit). ---
const KG_PER_LBM: f64 = 0.453_592_37; // international avoirdupois pound, exact.

// --- Force: canonical unit = newton. ---
const STANDARD_GRAVITY_M_PER_S2: f64 = 9.806_65; // standard gravity, exact by definition.
const N_PER_LBF: f64 = KG_PER_LBM * STANDARD_GRAVITY_M_PER_S2; // lbf = lbm * g0.

// --- Pressure/Stress: canonical unit = pascal. ---
const PA_PER_PSI: f64 = N_PER_LBF / (M_PER_IN * M_PER_IN); // psi = lbf / in^2.

// --- Temperature: canonical unit = kelvin (SI base unit, absolute). ---
const KELVIN_PER_DEGC_OFFSET: f64 = 273.15; // exact by definition of the Celsius scale.
const KELVIN_PER_DEGF_SCALE: f64 = 5.0 / 9.0; // 1 degF increment = 5/9 K increment.
// K = (F - 32) * 5/9 + 273.15 = F * 5/9 + (273.15 - 32 * 5/9).
const KELVIN_PER_DEGF_OFFSET: f64 = KELVIN_PER_DEGC_OFFSET - 32.0 * KELVIN_PER_DEGF_SCALE;

/// RFC-0004 §4's complete initial unit set, one `UnitDef` per (symbol,
/// dimension) pair. `Pa`/`kPa`/`MPa`/`GPa`/`psi`/`ksi` each appear
/// **twice** — once under `Dimension::Pressure`, once under
/// `Dimension::Stress` — because RFC-0004 §4 lists one shared unit set
/// under the combined heading "pressure/stress", and `AICAD-047`'s own
/// finding (`crates/cad-units/src/dimension_vector.rs`'s module doc
/// comment) is that these stay distinct named dimensions despite sharing
/// units. `lookup`/`lookup_any` below reflect that: looking a symbol up
/// under one specific dimension is exact; looking it up by symbol alone
/// can return more than one candidate, by design (see their own doc
/// comments).
pub const UNITS: &[UnitDef] = &[
    // Length.
    UnitDef {
        symbol: "nm",
        dimension: Dimension::Length,
        scale_to_canonical: 1e-9,
        affine_offset: None,
    },
    UnitDef {
        symbol: "um",
        dimension: Dimension::Length,
        scale_to_canonical: 1e-6,
        affine_offset: None,
    },
    UnitDef {
        symbol: "mm",
        dimension: Dimension::Length,
        scale_to_canonical: 1e-3,
        affine_offset: None,
    },
    UnitDef {
        symbol: "cm",
        dimension: Dimension::Length,
        scale_to_canonical: 1e-2,
        affine_offset: None,
    },
    UnitDef {
        symbol: "m",
        dimension: Dimension::Length,
        scale_to_canonical: 1.0,
        affine_offset: None,
    },
    UnitDef {
        symbol: "km",
        dimension: Dimension::Length,
        scale_to_canonical: 1e3,
        affine_offset: None,
    },
    UnitDef {
        symbol: "in",
        dimension: Dimension::Length,
        scale_to_canonical: M_PER_IN,
        affine_offset: None,
    },
    UnitDef {
        symbol: "ft",
        dimension: Dimension::Length,
        scale_to_canonical: M_PER_FT,
        affine_offset: None,
    },
    // Angle.
    UnitDef {
        symbol: "deg",
        dimension: Dimension::Angle,
        scale_to_canonical: RAD_PER_DEG,
        affine_offset: None,
    },
    UnitDef {
        symbol: "rad",
        dimension: Dimension::Angle,
        scale_to_canonical: 1.0,
        affine_offset: None,
    },
    // Mass.
    UnitDef {
        symbol: "mg",
        dimension: Dimension::Mass,
        scale_to_canonical: 1e-6,
        affine_offset: None,
    },
    UnitDef {
        symbol: "g",
        dimension: Dimension::Mass,
        scale_to_canonical: 1e-3,
        affine_offset: None,
    },
    UnitDef {
        symbol: "kg",
        dimension: Dimension::Mass,
        scale_to_canonical: 1.0,
        affine_offset: None,
    },
    UnitDef {
        symbol: "lbm",
        dimension: Dimension::Mass,
        scale_to_canonical: KG_PER_LBM,
        affine_offset: None,
    },
    // Force.
    UnitDef {
        symbol: "N",
        dimension: Dimension::Force,
        scale_to_canonical: 1.0,
        affine_offset: None,
    },
    UnitDef {
        symbol: "kN",
        dimension: Dimension::Force,
        scale_to_canonical: 1e3,
        affine_offset: None,
    },
    UnitDef {
        symbol: "lbf",
        dimension: Dimension::Force,
        scale_to_canonical: N_PER_LBF,
        affine_offset: None,
    },
    // Pressure.
    UnitDef {
        symbol: "Pa",
        dimension: Dimension::Pressure,
        scale_to_canonical: 1.0,
        affine_offset: None,
    },
    UnitDef {
        symbol: "kPa",
        dimension: Dimension::Pressure,
        scale_to_canonical: 1e3,
        affine_offset: None,
    },
    UnitDef {
        symbol: "MPa",
        dimension: Dimension::Pressure,
        scale_to_canonical: 1e6,
        affine_offset: None,
    },
    UnitDef {
        symbol: "GPa",
        dimension: Dimension::Pressure,
        scale_to_canonical: 1e9,
        affine_offset: None,
    },
    UnitDef {
        symbol: "psi",
        dimension: Dimension::Pressure,
        scale_to_canonical: PA_PER_PSI,
        affine_offset: None,
    },
    UnitDef {
        symbol: "ksi",
        dimension: Dimension::Pressure,
        scale_to_canonical: 1e3 * PA_PER_PSI,
        affine_offset: None,
    },
    // Stress (same units as Pressure — see doc comment above).
    UnitDef {
        symbol: "Pa",
        dimension: Dimension::Stress,
        scale_to_canonical: 1.0,
        affine_offset: None,
    },
    UnitDef {
        symbol: "kPa",
        dimension: Dimension::Stress,
        scale_to_canonical: 1e3,
        affine_offset: None,
    },
    UnitDef {
        symbol: "MPa",
        dimension: Dimension::Stress,
        scale_to_canonical: 1e6,
        affine_offset: None,
    },
    UnitDef {
        symbol: "GPa",
        dimension: Dimension::Stress,
        scale_to_canonical: 1e9,
        affine_offset: None,
    },
    UnitDef {
        symbol: "psi",
        dimension: Dimension::Stress,
        scale_to_canonical: PA_PER_PSI,
        affine_offset: None,
    },
    UnitDef {
        symbol: "ksi",
        dimension: Dimension::Stress,
        scale_to_canonical: 1e3 * PA_PER_PSI,
        affine_offset: None,
    },
    // Temperature (affine — see `affine_offset`'s own doc comment).
    UnitDef {
        symbol: "K",
        dimension: Dimension::Temperature,
        scale_to_canonical: 1.0,
        affine_offset: None,
    },
    UnitDef {
        symbol: "degC",
        dimension: Dimension::Temperature,
        scale_to_canonical: 1.0,
        affine_offset: Some(KELVIN_PER_DEGC_OFFSET),
    },
    UnitDef {
        symbol: "degF",
        dimension: Dimension::Temperature,
        scale_to_canonical: KELVIN_PER_DEGF_SCALE,
        affine_offset: Some(KELVIN_PER_DEGF_OFFSET),
    },
];

/// Look up a unit by its exact symbol under one specific dimension —
/// unambiguous even for symbols like `"Pa"` that exist under more than one
/// dimension, since the caller supplies which one it means (RFC-0004 §5's
/// same-dimension-only implicit-conversion rule always has a known target
/// dimension in context by the time a conversion is needed — see
/// `crates/cad-units/src/dimension_vector.rs`'s module doc comment for the
/// general shape of this argument).
pub fn lookup(symbol: &str, dimension: Dimension) -> Option<&'static UnitDef> {
    UNITS
        .iter()
        .find(|u| u.symbol == symbol && u.dimension == dimension)
}

/// Look up every unit registered under `symbol`, regardless of dimension.
/// Most symbols return exactly one match; `"Pa"`/`"kPa"`/`"MPa"`/`"GPa"`/
/// `"psi"`/`"ksi"` each return two (`Pressure` and `Stress`) — this
/// function surfaces that multiplicity rather than picking one, so a
/// caller (a future type checker resolving a bare unit suffix against
/// context) can see the full candidate set instead of an arbitrarily
/// disambiguated answer, consistent with `AGENTS.md`'s "ambiguity is an
/// error, never an arbitrary selection."
pub fn lookup_any(symbol: &str) -> impl Iterator<Item = &'static UnitDef> {
    UNITS.iter().filter(move |u| u.symbol == symbol)
}

/// Converting between units of different named dimensions is always
/// rejected (RFC-0004 §5: "different physical dimensions never implicitly
/// convert... there is no numeric escape hatch") — including between two
/// dimensions that happen to share identical units, like `Pressure` and
/// `Stress` (`AICAD-047`'s own finding): named-dimension identity, not
/// physical unit equivalence, is what this registry treats as
/// convertible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitConversionError {
    pub from: Dimension,
    pub to: Dimension,
}

impl fmt::Display for UnitConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cannot convert between different dimensions: {} and {}",
            self.from, self.to
        )
    }
}

impl std::error::Error for UnitConversionError {}

/// Convert an **absolute** quantity's value from `unit` to its dimension's
/// canonical unit (RFC-0004 §7: applies the affine offset, if any).
pub fn to_canonical_absolute(value: f64, unit: &UnitDef) -> f64 {
    let scaled = value * unit.scale_to_canonical;
    match unit.affine_offset {
        Some(offset) => scaled + offset,
        None => scaled,
    }
}

/// Convert a **delta**/difference quantity's value from `unit` to its
/// dimension's canonical unit (RFC-0004 §7: never applies the affine
/// offset — a 5°C difference converts to a 5K difference, not 278.15K).
pub fn to_canonical_delta(value: f64, unit: &UnitDef) -> f64 {
    value * unit.scale_to_canonical
}

/// Inverse of `to_canonical_absolute`.
pub fn from_canonical_absolute(canonical_value: f64, unit: &UnitDef) -> f64 {
    let shifted = match unit.affine_offset {
        Some(offset) => canonical_value - offset,
        None => canonical_value,
    };
    shifted / unit.scale_to_canonical
}

/// Inverse of `to_canonical_delta`.
pub fn from_canonical_delta(canonical_value: f64, unit: &UnitDef) -> f64 {
    canonical_value / unit.scale_to_canonical
}

/// Convert an **absolute** quantity's value from one unit to another,
/// requiring both to belong to the same named `Dimension`.
pub fn convert_absolute(
    value: f64,
    from: &UnitDef,
    to: &UnitDef,
) -> Result<f64, UnitConversionError> {
    if from.dimension != to.dimension {
        return Err(UnitConversionError {
            from: from.dimension,
            to: to.dimension,
        });
    }
    Ok(from_canonical_absolute(
        to_canonical_absolute(value, from),
        to,
    ))
}

/// Convert a **delta**/difference quantity's value from one unit to
/// another, requiring both to belong to the same named `Dimension`.
pub fn convert_delta(value: f64, from: &UnitDef, to: &UnitDef) -> Result<f64, UnitConversionError> {
    if from.dimension != to.dimension {
        return Err(UnitConversionError {
            from: from.dimension,
            to: to.dimension,
        });
    }
    Ok(from_canonical_delta(to_canonical_delta(value, from), to))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn every_rfc_0004_section_4_symbol_is_registered() {
        let length: [&str; 8] = ["nm", "um", "mm", "cm", "m", "km", "in", "ft"];
        for symbol in length {
            assert!(
                lookup(symbol, Dimension::Length).is_some(),
                "missing length unit {symbol}"
            );
        }
        for symbol in ["deg", "rad"] {
            assert!(
                lookup(symbol, Dimension::Angle).is_some(),
                "missing angle unit {symbol}"
            );
        }
        for symbol in ["mg", "g", "kg", "lbm"] {
            assert!(
                lookup(symbol, Dimension::Mass).is_some(),
                "missing mass unit {symbol}"
            );
        }
        for symbol in ["N", "kN", "lbf"] {
            assert!(
                lookup(symbol, Dimension::Force).is_some(),
                "missing force unit {symbol}"
            );
        }
        for symbol in ["Pa", "kPa", "MPa", "GPa", "psi", "ksi"] {
            assert!(
                lookup(symbol, Dimension::Pressure).is_some(),
                "missing pressure unit {symbol}"
            );
            assert!(
                lookup(symbol, Dimension::Stress).is_some(),
                "missing stress unit {symbol}"
            );
        }
        for symbol in ["K", "degC", "degF"] {
            assert!(
                lookup(symbol, Dimension::Temperature).is_some(),
                "missing temperature unit {symbol}"
            );
        }
    }

    #[test]
    fn lookup_rejects_symbol_dimension_mismatch() {
        assert!(lookup("mm", Dimension::Mass).is_none());
        assert!(lookup("kg", Dimension::Length).is_none());
        assert!(lookup("not-a-unit", Dimension::Length).is_none());
    }

    #[test]
    fn lookup_any_surfaces_the_pressure_stress_multiplicity() {
        let matches: Vec<Dimension> = lookup_any("Pa").map(|u| u.dimension).collect();
        assert_eq!(matches.len(), 2, "{matches:?}");
        assert!(matches.contains(&Dimension::Pressure));
        assert!(matches.contains(&Dimension::Stress));
    }

    #[test]
    fn lookup_any_returns_exactly_one_for_unambiguous_symbols() {
        let matches: Vec<Dimension> = lookup_any("mm").map(|u| u.dimension).collect();
        assert_eq!(matches, vec![Dimension::Length]);
    }

    #[test]
    fn length_round_trip_mm_to_m_and_back() {
        let mm = lookup("mm", Dimension::Length).unwrap();
        let m = lookup("m", Dimension::Length).unwrap();
        let in_m = convert_absolute(1500.0, mm, m).unwrap();
        assert!(approx_eq(in_m, 1.5, 1e-12), "{in_m}");
        let back = convert_absolute(in_m, m, mm).unwrap();
        assert!(approx_eq(back, 1500.0, 1e-9), "{back}");
    }

    #[test]
    fn inches_and_feet_known_values() {
        let inch = lookup("in", Dimension::Length).unwrap();
        let m = lookup("m", Dimension::Length).unwrap();
        assert!(approx_eq(
            convert_absolute(1.0, inch, m).unwrap(),
            0.0254,
            1e-15
        ));
        let ft = lookup("ft", Dimension::Length).unwrap();
        let in_per_ft = convert_absolute(1.0, ft, inch).unwrap();
        assert!(approx_eq(in_per_ft, 12.0, 1e-9), "{in_per_ft}");
    }

    #[test]
    fn force_lbf_known_value() {
        let lbf = lookup("lbf", Dimension::Force).unwrap();
        let n = lookup("N", Dimension::Force).unwrap();
        let newtons = convert_absolute(1.0, lbf, n).unwrap();
        // Standard published value: 1 lbf = 4.4482216152605 N.
        assert!(approx_eq(newtons, 4.448_221_615_260_5, 1e-9), "{newtons}");
    }

    #[test]
    fn psi_and_ksi_known_relationship() {
        let psi = lookup("psi", Dimension::Pressure).unwrap();
        let ksi = lookup("ksi", Dimension::Pressure).unwrap();
        let pa = lookup("Pa", Dimension::Pressure).unwrap();
        let psi_in_pa = convert_absolute(1.0, psi, pa).unwrap();
        // Standard published value: 1 psi = 6894.757293168... Pa.
        assert!(approx_eq(psi_in_pa, 6894.757293168, 1e-6), "{psi_in_pa}");
        let ksi_in_psi = convert_absolute(1.0, ksi, psi).unwrap();
        assert!(approx_eq(ksi_in_psi, 1000.0, 1e-9), "{ksi_in_psi}");
    }

    #[test]
    fn temperature_absolute_freezing_and_boiling_points() {
        let degc = lookup("degC", Dimension::Temperature).unwrap();
        let degf = lookup("degF", Dimension::Temperature).unwrap();
        let k = lookup("K", Dimension::Temperature).unwrap();

        assert!(approx_eq(
            convert_absolute(0.0, degc, k).unwrap(),
            273.15,
            1e-9
        ));
        assert!(approx_eq(
            convert_absolute(100.0, degc, k).unwrap(),
            373.15,
            1e-9
        ));
        // Freezing/boiling points of water in Fahrenheit.
        assert!(approx_eq(
            convert_absolute(32.0, degf, k).unwrap(),
            273.15,
            1e-9
        ));
        assert!(approx_eq(
            convert_absolute(212.0, degf, k).unwrap(),
            373.15,
            1e-9
        ));
        // Cross-check: 20 degC == 68 degF (well-known reference point).
        let twenty_c_in_f = convert_absolute(20.0, degc, degf).unwrap();
        assert!(approx_eq(twenty_c_in_f, 68.0, 1e-9), "{twenty_c_in_f}");
    }

    #[test]
    fn temperature_delta_never_applies_the_affine_offset() {
        let degc = lookup("degC", Dimension::Temperature).unwrap();
        let degf = lookup("degF", Dimension::Temperature).unwrap();
        let k = lookup("K", Dimension::Temperature).unwrap();

        // RFC-0004 §7's own example: a 5degC *difference* is a 5K
        // difference, not 278.15K.
        assert!(approx_eq(convert_delta(5.0, degc, k).unwrap(), 5.0, 1e-12));
        // A 9 degF difference is a 5K difference (5/9 scale, no offset).
        assert!(approx_eq(convert_delta(9.0, degf, k).unwrap(), 5.0, 1e-9));

        // The core affine invariant (RFC-0004 §7): absolute and delta
        // conversions of the same source value differ for an affine unit.
        let absolute = convert_absolute(20.0, degc, k).unwrap();
        let delta = convert_delta(20.0, degc, k).unwrap();
        assert!(
            !approx_eq(absolute, delta, 1e-6),
            "absolute={absolute} delta={delta}"
        );
        assert!(approx_eq(absolute, 293.15, 1e-9));
        assert!(approx_eq(delta, 20.0, 1e-9));
    }

    #[test]
    fn non_affine_units_have_identical_absolute_and_delta_conversion() {
        let mm = lookup("mm", Dimension::Length).unwrap();
        let m = lookup("m", Dimension::Length).unwrap();
        let absolute = convert_absolute(1500.0, mm, m).unwrap();
        let delta = convert_delta(1500.0, mm, m).unwrap();
        assert_eq!(absolute, delta);
    }

    #[test]
    fn conversion_rejects_different_physical_dimensions() {
        let mm = lookup("mm", Dimension::Length).unwrap();
        let kg = lookup("kg", Dimension::Mass).unwrap();
        let err = convert_absolute(1.0, mm, kg).unwrap_err();
        assert_eq!(
            err,
            UnitConversionError {
                from: Dimension::Length,
                to: Dimension::Mass
            }
        );
    }

    #[test]
    fn conversion_rejects_pressure_to_stress_despite_identical_units() {
        // AICAD-047's own finding: Pressure and Stress share a
        // DimensionVector, but they are still distinct named dimensions —
        // this registry must not treat "same unit spelling and scale" as
        // "convertible."
        let pa_pressure = lookup("Pa", Dimension::Pressure).unwrap();
        let pa_stress = lookup("Pa", Dimension::Stress).unwrap();
        assert_eq!(pa_pressure.scale_to_canonical, pa_stress.scale_to_canonical);
        let err = convert_absolute(1.0, pa_pressure, pa_stress).unwrap_err();
        assert_eq!(
            err,
            UnitConversionError {
                from: Dimension::Pressure,
                to: Dimension::Stress
            }
        );
    }

    #[test]
    fn error_display_names_both_dimensions() {
        let err = UnitConversionError {
            from: Dimension::Length,
            to: Dimension::Mass,
        };
        assert_eq!(
            err.to_string(),
            "cannot convert between different dimensions: Length and Mass"
        );
    }
}
