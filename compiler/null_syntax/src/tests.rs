//! Tests that Null's text goes through the shared front end: the spec's examples, codes, recovery.

use super::*;

/// The fenced `null` examples of the specification, in order.
fn spec_examples() -> Vec<&'static str> {
    let spec = include_str!("../../../docs/spec/null-v0.md");
    let mut out = Vec::new();
    let mut rest = spec;
    while let Some(start) = rest.find("```null\n") {
        let body = &rest[start + "```null\n".len()..];
        let end = body.find("```").expect("an unclosed example in the spec");
        out.push(&body[..end]);
        rest = &body[end + 3..];
    }
    out
}

#[test]
fn every_spec_example_parses_clean_and_prints_back() {
    let examples = spec_examples();
    assert_eq!(examples.len(), 5, "section 4 has five example files");
    for text in examples {
        let file = parse(text);
        assert!(
            diagnostics(&file, FileId(0)).is_empty(),
            "{text}\n{:?}",
            file.errors()
        );
        assert_eq!(file.print(), text);
    }
}

#[test]
fn the_spec_examples_give_the_trees_null_reads() {
    let text = spec_examples()[2];
    let file = parse(text);
    let nodes: Vec<_> = file.document().nodes().collect();
    let names: Vec<_> = nodes.iter().map(|n| n.name().into_owned()).collect();
    assert_eq!(names, ["object", "mesh"]);
    let pier = &nodes[0];
    assert_eq!(pier.handle().unwrap().text, "@pier");
    let children: Vec<_> = pier.children().map(|n| n.name().into_owned()).collect();
    assert_eq!(
        children,
        [
            "location",
            "rotation-euler",
            "scale",
            "data",
            "modifier",
            "modifier",
            "material"
        ]
    );
    let location = pier.children().next().unwrap();
    let list = location.args().next().unwrap();
    let words: Vec<_> = list.items().iter().map(|v| v.text()).collect();
    assert_eq!(words, ["0m", "-4m", "0.35m"]);
    // `{ width 4mm segments 3 }` is one child node, `width`, whose arguments are
    // `4mm segments 3`: a node ends only at a line feed, `;` or `}`. The spec's
    // example means two attributes; the checker (task 1a-06) must reject this form.
    let bevel = pier.children().nth(5).unwrap();
    let width = bevel.children().next().unwrap();
    assert_eq!(width.name(), "width");
    let args: Vec<_> = width.args().map(|v| v.text()).collect();
    assert_eq!(args, ["4mm", "segments", "3"]);
    assert_eq!(bevel.children().count(), 1);
}

#[test]
fn null_words_stay_words() {
    // Units, sRGB colours, socket links, paths and package handles are words the
    // schema reads; the shared syntax keeps them whole.
    let text = "x 90deg #ff8000 @tex.color media/3f9a.png @cosmosis-materials:oak 1e-3\n";
    let file = parse(text);
    assert!(file.errors().is_empty(), "{:?}", file.errors());
    let node = file.document().nodes().next().unwrap();
    let args: Vec<_> = node.args().map(|v| v.text()).collect();
    assert_eq!(
        args,
        [
            "90deg",
            "#ff8000",
            "@tex.color",
            "media/3f9a.png",
            "@cosmosis-materials:oak",
            "1e-3"
        ]
    );
}

#[test]
fn syntax_errors_carry_nulls_code() {
    let text = "scene @main {\n  fps (24)\n  frame-end 240\n";
    let file = parse(text);
    let found = diagnostics(&file, FileId(3));
    assert!(!found.is_empty());
    for d in &found {
        assert_eq!(d.code, "null-syntax");
        assert_eq!(d.primary.file, FileId(3));
    }
    // One broken line does not hide the rest (spec, section 5).
    let scene = file.document().nodes().next().unwrap();
    assert!(scene.children().any(|n| n.name() == "frame-end"));
    assert_eq!(file.print(), text);
}

#[test]
fn scowrs_keyword_attributes_reach_null_as_keywords() {
    // `at`, `to` and `len` before a value are Scowr's keyword attributes
    // (AGENTS.md, "The shared front end"); Null's checker sees them as such.
    let file = parse("constraint @aim damped-track to @target\n");
    assert!(file.errors().is_empty());
    let node = file.document().nodes().next().unwrap();
    let keyword = node
        .0
        .children()
        .filter_map(|c| c.as_node())
        .find(|c| c.kind() == NodeKind::Keyword);
    assert!(keyword.is_some(), "{:#?}", file.root());
}
