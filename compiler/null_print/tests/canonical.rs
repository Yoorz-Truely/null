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

const PROJECT: &str = include_str!("../../../schema/null-schema-project.json");

fn project_schema() -> &'static Schema {
    static S: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
    S.get_or_init(|| Schema::from_json(PROJECT).unwrap())
}

/// Every fenced example of the spec, those of forms not built yet included (task 1d-02b).
fn spec_examples() -> Vec<String> {
    let spec = include_str!("../../../docs/spec/null-v0.md");
    spec.split("```null")
        .skip(1)
        .map(|b| {
            let body = b.split_once('\n').map_or("", |x| x.1);
            format!("{}\n", &body[..body.find("\n```").unwrap()])
        })
        .collect()
}

#[test]
fn every_spec_example_is_canonical_twice_over_and_loses_nothing() {
    // Task 1d-02b: every construct (sub-objects, node trees, zones, groups, actions, drivers, data
    // files, components) comes out of `canonical` with every word and comment it had, and a
    // second pass changes nothing.
    for (i, text) in spec_examples().iter().enumerate() {
        let once = canonical(text, project_schema());
        assert_eq!(tokens(&once), tokens(text), "example {}:\n{once}", i + 1);
        assert_eq!(
            canonical(&once, project_schema()),
            once,
            "example {} is not stable",
            i + 1
        );
    }
}

#[test]
fn sub_objects_zones_and_f_curves_are_ordered_by_their_own_structs() {
    let schema = project_schema();
    // A bevel's settings in `BevelModifier`'s order, whatever order they are written in.
    let object =
        "object @o \"O\" {\n  modifier @b bevel \"B\" {\n    segments 3\n    width 4mm\n  }\n}\n";
    let out = canonical(object, schema);
    assert!(
        out.find("width").unwrap() < out.find("segments").unwrap(),
        "{out}"
    );
    // A zone: its items first, then its input lines, then `output`, then its nodes.
    let group = "geometry @g \"G\" {\n  nodes {\n    zone @loop repeat {\n      node @a transform\n      output { geometry @a.geometry }\n      iterations 4\n      repeat-item @geometry geometry \"Geometry\"\n    }\n  }\n}\n";
    let out = canonical(group, schema);
    let at = |w: &str| out.find(w).unwrap_or_else(|| panic!("{w} in {out}"));
    assert!(at("repeat-item") < at("iterations"), "{out}");
    assert!(at("iterations") < at("output {"), "{out}");
    assert!(at("output {") < at("node @a"), "{out}");
    // A group's `layout` last, after its `nodes`, its interface lines kept in order.
    let group = "geometry @g \"G\" {\n  layout { @a [0 0] }\n  nodes {\n    node @a transform\n  }\n  output @out geometry \"Out\"\n  input @in geometry \"In\"\n}\n";
    let out = canonical(group, schema);
    assert!(
        out.find("output @out").unwrap() < out.find("input @in").unwrap(),
        "{out}"
    );
    assert!(
        out.find("nodes {").unwrap() < out.find("layout {").unwrap(),
        "{out}"
    );
    // An f-curve: its settings in `FCurve`'s order, its keys as written.
    let action = "action @a \"A\" {\n  slot @s object \"S\" {\n    fcurve location index=2 {\n      key 12 1m\n      key 1 0m\n      extrapolation linear\n    }\n  }\n}\n";
    let out = canonical(action, schema);
    assert!(
        out.find("extrapolation").unwrap() < out.find("key 12").unwrap(),
        "{out}"
    );
    assert!(
        out.find("key 12").unwrap() < out.find("key 1 ").unwrap(),
        "{out}"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn a_shuffled_spec_example_is_stable_and_loses_nothing(seed in any::<u64>(), pick in any::<prop::sample::Index>()) {
        // Generated trees (task 1d-02b): every example of the spec with its blocks' lines in
        // another order.
        let files = spec_examples();
        let text = &files[pick.index(files.len())];
        let mixed = shuffled(text, seed);
        prop_assert_eq!(tokens(&mixed), tokens(text));
        // Formatting twice changes nothing, and every word and comment survives. Lines whose
        // order is meaning (a tree's nodes, a group's interface, keys, a component's datablocks)
        // keep the shuffled order, so the result need not be the example's own.
        let once = canonical(&mixed, project_schema());
        prop_assert_eq!(tokens(&once), tokens(text));
        prop_assert_eq!(canonical(&once, project_schema()), once);
    }
}

#[test]
fn a_line_of_other_whitespace_is_a_blank_line() {
    // Found by the `format` fuzz target (task 1d-02d): a line holding a vertical tab was no
    // blank line to the canonical pass, then one once the layout took the tab, so a second
    // pass gave other text.
    for text in ["i{\n\u{b}\nm}", "i {\n\u{c}\u{a0}\n  m\n}\n"] {
        let once = canonical(text, schema());
        assert_eq!(canonical(&once, schema()), once, "{text:?}");
        assert!(!once.contains("\n\n"), "{text:?} gave {once:?}");
    }
}

#[test]
fn a_groups_interface_comes_before_its_nodes() {
    // Section 10: a group's interface, then its `nodes`, then `layout`; a zone's items, its
    // input node's lines, `output`, then its nodes (the 1d-08a review found the interface last).
    let text = "geometry @g \"G\" {\n  nodes {\n    node @in group-input\n    node @out group-output { geometry @in.geometry }\n    zone @loop repeat {\n      node @x set-position\n      output { geometry @x.geometry }\n      repeat-item @geometry geometry \"Geometry\"\n      iterations 4\n    }\n  }\n  layout { @in [0 0] }\n  input @geometry geometry \"Geometry\"\n  output @geometry-out geometry \"Geometry\"\n}\n";
    let want = "geometry @g \"G\" {\n  input @geometry geometry \"Geometry\"\n  output @geometry-out geometry \"Geometry\"\n  nodes {\n    node @in group-input\n    node @out group-output { geometry @in.geometry }\n    zone @loop repeat {\n      repeat-item @geometry geometry \"Geometry\"\n      iterations 4\n      output { geometry @x.geometry }\n      node @x set-position\n    }\n  }\n  layout { @in [0 0] }\n}\n";
    let project =
        Schema::from_json(include_str!("../../../schema/null-schema-project.json")).unwrap();
    assert_eq!(canonical(text, &project), want);
    assert_eq!(canonical(want, &project), want);
}
