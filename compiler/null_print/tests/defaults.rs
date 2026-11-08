//! Defaults left out (spec, section 15; task 1d-02b): a line of its own holding its property's
//! default goes, and nothing else does.

use null_check::Schema;
use null_print::{Typed, canonical, default_of, is_default, without_defaults};

const PROJECT: &str = include_str!("../../../schema/null-schema-project.json");

fn schema() -> &'static Schema {
    static S: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
    S.get_or_init(|| Schema::from_json(PROJECT).unwrap())
}

#[test]
fn a_default_line_goes_and_a_set_one_stays() {
    let text = "object @o \"O\" {\n  rotation-mode xyz\n  pass-index 0\n  pass-index 3\n  scale [1 1 1]\n  scale [1 2 1]\n}\n";
    assert_eq!(
        without_defaults(text, schema()),
        "object @o \"O\" {\n  pass-index 3\n  scale [1 2 1]\n}\n"
    );
}

#[test]
fn what_is_not_surely_the_default_stays() {
    for text in [
        // A comment on the line or just above it: written on purpose.
        "object @o \"O\" {\n  pass-index 0 // on purpose\n}\n",
        "object @o \"O\" {\n  // Kept at zero for the compositor.\n  pass-index 0\n}\n",
        // One of a one-line block's attributes.
        "object @o \"O\" { pass-index 0; hide-render }\n",
        // A value read through the scene's unit scale: lengths are left to the checker's scene.
        "object @o \"O\" {\n  location [0m 0m 0m]\n}\n",
        // A `key=value` on the line.
        "object @o \"O\" {\n  rotation-mode xyz extra=1\n}\n",
        // An attribute the schema does not know, and text that does not parse.
        "object @o \"O\" {\n  not-a-property 0\n}\n",
        "object @o \"O\" {\n  pass-index 0\n",
    ] {
        assert_eq!(without_defaults(text, schema()), text, "{text}");
    }
}

#[test]
fn struct_blocks_and_modifiers_are_read_by_their_own_structs() {
    // A bevel's segments default to 1; its width set stays.
    let text =
        "object @o \"O\" {\n  modifier @b bevel \"B\" {\n    width 4mm\n    segments 1\n  }\n}\n";
    let out = without_defaults(text, schema());
    assert!(!out.contains("segments"), "{out}");
    assert!(out.contains("width 4mm"), "{out}");
}

#[test]
fn every_spec_example_keeps_its_words_but_its_defaults() {
    // Leaving out defaults then ordering is stable: a second pass changes nothing.
    let spec = include_str!("../../../docs/spec/null-v0.md");
    for block in spec.split("```null").skip(1) {
        let body = block.split_once('\n').map_or("", |x| x.1);
        let text = format!("{}\n", &body[..body.find("\n```").unwrap()]);
        let once = canonical(&without_defaults(&text, schema()), schema());
        let twice = canonical(&without_defaults(&once, schema()), schema());
        assert_eq!(twice, once, "{text}");
    }
}

#[test]
fn the_default_is_a_new_datablocks_and_a_matrix_is_row_by_row() {
    let schema = schema();
    let prop = |owner: &str, name: &str| {
        schema
            .properties_of(owner)
            .into_iter()
            .find(|p| p.name == name)
            .unwrap()
            .clone()
    };
    // `new` before RNA's `default`: a new collection's colour tag is `none`, not `color-01`.
    let tag = prop("Collection", "color-tag");
    assert!(is_default(&Typed::Enum("none".into()), &tag));
    assert!(!is_default(&Typed::Enum("color-01".into()), &tag));
    // A 4x4 matrix's default is the identity, row by row; a moved one is not.
    let m = prop("Object", "matrix-parent-inverse");
    let row = |xs: [f32; 4]| Typed::List(xs.iter().map(|x| Typed::F32(*x)).collect());
    let identity = Typed::List(vec![
        row([1.0, 0.0, 0.0, 0.0]),
        row([0.0, 1.0, 0.0, 0.0]),
        row([0.0, 0.0, 1.0, 0.0]),
        row([0.0, 0.0, 0.0, 1.0]),
    ]);
    assert_eq!(default_of(&m), Some(identity.clone()));
    let mut moved = identity;
    if let Typed::List(rows) = &mut moved
        && let Typed::List(first) = &mut rows[0]
    {
        first[3] = Typed::F32(2.0);
    }
    assert!(!is_default(&moved, &m));
    // Bits: `-0` is not `0`.
    let roughness = prop("Material", "roughness");
    let d = default_of(&roughness).unwrap();
    assert!(is_default(&d, &roughness));
    assert!(!is_default(&Typed::F32(-0.0), &roughness) || d == Typed::F32(-0.0));
}

#[test]
fn every_float32_default_in_the_schema_is_its_shortest_decimal() {
    // `default_of` reads a float32 default as `as_f64() as f32`, which is exact only while the
    // schema prints each float32 default as the shortest decimal that reads back to it (the
    // generator's `f32`); a longer decimal of the float's exact value would still read right,
    // but one that is not that float's would round once more. This pins the form (1d-02b's review).
    let json: serde_json::Value = serde_json::from_str(PROJECT).unwrap();
    let mut seen = 0;
    let mut wrong = Vec::new();
    for s in json["structs"].as_array().unwrap() {
        for p in s["properties"].as_array().unwrap() {
            if p["type"] != "float32" {
                continue;
            }
            for key in ["default", "new"] {
                let mut values = Vec::new();
                match &p[key] {
                    serde_json::Value::Number(n) => values.push(n.clone()),
                    serde_json::Value::Array(xs) => {
                        values.extend(xs.iter().filter_map(|x| x.as_number().cloned()))
                    }
                    _ => {}
                }
                for n in values {
                    seen += 1;
                    let x = n.as_f64().unwrap();
                    let f = x as f32;
                    // The decimal written is the float32's shortest, so it reads back as that
                    // float32 exactly, through f64 or directly.
                    if f64::from(f) != x && format!("{f}").parse::<f64>().unwrap() != x {
                        wrong.push(format!("{}.{} {key} {n}", s["rna"], p["name"]));
                    }
                }
            }
        }
    }
    assert!(seen > 1000, "{seen} float32 defaults");
    assert!(
        wrong.is_empty(),
        "{} not shortest float32: {:?}",
        wrong.len(),
        &wrong[..wrong.len().min(10)]
    );
}
