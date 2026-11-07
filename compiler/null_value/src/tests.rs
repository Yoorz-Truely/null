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
        "`rad` does not fit here: this value takes m, cm, mm, km, um, nm, pm, mi, yd, ft, in, thou or a bare number"
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

#[test]
fn a_cameras_lens_and_a_lights_power_read_in_their_own_units() {
    // `mm` on a lens is the lens's own millimetre (stored 50), not a length (0.05).
    let lens: f32 = read("50mm", Dimension::Lens).unwrap();
    assert_eq!(lens, 50.0);
    assert_eq!(print(lens, Some(Unit::LensMm)), "50mm");
    let length: f32 = read("50mm", Dimension::Length).unwrap();
    assert_eq!(length, 0.05);
    let power: f32 = read("800W", Dimension::Power).unwrap();
    assert_eq!(power, 800.0);
    assert_eq!(print(power, Some(Unit::W)), "800W");
    assert_eq!(
        read::<f32>("800W", Dimension::Length),
        Err(ReadError::WrongUnit {
            unit: Unit::W,
            dimension: Dimension::Length
        })
    );
    // A lens also takes the camera unit's other words (task 1c-01b): 5 cm is 50 mm.
    assert_eq!(read::<f32>("5cm", Dimension::Lens).unwrap(), 50.0);
    assert_eq!(
        read::<f32>("5deg", Dimension::Lens)
            .unwrap_err()
            .to_string(),
        "`deg` does not fit here: this value takes mm, cm, m, um or a bare number"
    );
}

#[test]
fn a_unit_in_the_wrong_case_suggests_the_right_one() {
    // From the 1a-04 review: `5MM` suggested `m`.
    for (word, dimension, unit) in [
        ("5MM", Dimension::Length, Unit::Mm),
        ("5Deg", Dimension::Angle, Unit::Deg),
        ("5w", Dimension::Power, Unit::W),
        ("5CM", Dimension::Length, Unit::Cm),
    ] {
        assert_eq!(
            read::<f32>(word, dimension),
            Err(ReadError::UnknownUnit {
                unit: word[1..].to_string(),
                suggestion: Some(unit)
            }),
            "{word}"
        );
    }
}

#[test]
fn whole_numbers_read_exactly() {
    for (word, value) in [
        ("0", 0),
        ("-0", 0),
        ("240", 240),
        ("+7", 7),
        ("-12", -12),
        ("1e3", 1000),
        ("2.0", 2),
        ("9223372036854775807", i64::MAX),
        ("-9223372036854775808", i64::MIN),
    ] {
        assert_eq!(read_int(word), Ok(value), "{word}");
    }
    for word in ["1.5", "inf", "nan", "9223372036854775808", "1e19", "1e999"] {
        assert_eq!(read_int(word), Err(ReadError::NotAnInteger), "{word}");
    }
    assert!(matches!(
        read_int("24fps"),
        Err(ReadError::UnknownUnit { .. })
    ));
    assert!(matches!(read_int("2m"), Err(ReadError::WrongUnit { .. })));
    assert_eq!(read_int("x"), Err(ReadError::NotANumber));
}

proptest! {
    #[test]
    fn a_value_written_in_a_unit_prints_back_in_it(m in 1..10u64.pow(17), e in -25i64..10, ft in any::<bool>(), deg in any::<bool>()) {
        // From the 1a-04 review: at 16 and 17 digits the printer sometimes fell back to
        // the bare unit although a decimal in deg, in or ft read back. Every value read
        // from such a decimal must print in that unit again (S21: the line's unit is kept).
        let unit = if deg { Unit::Deg } else if ft { Unit::Ft } else { Unit::In };
        let text = format!("{m}e{e}{unit}");
        let x: f64 = read(&text, unit.dimension()).unwrap();
        let printed = print_in(x, unit);
        prop_assert!(printed.is_some(), "{} printed bare as {}", text, print_bare(x));
        let back: f64 = read(&printed.unwrap(), unit.dimension()).unwrap();
        prop_assert_eq!(back.to_bits(), x.to_bits());
    }
}

#[test]
fn the_reviews_case_keeps_its_feet() {
    let x: f64 = read("-18750715569787871e-5ft", Dimension::Length).unwrap();
    let printed = print_in(x, Unit::Ft).unwrap();
    let back: f64 = read(&printed, Dimension::Length).unwrap();
    assert_eq!(back.to_bits(), x.to_bits());
    assert!(printed.ends_with("ft"), "{printed}");
}

#[test]
fn the_unit_of_a_word() {
    assert_eq!(unit_of("2.7m", Dimension::Length), Some(Unit::M));
    assert_eq!(unit_of("50mm", Dimension::Lens), Some(Unit::LensMm));
    assert_eq!(unit_of("50mm", Dimension::Length), Some(Unit::Mm));
    assert_eq!(unit_of("90deg", Dimension::Angle), Some(Unit::Deg));
    assert_eq!(unit_of("0.5", Dimension::Angle), None);
    assert_eq!(unit_of("5parsec", Dimension::Length), None);
}

// --- Every RNA unit (task 1c-01b; spec, section 6, "Units") ---------------------------------

/// Every unit but degrees, which have tests of their own above.
fn every_unit() -> Vec<Unit> {
    Unit::all().filter(|u| *u != Unit::Deg).collect()
}

/// `x` (finite, nonzero) as an exact ratio `n / d` of its magnitude.
fn float_ratio(x: f64) -> (BigUint, BigUint) {
    let bits = x.abs().to_bits();
    let (raw, frac) = ((bits >> 52) as i64, bits & ((1u64 << 52) - 1));
    let (m, e) = if raw == 0 {
        (frac, -1074)
    } else {
        (frac | (1u64 << 52), raw - 1075)
    };
    if e >= 0 {
        (BigUint::from(m) << e as u64, BigUint::from(1u32))
    } else {
        (BigUint::from(m), BigUint::from(1u32) << (-e) as u64)
    }
}

/// The exact value, in the internal unit, of `m` × 10^`e` written in `unit`, as (negative,
/// n, d), from the unit's own factor, independently of the reader.
fn exact_value(negative: bool, m: u64, e: i64, unit: Unit) -> (bool, BigUint, BigUint) {
    let ten = BigUint::from(10u32);
    let (mut n, mut d) = if e >= 0 {
        (BigUint::from(m) * ten.pow(e as u32), BigUint::from(1u32))
    } else {
        (BigUint::from(m), ten.pow((-e) as u32))
    };
    match unit.conv() {
        unit::Conv::Shift(k) if k >= 0 => n *= ten.pow(k as u32),
        unit::Conv::Shift(k) => d *= ten.pow((-k) as u32),
        unit::Conv::Ratio(a, b) => {
            n *= BigUint::from(a);
            d *= BigUint::from(b);
        }
        unit::Conv::Affine { offset, factor } => {
            let (on, od) = (BigUint::from(offset.0), BigUint::from(offset.1));
            let (a, b) = (&n * &od, &on * &d);
            let (neg, sum) = if negative && a > b {
                (true, a - b)
            } else if negative {
                (false, b - a)
            } else {
                (false, a + b)
            };
            return (
                neg,
                sum * BigUint::from(factor.0),
                d * od * BigUint::from(factor.1),
            );
        }
        unit::Conv::Pi => unreachable!("degrees are tested above"),
    }
    (negative, n, d)
}

/// `x` (finite) as an exact ratio of its magnitude, for a float32 of the scene.
fn f32_ratio(x: f32) -> (BigUint, BigUint) {
    float_ratio(f64::from(x))
}

/// [`exact_value`] in `scene`, from the scene's own floats, independently of the reader: divided
/// by the unit scale to the dimension's power (as `BKE_unit_value_scale`), and for frames times
/// fps / fps-base.
fn exact_in(negative: bool, m: u64, e: i64, unit: Unit, scene: &Scene) -> (bool, BigUint, BigUint) {
    let (neg, mut n, mut d) = exact_value(negative, m, e, unit);
    let k = match unit.dimension() {
        Dimension::Length | Dimension::Velocity | Dimension::Acceleration => 1,
        Dimension::Area | Dimension::Power => 2,
        Dimension::Volume | Dimension::Mass => 3,
        _ => 0,
    };
    if k > 0 {
        let (sn, sd) = f32_ratio(scene.scale_length);
        n *= sd.pow(k);
        d *= sn.pow(k);
    }
    if unit.dimension() == Dimension::Frames {
        let (bn, bd) = f32_ratio(scene.fps_base);
        n *= BigUint::from(scene.fps) * bd;
        d *= bn;
    }
    (neg, n, d)
}

/// Whether `got` is the exact `n / d` rounded once to the nearest float of its type: the exact
/// value lies between the midpoints to its neighbours (on them only for an even mantissa).
fn rounds_once(n: &BigUint, d: &BigUint, got: f64, neighbours: (f64, f64), even: bool) -> bool {
    let (gn, gd) = float_ratio(got);
    let mid = |other: f64| {
        let (on, od) = float_ratio(other);
        // (got + other) / 2 = (gn × od + on × gd) / (2 × gd × od)
        (&gn * &od + &on * &gd, BigUint::from(2u32) * &gd * &od)
    };
    let (lo_n, lo_d) = mid(neighbours.0);
    let (hi_n, hi_d) = mid(neighbours.1);
    let above_lo = n * &lo_d > &lo_n * d || (even && n * &lo_d == &lo_n * d);
    let below_hi = n * &hi_d < &hi_n * d || (even && n * &hi_d == &hi_n * d);
    above_lo && below_hi
}

proptest! {
    #[test]
    fn every_unit_prints_and_reads_back_bit_identical(x in f64_bits(), y in f32_bits()) {
        for unit in every_unit() {
            if let Some(text) = print_in(x, unit) {
                let back: f64 = read(&text, unit.dimension()).unwrap();
                prop_assert_eq!(back.to_bits(), x.to_bits(), "{} in {}", text, unit);
            }
            if let Some(text) = print_in(y, unit) {
                let back: f32 = read(&text, unit.dimension()).unwrap();
                prop_assert_eq!(back.to_bits(), y.to_bits(), "{} in {}", text, unit);
            }
        }
    }

    #[test]
    fn every_unit_converts_with_one_rounding(negative in any::<bool>(), m in 1..10u64.pow(12), e in -15i64..12) {
        for unit in every_unit() {
            let text = format!("{}{m}e{e}{unit}", if negative { "-" } else { "" });
            let (neg, n, d) = exact_in(negative, m, e, unit, &Scene::default());
            let x: f64 = read(&text, unit.dimension()).unwrap();
            if x != 0.0 && x.is_finite() {
                prop_assert_eq!(x.is_sign_negative(), neg, "{} gave {}", text, x);
                let a = x.abs();
                let neighbours = (f64::from_bits(a.to_bits() - 1), f64::from_bits(a.to_bits() + 1));
                prop_assert!(rounds_once(&n, &d, a, neighbours, a.to_bits().is_multiple_of(2)), "{} gave {:e}", text, x);
            }
            let y: f32 = read(&text, unit.dimension()).unwrap();
            if y != 0.0 && y.is_finite() {
                let a = y.abs();
                let neighbours = (f32::from_bits(a.to_bits() - 1) as f64, f32::from_bits(a.to_bits() + 1) as f64);
                prop_assert!(rounds_once(&n, &d, a as f64, neighbours, a.to_bits().is_multiple_of(2)), "{} gave {:e} (f32)", text, y);
            }
        }
    }

    #[test]
    fn a_short_value_in_any_unit_prints_back_as_written(m in 1..10u64.pow(6), e in -6i64..6) {
        // A value typed with a few digits in a unit reads back in that unit, as typed (S21).
        for unit in every_unit() {
            let text = format!("{}{unit}", Decimal::new(false, &m.to_string(), e).layout());
            let x: f64 = read(&text, unit.dimension()).unwrap();
            prop_assert_eq!(print_in(x, unit), Some(text.clone()), "{}", text);
        }
    }
}

#[test]
fn every_unit_of_the_spec_reads() {
    let cases: &[(&str, Dimension, f64)] = &[
        ("1.5m", Dimension::Length, 1.5),
        ("3um", Dimension::Length, 3e-6),
        ("1mi", Dimension::Length, 1609.344),
        ("2yd", Dimension::Length, 1.8288),
        ("1thou", Dimension::Length, 0.0000254),
        ("1cm2", Dimension::Area, 1e-4),
        ("1ft2", Dimension::Area, 0.09290304),
        ("1in3", Dimension::Volume, 0.000016387064),
        ("2.5kg", Dimension::Mass, 2.5),
        ("1lb", Dimension::Mass, 0.45359237),
        ("1oz", Dimension::Mass, 0.028349523125),
        ("1st", Dimension::Mass, 6.35029318),
        ("1tn", Dimension::Mass, 907.18474),
        ("1t", Dimension::Mass, 1000.0),
        ("1.5s", Dimension::Seconds, 1.5),
        ("2min", Dimension::Seconds, 120.0),
        ("1d", Dimension::Seconds, 86400.0),
        ("36km/h", Dimension::Velocity, 10.0),
        ("1mph", Dimension::Velocity, 0.44704),
        ("9.81m/s2", Dimension::Acceleration, 9.81),
        ("35mm", Dimension::Lens, 35.0),
        ("3.5cm", Dimension::Lens, 35.0),
        ("2kW", Dimension::Power, 2000.0),
        ("5mW", Dimension::Power, 0.005),
        ("20C", Dimension::Temperature, 293.15),
        ("-40F", Dimension::Temperature, 233.15),
        ("-273.15C", Dimension::Temperature, 0.0),
        ("550nm", Dimension::Wavelength, 550.0),
        ("0.55um", Dimension::Wavelength, 550.0),
        ("6500K", Dimension::ColorTemperature, 6500.0),
        ("2kHz", Dimension::Frequency, 2000.0),
    ];
    for (text, dim, want) in cases {
        let got: f64 = read(text, *dim).unwrap();
        assert_eq!(got, *want, "{text}");
    }
    // Temperatures print back in their own unit.
    let k: f32 = read("20C", Dimension::Temperature).unwrap();
    assert_eq!(
        print_in(k, Unit::from_suffix("C", Dimension::Temperature).unwrap()).as_deref(),
        Some("20C")
    );
    let zero: f64 = read("-273.15C", Dimension::Temperature).unwrap();
    assert_eq!(zero.to_bits(), 0f64.to_bits(), "absolute zero is +0");
    assert_eq!(
        print_in(
            0f64,
            Unit::from_suffix("C", Dimension::Temperature).unwrap()
        )
        .as_deref(),
        Some("-273.15C")
    );
    // Case matters: a milliwatt is not a megawatt.
    let mw: f64 = read("1mW", Dimension::Power).unwrap();
    let big: f64 = read("1MW", Dimension::Power).unwrap();
    assert_eq!((mw, big), (0.001, 1e6));
}

#[test]
fn every_rna_unit_has_its_dimension() {
    // The spec's table: one dimension per RNA unit but `NONE`, and frames (`TIME`, 1c-01h),
    // which take the words of seconds.
    let words: Vec<(Dimension, Vec<&str>)> = Dimension::ALL
        .iter()
        .map(|d| (*d, d.units().iter().map(|u| u.suffix()).collect()))
        .collect();
    assert_eq!(words.len(), 16);
    let of = |d: Dimension| words.iter().find(|(e, _)| *e == d).unwrap().1.clone();
    assert_eq!(of(Dimension::Frames), of(Dimension::Seconds));
    for (d, w) in &words {
        assert_eq!(w.is_empty(), *d == Dimension::None, "{d:?}");
    }
}

// --- The scene: unit scale and frame rate (task 1c-01h) ------------------------------------------

/// `n / d` to 130 significant digits, for the standard library to round: the correctly rounded
/// float unless the exact value lies within a relative 10^-129 of a rounding boundary.
fn oracle(negative: bool, n: &BigUint, d: &BigUint) -> String {
    let ten = BigUint::from(10u32);
    let lead = ((n.bits() as f64 - d.bits() as f64) * std::f64::consts::LOG10_2).floor() as i64;
    let s = 130 - lead;
    let q = if s >= 0 {
        n * ten.pow(s as u32) / d
    } else {
        n / (d * ten.pow((-s) as u32))
    };
    format!("{}{q}e{}", if negative { "-" } else { "" }, -s)
}

fn any_unit() -> impl Strategy<Value = Unit> {
    let units = every_unit();
    (0..units.len()).prop_map(move |i| units[i])
}

fn any_scene() -> impl Strategy<Value = Scene> {
    (
        prop_oneof![
            Just(1.0f32),
            Just(0.01f32),
            Just(0.3048f32),
            Just(1000.0f32),
            Just(1e-30f32),
            Just(3e30f32),
            1e-3f32..1e3f32,
        ],
        1u32..240,
        prop_oneof![Just(1.0f32), Just(1.001f32), 0.5f32..2.0],
    )
        .prop_map(|(scale_length, fps, fps_base)| Scene {
            scale_length,
            fps,
            fps_base,
        })
}

proptest! {
    #[test]
    fn every_unit_in_any_scene_is_one_rounding_from_the_exact_value(m in 1..10u64.pow(17), e in -30i64..30, negative in any::<bool>(), unit in any_unit(), scene in any_scene()) {
        let text = format!("{}{m}e{e}{unit}", if negative { "-" } else { "" });
        let (neg, n, d) = exact_in(negative, m, e, unit, &scene);
        let x: f64 = read_in(&text, unit.dimension(), &scene).unwrap();
        let y: f32 = read_in(&text, unit.dimension(), &scene).unwrap();
        if n == BigUint::from(0u32) {
            prop_assert_eq!(x.to_bits(), 0f64.to_bits(), "{}", text);
            prop_assert_eq!(y.to_bits(), 0f32.to_bits(), "{}", text);
        } else {
            let want = oracle(neg, &n, &d);
            prop_assert_eq!(x.to_bits(), want.parse::<f64>().unwrap().to_bits(), "{} in {:?}", text, scene);
            prop_assert_eq!(y.to_bits(), want.parse::<f32>().unwrap().to_bits(), "{} in {:?} (f32)", text, scene);
        }
    }

    #[test]
    fn every_float_in_any_scene_prints_and_reads_back_bit_identical(x in f64_bits(), y in f32_bits(), unit in any_unit(), scene in any_scene()) {
        if let Some(text) = print_in_scene(x, unit, &scene) {
            let back: f64 = read_in(&text, unit.dimension(), &scene).unwrap();
            prop_assert_eq!(back.to_bits(), x.to_bits(), "{:e} printed as {} in {:?}", x, text, scene);
        }
        if let Some(text) = print_in_scene(y, unit, &scene) {
            let back: f32 = read_in(&text, unit.dimension(), &scene).unwrap();
            prop_assert_eq!(back.to_bits(), y.to_bits(), "{:e} printed as {} in {:?}", y, text, scene);
        }
    }

    #[test]
    fn a_short_value_in_any_scene_prints_back_as_written(m in 1..10u64.pow(6), e in -6i64..6, unit in any_unit(), scene in any_scene()) {
        // As in a plain scene (S21), when the scene leaves the value finite and nonzero.
        let text = format!("{}{unit}", Decimal::new(false, &m.to_string(), e).layout());
        let x: f64 = read_in(&text, unit.dimension(), &scene).unwrap();
        if x.is_finite() && x != 0.0 {
            prop_assert_eq!(print_in_scene(x, unit, &scene), Some(text.clone()), "{} in {:?}", text, scene);
        }
    }

    #[test]
    fn the_scene_path_at_scale_one_is_the_plain_one(m in 1..10u64.pow(17), e in -30i64..30, negative in any::<bool>()) {
        // Every dimension but frames reads through the plain conversions at unit scale 1; the
        // scene's own arithmetic, forced, gives the same bits.
        let text = format!("{}{m}e{e}", if negative { "-" } else { "" });
        let value = Decimal::new(negative, &m.to_string(), e);
        for unit in every_unit().into_iter().filter(|u| u.dimension() != Dimension::Frames) {
            let plain: f64 = read(&format!("{text}{unit}"), unit.dimension()).unwrap();
            let scene: f64 = crate::exact::in_scene(&value, unit.conv(), unit.dimension(), &Scene::default());
            prop_assert_eq!(scene.to_bits(), plain.to_bits(), "{}{}", text, unit);
        }
    }
}

#[test]
fn the_scenes_unit_scale_divides_as_blender_does() {
    // At unit scale 0.01, 1m is 100 Blender units, 1m2 is 10,000 and 1kg is 10^6: the exact
    // quotients by the float 0.01, rounded once.
    let s = Scene {
        scale_length: 0.01,
        ..Scene::default()
    };
    let r = |w: &str, d: Dimension| read_in::<f64>(w, d, &s).unwrap();
    for (word, dimension) in [
        ("1m", Dimension::Length),
        ("1m2", Dimension::Area),
        ("1kg", Dimension::Mass),
        ("1W", Dimension::Power),
        ("1m/s", Dimension::Velocity),
    ] {
        let unit = unit_of(word, dimension).unwrap();
        let (neg, n, d) = exact_in(false, 1, 0, unit, &s);
        let want: f64 = oracle(neg, &n, &d).parse().unwrap();
        assert_eq!(r(word, dimension).to_bits(), want.to_bits(), "{word}");
    }
    // The float 0.01 is a little under a hundredth, so each is a little over.
    assert!(r("1m", Dimension::Length) > 100.0 && r("1m", Dimension::Length) < 100.000_01);
    assert!(r("1m2", Dimension::Area) > 1e4 && r("1m2", Dimension::Area) < 1.000_01e4);
    assert!(r("1kg", Dimension::Mass) > 1e6 && r("1kg", Dimension::Mass) < 1.000_01e6);
    // Never scaled: rotations, cameras, wavelengths, times; nor a bare number.
    assert_eq!(r("90deg", Dimension::Angle), std::f64::consts::FRAC_PI_2);
    assert_eq!(r("50mm", Dimension::Lens), 50.0);
    assert_eq!(r("2s", Dimension::Seconds), 2.0);
    assert_eq!(r("550nm", Dimension::Wavelength), 550.0);
    assert_eq!(r("3", Dimension::Length), 3.0);
    // A length prints back in the scene's terms.
    assert_eq!(
        print_in_scene(
            100.0f32,
            Unit::from_suffix("cm", Dimension::Length).unwrap(),
            &s
        )
        .as_deref(),
        Some("100cm")
    );
}

#[test]
fn a_time_in_frames_reads_through_the_frame_rate() {
    // A bare number is frames; seconds are times the frame rate, exactly.
    assert_eq!(read::<f32>("1.5s", Dimension::Frames), Ok(36.0));
    assert_eq!(read::<f32>("36", Dimension::Frames), Ok(36.0));
    assert_eq!(read::<f32>("1min", Dimension::Frames), Ok(1440.0));
    let ntsc = Scene {
        fps: 30,
        fps_base: 1.001,
        ..Scene::default()
    };
    // 1001 s at 30 / 1.001 fps is 30,000 frames, but for float32's 1.001, taken exactly.
    let frames: f64 = read_in("1001s", Dimension::Frames, &ntsc).unwrap();
    let unit = unit_of("1s", Dimension::Frames).unwrap();
    let (neg, n, d) = exact_in(false, 1001, 0, unit, &ntsc);
    assert_eq!(
        frames.to_bits(),
        oracle(neg, &n, &d).parse::<f64>().unwrap().to_bits()
    );
    assert!(frames < 30000.0 && frames > 29999.99);
    assert_eq!(
        print_in_scene(36.0f32, unit, &Scene::default()).as_deref(),
        Some("1.5s")
    );
    // The scale never touches frames.
    let scaled = Scene {
        scale_length: 0.01,
        ..Scene::default()
    };
    assert_eq!(read_in::<f32>("1.5s", Dimension::Frames, &scaled), Ok(36.0));
}
