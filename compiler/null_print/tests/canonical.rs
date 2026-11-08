//! Canonical order (spec, section 15; task 1c-01e): attributes in the schema's order, then the
//! formatter's layout. The slice's files, shuffled, come back in the one canonical form; formatting
//! twice changes nothing; nothing is lost, comments included.

use std::path::Path;

use null_check::{Schema, check};
use null_print::canonical;
use null_syntax::ast::Node;
use null_syntax::{FileId, parse};
use proptest::prelude::*;

const SLICE: &str = include_str!("../../../schema/null-schema.json");

fn schema() -> &'static Schema {
    static S: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
    S.get_or_init(|| Schema::from_json(SLICE).unwrap())
}

/// The slice's Null files: the checker's project and the syntax's material file.
fn slice_files() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut out = Vec::new();
    for dir in ["null_check/tests/slice", "null_syntax/tests/slice"] {
        let mut paths: Vec<_> = std::fs::read_dir(root.join(dir))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "noll"))
            .collect();
        paths.sort();
        out.extend(paths.iter().map(|p| std::fs::read_to_string(p).unwrap()));
    }
    out
}

/// The text's words, strings and comments, in any order: what canonical order must keep.
fn tokens(text: &str) -> Vec<String> {
    let mut v: Vec<String> = null_syntax::lex(text)
        .0
        .iter()
        .scan(0usize, |at, t| {
            let s = text[*at..*at + t.len as usize].to_string();
            *at += t.len as usize;
            Some((t.kind, s))
        })
        .filter(|(k, _)| {
            !matches!(
                k,
                null_syntax::TokenKind::Whitespace
                    | null_syntax::TokenKind::Newline
                    | null_syntax::TokenKind::Semicolon
            )
        })
        .map(|(_, s)| s)
        .collect();
    v.sort();
    v
}

#[test]
fn the_slice_files_come_out_in_rnas_order_and_lose_nothing() {
    for text in slice_files() {
        let out = canonical(&text, schema());
        assert_eq!(tokens(&out), tokens(&text), "{out}");
        assert_eq!(
            canonical(&out, schema()),
            out,
            "formatting twice changed it:\n{out}"
        );
        // Still the same project to the checker.
        let before = check(schema(), &[(FileId(0), &parse(&text))]).len();
        let after = check(schema(), &[(FileId(0), &parse(&out))]).len();
        assert_eq!(before, after, "{out}");
    }
}

#[test]
fn an_object_in_rnas_order() {
    let text = "\
object @pier \"Pier\" {
  // Where it stands.
  location [0m -4m 0.35m]
  scale [1 1 1]
  material @oak
  rotation-euler [0deg 0deg 12deg] // turned
  data mesh @deck
}
";
    assert_eq!(
        canonical(text, schema()),
        "\
object @pier \"Pier\" {
  data mesh @deck
  material @oak
  // Where it stands.
  location [0m -4m 0.35m]
  rotation-euler [0deg 0deg 12deg] // turned
  scale [1 1 1]
}
"
    );
    // A line's `key=value`s too, and a one-line block.
    assert_eq!(
        canonical(
            "object @boat \"Boat\" location=[1m 0m 0m] parent=@pier { scale [2 2 2]; data mesh @hull }\n",
            schema()
        ),
        "object @boat \"Boat\" parent=@pier location=[1m 0m 0m] { data mesh @hull; scale [2 2 2] }\n"
    );
}

#[test]
fn forms_stand_where_their_rna_property_stands() {
    let scene = "\
scene @main \"Main\" {
  object @pier
  units metric
  frame-end 240
  camera @cam
  object @cam
  frame-start 1
}
";
    // RNA's Scene: camera, …, frame-start, frame-end, …, unit-settings (units), …, collection
    // (members, which keep their order).
    assert_eq!(
        canonical(scene, schema()),
        "\
scene @main \"Main\" {
  camera @cam
  frame-start 1
  frame-end 240
  units metric
  object @pier
  object @cam
}
"
    );
    let units =
        "project \"P\" {\n  scene @main\n  units { length m; scale 0.01; system metric }\n}\n";
    assert_eq!(
        canonical(units, schema()),
        "project \"P\" {\n  units { system metric; scale 0.01; length m }\n  scene @main\n}\n"
    );
}

#[test]
fn a_node_keeps_its_place_and_orders_its_inputs() {
    let text = "\
material @m \"M\" {
  nodes {
    node @out output-material { surface @bsdf.bsdf }
    node @bsdf bsdf-principled { roughness 0.5; base-color [1 1 1 1] }
  }
  roughness 0.5
}
";
    let out = canonical(text, schema());
    assert!(
        out.find("@out").unwrap() < out.find("node @bsdf").unwrap(),
        "{out}"
    );
    assert!(
        out.contains("{ base-color [1 1 1 1]; roughness 0.5 }"),
        "{out}"
    );
    // RNA's Material lists `node-tree`, where `nodes` stands, before `roughness`.
    assert!(
        out.find("nodes {").unwrap() < out.find("  roughness 0.5\n").unwrap(),
        "{out}"
    );
}

/// `text` with each multi-line block's lines in another order, by `seed`; lines of one name keep
/// their order among themselves, as canonical order keeps them.
fn shuffled(text: &str, seed: u64) -> String {
    let file = parse(text);
    let mut out = String::new();
    let mut at = 0;
    let mut state = seed | 1;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for node in file.document().nodes() {
        let Some(block) = node.block() else { continue };
        if !block.text().contains('\n') {
            continue;
        }
        let children: Vec<Node<'_>> = Node(node.0).children().collect();
        // Comments and lines commented out stay where they are, so blocks holding any are left.
        if children.len() < 2 || block.text().contains('/') {
            continue;
        }
        let first = children[0].span().start;
        let last = children.last().unwrap().span().end;
        // A random order of the names, each name's lines in their order.
        let mut names: Vec<String> = Vec::new();
        for c in &children {
            if !names.contains(&c.name().to_string()) {
                names.push(c.name().to_string());
            }
        }
        for i in (1..names.len()).rev() {
            let j = (next() % (i as u64 + 1)) as usize;
            names.swap(i, j);
        }
        let mut lines = Vec::new();
        for n in &names {
            for c in children.iter().filter(|c| c.name() == *n) {
                lines.push(text[c.span().start..c.span().end].trim_end().to_string());
            }
        }
        out.push_str(&text[at..first]);
        out.push_str(&lines.join("\n  "));
        out.push('\n');
        at = last;
    }
    out.push_str(&text[at..]);
    out
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn a_shuffled_slice_file_comes_back_in_canonical_order(seed in any::<u64>(), pick in any::<prop::sample::Index>()) {
        let files = slice_files();
        let text = &files[pick.index(files.len())];
        let mixed = shuffled(text, seed);
        prop_assert_eq!(tokens(&mixed), tokens(text));
        prop_assert_eq!(canonical(&mixed, schema()), canonical(text, schema()));
    }
}

#[test]
fn a_node_is_ordered_by_its_trees_node() {
    // From 1b-03c's review: `invert` names a shader node and a compositor node; a material's
    // node is the shader tree's, whose declaration lists `fac` before `color`.
    let project =
        Schema::from_json(include_str!("../../../schema/null-schema-project.json")).unwrap();
    let text = "material @m \"M\" {\n  nodes {\n    node @inv invert { color [1 1 1 1]; fac 0.5 }\n  }\n}\n";
    let out = canonical(text, &project);
    assert!(out.contains("{ fac 0.5; color [1 1 1 1] }"), "{out}");
}

#[test]
fn a_cycles_block_is_ordered_by_cycles_struct() {
    // S70: an add-on's struct is written as a block, ordered as that struct is; the block stands
    // where the attached property stands, after the scene's own properties.
    let project =
        Schema::from_json(include_str!("../../../schema/null-schema-project.json")).unwrap();
    let text = "scene @s \"S\" {\n  cycles { use-denoising; samples 64; device gpu }\n  frame-start 1\n}\n";
    let out = canonical(text, &project);
    assert_eq!(canonical(&out, &project), out);
    assert!(
        out.find("frame-start").unwrap() < out.find("cycles").unwrap(),
        "{out}"
    );
    let block = &out[out.find("cycles").unwrap()..];
    assert!(
        block.find("device").unwrap() < block.find("samples").unwrap(),
        "{out}"
    );
}
