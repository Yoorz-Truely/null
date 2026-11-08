//! Every RNA type read and printed exactly (task 1c-02b): property tests over the schema of all
//! project data's own properties, each over its hard range. Printing then reading gives back the
//! same value, bit for bit: integers at their bounds, ±0, subnormals and the extremes of floats,
//! every enum item, every flag set, strings that are UTF-8 and strings that are not, arrays,
//! nested arrays and transform matrices.

use null_check::{Property, Schema};
use null_print::{Printing, Typed, print_value, read_value};
use null_value::{Scene, Unit};
use proptest::prelude::*;

const PROJECT: &str = include_str!("../../../schema/null-schema-project.json");

fn schema() -> &'static Schema {
    static S: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
    S.get_or_init(|| Schema::from_json(PROJECT).unwrap())
}

/// The project schema's settable properties of type `ty`: scalars (`array` none) or arrays.
fn properties(ty: &str, array: bool) -> Vec<&'static Property> {
    schema()
        .structs
        .iter()
        .flat_map(|s| s.properties.iter())
        .filter(|p| p.ty == ty && !p.readonly && (p.array.is_some() == array))
        .collect()
}

fn round_trip(v: &Typed, p: &Property, how: &Printing) -> Result<(), String> {
    let text = print_value(v, p, how).map_err(|e| format!("print: {e}"))?;
    let back = read_value(&text, p, &how.scene).map_err(|e| format!("read `{text}`: {e}"))?;
    if !same(&back, v) {
        return Err(format!(
            "`{}`: {v:?} printed `{text}` reads {back:?}",
            p.name
        ));
    }
    Ok(())
}

/// Equal, floats by their bits, NaN by being NaN (it keeps no sign or payload, section 6).
fn same(a: &Typed, b: &Typed) -> bool {
    match (a, b) {
        (Typed::F32(x), Typed::F32(y)) => x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan()),
        (Typed::F64(x), Typed::F64(y)) => x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan()),
        (Typed::List(x), Typed::List(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same(a, b))
        }
        _ => a == b,
    }
}

fn plain() -> Printing {
    Printing::default()
}

#[test]
fn booleans_both_ways() {
    let props = properties("bool", false);
    assert!(props.len() > 1000, "{}", props.len());
    for p in props {
        for b in [true, false] {
            round_trip(&Typed::Bool(b), p, &plain()).unwrap();
        }
    }
}

#[test]
fn every_integer_property_at_its_bounds_and_zero() {
    let props = properties("int", false);
    assert!(props.len() > 300, "{}", props.len());
    for p in props {
        let r = p.range.as_ref().unwrap().hard;
        let (lo, hi) = (r[0] as i64, r[1] as i64);
        for n in [
            lo,
            hi,
            lo.saturating_add(1),
            hi.saturating_sub(1),
            0.clamp(lo, hi),
        ] {
            round_trip(&Typed::Int(n), p, &plain()).unwrap();
        }
        // Past either bound, as a `.blend` can hold it, is written and read back as it is
        // (the lead's call on 1c-02b); the checker gives a note. Past 32 bits is refused.
        if hi < i64::from(i32::MAX) {
            round_trip(&Typed::Int(hi + 1), p, &plain()).unwrap();
        }
        assert!(read_value(&(i64::from(i32::MAX) + 1).to_string(), p, &Scene::default()).is_err());
    }
}

/// Float32 values a property may hold: the given bits, folded into its hard range.
fn in_range(bits: u32, p: &Property) -> f32 {
    let x = f32::from_bits(bits);
    let r = p.range.as_ref().map_or([f64::MIN, f64::MAX], |r| r.hard);
    if x.is_nan() || f64::from(x) < r[0] || f64::from(x) > r[1] {
        // A value of the range's own: its ends, as float32, rounded inward.
        let lo = r[0] as f32;
        if f64::from(lo) >= r[0] {
            lo
        } else {
            lo.next_up()
        }
    } else {
        x
    }
}

const SPECIAL: [u32; 10] = [
    0x0000_0000, // +0
    0x8000_0000, // -0
    0x0000_0001, // the least subnormal
    0x807f_ffff, // the greatest negative subnormal
    0x0080_0000, // the least normal
    0x7f7f_ffff, // the greatest float
    0xff7f_ffff, // the most negative
    0x3f80_0000, // 1
    0x3dcc_cccd, // 0.1
    0x4049_0fdb, // π
];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn every_float_property_round_trips_in_its_unit(
        bits in prop_oneof![prop::sample::select(SPECIAL.to_vec()), any::<u32>()],
        pick in any::<prop::sample::Index>(),
        scale in prop_oneof![Just(1.0f32), Just(0.01), Just(2.5)],
        length in prop_oneof![Just(None), Just(Some(Unit::M)), Just(Some(Unit::Cm)), Just(Some(Unit::Ft))],
        angle in prop_oneof![Just(None), Just(Some(Unit::Deg))],
    ) {
        let props = properties("float32", false);
        let p = props[pick.index(props.len())];
        let x = in_range(bits, p);
        let how = Printing { scene: Scene { scale_length: scale, ..Scene::default() }, length, angle };
        prop_assert_eq!(round_trip(&Typed::F32(x), p, &how), Ok(()));
    }

    #[test]
    fn every_float_array_round_trips(
        bits in prop::collection::vec(any::<u32>(), 32),
        pick in any::<prop::sample::Index>(),
        length in prop_oneof![Just(None), Just(Some(Unit::Mm))],
    ) {
        let props: Vec<_> = properties("float32", true).into_iter().filter(|p| !p.is_dynamic()).collect();
        let p = props[pick.index(props.len())];
        let dims = p.dimensions();
        let value = |i: usize| Typed::F32(in_range(bits[i % bits.len()], p));
        let v = match dims.as_slice() {
            [n] => Typed::List((0..*n).map(value).collect()),
            [r, c] => Typed::List((0..*r).map(|i| Typed::List((0..*c).map(|j| value(i * c + j)).collect())).collect()),
            _ => unreachable!(),
        };
        let how = Printing { length, ..Printing::default() };
        prop_assert_eq!(round_trip(&v, p, &how), Ok(()));
    }

    #[test]
    fn strings_of_any_bytes_but_nul(bytes in prop::collection::vec(1u8..=255, 0..64)) {
        let p = properties("string", false)[0];
        let v = Typed::Text(bytes.clone());
        prop_assert_eq!(round_trip(&v, p, &plain()), Ok(()));
        let text = print_value(&v, p, &plain()).unwrap();
        // UTF-8 is a string, anything else the list of its bytes (S65).
        prop_assert_eq!(text.starts_with('"'), std::str::from_utf8(&bytes).is_ok());
    }
}

#[test]
fn every_enum_item_of_every_enum() {
    let props: Vec<_> = properties("enum", false)
        .into_iter()
        .filter(|p| !p.flags)
        .collect();
    assert!(props.len() > 500, "{}", props.len());
    let mut items = 0;
    for p in props {
        for i in p.items.iter().flatten() {
            round_trip(&Typed::Enum(i.name.clone()), p, &plain()).unwrap();
            items += 1;
        }
    }
    assert!(items > 3000, "{items}");
}

#[test]
fn every_flag_set_of_every_flag_enum() {
    let props: Vec<_> = properties("enum", false)
        .into_iter()
        .filter(|p| p.flags)
        .collect();
    assert!(!props.is_empty());
    let mut sets = 0;
    for p in props {
        let names: Vec<String> = p.items.iter().flatten().map(|i| i.name.clone()).collect();
        // Every subset where there are few items, else every single item, every pair and all.
        let subsets: Vec<Vec<String>> = if names.len() <= 10 {
            (0..1u32 << names.len())
                .map(|m| {
                    names
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| m >> i & 1 == 1)
                        .map(|(_, n)| n.clone())
                        .collect()
                })
                .collect()
        } else {
            let mut v = vec![Vec::new(), names.clone()];
            for i in 0..names.len() {
                v.push(vec![names[i].clone()]);
                for j in i + 1..names.len() {
                    v.push(vec![names[i].clone(), names[j].clone()]);
                }
            }
            v
        };
        for s in subsets {
            round_trip(&Typed::Flags(s), p, &plain()).unwrap();
            sets += 1;
        }
        // Another order, or an item twice, is refused with the schema's order (section 6).
        if names.len() >= 2 {
            let backwards = format!("[{} {}]", names[1], names[0]);
            let e = read_value(&backwards, p, &Scene::default()).unwrap_err();
            assert!(e.0.contains(&format!("[{} {}]", names[0], names[1])), "{e}");
            assert!(read_value(&format!("[{0} {0}]", names[0]), p, &Scene::default()).is_err());
        }
    }
    assert!(sets > 100, "{sets}");
}

#[test]
fn infinities_and_values_past_the_range_round_trip() {
    // Infinity is treated as NaN is: a value Blender can hold, written `inf` and `-inf` and read
    // back exactly, whatever the property's range (the lead's call on 1c-02b).
    let location = schema()
        .properties_of("Object")
        .into_iter()
        .find(|p| p.name == "location")
        .unwrap();
    let v = Typed::List(vec![
        Typed::F32(f32::INFINITY),
        Typed::F32(f32::NEG_INFINITY),
        Typed::F32(f32::NAN),
    ]);
    let how = Printing {
        length: Some(Unit::M),
        ..Printing::default()
    };
    assert_eq!(print_value(&v, location, &how).unwrap(), "[inf -inf nan]");
    round_trip(&v, location, &how).unwrap();
    let roughness = schema()
        .properties_of("Material")
        .into_iter()
        .find(|p| p.name == "roughness")
        .unwrap();
    round_trip(&Typed::F32(1.5), roughness, &plain()).unwrap();
    round_trip(&Typed::F32(f32::INFINITY), roughness, &plain()).unwrap();
}

#[test]
fn a_run_time_item_that_is_not_one_word_is_a_string() {
    // From the 1c-02b review: the items of run-time enums come from data (attribute and UV map
    // names), which may hold spaces and capitals, or be empty.
    let p = properties("enum", false)
        .into_iter()
        .find(|p| p.items_at_run_time && !p.flags)
        .unwrap();
    for name in ["Some Item", "", "UVMap", "a\"b", "some-item"] {
        let v = Typed::Enum(name.to_string());
        round_trip(&v, p, &plain()).unwrap();
        let text = print_value(&v, p, &plain()).unwrap();
        // Always a string, so never read as a misspelling of a listed item (the 1c-02b review).
        assert!(text.starts_with('"'), "{name}: {text}");
    }
}

#[test]
fn integers_past_32_bits_are_refused_both_ways() {
    // Blender cannot store them, unlike a value past the range (the 1c-02b review).
    let p = schema()
        .properties_of("Scene")
        .into_iter()
        .find(|p| p.name == "frame-start")
        .unwrap();
    for n in [1i64 << 31, i64::MIN, -(1i64 << 31) - 1] {
        assert!(print_value(&Typed::Int(n), p, &plain()).is_err(), "{n}");
        assert!(
            read_value(&n.to_string(), p, &Scene::default()).is_err(),
            "{n}"
        );
    }
    round_trip(&Typed::Int(i64::from(i32::MAX)), p, &plain()).unwrap();
}

#[test]
fn strings_have_their_limits() {
    let mut p = properties("string", false)[0].clone();
    p.max_length = Some(4);
    let s = Scene::default();
    // Three bytes and the closing NUL fit; four do not, and a 0 byte never does.
    assert_eq!(
        read_value("\"abc\"", &p, &s).unwrap(),
        Typed::Text(b"abc".to_vec())
    );
    assert!(read_value("\"abcd\"", &p, &s).is_err());
    assert!(print_value(&Typed::Text(b"abcd".to_vec()), &p, &plain()).is_err());
    assert!(read_value("[97 0 98]", &p, &s).is_err());
    // A byte list that is valid UTF-8 is written as a string instead.
    assert!(read_value("[97 98]", &p, &s).is_err());
    assert_eq!(read_value("[252]", &p, &s).unwrap(), Typed::Text(vec![252]));
    // Escapes, as Null shares them with Scowr.
    p.max_length = None;
    let v = Typed::Text("a\"b\\c\nd\te\u{7}".as_bytes().to_vec());
    round_trip(&v, &p, &plain()).unwrap();
}

#[test]
fn a_transform_matrix_holds_lengths_in_its_translation() {
    let p = schema()
        .properties_of("Object")
        .into_iter()
        .find(|p| p.name == "matrix-basis")
        .unwrap()
        .clone();
    let s = Scene::default();
    let v = read_value("[[1 0 0 2m] [0 1 0 0m] [0 0 1 0m] [0 0 0 1]]", &p, &s).unwrap();
    let how = Printing {
        length: Some(Unit::M),
        ..Printing::default()
    };
    assert_eq!(
        print_value(&v, &p, &how).unwrap(),
        "[[1 0 0 2m] [0 1 0 0m] [0 0 1 0m] [0 0 0 1]]"
    );
    assert!(read_value("[[1m 0 0 2m] [0 1 0 0m] [0 0 1 0m] [0 0 0 1]]", &p, &s).is_err());
}

#[test]
fn integer_frames_stay_bare() {
    // `frame-start` is an integer with RNA's time unit: a whole number of frames, never seconds.
    let p = schema()
        .properties_of("Scene")
        .into_iter()
        .find(|p| p.name == "frame-start")
        .unwrap();
    assert_eq!(p.ty, "int");
    assert!(read_value("1s", p, &Scene::default()).is_err());
    round_trip(&Typed::Int(250), p, &plain()).unwrap();
}

#[test]
fn a_dynamic_array_prints_as_a_list_and_reads_back() {
    // Found by Sau's round-trip harness (task 1c-02d): a dynamic array printed as a scalar.
    let p = properties("float32", true)
        .into_iter()
        .find(|p| p.is_dynamic())
        .expect("a dynamic float array in the schema");
    let value = Typed::List(vec![Typed::F32(0.5), Typed::F32(-0.0), Typed::F32(3.0)]);
    let text = print_value(&value, p, &plain()).unwrap();
    assert_eq!(text, "[0.5 -0 3]");
    assert_eq!(read_value(&text, p, &Scene::default()).unwrap(), value);
}
