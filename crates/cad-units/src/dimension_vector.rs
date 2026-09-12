//! `DimensionVector` — RFC-0004 §5's structural canonicalization: "derived
//! dimensions are canonicalized structurally, by dimension exponents...
//! not by how a unit is spelled in source."
//!
//! ## Base axes
//!
//! Six orthogonal base exponents are enough to express every one of
//! `cad_types::Dimension`'s 21 minimum first-class dimensions
//! structurally: length (L), mass (M), time (T), temperature (Θ), angle
//! (A), and electric current (I). Angle is kept as its own axis — *not*
//! folded into "dimensionless" the way plain SI treats radians — precisely
//! so `AngularVelocity` (`Angle^1 * Time^-1`) is structurally
//! distinguishable from `Frequency` (`Time^-1`): RFC-0004 §3 lists both as
//! separate first-class dimensions, and a plain-SI encoding (angle
//! dimensionless) would collapse them to an identical vector.
//!
//! ## Known, intentional multiplicity: this is *not* a 1:1 mapping
//!
//! Some of RFC-0004 §3's named dimensions share an identical base-exponent
//! vector under ordinary physics:
//! - `Pressure` and `Stress` are both `Force / Area`. RFC-0004 §4 itself
//!   confirms this is expected, not an oversight — it lists one shared
//!   unit set, "Pa kPa MPa GPa psi ksi", for both under one heading,
//!   "Pressure/stress".
//! - `Torque` and `Energy` are both `Force * Length` (N·m physically
//!   equals J; torque and energy differ by convention/vector nature, not
//!   by scalar dimensional analysis).
//!
//! RFC-0004 §3 nonetheless keeps all four as distinct named dimensions —
//! engineers treat "a pressure" and "a stress" (or "a torque" and "an
//! energy") as meaningfully different concepts despite sharing SI base
//! dimensions.
//!
//! Consequently `DimensionVector` cannot be used to *infer* a unique named
//! `Dimension` from raw exponents alone (a `Force * Length` vector does
//! not by itself say whether the source meant `Torque` or `Energy`), and
//! this module does not attempt to — there is no `DimensionVector ->
//! Dimension` reverse lookup here. What it does provide is *compatibility
//! verification against an expected dimension*, which is well-defined even
//! when several named dimensions would match the same bare vector:
//! RFC-0004's own worked example (`let area: Area = width * height;`)
//! always has a target type from an annotation, parameter type, or return
//! type to check a computed vector against
//! (`DimensionVector::of(Dimension::Area) == computed_vector`). Deciding
//! what an *unannotated* expression like `force * length` type-checks as
//! (if anything, absent a target) is `AICAD-049`'s "dimensional arithmetic
//! type rules" — deliberately not decided here.

use cad_types::Dimension;
use std::fmt;
use std::ops::{Div, Mul};

/// Exponents of the six base physical axes (length, mass, time,
/// temperature, angle, current) that together express every
/// `cad_types::Dimension` structurally (see module doc comment).
///
/// All of RFC-0004 §3's minimum dimensions have small integer exponents —
/// no roots or fractional powers appear anywhere in the frozen list (the
/// largest magnitude used is `Volume`'s length exponent, 3) — so `i8` is
/// exact and comfortably wide enough.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DimensionVector {
    pub length: i8,
    pub mass: i8,
    pub time: i8,
    pub temperature: i8,
    pub angle: i8,
    pub current: i8,
}

impl DimensionVector {
    /// The dimensionless vector (all exponents zero) — e.g. a plain `Int`/
    /// `Float` count, or the result of a dimension divided by itself.
    pub const DIMENSIONLESS: DimensionVector = DimensionVector {
        length: 0,
        mass: 0,
        time: 0,
        temperature: 0,
        angle: 0,
        current: 0,
    };

    const fn new(length: i8, mass: i8, time: i8, temperature: i8, angle: i8, current: i8) -> Self {
        DimensionVector {
            length,
            mass,
            time,
            temperature,
            angle,
            current,
        }
    }

    /// The canonical structural vector for one of `cad_types::Dimension`'s
    /// 21 named dimensions (RFC-0004 §3) — a total, unambiguous, one-way
    /// mapping (see module doc comment for why the reverse direction is
    /// deliberately not provided).
    pub const fn of(dimension: Dimension) -> DimensionVector {
        match dimension {
            Dimension::Length => Self::new(1, 0, 0, 0, 0, 0),
            Dimension::Area => Self::new(2, 0, 0, 0, 0, 0),
            Dimension::Volume => Self::new(3, 0, 0, 0, 0, 0),
            Dimension::Angle => Self::new(0, 0, 0, 0, 1, 0),
            Dimension::Time => Self::new(0, 0, 1, 0, 0, 0),
            Dimension::Mass => Self::new(0, 1, 0, 0, 0, 0),
            Dimension::Temperature => Self::new(0, 0, 0, 1, 0, 0),
            // Force = Mass * Length / Time^2.
            Dimension::Force => Self::new(1, 1, -2, 0, 0, 0),
            // Torque = Force * Length. Shares a vector with Energy (below)
            // — see module doc comment.
            Dimension::Torque => Self::new(2, 1, -2, 0, 0, 0),
            // Pressure = Force / Area. Shares a vector with Stress
            // (below), per RFC-0004 §4's own shared "Pressure/stress"
            // unit heading.
            Dimension::Pressure => Self::new(-1, 1, -2, 0, 0, 0),
            Dimension::Stress => Self::new(-1, 1, -2, 0, 0, 0),
            // Energy = Force * Length. Shares a vector with Torque (above)
            // — see module doc comment.
            Dimension::Energy => Self::new(2, 1, -2, 0, 0, 0),
            // Power = Energy / Time.
            Dimension::Power => Self::new(2, 1, -3, 0, 0, 0),
            // Density = Mass / Volume.
            Dimension::Density => Self::new(-3, 1, 0, 0, 0, 0),
            // Velocity = Length / Time.
            Dimension::Velocity => Self::new(1, 0, -1, 0, 0, 0),
            // Acceleration = Length / Time^2.
            Dimension::Acceleration => Self::new(1, 0, -2, 0, 0, 0),
            // AngularVelocity = Angle / Time — distinguished from
            // Frequency (below) only by the angle exponent; see module
            // doc comment on why Angle is kept as its own axis.
            Dimension::AngularVelocity => Self::new(0, 0, -1, 0, 1, 0),
            // Frequency = 1 / Time.
            Dimension::Frequency => Self::new(0, 0, -1, 0, 0, 0),
            Dimension::Current => Self::new(0, 0, 0, 0, 0, 1),
            // Voltage = Power / Current.
            Dimension::Voltage => Self::new(2, 1, -3, 0, 0, -1),
            // Resistance = Voltage / Current.
            Dimension::Resistance => Self::new(2, 1, -3, 0, 0, -2),
        }
    }

    /// Whether this vector has every exponent zero (`DIMENSIONLESS`).
    pub const fn is_dimensionless(self) -> bool {
        self.length == 0
            && self.mass == 0
            && self.time == 0
            && self.temperature == 0
            && self.angle == 0
            && self.current == 0
    }

    /// Raise every exponent to `power` (e.g. squaring a `Length` vector
    /// yields `Area`'s vector). A private helper kept `pub(crate)`-free for
    /// now since no `AICAD-047` test or consumer needs a public API for it
    /// yet beyond the `Mul`/`Div` operators below — added the moment a
    /// real consumer needs `a^n` for `n != 1, -1` (e.g. `AICAD-049`'s
    /// exponent-typed generics, if any turn out to need it).
    #[allow(dead_code)]
    const fn powi(self, power: i8) -> DimensionVector {
        DimensionVector {
            length: self.length * power,
            mass: self.mass * power,
            time: self.time * power,
            temperature: self.temperature * power,
            angle: self.angle * power,
            current: self.current * power,
        }
    }
}

/// Combines two vectors as if multiplying the quantities they describe
/// (exponents add) — e.g. `DimensionVector::of(Length) *
/// DimensionVector::of(Length) == DimensionVector::of(Area)`.
impl Mul for DimensionVector {
    type Output = DimensionVector;

    fn mul(self, rhs: DimensionVector) -> DimensionVector {
        DimensionVector {
            length: self.length + rhs.length,
            mass: self.mass + rhs.mass,
            time: self.time + rhs.time,
            temperature: self.temperature + rhs.temperature,
            angle: self.angle + rhs.angle,
            current: self.current + rhs.current,
        }
    }
}

/// Combines two vectors as if dividing the quantities they describe
/// (exponents subtract) — e.g. `DimensionVector::of(Length) /
/// DimensionVector::of(Time) == DimensionVector::of(Velocity)`.
impl Div for DimensionVector {
    type Output = DimensionVector;

    fn div(self, rhs: DimensionVector) -> DimensionVector {
        DimensionVector {
            length: self.length - rhs.length,
            mass: self.mass - rhs.mass,
            time: self.time - rhs.time,
            temperature: self.temperature - rhs.temperature,
            angle: self.angle - rhs.angle,
            current: self.current - rhs.current,
        }
    }
}

impl fmt::Display for DimensionVector {
    /// Renders as an exponent product, e.g. `L^1 M^1 T^-2`, omitting
    /// zero-exponent axes (`Dimensionless` when every axis is zero). Not
    /// meant to match any source-level unit syntax — this is a debugging/
    /// diagnostic aid over the structural vector itself, not a unit
    /// literal (`crates/cad-units`'s unit registry, `AICAD-048`, owns
    /// actual unit-literal spelling).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_dimensionless() {
            return f.write_str("Dimensionless");
        }
        let axes: [(&str, i8); 6] = [
            ("L", self.length),
            ("M", self.mass),
            ("T", self.time),
            ("Θ", self.temperature),
            ("A", self.angle),
            ("I", self.current),
        ];
        let mut first = true;
        for (symbol, exponent) in axes {
            if exponent == 0 {
                continue;
            }
            if !first {
                f.write_str(" ")?;
            }
            write!(f, "{symbol}^{exponent}")?;
            first = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn of_is_total_over_every_named_dimension() {
        // No panic across all 21 variants confirms `of` is exhaustive
        // (the `match` has no wildcard arm, so this also doubles as a
        // compile-time exhaustiveness guard against a silently-missed
        // variant if `cad_types::Dimension` ever grows one).
        for d in Dimension::ALL {
            let _ = DimensionVector::of(d);
        }
    }

    #[test]
    fn length_area_volume_are_powers_of_length() {
        let l = DimensionVector::of(Dimension::Length);
        assert_eq!(l * l, DimensionVector::of(Dimension::Area));
        assert_eq!(l * l * l, DimensionVector::of(Dimension::Volume));
    }

    #[test]
    fn velocity_is_length_over_time() {
        let l = DimensionVector::of(Dimension::Length);
        let t = DimensionVector::of(Dimension::Time);
        assert_eq!(l / t, DimensionVector::of(Dimension::Velocity));
    }

    #[test]
    fn acceleration_is_velocity_over_time() {
        let v = DimensionVector::of(Dimension::Velocity);
        let t = DimensionVector::of(Dimension::Time);
        assert_eq!(v / t, DimensionVector::of(Dimension::Acceleration));
    }

    #[test]
    fn force_is_mass_times_acceleration() {
        let m = DimensionVector::of(Dimension::Mass);
        let a = DimensionVector::of(Dimension::Acceleration);
        assert_eq!(m * a, DimensionVector::of(Dimension::Force));
    }

    #[test]
    fn pressure_is_force_over_area() {
        let f = DimensionVector::of(Dimension::Force);
        let area = DimensionVector::of(Dimension::Area);
        assert_eq!(f / area, DimensionVector::of(Dimension::Pressure));
    }

    #[test]
    fn density_is_mass_over_volume() {
        let m = DimensionVector::of(Dimension::Mass);
        let v = DimensionVector::of(Dimension::Volume);
        assert_eq!(m / v, DimensionVector::of(Dimension::Density));
    }

    #[test]
    fn power_is_energy_over_time() {
        let e = DimensionVector::of(Dimension::Energy);
        let t = DimensionVector::of(Dimension::Time);
        assert_eq!(e / t, DimensionVector::of(Dimension::Power));
    }

    #[test]
    fn angular_velocity_is_angle_over_time() {
        let angle = DimensionVector::of(Dimension::Angle);
        let t = DimensionVector::of(Dimension::Time);
        assert_eq!(angle / t, DimensionVector::of(Dimension::AngularVelocity));
    }

    #[test]
    fn voltage_and_resistance_derive_from_power_and_current() {
        let power = DimensionVector::of(Dimension::Power);
        let current = DimensionVector::of(Dimension::Current);
        let voltage = power / current;
        assert_eq!(voltage, DimensionVector::of(Dimension::Voltage));
        assert_eq!(
            voltage / current,
            DimensionVector::of(Dimension::Resistance)
        );
    }

    #[test]
    fn angle_axis_distinguishes_angular_velocity_from_frequency() {
        // The exact collision this module's doc comment explains keeping
        // Angle as its own axis avoids: without it, both would reduce to
        // a bare Time^-1 vector.
        assert_ne!(
            DimensionVector::of(Dimension::AngularVelocity),
            DimensionVector::of(Dimension::Frequency)
        );
    }

    #[test]
    fn pressure_and_stress_intentionally_share_a_vector() {
        // Documented, expected multiplicity (RFC-0004 §4's shared
        // "Pressure/stress" unit heading) — not a bug, and specifically
        // why there is no `DimensionVector -> Dimension` reverse lookup.
        assert_eq!(
            DimensionVector::of(Dimension::Pressure),
            DimensionVector::of(Dimension::Stress)
        );
    }

    #[test]
    fn torque_and_energy_intentionally_share_a_vector() {
        assert_eq!(
            DimensionVector::of(Dimension::Torque),
            DimensionVector::of(Dimension::Energy)
        );
    }

    #[test]
    fn division_by_self_is_dimensionless() {
        for d in Dimension::ALL {
            let v = DimensionVector::of(d);
            assert!(
                (v / v).is_dimensionless(),
                "{d:?} / itself should be dimensionless"
            );
        }
        assert!(DimensionVector::DIMENSIONLESS.is_dimensionless());
    }

    #[test]
    fn mul_and_div_are_inverse() {
        let f = DimensionVector::of(Dimension::Force);
        let l = DimensionVector::of(Dimension::Length);
        assert_eq!((f * l) / l, f);
        assert_eq!((f / l) * l, f);
    }

    #[test]
    fn distinct_named_dimensions_can_still_compare_unequal() {
        // Sanity check that this isn't a degenerate all-equal type: most
        // pairs *don't* collide.
        assert_ne!(
            DimensionVector::of(Dimension::Length),
            DimensionVector::of(Dimension::Mass)
        );
        assert_ne!(
            DimensionVector::of(Dimension::Force),
            DimensionVector::of(Dimension::Energy)
        );
    }

    #[test]
    fn display_omits_zero_exponents() {
        assert_eq!(DimensionVector::DIMENSIONLESS.to_string(), "Dimensionless");
        assert_eq!(DimensionVector::of(Dimension::Length).to_string(), "L^1");
        assert_eq!(
            DimensionVector::of(Dimension::Frequency).to_string(),
            "T^-1"
        );
        assert_eq!(
            DimensionVector::of(Dimension::Force).to_string(),
            "L^1 M^1 T^-2"
        );
    }

    #[test]
    fn powi_matches_repeated_multiplication() {
        let l = DimensionVector::of(Dimension::Length);
        assert_eq!(l.powi(2), l * l);
        assert_eq!(l.powi(3), l * l * l);
        assert_eq!(l.powi(0), DimensionVector::DIMENSIONLESS);
        assert_eq!(l.powi(-1), DimensionVector::DIMENSIONLESS / l);
    }
}
