//! Exact arithmetic for conversions no decimal shift can do: π, and rounding a ratio once.
//!
//! Degrees become radians by multiplying by π/180, which is irrational, so no
//! finite computation gives the exact product. [`degrees`] brackets it instead:
//! π lies between two fractions over 2^k, which bracket the product, and if both
//! ends round to the same float, so does the exact product, since rounding is
//! monotonic. Otherwise it doubles `k` and tries again. The product of a nonzero
//! decimal and π is never a decimal or a tie, so this always ends, almost always
//! at the first try.

use std::sync::OnceLock;

use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::{One, Zero};

use crate::number::{Decimal, Float};
use crate::unit::Conv;

/// Bits of π for the first try.
const FIRST_BITS: u64 = 1024;

/// Fractions `lo / 2^bits` < π < `hi / 2^bits`, by Machin's formula
/// π = 16 atan(1/5) - 4 atan(1/239), summed in fixed point with a bound on the error.
pub(crate) fn pi_bounds(bits: u64) -> (BigUint, BigUint) {
    const GUARD: u64 = 64;
    let scale = BigUint::one() << (bits + GUARD);
    let (a5, n5) = atan_inverse(5, &scale);
    let (a239, n239) = atan_inverse(239, &scale);
    // Each term is off by less than 2 (two floors), and the first term left out
    // floored to zero, so the tail is below 2: the error is below 2(n + 1) per atan.
    let error = BigUint::from(16 * 2 * (n5 + 1) + 4 * 2 * (n239 + 1));
    let approx = (a5 * 16u32) - (a239 * 4u32);
    let lo = (&approx - &error) >> GUARD;
    let hi = ((approx + error) >> GUARD) + 1u32;
    (lo, hi)
}

/// The cached bounds for [`FIRST_BITS`].
fn pi_first() -> &'static (BigUint, BigUint) {
    static PI: OnceLock<(BigUint, BigUint)> = OnceLock::new();
    PI.get_or_init(|| pi_bounds(FIRST_BITS))
}

/// atan(1/`x`) × `scale`, approximately, and how many terms were summed.
fn atan_inverse(x: u32, scale: &BigUint) -> (BigUint, u64) {
    let x2 = BigUint::from(x) * x;
    let mut power = scale / x; // scale / x^(2k+1)
    let mut sum = BigUint::zero();
    let mut negative_sum = BigUint::zero();
    let mut k: u64 = 0;
    while !power.is_zero() {
        let term = &power / (2 * k + 1);
        if k.is_multiple_of(2) {
            sum += term;
        } else {
            negative_sum += term;
        }
        power /= &x2;
        k += 1;
    }
    (sum - negative_sum, k)
}

/// The positive ratio `n` / `d` rounded once, to nearest even, into `F`; infinity
/// past its range, zero or a subnormal below it.
pub(crate) fn round_ratio<F: Float>(n: &BigUint, d: &BigUint) -> F {
    if n.is_zero() {
        return F::zero(false);
    }
    let p = F::PRECISION as i64;
    // q = floor(n / (d × 2^e)) with exactly p bits, or fewer when e is at the bottom.
    let mut e = n.bits() as i64 - d.bits() as i64 - p;
    let (mut q, mut r, mut den);
    loop {
        e = e.max(F::MIN_EXP);
        (q, r, den) = divide(n, d, e);
        let bits = q.bits() as i64;
        if bits > p {
            e += bits - p;
        } else if bits < p && e > F::MIN_EXP {
            e -= p - bits;
        } else {
            break;
        }
    }
    let twice: BigUint = r << 1u32;
    if twice > den || (twice == den && q.is_odd()) {
        q += 1u32;
        if q.bits() as i64 > p {
            q >>= 1u32;
            e += 1;
        }
    }
    if e + q.bits() as i64 > F::MAX_EXP {
        return F::infinity(false);
    }
    let mantissa = q.iter_u64_digits().next().unwrap_or(0);
    F::from_parts(false, mantissa, e)
}

/// Quotient, remainder and divisor of `n` / (`d` × 2^`e`).
fn divide(n: &BigUint, d: &BigUint, e: i64) -> (BigUint, BigUint, BigUint) {
    let (num, den) = if e >= 0 {
        (n.clone(), d << e as u64)
    } else {
        (n << (-e) as u64, d.clone())
    };
    let (q, r) = num.div_rem(&den);
    (q, r, den)
}

/// `value` degrees in radians, the exact product rounded once into `F`.
pub(crate) fn degrees<F: Float>(value: &Decimal) -> F {
    if value.is_zero() {
        return F::zero(value.negative);
    }
    // |value| × π/180 is above 10^lead × 0.017 and below 10^(lead + 1) × 0.018:
    // past f64's range above 10^310, and below half its least subnormal under
    // 10^-326. Settling these first keeps the integers below about 10^800.
    let lead = value.leading_exp();
    let magnitude: F = if lead > 400 {
        F::infinity(false)
    } else if lead < -400 {
        F::zero(false)
    } else {
        bracket(value)
    };
    if value.negative {
        magnitude.negated()
    } else {
        magnitude
    }
}

/// The positive product |`value`| × π/180, bracketed until both ends round alike.
fn bracket<F: Float>(value: &Decimal) -> F {
    let mut bits = FIRST_BITS;
    loop {
        let fresh;
        let (lo, hi) = if bits == FIRST_BITS {
            pi_first()
        } else {
            fresh = pi_bounds(bits);
            &fresh
        };
        // Long inputs are cut to the digits this precision can use; the cut digits
        // widen the bracket by one in the last digit kept.
        let keep = (bits / 3) as usize;
        let (digits, exp, cut) = if value.digits.len() > keep {
            let cut = value.digits[keep..].bytes().any(|b| b != b'0');
            let exp = value.exp + (value.digits.len() - keep) as i64;
            (&value.digits[..keep], exp, cut)
        } else {
            (value.digits.as_str(), value.exp, false)
        };
        let d_lo = BigUint::parse_bytes(digits.as_bytes(), 10).expect("decimal digits");
        let d_hi = if cut { &d_lo + 1u32 } else { d_lo.clone() };
        let ten = BigUint::from(10u32);
        let mut den = BigUint::from(180u32) << bits;
        let (n_lo, n_hi) = if exp >= 0 {
            let scale = ten.pow(exp as u32);
            (d_lo * &scale * lo, d_hi * &scale * hi)
        } else {
            den *= ten.pow((-exp) as u32);
            (d_lo * lo, d_hi * hi)
        };
        let a: F = round_ratio(&n_lo, &den);
        let b: F = round_ratio(&n_hi, &den);
        if a.bits() == b.bits() {
            return a;
        }
        bits *= 2;
    }
}

/// A decimal as an exact ratio `n / d` of non-negative integers, its sign apart.
fn decimal_ratio(value: &Decimal) -> (BigUint, BigUint) {
    let digits = BigUint::parse_bytes(value.digits.as_bytes(), 10).unwrap_or_default();
    let ten = BigUint::from(10u32);
    if value.exp >= 0 {
        (digits * ten.pow(value.exp as u32), BigUint::one())
    } else {
        (digits, ten.pow((-value.exp) as u32))
    }
}

/// Past these leading powers of ten, a value times any unit's factor (from about 10^-14 to
/// 10^10) is beyond every float's range, or below half its least subnormal.
const LEAD_LIMIT: i64 = 400;

/// `value` × `n` / `d`, the exact product rounded once into `F`.
pub(crate) fn ratio<F: Float>(value: &Decimal, n: u128, d: u128) -> F {
    if value.is_zero() {
        return F::zero(value.negative);
    }
    let lead = value.leading_exp();
    let magnitude: F = if lead > LEAD_LIMIT {
        F::infinity(false)
    } else if lead < -LEAD_LIMIT {
        F::zero(false)
    } else {
        let (vn, vd) = decimal_ratio(value);
        round_ratio(&(vn * BigUint::from(n)), &(vd * BigUint::from(d)))
    };
    if value.negative {
        magnitude.negated()
    } else {
        magnitude
    }
}

/// (`value` + `offset`) × `factor`, exactly, then rounded once into `F`: a temperature in
/// degrees Celsius or Fahrenheit as kelvin. Zero is `+0`.
pub(crate) fn affine<F: Float>(value: &Decimal, offset: (u128, u128), factor: (u128, u128)) -> F {
    let lead = value.leading_exp();
    if !value.is_zero() && lead > LEAD_LIMIT {
        return F::infinity(value.negative);
    }
    // Below the limit the value is far too small to move the result past a rounding boundary,
    // since the offset is a short decimal: it is taken as zero.
    let (vn, vd) = if value.is_zero() || lead < -LEAD_LIMIT {
        (BigUint::zero(), BigUint::one())
    } else {
        decimal_ratio(value)
    };
    let (on, od) = (BigUint::from(offset.0), BigUint::from(offset.1));
    // value + offset = (±vn × od + on × vd) / (vd × od)
    let (a, b) = (&vn * &od, &on * &vd);
    let (negative, sum) = if value.negative && a > b {
        (true, a - b)
    } else if value.negative {
        (false, b - a)
    } else {
        (false, a + b)
    };
    let den = vd * od * BigUint::from(factor.1);
    let num = sum * BigUint::from(factor.0);
    let magnitude: F = round_ratio(&num, &den);
    if negative && !num.is_zero() {
        magnitude.negated()
    } else {
        magnitude
    }
}

/// |`value`| expressed in `unit` to about 40 significant digits, for finding the
/// shortest decimal in that unit: exactly divided for inches (0.0254 m) and feet
/// (0.3048 m), and through π's bracket for degrees. `None` for zero and non-finite
/// values, and for units that are decimal shifts.
pub(crate) fn in_unit<F: Float>(value: F, conv: Conv) -> Option<Decimal> {
    let x = value.to_f64();
    if !x.is_finite() || (x == 0.0 && !matches!(conv, Conv::Affine { .. })) {
        return None;
    }
    // |x| = m × 2^e exactly.
    let bits = x.abs().to_bits();
    let (raw_exp, frac) = ((bits >> 52) as i64, bits & ((1u64 << 52) - 1));
    let (m, e) = if raw_exp == 0 {
        (frac, -1074)
    } else {
        (frac | (1u64 << 52), raw_exp - 1075)
    };
    let (mut num, mut den) = if e >= 0 {
        (BigUint::from(m) << e as u64, BigUint::one())
    } else {
        (BigUint::from(m), BigUint::one() << (-e) as u64)
    };
    let mut negative = x < 0.0;
    if x == 0.0 {
        num = BigUint::zero();
    }
    match conv {
        Conv::Ratio(n, d) => {
            num *= BigUint::from(d);
            den *= BigUint::from(n);
        }
        Conv::Pi => {
            let (lo, _) = pi_first();
            num = (num * 180u32) << FIRST_BITS;
            den *= lo;
        }
        Conv::Affine { offset, factor } => {
            // x / factor - offset = (±num × fd × od - on × den × fn) / (den × fn × od)
            let (fnum, fden) = (BigUint::from(factor.0), BigUint::from(factor.1));
            let (on, od) = (BigUint::from(offset.0), BigUint::from(offset.1));
            let a = &num * &fden * &od;
            let b = &on * &den * &fnum;
            let (neg, diff) = if negative {
                (true, a + b)
            } else if a >= b {
                (false, a - b)
            } else {
                (true, b - a)
            };
            if diff.is_zero() {
                return Some(Decimal::new(false, "", 0));
            }
            negative = neg;
            num = diff;
            den = den * fnum * od;
        }
        Conv::Shift(_) => return None,
    }
    // 40 significant digits: q = floor(num × 10^s / den) with s chosen so q has about 40.
    let ten = BigUint::from(10u32);
    let magnitude = num.bits() as i64 - den.bits() as i64; // log2 of the ratio, roughly
    let s = 40 - (magnitude as f64 * std::f64::consts::LOG10_2).floor() as i64;
    let q = if s >= 0 {
        (num * ten.pow(s as u32)) / den
    } else {
        num / (den * ten.pow((-s) as u32))
    };
    Some(Decimal {
        negative,
        digits: q.to_string(),
        exp: -s,
    })
}
