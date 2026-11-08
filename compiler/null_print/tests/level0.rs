//! Level 0 data (spec, section 9; task 1d-06b): one value of every attribute data type reads
//! and prints back bit for bit, as write-back and the printer write them.

use null_check::level0::{DATA_TYPES, element};
use null_print::{Printing, Typed, print_value, read_value};
use null_value::Scene;

/// Bits, for floats; the value itself otherwise.
fn bits(t: &Typed) -> String {
    match t {
        Typed::F32(x) => format!("f{:08x}", x.to_bits()),
        Typed::F64(x) => format!("d{:016x}", x.to_bits()),
        Typed::List(xs) => format!("[{}]", xs.iter().map(bits).collect::<Vec<_>>().join(" ")),
        other => format!("{other:?}"),
    }
}

#[test]
fn every_attribute_data_type_reads_and_prints_back_bit_for_bit() {
    let samples: &[(&str, &[&str])] = &[
        (
            "float",
            &["0", "-0", "0.1", "1e-45", "3.4028235e38", "-2.5"],
        ),
        ("int", &["0", "-1", "2147483647", "-2147483648"]),
        ("boolean", &["on", "off"]),
        ("float-vector", &["[0 0.1 -0]", "[1e-45 2 3]"]),
        ("float2", &["[0.25 0.75]", "[-0 1]"]),
        ("float4", &["[1 2 3 4]"]),
        ("float-color", &["[0.8 0.62 0.41 1]", "[0 0 0 0]"]),
        ("byte-color", &["[255 0 128 255]"]),
        ("quaternion", &["[1 0 0 0]", "[0.70710677 0 0.70710677 0]"]),
        (
            "float4x4",
            &["[[1 0 0 2] [0 1 0 0] [0 0 1 -0.5] [0 0 0 1]]"],
        ),
        ("int8", &["-128", "127"]),
        ("int16-2d", &["[-32768 32767]"]),
        ("int32-2d", &["[1 -2]"]),
        ("string", &["\"heat\"", "\"Pïer \\\"q\\\"\""]),
    ];
    assert_eq!(
        samples.iter().map(|(t, _)| *t).collect::<Vec<_>>(),
        DATA_TYPES,
        "a sample for every data type"
    );
    let scene = Scene::default();
    for (ty, texts) in samples {
        let p = element(ty).unwrap();
        for text in *texts {
            let value =
                read_value(text, &p, &scene).unwrap_or_else(|e| panic!("{ty} {text}: {e:?}"));
            let printed = print_value(&value, &p, &Printing::default())
                .unwrap_or_else(|e| panic!("{ty} {text}: {e:?}"));
            assert_eq!(&printed, text, "{ty} prints back as written");
            let again = read_value(&printed, &p, &scene).unwrap();
            assert_eq!(
                bits(&again),
                bits(&value),
                "{ty} {text} reads back to the same bits"
            );
        }
    }
}
