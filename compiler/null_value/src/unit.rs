//! Units: which a property takes, how a written value becomes its stored bits, and back.

use std::fmt;

use crate::exact;
use crate::number::{Decimal, Float, Number, scan};

/// What a property measures, from its RNA unit: which units it takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dimension {
    /// No unit: a bare number only (a count, a factor, a colour channel).
    None,
    /// A length, stored in metres (Blender units at unit scale 1).
    Length,
    /// An angle, stored in radians.
    Angle,
}

impl Dimension {
    /// The units a value of this dimension may be written in.
    pub fn units(self) -> &'static [Unit] {
        match self {
            Dimension::None => &[],
            Dimension::Length => &[Unit::M, Unit::Cm, Unit::Mm, Unit::Km, Unit::In, Unit::Ft],
            Dimension::Angle => &[Unit::Rad, Unit::Deg],
        }
    }
}

/// A unit a value may be written in (spec, section 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Unit {
    /// Metres, the internal unit of length.
    M,
    /// Centimetres: 10^-2 m.
    Cm,
    /// Millimetres: 10^-3 m.
    Mm,
    /// Kilometres: 10^3 m.
    Km,
    /// Inches: exactly 0.0254 m.
    In,
    /// Feet: exactly 0.3048 m.
    Ft,
    /// Radians, the internal unit of angle.
    Rad,
    /// Degrees: π/180 rad.
    Deg,
}

impl Unit {
    /// Every unit.
    pub const ALL: [Unit; 8] = [
        Unit::M,
        Unit::Cm,
        Unit::Mm,
        Unit::Km,
        Unit::In,
        Unit::Ft,
        Unit::Rad,
        Unit::Deg,
    ];

    /// How it is written after the number.
    pub fn suffix(self) -> &'static str {
        match self {
            Unit::M => "m",
            Unit::Cm => "cm",
            Unit::Mm => "mm",
            Unit::Km => "km",
            Unit::In => "in",
            Unit::Ft => "ft",
            Unit::Rad => "rad",
            Unit::Deg => "deg",
        }
    }

    /// What it measures.
    pub fn dimension(self) -> Dimension {
        match self {
            Unit::M | Unit::Cm | Unit::Mm | Unit::Km | Unit::In | Unit::Ft => Dimension::Length,
            Unit::Rad | Unit::Deg => Dimension::Angle,
        }
    }

    /// The unit written as `suffix`.
    pub fn from_suffix(suffix: &str) -> Option<Unit> {
        Unit::ALL.into_iter().find(|u| u.suffix() == suffix)
    }

    /// A value in this unit, as an exact decimal in the internal unit, where a
    /// decimal shift or a whole multiple gives it; `None` for degrees.
    fn to_internal(self, value: &Decimal) -> Option<Decimal> {
        Some(match self {
            Unit::M | Unit::Rad => value.clone(),
            Unit::Cm => value.shifted(-2),
            Unit::Mm => value.shifted(-3),
            Unit::Km => value.shifted(3),
            Unit::In => value.times(254).shifted(-4),
            Unit::Ft => value.times(3048).shifted(-4),
            Unit::Deg => return None,
        })
    }

    /// One unit in the internal unit, approximately, to find candidates for printing.
    fn approx_factor(self) -> f64 {
        match self {
            Unit::M | Unit::Rad => 1.0,
            Unit::Cm => 0.01,
            Unit::Mm => 0.001,
            Unit::Km => 1000.0,
            Unit::In => 0.0254,
            Unit::Ft => 0.3048,
            Unit::Deg => std::f64::consts::PI / 180.0,
        }
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.suffix())
    }
}

/// Why a word is not a value of the dimension asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadError {
    /// The word does not start with a number.
    NotANumber,
    /// The number is followed by something that is not a unit.
    UnknownUnit {
        /// What followed the number.
        unit: String,
        /// The closest unit the dimension takes, if one is close.
        suggestion: Option<Unit>,
    },
    /// The unit is real but the property cannot take it (a length written in `deg`).
    WrongUnit {
        /// The unit written.
        unit: Unit,
        /// What the property measures.
        dimension: Dimension,
    },
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let takes = |f: &mut fmt::Formatter<'_>, d: Dimension| {
            let units = d.units();
            if units.is_empty() {
                write!(f, "this value takes a bare number, with no unit")
            } else {
                let names: Vec<_> = units.iter().map(|u| u.suffix()).collect();
                write!(f, "this value takes {} or a bare number", names.join(", "))
            }
        };
        match self {
            ReadError::NotANumber => write!(f, "expected a number"),
            ReadError::UnknownUnit { unit, suggestion } => {
                write!(f, "`{unit}` is not a unit")?;
                if let Some(s) = suggestion {
                    write!(f, "; did you mean `{s}`?")?;
                }
                Ok(())
            }
            ReadError::WrongUnit { unit, dimension } => {
                write!(f, "`{unit}` does not fit here: ")?;
                takes(f, *dimension)
            }
        }
    }
}

impl std::error::Error for ReadError {}

/// Reads `word` as a value of `dimension` stored as `F`: a bare number in the internal
/// unit, or a number and a unit the dimension takes. The exact written value is
/// rounded once, to nearest even, into `F` (spec, section 6; Sau's decisions S21 and
/// S22). Lengths are read at unit scale 1.
pub fn read<F: Float>(word: &str, dimension: Dimension) -> Result<F, ReadError> {
    let (number, used) = scan(word).ok_or(ReadError::NotANumber)?;
    let suffix = &word[used..];
    let unit = if suffix.is_empty() {
        None
    } else {
        let unit = Unit::from_suffix(suffix).ok_or_else(|| ReadError::UnknownUnit {
            unit: suffix.to_string(),
            suggestion: closest_unit(suffix, dimension),
        })?;
        if unit.dimension() != dimension {
            return Err(ReadError::WrongUnit { unit, dimension });
        }
        Some(unit)
    };
    Ok(match number {
        Number::NaN => F::nan(),
        Number::Infinite { negative } => F::infinity(negative),
        Number::Finite(value) => match unit {
            None => value.round(),
            Some(Unit::Deg) => exact::degrees(&value),
            Some(u) => u.to_internal(&value).expect("a decimal unit").round(),
        },
    })
}

/// The unit of `dimension` nearest to `text` by edit distance, if within two edits.
fn closest_unit(text: &str, dimension: Dimension) -> Option<Unit> {
    dimension
        .units()
        .iter()
        .map(|&u| (edit_distance(text, u.suffix()), u))
        .filter(|&(d, _)| d <= 2)
        .min_by_key(|&(d, _)| d)
        .map(|(_, u)| u)
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

/// `value` in its internal unit as a bare number: the shortest decimal that reads
/// back to exactly its bits, in the canonical layout (`0.1`, `-0`, `1.5e-7`, `inf`,
/// `nan`).
pub fn print_bare<F: Float>(value: F) -> String {
    if value.is_nan() {
        "nan".to_string()
    } else if value.is_infinite() {
        if value.is_sign_negative() {
            "-inf"
        } else {
            "inf"
        }
        .to_string()
    } else {
        value.shortest().layout()
    }
}

/// `value` written in `unit`, if a decimal of at most 17 significant digits in that
/// unit reads back to exactly its bits: the shortest such decimal. Metres,
/// centimetres, millimetres and kilometres always succeed for finite values;
/// degrees, inches and feet may not. `None` for infinities and NaN.
pub fn print_in<F: Float>(value: F, unit: Unit) -> Option<String> {
    if value.is_nan() || value.is_infinite() {
        return None;
    }
    let suffix = unit.suffix();
    let shortest = value.shortest();
    let shift = match unit {
        Unit::M | Unit::Rad => Some(0),
        Unit::Cm => Some(2),
        Unit::Mm => Some(3),
        Unit::Km => Some(-3),
        _ => None,
    };
    if let Some(shift) = shift {
        return Some(format!("{}{suffix}", shortest.shifted(shift).layout()));
    }
    if shortest.is_zero() {
        return Some(format!("{}{suffix}", shortest.layout()));
    }
    let target = value.bits();
    let reads_back = |d: &Decimal| {
        read::<F>(&format!("{}{suffix}", d.layout()), unit.dimension())
            .is_ok_and(|v| v.bits() == target)
    };
    let approx = value.to_f64() / unit.approx_factor();
    if !approx.is_finite() || approx == 0.0 {
        return None;
    }
    for digits in 1..=17usize {
        let rounded = round_to_digits(approx, digits);
        let mantissa: u64 = rounded.digits.parse().unwrap_or(0);
        let exp = rounded.exp;
        let mut candidates = vec![Decimal::new(rounded.negative, &rounded.digits, exp)];
        for m in [mantissa.checked_sub(1), mantissa.checked_add(1)]
            .into_iter()
            .flatten()
        {
            candidates.push(Decimal::new(rounded.negative, &m.to_string(), exp));
        }
        if let Some(found) = candidates.iter().find(|d| reads_back(d)) {
            return Some(format!("{}{suffix}", found.layout()));
        }
    }
    None
}

/// `x` rounded to `digits` significant digits, as a decimal whose digit string has
/// exactly that many digits (trailing zeros kept, so neighbours are one apart).
fn round_to_digits(x: f64, digits: usize) -> Decimal {
    let sci = format!("{:.*e}", digits - 1, x);
    let (mantissa, exp) = sci.split_once('e').expect("an exponent");
    let exp: i64 = exp.parse().expect("a whole exponent");
    let negative = mantissa.starts_with('-');
    let mantissa = mantissa.trim_start_matches('-').replace('.', "");
    Decimal {
        negative,
        exp: exp - (mantissa.len() as i64 - 1),
        digits: mantissa,
    }
}

/// `value` in `display` where that reads back exactly, otherwise bare in the
/// internal unit (spec, section 6, the conversion rule).
pub fn print<F: Float>(value: F, display: Option<Unit>) -> String {
    display
        .and_then(|u| print_in(value, u))
        .unwrap_or_else(|| print_bare(value))
}
