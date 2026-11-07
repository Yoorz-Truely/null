//! Units: which a property takes, how a written value becomes its stored bits, and back.

use std::fmt;

use crate::exact;
use crate::number::{Decimal, Float, Number, scan};

/// What a property measures, from its RNA unit: which units it takes (spec, section 6, "Units").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dimension {
    /// No unit: a bare number only (a count, a factor, a colour channel, a frame).
    None,
    /// A length, stored in Blender units (metres at unit scale 1).
    Length,
    /// An area, in square Blender units.
    Area,
    /// A volume, in cubic Blender units.
    Volume,
    /// A mass, stored in kilograms.
    Mass,
    /// An angle, stored in radians.
    Angle,
    /// A time in seconds, whatever the scene's frame rate (RNA's absolute time).
    Seconds,
    /// A velocity, in Blender units per second.
    Velocity,
    /// An acceleration, in Blender units per second squared.
    Acceleration,
    /// A camera's focal length or sensor size, stored in millimetres (RNA's camera unit).
    Lens,
    /// Power, stored in watts (also a light's energy, a refinement of Null's: RNA gives none).
    Power,
    /// A temperature, stored in kelvin.
    Temperature,
    /// A wavelength, stored in nanometres.
    Wavelength,
    /// A colour temperature, stored in kelvin.
    ColorTemperature,
    /// A frequency, stored in hertz.
    Frequency,
}

impl Dimension {
    /// Every dimension.
    pub const ALL: [Dimension; 15] = [
        Dimension::None,
        Dimension::Length,
        Dimension::Area,
        Dimension::Volume,
        Dimension::Mass,
        Dimension::Angle,
        Dimension::Seconds,
        Dimension::Velocity,
        Dimension::Acceleration,
        Dimension::Lens,
        Dimension::Power,
        Dimension::Temperature,
        Dimension::Wavelength,
        Dimension::ColorTemperature,
        Dimension::Frequency,
    ];

    /// The units a value of this dimension may be written in, the internal one first.
    pub fn units(self) -> &'static [Unit] {
        UNITS[self as usize].as_slice()
    }

    fn defs(self) -> &'static [Def] {
        DEFS[self as usize]
    }
}

/// How a unit's value becomes the internal unit's: `value × 10^k`, `value × n / d`, `value ×
/// π/180`, or `(value + a/b) × n/d`, each exactly.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Conv {
    Shift(i64),
    Ratio(u128, u128),
    Pi,
    Affine {
        offset: (u128, u128),
        factor: (u128, u128),
    },
}

#[derive(Clone, Copy, Debug)]
struct Def {
    suffix: &'static str,
    conv: Conv,
}

const fn d(suffix: &'static str, conv: Conv) -> Def {
    Def { suffix, conv }
}

use Conv::{Affine, Pi, Ratio, Shift};

// Exact factors: the inch is 0.0254 m, the foot 0.3048 m, the yard 0.9144 m, the mile
// 1609.344 m, the thou 0.0000254 m; the pound 0.45359237 kg (Blender's values, which are the
// international definitions).
const MI: u128 = 1_609_344; // ÷ 10^3
const YD: u128 = 9_144; // ÷ 10^4
const FT: u128 = 3_048; // ÷ 10^4
const IN: u128 = 254; // ÷ 10^4
const THOU: u128 = 254; // ÷ 10^7
const fn p10(k: u32) -> u128 {
    10u128.pow(k)
}

const NONE: &[Def] = &[];
const LENGTH: &[Def] = &[
    d("m", Shift(0)),
    d("cm", Shift(-2)),
    d("mm", Shift(-3)),
    d("km", Shift(3)),
    d("um", Shift(-6)),
    d("nm", Shift(-9)),
    d("pm", Shift(-12)),
    d("mi", Ratio(MI, p10(3))),
    d("yd", Ratio(YD, p10(4))),
    d("ft", Ratio(FT, p10(4))),
    d("in", Ratio(IN, p10(4))),
    d("thou", Ratio(THOU, p10(7))),
];
const AREA: &[Def] = &[
    d("m2", Shift(0)),
    d("cm2", Shift(-4)),
    d("mm2", Shift(-6)),
    d("km2", Shift(6)),
    d("um2", Shift(-12)),
    d("nm2", Shift(-18)),
    d("pm2", Shift(-24)),
    d("mi2", Ratio(MI * MI, p10(6))),
    d("yd2", Ratio(YD * YD, p10(8))),
    d("ft2", Ratio(FT * FT, p10(8))),
    d("in2", Ratio(IN * IN, p10(8))),
    d("thou2", Ratio(THOU * THOU, p10(14))),
];
const VOLUME: &[Def] = &[
    d("m3", Shift(0)),
    d("cm3", Shift(-6)),
    d("mm3", Shift(-9)),
    d("km3", Shift(9)),
    d("um3", Shift(-18)),
    d("nm3", Shift(-27)),
    d("pm3", Shift(-36)),
    d("mi3", Ratio(MI * MI * MI, p10(9))),
    d("yd3", Ratio(YD * YD * YD, p10(12))),
    d("ft3", Ratio(FT * FT * FT, p10(12))),
    d("in3", Ratio(IN * IN * IN, p10(12))),
    d("thou3", Ratio(THOU * THOU * THOU, p10(21))),
];
const MASS: &[Def] = &[
    d("kg", Shift(0)),
    d("g", Shift(-3)),
    d("mg", Shift(-6)),
    d("t", Shift(3)),
    d("lb", Ratio(45_359_237, p10(8))),
    d("oz", Ratio(28_349_523_125, p10(12))),
    d("st", Ratio(635_029_318, p10(8))),
    d("cwt", Ratio(45_359_237, p10(6))),
    d("tn", Ratio(90_718_474, p10(5))),
];
const ANGLE: &[Def] = &[d("rad", Shift(0)), d("deg", Pi)];
const SECONDS: &[Def] = &[
    d("s", Shift(0)),
    d("ms", Shift(-3)),
    d("us", Shift(-6)),
    d("min", Ratio(60, 1)),
    d("h", Ratio(3_600, 1)),
    d("d", Ratio(86_400, 1)),
];
const VELOCITY: &[Def] = &[
    d("m/s", Shift(0)),
    d("km/h", Ratio(5, 18)),
    d("ft/s", Ratio(FT, p10(4))),
    d("mph", Ratio(44_704, p10(5))),
];
const ACCELERATION: &[Def] = &[d("m/s2", Shift(0)), d("ft/s2", Ratio(FT, p10(4)))];
const LENS: &[Def] = &[
    d("mm", Shift(0)),
    d("cm", Shift(1)),
    d("m", Shift(3)),
    d("um", Shift(-3)),
];
const POWER: &[Def] = &[
    d("W", Shift(0)),
    d("kW", Shift(3)),
    d("MW", Shift(6)),
    d("GW", Shift(9)),
    d("mW", Shift(-3)),
    d("uW", Shift(-6)),
    d("nW", Shift(-9)),
];
const TEMPERATURE: &[Def] = &[
    d("K", Shift(0)),
    d(
        "C",
        Affine {
            offset: (27_315, 100),
            factor: (1, 1),
        },
    ),
    d(
        "F",
        Affine {
            offset: (45_967, 100),
            factor: (5, 9),
        },
    ),
];
const WAVELENGTH: &[Def] = &[
    d("nm", Shift(0)),
    d("um", Shift(3)),
    d("mm", Shift(6)),
    d("pm", Shift(-3)),
];
const COLOR_TEMPERATURE: &[Def] = &[d("K", Shift(0))];
const FREQUENCY: &[Def] = &[d("Hz", Shift(0)), d("kHz", Shift(3))];

/// Each dimension's units, in the order of [`Dimension::ALL`].
const DEFS: [&[Def]; 15] = [
    NONE,
    LENGTH,
    AREA,
    VOLUME,
    MASS,
    ANGLE,
    SECONDS,
    VELOCITY,
    ACCELERATION,
    LENS,
    POWER,
    TEMPERATURE,
    WAVELENGTH,
    COLOR_TEMPERATURE,
    FREQUENCY,
];

static UNITS: std::sync::LazyLock<[Vec<Unit>; 15]> = std::sync::LazyLock::new(|| {
    Dimension::ALL.map(|dim| {
        (0..dim.defs().len())
            .map(|i| Unit {
                dim,
                index: i as u8,
            })
            .collect()
    })
});

/// A unit a value may be written in (spec, section 6, "Units"): a dimension's word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Unit {
    dim: Dimension,
    index: u8,
}

#[allow(non_upper_case_globals)]
impl Unit {
    /// Metres, the internal unit of length.
    pub const M: Unit = Unit {
        dim: Dimension::Length,
        index: 0,
    };
    /// Centimetres: 10^-2 m.
    pub const Cm: Unit = Unit {
        dim: Dimension::Length,
        index: 1,
    };
    /// Millimetres: 10^-3 m.
    pub const Mm: Unit = Unit {
        dim: Dimension::Length,
        index: 2,
    };
    /// Kilometres: 10^3 m.
    pub const Km: Unit = Unit {
        dim: Dimension::Length,
        index: 3,
    };
    /// Inches: exactly 0.0254 m.
    pub const In: Unit = Unit {
        dim: Dimension::Length,
        index: 10,
    };
    /// Feet: exactly 0.3048 m.
    pub const Ft: Unit = Unit {
        dim: Dimension::Length,
        index: 9,
    };
    /// Radians, the internal unit of angle.
    pub const Rad: Unit = Unit {
        dim: Dimension::Angle,
        index: 0,
    };
    /// Degrees: π/180 rad.
    pub const Deg: Unit = Unit {
        dim: Dimension::Angle,
        index: 1,
    };
    /// Millimetres of focal length, the internal unit of a camera's lens: `50mm` is 50.
    pub const LensMm: Unit = Unit {
        dim: Dimension::Lens,
        index: 0,
    };
    /// Watts, the internal unit of power.
    pub const W: Unit = Unit {
        dim: Dimension::Power,
        index: 0,
    };

    /// Every unit, dimension by dimension.
    pub fn all() -> impl Iterator<Item = Unit> {
        Dimension::ALL
            .into_iter()
            .flat_map(|d| d.units().iter().copied())
    }

    fn def(self) -> &'static Def {
        &self.dim.defs()[self.index as usize]
    }

    /// How it is written after the number.
    pub fn suffix(self) -> &'static str {
        self.def().suffix
    }

    /// What it measures.
    pub fn dimension(self) -> Dimension {
        self.dim
    }

    pub(crate) fn conv(self) -> Conv {
        self.def().conv
    }

    /// The unit written as `suffix`, the one of `dimension` first (`mm` is a length, and a
    /// camera's lens in millimetres, and a wavelength).
    pub fn from_suffix(suffix: &str, dimension: Dimension) -> Option<Unit> {
        let mut units = dimension.units().iter().copied().chain(Unit::all());
        units.find(|u| u.suffix() == suffix)
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
    /// The number is not a whole number, or is past the range of 64-bit integers.
    NotAnInteger,
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
            ReadError::NotAnInteger => write!(f, "expected a whole number"),
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
        let unit = Unit::from_suffix(suffix, dimension).ok_or_else(|| ReadError::UnknownUnit {
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
        Number::Finite(value) => match unit.map(Unit::conv) {
            None => value.round(),
            Some(Shift(k)) => value.shifted(k).round(),
            Some(Pi) => exact::degrees(&value),
            Some(Ratio(n, d)) => exact::ratio(&value, n, d),
            Some(Affine { offset, factor }) => exact::affine(&value, offset, factor),
        },
    })
}

/// The unit `word` is written in, if it is a number followed by a unit of any kind
/// (`mm` read as `dimension`'s where both have one). `None` for a bare number.
pub fn unit_of(word: &str, dimension: Dimension) -> Option<Unit> {
    let (_, used) = scan(word)?;
    let suffix = &word[used..];
    (!suffix.is_empty())
        .then(|| Unit::from_suffix(suffix, dimension))
        .flatten()
}

/// Reads `word` as a whole number with no unit, exactly (DNA's integer properties, and
/// frames, which are bare numbers). `1e3` is a whole number; `1.5` and `inf` are not.
pub fn read_int(word: &str) -> Result<i64, ReadError> {
    let (number, used) = scan(word).ok_or(ReadError::NotANumber)?;
    let suffix = &word[used..];
    if !suffix.is_empty() {
        return Err(match Unit::from_suffix(suffix, Dimension::None) {
            Some(unit) => ReadError::WrongUnit {
                unit,
                dimension: Dimension::None,
            },
            None => ReadError::UnknownUnit {
                unit: suffix.to_string(),
                suggestion: None,
            },
        });
    }
    let Number::Finite(value) = number else {
        return Err(ReadError::NotAnInteger);
    };
    if value.is_zero() {
        return Ok(0);
    }
    if value.exp < 0 || value.leading_exp() > 18 {
        return Err(ReadError::NotAnInteger);
    }
    let digits = format!("{}{}", value.digits, "0".repeat(value.exp as usize));
    let magnitude: i128 = digits.parse().map_err(|_| ReadError::NotAnInteger)?;
    let signed = if value.negative {
        -magnitude
    } else {
        magnitude
    };
    i64::try_from(signed).map_err(|_| ReadError::NotAnInteger)
}

/// The unit of `dimension` nearest to `text`: one that differs only in case (`MM` is
/// `mm`), else the nearest by edit distance within two edits.
fn closest_unit(text: &str, dimension: Dimension) -> Option<Unit> {
    if let Some(&u) = dimension
        .units()
        .iter()
        .find(|u| u.suffix().eq_ignore_ascii_case(text))
    {
        return Some(u);
    }
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
/// unit reads back to exactly its bits: the shortest such decimal. A unit that is a
/// decimal shift of the internal one (centimetres, watts) always succeeds for finite values;
/// degrees, inches, feet and the rest may not. `None` for infinities and NaN.
pub fn print_in<F: Float>(value: F, unit: Unit) -> Option<String> {
    if value.is_nan() || value.is_infinite() {
        return None;
    }
    let suffix = unit.suffix();
    let shortest = value.shortest();
    let conv = unit.conv();
    if let Shift(k) = conv {
        return Some(format!("{}{suffix}", shortest.shifted(-k).layout()));
    }
    if shortest.is_zero() && !matches!(conv, Affine { .. }) {
        return Some(format!("{}{suffix}", shortest.layout()));
    }
    let target = value.bits();
    let reads_back = |d: &Decimal| {
        read::<F>(&format!("{}{suffix}", d.layout()), unit.dimension())
            .is_ok_and(|v| v.bits() == target)
    };
    // The value in this unit to about 40 digits, exactly derived, so rounding it to
    // each length finds the decimal that reads back (the 1a-04 review found an f64
    // estimate missing some at 16 and 17 digits).
    let precise = exact::in_unit(value, conv)?;
    for digits in 1..=17usize {
        let rounded = round_decimal(&precise, digits);
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

/// `d` rounded half up to `digits` significant digits, as a decimal whose digit
/// string has exactly that many digits (trailing zeros kept, so neighbours are one apart).
fn round_decimal(d: &Decimal, digits: usize) -> Decimal {
    let all = d.digits.as_bytes();
    let mut kept: Vec<u8> = all.iter().take(digits).copied().collect();
    while kept.len() < digits {
        kept.push(b'0');
    }
    let mut exp = d.exp + all.len() as i64 - digits as i64;
    if all.get(digits).is_some_and(|&b| b >= b'5') {
        let mut i = kept.len();
        loop {
            if i == 0 {
                kept.insert(0, b'1');
                kept.pop();
                exp += 1;
                break;
            }
            i -= 1;
            if kept[i] == b'9' {
                kept[i] = b'0';
            } else {
                kept[i] += 1;
                break;
            }
        }
    }
    Decimal {
        negative: d.negative,
        digits: String::from_utf8(kept).expect("ASCII digits"),
        exp,
    }
}

/// `value` in `display` where that reads back exactly, otherwise bare in the
/// internal unit (spec, section 6, the conversion rule).
pub fn print<F: Float>(value: F, display: Option<Unit>) -> String {
    display
        .and_then(|u| print_in(value, u))
        .unwrap_or_else(|| print_bare(value))
}
