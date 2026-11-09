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
    // Section 4 has five example files; sections 8, 10 and 11 have their own (task 1d-07a).
    assert!(examples.len() >= 5, "{} examples", examples.len());
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
    // `{ width 4mm; segments 3 }`: two child nodes, since a node ends only at a
    // line feed, `;` or `}` (Sau's decision S59 corrected the spec's example).
    let bevel = pier.children().nth(5).unwrap();
    let attrs: Vec<_> = bevel
        .children()
        .map(|n| (n.name().into_owned(), n.args().next().unwrap().text()))
        .collect();
    assert_eq!(
        attrs,
        [("width".to_string(), "4mm"), ("segments".to_string(), "3")]
    );
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

/// Every fenced example of the specification, those of forms not built yet (`null (<task>)`)
/// included, in order.
fn all_spec_examples() -> Vec<&'static str> {
    let spec = include_str!("../../../docs/spec/null-v0.md");
    spec.split("```null")
        .skip(1)
        .map(|b| {
            let body = b.split_once('\n').map_or("", |x| x.1);
            &body[..body.find("```").expect("an unclosed example in the spec")]
        })
        .collect()
}

#[test]
fn every_spec_example_is_read_as_v0_constructs_and_prints_back() {
    // Task 1d-02a: each example parses with no error, prints back byte for byte from its tree,
    // and is read as Null's constructs, one snapshot each.
    let examples = all_spec_examples();
    assert!(examples.len() >= 19, "{} examples", examples.len());
    let mut seen = std::collections::BTreeSet::new();
    for (i, text) in examples.iter().enumerate() {
        let file = parse(text);
        assert!(
            file.errors().is_empty(),
            "example {}: {:?}",
            i + 1,
            file.errors()
        );
        assert_eq!(file.print(), *text, "example {}", i + 1);
        let items = v0::outline(&file);
        fn all(items: &[v0::Item], out: &mut std::collections::BTreeSet<&'static str>) {
            for i in items {
                out.insert(i.what);
                all(&i.children, out);
            }
        }
        all(&items, &mut seen);
        insta::assert_snapshot!(format!("spec_example_{:02}", i + 1), v0::render(&items));
    }
    // Every construct the spec writes is in some example (section 19's `raw` block since Sau's
    // 1d-05b).
    for construct in [
        "project",
        "datablock",
        "sub-object",
        "node tree",
        "node",
        "link",
        "zone",
        "zone item",
        "zone output",
        "layout",
        "position",
        "interface socket",
        "f-curve",
        "key",
        "channels",
        "plays",
        "driver",
        "driver variable",
        "driver target",
        "data file",
        "data file reference",
        "component",
        "parameter",
        "placement",
        "pattern",
        "condition",
        "otherwise",
        "override",
        "raw block",
    ] {
        assert!(
            seen.contains(construct),
            "no example has a {construct}: {seen:?}"
        );
    }
}
