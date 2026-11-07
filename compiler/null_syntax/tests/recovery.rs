//! Error recovery for the slice: broken input gives a whole tree and diagnostics, never a panic.

use std::path::Path;

use null_syntax::{FileId, SourceMap, diagnostics, invariants, messages, parse};

/// Broken slice text, and a node that must still be found after the break.
const BROKEN: [(&str, &str, &str); 12] = [
    (
        "unclosed-block",
        "scene @main {\n  fps 24\nobject @pier {\n  location [0 0 1m]\n",
        "location",
    ),
    (
        "stray-brace",
        "object @pier {\n  scale [1 1 1]\n}\n}\nobject @post { location [1m 0 0] }\n",
        "post",
    ),
    (
        "unclosed-list",
        "mesh @plank {\n  vertices [[0 0 0] [1 0 0]\n  faces [[0 1 2]]\n}\nlight @sun { energy 3W }\n",
        "sun",
    ),
    (
        "stray-bracket",
        "object @pier { location 0 0 1m] }\nobject @post\n",
        "post",
    ),
    (
        "reserved-paren",
        "object @pier { location (0 0 1m) }\nobject @post\n",
        "post",
    ),
    (
        "unclosed-string",
        "object @pier \"Pier {\n  location [0 0 1m]\n}\nobject @post\n",
        "post",
    ),
    (
        "missing-value",
        "object @pier location= {\n  scale [1 1 1]\n}\nobject @post\n",
        "post",
    ),
    (
        "property-as-node",
        "scene @main {\n  frame-start=1\n  frame-end 240\n}\n",
        "frame-end",
    ),
    (
        "missing-key",
        "camera @lens =35mm { clip-start 0.1m }\n",
        "clip-start",
    ),
    (
        "block-without-name",
        "{ fps 24 }\nscene @main { fps 24 }\n",
        "main",
    ),
    (
        "text-after-block",
        "material @oak { roughness 0.5 } base-color=[1 1 1]\nmaterial @ash\n",
        "ash",
    ),
    (
        "bad-escape",
        "object @pier \"P\\qier\\u{zz}\"\nobject @post\n",
        "post",
    ),
];

#[test]
fn broken_slice_text_gives_a_tree_and_diagnostics() {
    let mut rendered = String::new();
    for (name, text, survivor) in BROKEN {
        let file = parse(text);
        assert_eq!(file.print(), text, "{name}: lossless");
        let mut map = SourceMap::new();
        let id = map.add(format!("{name}.noll"), text);
        let found = diagnostics(&file, id);
        assert!(
            found.iter().any(|d| d.code == "null-syntax"),
            "{name}: no syntax error"
        );
        // The rest of the file still parses: the node after the break is there.
        let mut names = Vec::new();
        let mut stack: Vec<_> = file.document().nodes().collect();
        while let Some(node) = stack.pop() {
            names.push(node.name().into_owned());
            if let Some(h) = node.handle() {
                names.push(h.text.trim_start_matches('@').to_string());
            }
            stack.extend(node.children());
        }
        assert!(
            names.iter().any(|n| n == survivor),
            "{name}: `{survivor}` was lost: {names:?}"
        );
        invariants::check(text, 0, 0, "");
        rendered.push_str(&map.render_all(&found));
    }
    insta::assert_snapshot!("broken-slice", rendered);
}

#[test]
fn a_property_written_as_a_node_is_fixed_with_a_space() {
    let text = "scene @main {\n  frame-start=1\n}\n";
    let file = parse(text);
    let found = diagnostics(&file, FileId(0));
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].message,
        "a child node is its name, a space and its value: `frame-start …`; `frame-start=…` goes on its owner's line"
    );
    let edits: Vec<_> = found[0].fixes[0]
        .edits
        .iter()
        .map(|e| (e.span, e.replacement.as_str()))
        .collect();
    let fixed = null_syntax::apply_edits(text, &edits);
    assert_eq!(fixed, "scene @main {\n  frame-start 1\n}\n");
    assert!(diagnostics(&parse(&fixed), FileId(0)).is_empty());
}

#[test]
fn messages_name_nulls_examples_not_scowrs() {
    let cases = [
        ("{ fps 24 }\n", "such as object { ... }"),
        ("camera @lens =35mm\n", "such as frame-start=1"),
        ("object \"\\u{zz}\"\n", "such as \\u{e9}"),
    ];
    for (text, example) in cases {
        let found = diagnostics(&parse(text), FileId(0));
        assert!(
            found.iter().any(|d| d.message.contains(example)),
            "{text:?}: {found:?}"
        );
    }
}

#[test]
fn the_known_messages_match_by_pattern() {
    assert!(messages::is_known("expected a node name, found `]`"));
    assert!(messages::is_known(
        "this raw string is never closed; add \"##"
    ));
    assert!(messages::is_known("expected a value after location="));
    assert!(!messages::is_known("expected a note, such as C4"));
    assert!(!messages::is_known("expected a node name"));
}

/// A small deterministic generator, so the mutations are the same on every run.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

#[test]
fn mutated_slice_files_keep_every_invariant() {
    // Pieces that break the syntax, inserted, deleted and replaced at random.
    const PIECES: [&str; 22] = [
        "{", "}", "[", "]", "(", ")", "\"", "#\"", "\"#", "\\", "\\\n", "//", "/*", "*/", "/-",
        ";", "=", "\n", "\r\n", "\t", "é", "\u{2028}",
    ];
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/slice");
    let mut texts: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| std::fs::read_to_string(e.unwrap().path()).unwrap())
        .collect();
    texts.sort();
    let mut rng = Lcg(0x0005_eed0_f5a0);
    let mut broken = 0;
    for round in 0..3000 {
        let mut text = texts[round % texts.len()].clone();
        for _ in 0..1 + rng.below(4) {
            let mut at = rng.below(text.len() + 1);
            while !text.is_char_boundary(at) {
                at -= 1;
            }
            let piece = PIECES[rng.below(PIECES.len())];
            match rng.below(3) {
                0 => text.insert_str(at, piece),
                1 => {
                    let mut end = (at + 1 + rng.below(8)).min(text.len());
                    while !text.is_char_boundary(end) {
                        end -= 1;
                    }
                    text.replace_range(at..end, "");
                }
                _ => {
                    let mut end = (at + 1).min(text.len());
                    while !text.is_char_boundary(end) {
                        end += 1;
                    }
                    text.replace_range(at..end, piece);
                }
            }
        }
        if !parse(&text).errors().is_empty() {
            broken += 1;
        }
        let (a, b) = (rng.below(text.len() + 1), rng.below(text.len() + 1));
        let replacement = PIECES[rng.below(PIECES.len())];
        invariants::check(&text, a, b, replacement);
    }
    // Most mutations break the file, so recovery is what is being tested.
    assert!(
        broken > 1500,
        "only {broken} of 3000 mutations broke the syntax"
    );
}
