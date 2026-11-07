//! Decimal numbers as written, the two float types Blender stores, and the canonical layout.

use std::fmt::Debug;

/// How far a written power of ten is taken: far past where every float is zero or
/// infinite, and small enough that shifting it never overflows.
const EXP_LIMIT: i64 = 1_000_000_000;

/// A number as written, before it is rounded into a stored type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Number {
    /// A finite decimal.
    Finite(Decimal),
    /// `inf` or `-inf`.
    Infinite {
        /// Whether it is `-inf`.
        negative: bool,
    },
    /// `nan`.
    NaN,
}

/// A finite decimal: `digits` × 10^`exp`, exactly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decimal {
    /// Whether a `-` was written (so `-0` keeps its sign).
    pub negative: bool,
    /// The significant digits in ASCII, with no leading or trailing zeros; empty for zero.
    pub digits: String,
    /// The power of ten of the last digit.
    pub exp: i64,
}

impl Decimal {
    /// A decimal from digits that may have leading and trailing zeros.
    pub fn new(negative: bool, digits: &str, exp: i64) -> Decimal {
        let digits = digits.trim_start_matches('0');
        let trimmed = digits.trim_end_matches('0');
        let exp = if trimmed.is_empty() {
            0
        } else {
            exp.saturating_add((digits.len() - trimmed.len()) as i64)
        };
        Decimal {
            negative,
            digits: trimmed.to_string(),
            exp: exp.clamp(-EXP_LIMIT, EXP_LIMIT),
        }
    }

    /// Whether it is zero (of either sign).
    pub fn is_zero(&self) -> bool {
        self.digits.is_empty()
    }

    /// The power of ten of the first digit (the exponent in scientific notation).
    pub fn leading_exp(&self) -> i64 {
        self.exp + self.digits.len() as i64 - 1
    }

    /// The same number times 10^`shift`.
    pub fn shifted(&self, shift: i64) -> Decimal {
        Decimal::new(self.negative, &self.digits, self.exp.saturating_add(shift))
    }

    /// The same number times a small whole number, exactly.
    pub fn times(&self, factor: u32) -> Decimal {
        let mut out = Vec::with_capacity(self.digits.len() + 10);
        let mut carry: u64 = 0;
        for d in self.digits.bytes().rev() {
            let v = (d - b'0') as u64 * factor as u64 + carry;
            out.push(b'0' + (v % 10) as u8);
            carry = v / 10;
        }
        while carry > 0 {
            out.push(b'0' + (carry % 10) as u8);
            carry /= 10;
        }
        out.reverse();
        Decimal::new(self.negative, std::str::from_utf8(&out).unwrap(), self.exp)
    }

    /// Rounded once, to nearest even, into `F`, by the standard library's correctly
    /// rounded reading of the exact decimal.
    pub fn round<F: Float>(&self) -> F {
        if self.is_zero() {
            return F::zero(self.negative);
        }
        let sign = if self.negative { "-" } else { "" };
        F::parse_exact(&format!("{sign}{}e{}", self.digits, self.exp))
    }

    /// The canonical text: plain decimal for a leading power of ten from -6 to 20,
    /// otherwise scientific (`1.5e-7`, `2e21`), never a `+` and never a trailing zero.
    pub fn layout(&self) -> String {
        let sign = if self.negative { "-" } else { "" };
        if self.is_zero() {
            return format!("{sign}0");
        }
        let lead = self.leading_exp();
        let d = &self.digits;
        if (-6..=20).contains(&lead) {
            if self.exp >= 0 {
                format!("{sign}{d}{}", "0".repeat(self.exp as usize))
            } else {
                let point = d.len() as i64 + self.exp;
                if point > 0 {
                    let (int, frac) = d.split_at(point as usize);
                    format!("{sign}{int}.{frac}")
                } else {
                    format!("{sign}0.{}{d}", "0".repeat((-point) as usize))
                }
            }
        } else if d.len() == 1 {
            format!("{sign}{d}e{lead}")
        } else {
            format!("{sign}{}.{}e{lead}", &d[..1], &d[1..])
        }
    }
}

/// Reads the longest number at the start of `text`: an optional sign, digits with an
/// optional fraction (at least one digit in all), an optional exponent; or `inf`,
/// `-inf`, `nan`. Returns the number and how many bytes it took.
pub(crate) fn scan(text: &str) -> Option<(Number, usize)> {
    let b = text.as_bytes();
    let mut i = 0;
    let negative = match b.first() {
        Some(b'-') => {
            i = 1;
            true
        }
        Some(b'+') => {
            i = 1;
            false
        }
        _ => false,
    };
    if text[i..].starts_with("inf") {
        return Some((Number::Infinite { negative }, i + 3));
    }
    if i == 0 && text.starts_with("nan") {
        return Some((Number::NaN, 3));
    }
    let int_start = i;
    while b.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    let int = &text[int_start..i];
    let mut frac = "";
    if b.get(i) == Some(&b'.') {
        let start = i + 1;
        let mut j = start;
        while b.get(j).is_some_and(u8::is_ascii_digit) {
            j += 1;
        }
        frac = &text[start..j];
        if !(int.is_empty() && frac.is_empty()) {
            i = j;
        }
    }
    if int.is_empty() && frac.is_empty() {
        return None;
    }
    let mut exp: i64 = 0;
    if matches!(b.get(i), Some(b'e' | b'E')) {
        let mut j = i + 1;
        let exp_negative = match b.get(j) {
            Some(b'-') => {
                j += 1;
                true
            }
            Some(b'+') => {
                j += 1;
                false
            }
            _ => false,
        };
        let start = j;
        let mut value: i64 = 0;
        while let Some(d) = b.get(j).filter(|d| d.is_ascii_digit()) {
            value = (value * 10 + (d - b'0') as i64).min(EXP_LIMIT);
            j += 1;
        }
        if j > start {
            exp = if exp_negative { -value } else { value };
            i = j;
        }
    }
    let digits = format!("{int}{frac}");
    let decimal = Decimal::new(negative, &digits, exp - frac.len() as i64);
    Some((Number::Finite(decimal), i))
}

/// A float type Blender stores: `f32` or `f64`.
pub trait Float: Copy + Debug + PartialEq + private::Sealed + 'static {
    /// Bits of precision, the hidden bit included: 24 or 53.
    const PRECISION: u32;
    /// The power of two of the smallest subnormal's one bit: -149 or -1074.
    const MIN_EXP: i64;
    /// Every finite value is below 2 to this power: 128 or 1024.
    const MAX_EXP: i64;
    /// The name of the type, for messages.
    const NAME: &'static str;

    /// The value's bits, widened to 64.
    fn bits(self) -> u64;
    /// The value widened to `f64`, exactly.
    fn to_f64(self) -> f64;
    /// Whether it is NaN.
    fn is_nan(self) -> bool;
    /// Whether it is infinite.
    fn is_infinite(self) -> bool;
    /// Whether its sign bit is set.
    fn is_sign_negative(self) -> bool;
    /// The value with its sign flipped.
    fn negated(self) -> Self;
    /// A signed zero.
    fn zero(negative: bool) -> Self;
    /// A signed infinity.
    fn infinity(negative: bool) -> Self;
    /// The quiet NaN.
    fn nan() -> Self;
    /// `mantissa` × 2^`exp`, which must be exactly representable.
    fn from_parts(negative: bool, mantissa: u64, exp: i64) -> Self;
    /// The standard library's correctly rounded reading of decimal text it accepts.
    fn parse_exact(text: &str) -> Self;
    /// The shortest decimal that reads back to these bits (`{:e}` of the standard
    /// library). The value must be finite.
    fn shortest(self) -> Decimal;
}

mod private {
    pub trait Sealed {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
}

/// 2^`k` as an `f64`, for `k` from -1074 to 1023.
fn pow2(k: i64) -> f64 {
    if k >= -1022 {
        f64::from_bits(((k + 1023) as u64) << 52)
    } else {
        f64::from_bits(1u64 << (k + 1074))
    }
}

fn shortest_of(sci: &str) -> Decimal {
    let (mantissa, exp) = sci
        .split_once('e')
        .expect("`{:e}` always writes an exponent");
    let exp: i64 = exp.parse().expect("`{:e}` writes a whole exponent");
    let negative = mantissa.starts_with('-');
    let mantissa = mantissa.trim_start_matches('-');
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    Decimal::new(negative, &format!("{int}{frac}"), exp - frac.len() as i64)
}

macro_rules! float_impl {
    ($t:ty, $precision:expr, $min:expr, $max:expr) => {
        impl Float for $t {
            const PRECISION: u32 = $precision;
            const MIN_EXP: i64 = $min;
            const MAX_EXP: i64 = $max;
            const NAME: &'static str = stringify!($t);

            fn bits(self) -> u64 {
                self.to_bits() as u64
            }
            fn to_f64(self) -> f64 {
                self as f64
            }
            fn is_nan(self) -> bool {
                <$t>::is_nan(self)
            }
            fn is_infinite(self) -> bool {
                <$t>::is_infinite(self)
            }
            fn is_sign_negative(self) -> bool {
                <$t>::is_sign_negative(self)
            }
            fn negated(self) -> Self {
                -self
            }
            fn zero(negative: bool) -> Self {
                if negative { -0.0 } else { 0.0 }
            }
            fn infinity(negative: bool) -> Self {
                if negative {
                    <$t>::NEG_INFINITY
                } else {
                    <$t>::INFINITY
                }
            }
            fn nan() -> Self {
                <$t>::NAN
            }
            fn from_parts(negative: bool, mantissa: u64, exp: i64) -> Self {
                // Exact: the mantissa has at most 53 bits, and the product is
                // representable in the target type, so also in f64.
                let magnitude = (mantissa as f64 * pow2(exp)) as $t;
                if negative { -magnitude } else { magnitude }
            }
            fn parse_exact(text: &str) -> Self {
                text.parse().expect("a decimal the standard library reads")
            }
            fn shortest(self) -> Decimal {
                shortest_of(&format!("{self:e}"))
            }
        }
    };
}

float_impl!(f32, 24, -149, 128);
float_impl!(f64, 53, -1074, 1024);
