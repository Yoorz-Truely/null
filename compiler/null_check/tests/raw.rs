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
        ("precise 0x3ff0000000000000", codes::TYPE, "write `1`"),
        (
            "loc_legacy [0 0x3f800000 0]",
            codes::TYPE,
            "only for a NaN other than `nan`",
        ),
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

#[test]
fn a_floats_bits_that_are_not_a_nans_are_refused_with_the_decimal_as_the_fix() {
    let found = errors(
        &schema(true),
        &object("    loc_legacy [0 0x3f800000 0x7fc00000]"),
    );
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].fixes[0].edits[0].replacement, "1");
    let quiet = errors(&schema(true), &object("    loc_legacy [0 0 0x7fc00000]"));
    assert_eq!(quiet[0].fixes[0].edits[0].replacement, "nan");
}

#[test]
fn an_embedded_struct_arrays_element_is_a_step_of_its_index() {
    // Sau's task 1d-05i: the committed raw table names each element of `Scene.orientation_slots[4]`
    // by its index, a step of the path, so every slot's field is written.
    let project = Schema::from_json(include_str!("../../../schema/null-schema-project.json"))
        .unwrap()
        .with_raw(RawTable::from_json(include_str!("../../../schema/null-raw.json")).unwrap());
    let scene = |raw: &str| format!("scene @s \"S\" {{\n  raw {{\n{raw}\n  }}\n}}\n");
    let clean = scene(
        "    orientation_slots.0.index_custom -1\n    orientation_slots.3.index_custom 2\n    r.mblur_shutter_curve.cm.2.maxtable 1",
    );
    let found = errors(&project, &clean);
    assert!(found.is_empty(), "{found:?}");
    let found = errors(&project, &scene("    orientation_slots.4.index_custom 2"));
    assert!(
        found
            .iter()
            .any(|d| d.code == codes::UNKNOWN_PROPERTY && d.is_error()),
        "{found:?}"
    );
}

#[test]
fn a_v0_raw_path_without_an_elements_index_reads_as_element_0_with_a_warning() {
    // Sau's decision S94: a path that leaves out an embedded struct array's index, which a v0 file
    // may hold, still reads, as element 0, with a warning whose fix is the new form; written
    // beside element 0's own path it is the same field twice.
    let project = Schema::from_json(include_str!("../../../schema/null-schema-project.json"))
        .unwrap()
        .with_raw(RawTable::from_json(include_str!("../../../schema/null-raw.json")).unwrap());
    let scene = |raw: &str| format!("scene @s \"S\" {{\n  raw {{\n{raw}\n  }}\n}}\n");
    let found = errors(
        &project,
        &scene("    orientation_slots.index_custom 2\n    r.mblur_shutter_curve.cm.maxtable 1"),
    );
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found
            .iter()
            .all(|d| !d.is_error() && d.code == codes::RAW_OLD_PATH),
        "{found:?}"
    );
    assert!(
        found[0]
            .message
            .contains("reads as element 0: write `orientation_slots.0.index_custom`"),
        "{found:?}"
    );
    assert!(
        found[1]
            .message
            .contains("write `r.mblur_shutter_curve.cm.0.maxtable`"),
        "{found:?}"
    );
    // Its value is still held to the field's type.
    let found = errors(&project, &scene("    orientation_slots.index_custom 1.5"));
    assert!(found.iter().any(|d| d.is_error()), "{found:?}");
    let found = errors(
        &project,
        &scene("    orientation_slots.index_custom 2\n    orientation_slots.0.index_custom 2"),
    );
    assert!(found.iter().any(|d| d.code == codes::REPEATED), "{found:?}");
}

#[test]
fn a_masked_raw_field_takes_only_its_masks_bits() {
    // Sau's task 1b-03n: the committed raw table carries Scene's `flag` masked to SCE_FRAME_DROP
    // (8), which `sync-mode` cannot name under `audio-sync`.
    let project = Schema::from_json(include_str!("../../../schema/null-schema-project.json"))
        .unwrap()
        .with_raw(RawTable::from_json(include_str!("../../../schema/null-raw.json")).unwrap());
    let scene = |raw: &str| format!("scene @s \"S\" {{\n  raw {{\n{raw}\n  }}\n}}\n");
    let found = errors(&project, &scene("    flag 8"));
    assert!(found.is_empty(), "{found:?}");
    let found = errors(&project, &scene("    flag 9"));
    assert!(
        found
            .iter()
            .any(|d| d.is_error() && d.message.contains("carries only the bits 8")),
        "{found:?}"
    );
}

#[test]
fn a_masked_raw_line_that_contradicts_its_enums_state_is_a_warning() {
    // Sau's task 1b-03n, round 2: `sync-mode frame-drop` sets SCE_FRAME_DROP, so a raw `flag 0`
    // beside it contradicts it; under `audio-sync` the bit is free.
    let project = Schema::from_json(include_str!("../../../schema/null-schema-project.json"))
        .unwrap()
        .with_raw(RawTable::from_json(include_str!("../../../schema/null-raw.json")).unwrap());
    let scene = |mode: &str, flag: &str| {
        format!("scene @s \"S\" {{\n  sync-mode {mode}\n  raw {{\n    flag {flag}\n  }}\n}}\n")
    };
    let found = errors(&project, &scene("frame-drop", "0"));
    assert!(
        found.iter().any(|d| !d.is_error()
            && d.message
                .contains("`sync-mode frame-drop` sets `flag`'s bits 8 to 8")),
        "{found:?}"
    );
    for (mode, flag) in [
        ("frame-drop", "8"),
        ("audio-sync", "0"),
        ("audio-sync", "8"),
        ("none", "0"),
    ] {
        let found = errors(&project, &scene(mode, flag));
        assert!(found.is_empty(), "{mode} {flag}: {found:?}");
    }
    let found = errors(&project, &scene("none", "8"));
    assert!(found.iter().any(|d| !d.is_error()), "{found:?}");
}
