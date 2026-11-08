//! Flag sets and free-text enum items in the checker (task 1c-02b): a flag set is a list of its
//! enum's items, each once, in the schema's item order; an item whose name is free text is a
//! string. A small schema stands in, as no datablock type of Blender's holds a flag set of its
//! own (they are on modifiers and nodes, read through forms of later tasks).

use null_check::{Schema, check, codes};
use null_syntax::{Diagnostic, FileId, parse};

const SCHEMA: &str = r#"{
  "format": 1,
  "structs": [
    {"kind": "id", "rna": "ID", "properties": []},
    {"kind": "thing", "rna": "Thing", "base": "ID", "properties": [
      {"name": "delimit", "type": "enum", "flags": true, "items": [
        {"name": "normal"}, {"name": "material"}, {"name": "seam"}]},
      {"name": "space", "type": "enum", "items": [
        {"name": "ACES 1.3 sRGB", "quoted": true}, {"name": "linear"}]}
    ]}
  ],
  "nodes": [],
  "primitives": []
}"#;

fn errors(text: &str) -> Vec<Diagnostic> {
    let schema = Schema::from_json(SCHEMA).unwrap();
    let file = parse(text);
    check(&schema, &[(FileId(0), &file)])
}

fn thing(body: &str) -> String {
    format!("thing @t \"T\" {{\n  {body}\n}}\n")
}

#[test]
fn a_flag_set_is_a_list_in_the_schemas_order() {
    for ok in [
        "delimit []",
        "delimit [normal]",
        "delimit [normal seam]",
        "delimit [normal material seam]",
    ] {
        let found = errors(&thing(ok));
        assert!(found.is_empty(), "{ok}: {found:#?}");
    }
    let order = errors(&thing("delimit [seam normal]"));
    assert_eq!(order.len(), 1, "{order:#?}");
    assert_eq!(order[0].code, codes::TYPE);
    assert_eq!(order[0].fixes[0].edits[0].replacement, "[normal seam]");
    let twice = errors(&thing("delimit [normal normal]"));
    assert_eq!(twice[0].code, codes::TYPE);
    let unknown = errors(&thing("delimit [normal sam]"));
    assert_eq!(unknown[0].code, codes::ENUM);
    assert_eq!(unknown[0].fixes[0].edits[0].replacement, "seam");
    let bare = errors(&thing("delimit normal"));
    assert_eq!(bare[0].code, codes::TYPE);
}

#[test]
fn a_free_text_item_is_a_string() {
    assert!(errors(&thing("space \"ACES 1.3 sRGB\"")).is_empty());
    assert!(errors(&thing("space linear")).is_empty());
    assert!(!errors(&thing("space \"linear\"")).is_empty());
    assert!(!errors(&thing("space \"ACES 2.0\"")).is_empty());
}

#[test]
fn an_integer_past_32_bits_is_an_error_not_a_note() {
    // From the 1c-02b review: past the range is a note, since Blender can store it; past 32
    // bits it cannot, so that is an error.
    let schema = Schema::from_json(include_str!("../../../schema/null-schema.json")).unwrap();
    let file = parse("scene @s \"S\" {\n  frame-start 3000000000\n}\n");
    let found = check(&schema, &[(FileId(0), &file)]);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].code, codes::RANGE);
    assert_eq!(found[0].severity, null_syntax::Severity::Error);
    let file = parse("scene @s \"S\" {\n  frame-start 2000000000\n}\n");
    let found = check(&schema, &[(FileId(0), &file)]);
    assert!(
        found
            .iter()
            .all(|d| d.severity == null_syntax::Severity::Note),
        "{found:#?}"
    );
}

const BOOLS: &str = r#"{
  "format": 1,
  "structs": [
    {"kind": "id", "rna": "ID", "properties": []},
    {"kind": "thing", "rna": "Thing", "base": "ID", "properties": [
      {"name": "lock", "type": "bool", "array": [3]},
      {"name": "mask", "type": "bool", "array": "dynamic", "min-length": 1},
      {"name": "hidden", "type": "bool"}
    ]}
  ],
  "nodes": [],
  "primitives": []
}"#;

#[test]
fn a_boolean_array_is_a_list_of_on_and_off() {
    // Sau's 1c-02e: `lock-location [on off on]`, as the spec's arrays are written.
    let schema = Schema::from_json(BOOLS).unwrap();
    let found = |body: &str| {
        let file = parse(&thing(body));
        check(&schema, &[(FileId(0), &file)])
    };
    for ok in [
        "lock [on off on]",
        "mask [off]",
        "mask [on on off on]",
        "hidden",
        "hidden off",
    ] {
        assert!(found(ok).is_empty(), "{ok}: {:#?}", found(ok));
    }
    for (bad, says) in [
        ("lock [on off]", "takes 3 values, each on or off"),
        ("lock on", "as a list such as [off off off]"),
        ("lock [on maybe on]", "`lock` is on or off"),
        ("mask []", "this list has 0"),
        ("hidden [on]", "`hidden` is on or off"),
    ] {
        let errors = found(bad);
        assert_eq!(errors.len(), 1, "{bad}: {errors:#?}");
        assert_eq!(errors[0].code, codes::TYPE);
        assert!(
            errors[0].message.contains(says),
            "{bad}: {}",
            errors[0].message
        );
    }
    // A fix still reaches each item.
    let yes = found("lock [on yes on]");
    assert_eq!(yes[0].fixes[0].edits[0].replacement, "on");
}
