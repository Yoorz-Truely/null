//! Components, parameters, placements, patterns, conditions, overrides and countdown recursion
//! (spec, section 8; Sau's task 1d-03b): the spec's examples check clean, and each mistake is an
//! error, most with a fix. Snapshots hold every message.

use null_check::{Schema, check};
use null_syntax::{Diagnostic, FileId, Parse, SourceMap, parse};

const PROJECT: &str = include_str!("../../../schema/null-schema-project.json");

fn schema() -> &'static Schema {
    static S: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
    S.get_or_init(|| Schema::from_json(PROJECT).unwrap())
}

/// Checks `texts` as one project; the map, for rendering, and the problems.
fn run(texts: &[(&str, &str)]) -> (SourceMap, Vec<Diagnostic>) {
    let mut map = SourceMap::new();
    let parses: Vec<(FileId, Parse)> = texts
        .iter()
        .map(|(name, text)| (map.add(*name, *text), parse(text)))
        .collect();
    for (id, p) in &parses {
        let syntax = null_syntax::diagnostics(p, *id);
        assert!(syntax.is_empty(), "{}", map.render_all(&syntax));
    }
    let files: Vec<_> = parses.iter().map(|(i, p)| (*i, p)).collect();
    let found = check(schema(), &files);
    (map, found)
}

/// The rendered problems of `texts`.
fn problems(texts: &[(&str, &str)]) -> String {
    let (map, found) = run(texts);
    map.render_all(&found)
}

mod examples;
use examples::{BRANCH, MATERIALS, PIER, PLANK};

#[test]
fn the_spec_s_pier_and_tree_check_clean() {
    let tree = "collection @woods \"Woods\" {\n  use @branch @tree { depth 3 }\n}\n";
    let p = problems(&[
        ("materials.noll", MATERIALS),
        ("components/plank.noll", PLANK),
        ("objects/pier.noll", PIER),
        ("components/branch.noll", BRANCH),
        ("objects/woods.noll", tree),
    ]);
    assert_eq!(p, "");
}

/// The pier, the tree and the materials, with `edits` (from, to) made to the file named.
fn project_with(file: &str, edits: &[(&str, &str)]) -> String {
    let mut files = vec![
        ("materials.noll", MATERIALS.to_string()),
        ("components/plank.noll", PLANK.to_string()),
        ("objects/pier.noll", PIER.to_string()),
        ("components/branch.noll", BRANCH.to_string()),
        (
            "objects/woods.noll",
            "collection @woods \"Woods\" {\n  use @branch @tree { depth 3 }\n}\n".to_string(),
        ),
    ];
    for (name, text) in &mut files {
        if *name == file {
            for (from, to) in edits {
                assert!(text.contains(from), "{name} has no {from}");
                *text = text.replacen(from, to, 1);
            }
        }
    }
    let refs: Vec<(&str, &str)> = files.iter().map(|(n, t)| (*n, t.as_str())).collect();
    problems(&refs)
}

#[test]
fn a_component_s_own_mistakes() {
    // No root object; a line that is no construct; a parameter twice, misspelt types, a default
    // off its list, a countdown that is no whole number, a word with no list.
    let p = problems(&[(
        "components.noll",
        r#"component @empty "Empty" {
  param size 1m
}

component @crate "Crate" {
  param size 1m
  param size 2m
  param height lenght
  param state molten { one-of [solid liquid] }
  param depth 1.5 { countdown }
  param finish oak
  param tint fancy
  param skin material @nowhere
  object @crate "Crate" {}
  materail @oak
}
"#,
    )]);
    insta::assert_snapshot!("component", p);
}

#[test]
fn a_placement_s_mistakes() {
    let p = problems(&[
        ("materials.noll", MATERIALS),
        ("components/plank.noll", PLANK),
        (
            "objects/pier.noll",
            r#"collection @pier "Pier" {
  use @plnk @a
  use @plank
  use @plank @b { state lqiuid; lenght 2m; length 3 }
  use @plank @c count=2.5
  use @plank @d each=[1m 2m]
  use @plank @e as=size
  use @plank @f count=2 each=[1m] as=size
  use @plank @g cnt=3
  use @plank @h { state 3 }
}
"#,
        ),
    ]);
    insta::assert_snapshot!("placement", p);
}

#[test]
fn a_required_parameter_must_be_given() {
    let p = problems(&[(
        "railing.noll",
        r#"component @rail "Rail" {
  param length length
  param state word { one-of [new worn] }
  object @rail "Rail" { scale [length/1m 1 1] }
}

collection @deck "Deck" {
  use @rail @r1 { state worn }
  use @rail @r2 { length 2m; state new }
}
"#,
    )]);
    insta::assert_snapshot!("required", p);
}

#[test]
fn expressions_are_checked_for_their_units_and_names() {
    let p = project_with(
        "objects/pier.noll",
        &[("n*25cm", "n*25cm*2m"), ("0.35m]", "n-1]")],
    ) + &project_with(
        "components/plank.noll",
        &[("length/1m", "lenght/1m"), ("size=1m", "size=length+1")],
    );
    insta::assert_snapshot!("expressions", p);
}

#[test]
fn conditions_are_quoted_comparisons_of_parameters() {
    let p = project_with(
        "components/plank.noll",
        &[("if \"state == solid\"", "if \"state == molten\"")],
    ) + &project_with(
        "components/plank.noll",
        &[("if \"state == solid\"", "if \"state == 3m\"")],
    ) + &project_with(
        "components/plank.noll",
        &[("if \"state == solid\"", "if \"stat == solid\"")],
    ) + &project_with(
        "components/plank.noll",
        &[(
            "if \"state == solid\" { material @weathered-oak }\n    else {",
            "else {",
        )],
    );
    insta::assert_snapshot!("conditions", p);
}

#[test]
fn overrides_and_copies_are_found_by_their_paths() {
    let p = problems(&[
        ("materials.noll", MATERIALS),
        ("components/plank.noll", PLANK),
        ("components/branch.noll", BRANCH),
        (
            "objects/pier.noll",
            r#"collection @pier "Pier" {
  use @plank @plank count=20
  use @branch @tree
  override @plank.21 { state liquid }
  override @plank { state liquid }
  override @plank.7/@plank-mesch { }
  override @plank.7/@plank-mesh { primitive cube size=2m }
  override @tree.2 { length 2m }
  override @tree/@left/@left { length 40cm }
  override @tree/@left/@leftt { length 40cm }
  override @plank.7/@fluid { }
}

object @lamp "Lamp" { parent @plank.3 }
object @other "Other" { data mesh @plank.3 }
object @third "Third" { data mesh @plank.3/@plank-mesh }
"#,
        ),
    ]);
    insta::assert_snapshot!("overrides", p);
}

#[test]
fn recursion_counts_down_or_is_an_error() {
    // No countdown parameter; a value that does not get smaller; a placement outside the `if`
    // that ends it.
    let p = problems(&[(
        "trees.noll",
        r#"component @loop "Loop" {
  object @loop "Loop" {}
  use @loop @again
}

component @grow "Grow" {
  param depth 3 { countdown }
  object @grow "Grow" {}
  if "depth > 0" {
    use @grow @up { depth "depth + 1" }
  }
  use @grow @side { depth "depth - 1" }
}
"#,
    )]);
    insta::assert_snapshot!("recursion", p);
}

#[test]
fn every_construct_checks_clean_where_it_is_right() {
    // Each kind of parameter, used where its kind goes; a pattern over a list; a placement in
    // a component that is not recursion; an override of a copy's own mesh; conditions in a
    // component's block and in a datablock's.
    let p = problems(&[
        ("materials.noll", MATERIALS),
        (
            "components/post.noll",
            r#"component @post "Post" {
  param height length
  param hidden off
  param finish material @weathered-oak
  param count 3
  param angle 15deg
  param kind round { one-of [round square] }
  object @post "Post" {
    data mesh @post-mesh
    material finish
    hide-render hidden
    pass-index count
    rotation-euler [0deg 0deg angle]
    if "kind == square" { scale [1 1 height/1m] }
    else { scale [0.5 0.5 height/1m] }
  }
  mesh @post-mesh "Post" { primitive cylinder radius=height*0.05 depth=height }
}

component @fence "Fence" {
  param posts 4
  object @fence "Fence" {}
  if "posts > 2" {
    use @post @first { height 1m; parent @fence }
  }
  use @post @row count=posts { height "(n - 1) * 0.5m + 1m"; location [n*2m 0m 0m] }
}
"#,
        ),
        (
            "objects/yard.noll",
            r#"collection @yard "Yard" {
  use @fence @north { posts 5 }
  use @post @gate each=[1.2m 1.5m] as=size { height size; kind square }
  override @north/@row.2 { angle 30deg }
  override @gate.1/@post-mesh { primitive cylinder radius=10cm }
}

object @sign "Sign" { parent @gate.2 }
"#,
        ),
    ]);
    assert_eq!(p, "");
}
