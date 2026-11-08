//! The `raw` block in the checker (spec, section 19; Sau's task 1d-05b): DNA fields RNA does
//! not reach, each a flat path with a value of its DNA type, held to the raw table. A small
//! schema and table stand in for Sau's build's.

use null_check::raw::RawTable;
use null_check::{Schema, check, codes};
use null_syntax::{Diagnostic, FileId, parse};

const SCHEMA: &str = r#"{
  "format": 1,
  "structs": [
    {"kind": "id", "rna": "ID", "properties": []},
    {"kind": "object", "rna": "Object", "base": "ID", "properties": [
      {"name": "pass-index", "type": "int"},
      {"name": "empty-display-size", "type": "float32"}
    ]},
    {"kind": "mesh", "rna": "Mesh", "base": "ID", "properties": []}
  ],
  "nodes": [],
  "primitives": []
}"#;

const RAW: &str = r#"{
  "format": 1,
  "structs": {
    "Object": [
      {"path": "transflag", "type": "short"},
      {"path": "loc_legacy", "type": "float", "dims": [3]},
      {"path": "pad_name", "type": "char", "dims": [4]},
      {"path": "r.mode", "type": "int"},
      {"path": "big", "type": "uint64_t"},
      {"path": "precise", "type": "double"}
    ]
  }
}"#;

fn schema(raw: bool) -> Schema {
    let s = Schema::from_json(SCHEMA).unwrap();
    if raw {
        s.with_raw(RawTable::from_json(RAW).unwrap())
    } else {
        s
    }
}

fn errors(schema: &Schema, text: &str) -> Vec<Diagnostic> {
    let file = parse(text);
    let mut all = null_syntax::diagnostics(&file, FileId(0));
    all.extend(check(schema, &[(FileId(0), &file)]));
    all
}

fn object(raw: &str) -> String {
    format!("object @o \"O\" {{\n  raw {{\n{raw}\n  }}\n}}\n")
}

#[test]
fn every_raw_field_of_its_type_checks_clean() {
    let text = object(
        "    transflag -4\n    loc_legacy [0.1 -0 0x7fa00001]\n    pad_name [80 0 255 1]\n    r.mode 2097152\n    big 18446744073709551615\n    precise 0x7ff0000000000001",
    );
    let found = errors(&schema(true), &text);
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn a_raw_field_of_the_wrong_type_or_shape_is_refused() {
    for (line, code, says) in [
        ("transflag 40000", codes::TYPE, "out of its range"),
        ("transflag 1.5", codes::TYPE, "is not one"),
        ("loc_legacy [1 2]", codes::TYPE, "this list has 2"),
        ("loc_legacy 1", codes::TYPE, "a list of 3 floats"),
        ("pad_name [256 0 0 0]", codes::TYPE, "out of its range"),
        ("precise 0x7fa00001", codes::TYPE, "16 hexadecimal digits"),
        (
            "transflg 1",
            codes::UNKNOWN_PROPERTY,
            "did you mean `transflag`",
        ),
        ("transflag 1 2", codes::TYPE, "takes one value"),
        ("r { mode 1 }", codes::TYPE, "flat DNA paths"),
    ] {
        let found = errors(&schema(true), &object(&format!("    {line}")));
        assert_eq!(found.len(), 1, "{line}: {found:#?}");
        assert_eq!(found[0].code, code, "{line}");
        assert!(
            found[0].message.contains(says),
            "{line}: {}",
            found[0].message
        );
    }
    let twice = errors(&schema(true), &object("    transflag 1\n    transflag 2"));
    assert_eq!(twice[0].code, codes::REPEATED);
    // A struct the table lists no fields for.
    let mesh = errors(
        &schema(true),
        "mesh @m \"M\" {\n  raw {\n    flag 1\n  }\n}\n",
    );
    assert!(
        mesh[0]
            .message
            .contains("no raw fields in this build's raw table"),
        "{mesh:#?}"
    );
}

#[test]
fn without_a_raw_table_the_lines_are_not_checked_against_dna() {
    assert!(errors(&schema(false), &object("    anything_dna 1")).is_empty());
    let bad = errors(&schema(false), "object @o \"O\" {\n  raw 1\n}\n");
    assert_eq!(bad[0].code, codes::TYPE);
}

#[test]
fn a_floats_bits_are_taken_only_in_a_raw_block() {
    let found = errors(
        &schema(true),
        "object @o \"O\" {\n  empty-display-size 0x7fa00001\n}\n",
    );
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].message.contains("only in a raw block"),
        "{}",
        found[0].message
    );
}
