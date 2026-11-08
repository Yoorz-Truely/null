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
