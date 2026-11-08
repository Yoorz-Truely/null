//! Defaults left out (spec, section 15; task 1d-02b): a line of its own holding its property's
//! default goes, and nothing else does.

use null_check::Schema;
use null_print::{canonical, without_defaults};

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
