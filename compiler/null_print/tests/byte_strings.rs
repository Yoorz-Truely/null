//! S65's byte lists wherever a string goes (task 1c-02g): a string Blender holds that is not
//! UTF-8, an old file path or a name from another system, is written as the list of its bytes.
//! The checker takes it in a string property and holds it to the field's `max-length` (task
//! 1b-03h) in bytes, as it does a string; the formatter and write-back's printer give it back
//! unchanged. A datablock's display name is UTF-8 (Sau's S72, task 1c-02i): a byte list there
//! is an error, with the name Blender would make of it as the fix.

use null_check::{Schema, check, codes};
use null_print::{Printing, Typed, canonical, format, print_value, read_value};
use null_syntax::{FileId, parse};
use null_value::Scene;

const PROJECT: &str = include_str!("../../../schema/null-schema-project.json");

/// The project schema with the field lengths Sau's build gives (1b-03h): `ID.name` 256 bytes,
/// `Image.filepath` 1024 (`FILE_MAX`), each with its closing NUL. A schema from before that task
/// has none, so the test sets them; once it has them, they must be these.
fn schema() -> Schema {
    let mut schema = Schema::from_json(PROJECT).unwrap();
    for s in &mut schema.structs {
        for p in &mut s.properties {
            let want = match (s.rna.as_str(), p.name.as_str()) {
                ("ID", "name") => 256,
                ("Image", "filepath") => 1024,
                _ => continue,
            };
            match p.max_length {
                Some(given) => assert_eq!(given, want, "{}.{}", s.rna, p.name),
                None => p.max_length = Some(want),
            }
        }
    }
    schema
}

fn codes_of(schema: &Schema, text: &str) -> Vec<(String, String)> {
    let parse = parse(text);
    check(schema, &[(FileId(0), &parse)])
        .into_iter()
        .map(|d| (d.code.to_string(), d.message))
        .collect()
}

/// `//müller.png` in Latin-1: `//m`, the byte 252, `ller.png`.
const LATIN1: &str = "[47 47 109 252 108 108 101 114 46 112 110 103]";

fn bytes(list: &[u8]) -> String {
    let words: Vec<String> = list.iter().map(u8::to_string).collect();
    format!("[{}]", words.join(" "))
}

#[test]
fn a_non_utf8_path_as_a_byte_list_checks_clean_and_prints_back() {
    let schema = schema();
    let text = format!("image @tex \"Tex\" {{\n  filepath {LATIN1}\n}}\n");
    assert_eq!(codes_of(&schema, &text), vec![]);
    // The formatter and canonical order leave it as written.
    assert_eq!(format(&text), text);
    assert_eq!(canonical(&text, &schema), text);
    // Write-back's printer reads it as its bytes and prints the same list.
    let p = schema
        .properties_of("Image")
        .into_iter()
        .find(|p| p.name == "filepath")
        .unwrap()
        .clone();
    let value = read_value(LATIN1, &p, &Scene::default()).unwrap();
    assert_eq!(
        value,
        Typed::Text(vec![
            47, 47, 109, 252, 108, 108, 101, 114, 46, 112, 110, 103
        ])
    );
    assert_eq!(
        print_value(&value, &p, &Printing::default()).unwrap(),
        LATIN1
    );
}

#[test]
fn a_byte_list_is_held_to_the_field_in_bytes() {
    let schema = schema();
    // 1,023 bytes and the closing NUL fill `FILE_MAX`; one more byte is over.
    let mut fits = vec![252u8];
    fits.extend(std::iter::repeat_n(b'a', 1022));
    let line = |list: &[u8]| format!("image @tex \"Tex\" {{\n  filepath {}\n}}\n", bytes(list));
    assert_eq!(codes_of(&schema, &line(&fits)), vec![]);
    let mut over = fits.clone();
    over.push(b'a');
    let found = codes_of(&schema, &line(&over));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, codes::RANGE);
    assert!(
        found[0]
            .1
            .contains("`filepath` holds at most 1023 bytes, not 1024"),
        "{}",
        found[0].1
    );
}

#[test]
fn a_display_name_as_a_byte_list_is_an_error_with_its_utf8_reading_as_the_fix() {
    // A datablock's name is UTF-8 (Sau's S72, task 1c-02i): Blender strips every byte that is
    // not, so `Pür` in Latin-1 would be `Pr`, and the fix says so.
    let schema = schema();
    let text = "object @pur [80 252 114]\n";
    let parsed = parse(text);
    let found = check(&schema, &[(FileId(0), &parsed)]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, codes::TYPE);
    assert!(
        found[0].message.contains("a display name is UTF-8 text"),
        "{}",
        found[0].message
    );
    let fix = &found[0].fixes[0].edits[0].replacement;
    assert_eq!(fix, "\"Pr\"");
    let fixed = text.replace("[80 252 114]", fix);
    assert_eq!(codes_of(&schema, &fixed), vec![]);
    // A collection defined in a scene too, and with an escape the fix keeps exact.
    let scene = "scene @s \"S\" {\n  collection @c [34 252 13] {\n  }\n}\n";
    let parsed = parse(scene);
    let found = check(&schema, &[(FileId(0), &parsed)]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].fixes[0].edits[0].replacement, "\"\\\"\\u{d}\"");
    // Nothing of it is UTF-8: an error with no fix.
    let found = codes_of(&schema, "object @pur [252 255]\n");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].1.contains("none of these bytes is"),
        "{}",
        found[0].1
    );
    // A byte list that is UTF-8, or not bytes, keeps 1c-02g's errors.
    let found = codes_of(&schema, "object @pier [80 105 101 114]\n");
    assert!(
        found[0].1.contains("written as a string: \"Pier\""),
        "{found:?}"
    );
    let found = codes_of(&schema, "object @pier [80 0 252]\n");
    assert!(found[0].1.contains("cannot hold a 0 byte"), "{found:?}");
    // A string property still takes a byte list (the path test above), and a string name is
    // held to the field as before.
    let mut long = String::from("P");
    long.push_str(&"a".repeat(255));
    let found = codes_of(&schema, &format!("object @pur \"{long}\"\n"));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, codes::RANGE);
}

#[test]
fn a_byte_list_is_only_for_bytes_that_are_not_utf8() {
    let schema = schema();
    let line = |list: &str| format!("image @tex \"Tex\" {{\n  filepath {list}\n}}\n");
    // UTF-8 bytes are a string: one spelling (S65), with the string as the fix.
    let found = codes_of(&schema, &line("[47 47 116 46 112 110 103]"));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, codes::TYPE);
    assert!(
        found[0].1.contains("written as a string: \"//t.png\""),
        "{}",
        found[0].1
    );
    // A byte is 1 to 255, and Blender's strings end at a 0.
    for wrong in ["[47 256 252]", "[47 -1 252]", "[47 a 252]"] {
        let found = codes_of(&schema, &line(wrong));
        assert_eq!(found.len(), 1, "{wrong}: {found:?}");
        assert!(
            found[0].1.contains("whole numbers 1 to 255"),
            "{wrong}: {}",
            found[0].1
        );
    }
    // The fix is in Null's escapes, so it reads back as the same bytes: a carriage return is
    // `\u{d}`, not Rust's `\r`, which Null reads as two characters (1c-02g's review).
    let text = line("[47 13 116]");
    let parsed = parse(&text);
    let found = check(&schema, &[(FileId(0), &parsed)]);
    assert_eq!(found.len(), 1, "{found:?}");
    let fix = found[0].fixes[0].edits[0].replacement.clone();
    assert_eq!(fix, "\"/\\u{d}t\"");
    assert_eq!(codes_of(&schema, &line(&fix)), vec![]);
    let p = schema
        .properties_of("Image")
        .into_iter()
        .find(|p| p.name == "filepath")
        .unwrap()
        .clone();
    assert_eq!(
        read_value(&fix, &p, &Scene::default()).unwrap(),
        Typed::Text(vec![47, 13, 116])
    );
    // A 0 byte is that error, whether or not the rest is UTF-8.
    let found = codes_of(&schema, &line("[97 0 98]"));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].1.contains("cannot hold a 0 byte"), "{found:?}");
    let found = codes_of(&schema, &line("[47 0 252]"));
    assert!(
        found
            .iter()
            .any(|(_, m)| m.contains("cannot hold a 0 byte")),
        "{found:?}"
    );
}
