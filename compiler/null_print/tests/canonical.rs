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
fn a_comment_after_an_opening_brace_or_a_semicolon_stays_on_its_line() {
    // From the 1c-01e review.
    let text = "object @a \"A\" { // the deck\n  scale [1 1 1]\n  location [0m 0m 0m]\n}\n";
    assert_eq!(
        canonical(text, schema()),
        "object @a \"A\" { // the deck\n  location [0m 0m 0m]\n  scale [1 1 1]\n}\n"
    );
    let semicolon =
        "object @a \"A\" {\n  scale [1 1 1]; // after the scale\n  location [0m 0m 0m]\n}\n";
    let out = canonical(semicolon, schema());
    assert_eq!(canonical(&out, schema()), out);
    assert_eq!(tokens(&out), tokens(semicolon), "{out}");
}

#[test]
fn units_keep_the_specs_short_order() {
    // The lead's call on the 1c-01e review: S35's form is already canonical.
    let text = "project \"P\" {\n  units metric scale=1 length=m rotation=deg\n  scene @main\n}\n";
    assert_eq!(canonical(text, schema()), text);
    let shuffled =
        "project \"P\" {\n  units metric rotation=deg length=m scale=1\n  scene @main\n}\n";
    assert_eq!(canonical(shuffled, schema()), text);
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
        // Block comments, lines commented out and a comment after `{` stay where they are, so
        // blocks holding any are left; a comment on its own line travels with the line below.
        let opening = &block.text()[..block.text().find('\n').unwrap_or(0)];
        if children.len() < 2
            || block.text().contains("/*")
            || block.text().contains("/-")
            || opening.contains("//")
        {
            continue;
        }
        let first = block.span().start + block.text().find('\n').unwrap_or(0) + 1;
        let last = children.last().unwrap().span().end;
        let mut units: Vec<(String, String)> = Vec::new();
        let mut prev = first;
        for c in &children {
            let unit = text[prev..c.span().end]
                .trim_matches('\n')
                .trim()
                .to_string();
            units.push((c.name().to_string(), unit));
            prev = c.span().end;
        }
        // A random order of the names, each name's lines in their order.
        let mut names: Vec<String> = Vec::new();
        for (n, _) in &units {
            if !names.contains(n) {
                names.push(n.clone());
            }
        }
        for i in (1..names.len()).rev() {
            let j = (next() % (i as u64 + 1)) as usize;
            names.swap(i, j);
        }
        let mut lines = Vec::new();
        for n in &names {
            for (_, u) in units.iter().filter(|(m, _)| m == n) {
                lines.push(u.clone());
            }
        }
        out.push_str(&text[at..first]);
        out.push_str("  ");
        out.push_str(&lines.join("\n  "));
        out.push('\n');
        at = last;
    }
    out.push_str(&text[at..]);
    out
}

/// Comments on lines of their own, which travel with the line below them.
const COMMENTED: &str = "\
object @pier \"Pier\" {
  // Where it stands.
  location [0m -4m 0.35m]
  scale [1 1 1]
  // The deck's mesh,
  // shared with nothing.
  data mesh @deck
  rotation-euler [0deg 0deg 12deg] // turned
}

camera @lens \"Lens\" {
  clip-end 1000m
  // Short for a harbour.
  lens 35mm
  type persp
}
";

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn a_shuffled_slice_file_comes_back_in_canonical_order(seed in any::<u64>(), pick in any::<prop::sample::Index>()) {
        // The slice's files, and one whose blocks hold comments on lines of their own.
        let mut files = slice_files();
        files.push(COMMENTED.to_string());
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

#[test]
fn an_empty_pointer_round_trips() {
    // Sau's task 1c-02j: `parent none`, as write-back writes it, checks clean and comes back from
    // the formatter and the canonical order unchanged.
    let text = "object @lamp \"Lamp\" {\n  parent none\n  location [0m -8m 2.5m]\n}\n";
    let parsed = parse(text);
    assert_eq!(check(schema(), &[(FileId(0), &parsed)]), vec![]);
    assert_eq!(null_print::format(text), text);
    assert_eq!(canonical(text, schema()), text);
}
