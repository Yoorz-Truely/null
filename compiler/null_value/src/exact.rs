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
