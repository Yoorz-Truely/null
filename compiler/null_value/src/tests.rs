//! Property and example tests: bit-identical round trips, one rounding, values read as written.

use num_bigint::BigUint;
use proptest::prelude::*;

use super::*;
use crate::exact::{pi_bounds, round_ratio};
use crate::number::Decimal;

fn f32_bits() -> impl Strategy<Value = f32> {
    any::<u32>()
        .prop_map(f32::from_bits)
        .prop_filter("not NaN", |x| !x.is_nan())
}

fn f64_bits() -> impl Strategy<Value = f64> {
    any::<u64>()
        .prop_map(f64::from_bits)
        .prop_filter("not NaN", |x| !x.is_nan())
}

/// A canonical decimal of at most `max_digits` significant digits, from 10^-12 to 10^12.
fn short_decimal(max_digits: u32) -> impl Strategy<Value = String> {
    (any::<bool>(), 1..10u64.pow(max_digits), -12i64..12).prop_map(|(negative, m, e)| {
        let exp = e - m.to_string().len() as i64 + 1;
        Decimal::new(negative, &m.to_string(), exp).layout()
    })
}

proptest! {
    #[test]
    fn every_f32_prints_and_reads_back_bit_identical(x in f32_bits()) {
        let text = print_bare(x);
        let back: f32 = read(&text, Dimension::None).unwrap();
        prop_assert_eq!(back.to_bits(), x.to_bits(), "{}", text);
    }

    #[test]
    fn every_f64_prints_and_reads_back_bit_identical(x in f64_bits()) {
        let text = print_bare(x);
        let back: f64 = read(&text, Dimension::None).unwrap();
        prop_assert_eq!(back.to_bits(), x.to_bits(), "{}", text);
    }

    #[test]
    fn every_length_prints_in_any_unit_and_reads_back(x in f64_bits(), y in f32_bits()) {
        for unit in Dimension::Length.units() {
            let text = print(x, Some(*unit));
            let back: f64 = read(&text, Dimension::Length).unwrap();
            prop_assert_eq!(back.to_bits(), x.to_bits(), "{}", text);
            let text = print(y, Some(*unit));
            let back: f32 = read(&text, Dimension::Length).unwrap();
            prop_assert_eq!(back.to_bits(), y.to_bits(), "{}", text);
        }
    }

    #[test]
    fn every_angle_prints_in_degrees_or_radians_and_reads_back(x in f64_bits(), y in f32_bits()) {
        let text = print(x, Some(Unit::Deg));
        let back: f64 = read(&text, Dimension::Angle).unwrap();
        prop_assert_eq!(back.to_bits(), x.to_bits(), "{}", text);
        let text = print(y, Some(Unit::Deg));
        let back: f32 = read(&text, Dimension::Angle).unwrap();
        prop_assert_eq!(back.to_bits(), y.to_bits(), "{}", text);
    }

    #[test]
    fn short_f32_values_read_back_as_written(d in short_decimal(6)) {
        for unit in [Unit::M, Unit::Cm, Unit::Mm, Unit::Deg] {
            let text = format!("{d}{unit}");
            let x: f32 = read(&text, unit.dimension()).unwrap();
            prop_assert_eq!(print(x, Some(unit)), text);
        }
        let x: f32 = read(&d, Dimension::None).unwrap();
        prop_assert_eq!(print_bare(x), d);
    }

    #[test]
    fn short_f64_values_read_back_as_written(d in short_decimal(15)) {
        for unit in [Unit::M, Unit::Cm, Unit::Mm, Unit::Deg] {
            let text = format!("{d}{unit}");
            let x: f64 = read(&text, unit.dimension()).unwrap();
            prop_assert_eq!(print(x, Some(unit)), text);
        }
        let x: f64 = read(&d, Dimension::None).unwrap();
        prop_assert_eq!(print_bare(x), d);
    }

    #[test]
    fn a_ratio_rounds_as_the_exact_decimal_does(m in any::<u64>(), k in 0u32..400) {
        // m / 10^k against the standard library's correctly rounded reading.
        let n = BigUint::from(m);
        let d = BigUint::from(10u32).pow(k);
        let text = format!("{m}e-{k}");
        let a: f64 = round_ratio(&n, &d);
        prop_assert_eq!(a.to_bits(), text.parse::<f64>().unwrap().to_bits());
        let b: f32 = round_ratio(&n, &d);
        prop_assert_eq!(b.to_bits(), text.parse::<f32>().unwrap().to_bits());
    }

    #[test]
    fn degrees_are_rounded_once(m in 1..10u64.pow(15), e in -20i64..20) {
        // The exact product lies between the two neighbours of the f64 product, and
        // the f32 result is one of the two f32 neighbours of the exact product.
        let text = format!("{m}e{e}deg");
        let x: f64 = read(&text, Dimension::Angle).unwrap();
        let approx = format!("{m}e{e}").parse::<f64>().unwrap() * (std::f64::consts::PI / 180.0);
        let ulps = (x.to_bits() as i64 - approx.to_bits() as i64).abs();
        prop_assert!(ulps <= 2, "{} gives {:e}, approximately {:e}", text, x, approx);
        let y: f32 = read(&text, Dimension::Angle).unwrap();
        let close = (y as f64 - x).abs() <= (y.next_up() as f64 - y.next_down() as f64);
        prop_assert!(close, "{} gives {:e} for f32", text, y);
    }
}

/// π to 120 decimals, from Python's `decimal` module (its documented recipe), as an
/// oracle independent of Machin's formula and of binary arithmetic.
const PI_DECIMAL: &str = "3141592653589793238462643383279502884197169399375105820974944592307816406286208998628034825342117067982148086513282306647";

proptest! {
    #[test]
    fn degrees_match_a_decimal_oracle(m in 1..u64::MAX, e in -60i64..60) {
        // m × 10^e × π/180 to about 100 significant digits, read by the standard
        // library: the correctly rounded result unless the product is within 10^-100
        // of a rounding boundary.
        let pi = BigUint::parse_bytes(PI_DECIMAL.as_bytes(), 10).unwrap();
        let q = (BigUint::from(m) * pi) / 180u32;
        let oracle = format!("{q}e{}", e - 120);
        let text = format!("{m}e{e}deg");
        let x: f64 = read(&text, Dimension::Angle).unwrap();
        prop_assert_eq!(x.to_bits(), oracle.parse::<f64>().unwrap().to_bits(), "{}", text);
        let y: f32 = read(&text, Dimension::Angle).unwrap();
        prop_assert_eq!(y.to_bits(), oracle.parse::<f32>().unwrap().to_bits(), "{}", text);
    }
}

#[test]
fn long_degree_values_are_read_exactly() {
    // Beyond the digits the first bracket keeps (341): the cut widens the bracket.
    let pi = BigUint::parse_bytes(PI_DECIMAL.as_bytes(), 10).unwrap();
    for (len, tail) in [(400, "7"), (1000, "1"), (3000, "0"), (3000, "5")] {
        let digits = format!("{}{tail}", "3".repeat(len));
        let m = BigUint::parse_bytes(digits.as_bytes(), 10).unwrap();
        let e = -(len as i64);
        let oracle = format!("{}e{}", (m * &pi) / 180u32, e - 120);
        let text = format!("{digits}e{e}deg");
        let x: f64 = read(&text, Dimension::Angle).unwrap();
        assert_eq!(
            x.to_bits(),
            oracle.parse::<f64>().unwrap().to_bits(),
            "{len}"
        );
        let y: f32 = read(&text, Dimension::Angle).unwrap();
        assert_eq!(
            y.to_bits(),
            oracle.parse::<f32>().unwrap().to_bits(),
            "{len}"
        );
    }
}

#[test]
fn the_tasks_examples_read_back_as_written() {
    for (text, unit) in [("90deg", Unit::Deg), ("2.7m", Unit::M), ("5mm", Unit::Mm)] {
        let x: f32 = read(text, unit.dimension()).unwrap();
        assert_eq!(print(x, Some(unit)), text);
        let y: f64 = read(text, unit.dimension()).unwrap();
        assert_eq!(print(y, Some(unit)), text);
    }
    assert_eq!(print(0.1f32, None), "0.1");
    assert_eq!(print(0.1f64, None), "0.1");
}

#[test]
fn degrees_give_the_correctly_rounded_radians() {
    use std::f32::consts as c32;
    use std::f64::consts as c64;
    let cases: [(&str, f32, f64); 6] = [
        ("90deg", c32::FRAC_PI_2, c64::FRAC_PI_2),
        ("180deg", c32::PI, c64::PI),
        ("45deg", c32::FRAC_PI_4, c64::FRAC_PI_4),
        ("30deg", c32::FRAC_PI_6, c64::FRAC_PI_6),
        ("360deg", c32::TAU, c64::TAU),
        ("-60deg", -c32::FRAC_PI_3, -c64::FRAC_PI_3),
    ];
    for (text, a, b) in cases {
        let x: f32 = read(text, Dimension::Angle).unwrap();
        assert_eq!(x.to_bits(), a.to_bits(), "{text}");
        let y: f64 = read(text, Dimension::Angle).unwrap();
        assert_eq!(y.to_bits(), b.to_bits(), "{text}");
    }
}

#[test]
fn units_are_one_rounding_from_the_exact_value() {
    // 2.7cm is the f32 nearest 0.027, never 2.7 × 0.01 rounded twice.
    let x: f32 = read("2.7cm", Dimension::Length).unwrap();
    assert_eq!(x, "0.027".parse::<f32>().unwrap());
    let x: f64 = read("1in", Dimension::Length).unwrap();
    assert_eq!(x, 0.0254);
    let x: f64 = read("3ft", Dimension::Length).unwrap();
    assert_eq!(x, "0.9144".parse::<f64>().unwrap());
    let x: f64 = read("1.5km", Dimension::Length).unwrap();
    assert_eq!(x, 1500.0);
    let x: f32 = read("1.5707964rad", Dimension::Angle).unwrap();
    assert_eq!(x, std::f32::consts::FRAC_PI_2);
}

#[test]
fn signs_zeros_and_infinities_keep_their_bits() {
    for text in ["-0", "0", "inf", "-inf"] {
        let x: f32 = read(text, Dimension::None).unwrap();
        assert_eq!(print_bare(x), text);
    }
    let x: f64 = read("-0deg", Dimension::Angle).unwrap();
    assert_eq!(x.to_bits(), (-0.0f64).to_bits());
    let x: f32 = read("nan", Dimension::None).unwrap();
    assert!(x.is_nan());
    assert_eq!(print_bare(f32::NAN), "nan");
    // Far past either end, quickly.
    let x: f64 = read("1e999999999999deg", Dimension::Angle).unwrap();
    assert_eq!(x, f64::INFINITY);
    let x: f64 = read("-1e-999999999999deg", Dimension::Angle).unwrap();
    assert_eq!(x.to_bits(), (-0.0f64).to_bits());
    let x: f32 = read("1e39", Dimension::None).unwrap();
    assert_eq!(x, f32::INFINITY);
}

#[test]
fn the_layout_is_canonical() {
    let cases: [(f64, &str); 9] = [
        (0.1, "0.1"),
        (100.0, "100"),
        (1e20, "100000000000000000000"),
        (1e21, "1e21"),
        (0.000001, "0.000001"),
        (1.5e-7, "1.5e-7"),
        (-2.5, "-2.5"),
        (f64::MAX, "1.7976931348623157e308"),
        (5e-324, "5e-324"),
    ];
    for (x, text) in cases {
        assert_eq!(print_bare(x), text);
    }
    // Written forms the reader accepts and the printer does not use.
    for (text, x) in [("+1.50", 1.5), (".5", 0.5), ("5.", 5.0), ("1E3", 1000.0)] {
        assert_eq!(read::<f64>(text, Dimension::None).unwrap(), x, "{text}");
    }
}

#[test]
fn wrong_and_unknown_units_are_errors_with_suggestions() {
    assert_eq!(
        read::<f32>("5deg", Dimension::Length),
        Err(ReadError::WrongUnit {
            unit: Unit::Deg,
            dimension: Dimension::Length
        })
    );
    assert_eq!(
        read::<f32>("5mn", Dimension::Length),
        Err(ReadError::UnknownUnit {
            unit: "mn".into(),
            suggestion: Some(Unit::M)
        })
    );
    assert_eq!(
        read::<f32>("5dge", Dimension::Angle)
            .unwrap_err()
            .to_string(),
        "`dge` is not a unit; did you mean `deg`?"
    );
    assert_eq!(
        read::<f32>("5m", Dimension::None).unwrap_err().to_string(),
        "`m` does not fit here: this value takes a bare number, with no unit"
    );
    assert_eq!(
        read::<f32>("2rad", Dimension::Length)
            .unwrap_err()
            .to_string(),
        "`rad` does not fit here: this value takes m, cm, mm, km, in, ft or a bare number"
    );
    assert_eq!(
        read::<f32>("deg", Dimension::Angle),
        Err(ReadError::NotANumber)
    );
    assert_eq!(
        read::<f32>("-", Dimension::None),
        Err(ReadError::NotANumber)
    );
}

#[test]
fn pi_is_bracketed() {
    let (lo, hi) = pi_bounds(200);
    assert!(hi > lo && &hi - &lo <= BigUint::from(2u32));
    // The top 53 bits agree with f64's π, correctly rounded.
    let top = |x: &BigUint| ((x >> (200 - 52)) + 1u32) >> 1;
    let pi53 = (std::f64::consts::PI * 2f64.powi(51)) as u64;
    assert_eq!(top(&lo), BigUint::from(pi53));
    assert_eq!(top(&hi), BigUint::from(pi53));
    // Bounds at a higher precision lie inside these.
    let (lo2, hi2) = pi_bounds(400);
    let (a, b) = (&lo2 >> 200u32, &hi2 >> 200u32);
    assert!(lo <= a && a <= b && b <= hi);
}

/// Every one of the 2^32 f32 bit patterns but NaN prints and reads back to its
/// bits. Takes minutes; run with `cargo test --release -p null_value -- --ignored`.
#[test]
#[ignore]
fn all_f32_round_trip_exhaustively() {
    let mut checked: u64 = 0;
    for bits in 0..=u32::MAX {
        let x = f32::from_bits(bits);
        if x.is_nan() {
            continue;
        }
        let back: f32 = read(&print_bare(x), Dimension::None).unwrap();
        assert_eq!(back.to_bits(), bits, "{x:e}");
        checked += 1;
    }
    assert_eq!(checked, (1u64 << 32) - 2 * ((1u64 << 23) - 1));
}
