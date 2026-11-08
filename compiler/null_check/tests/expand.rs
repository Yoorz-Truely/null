//! Expansion (spec, section 8; Sau's task 1d-03c): placements made into the datablocks they
//! make, each copy with its handle; the expanded project checks clean, expands the same every
//! time, and a recursion that does not count down or an expansion past the limit is an error.

use null_check::{Expanded, Schema, check, check_expanded, expand};
use null_syntax::{Diagnostic, FileId, Parse, SourceMap, parse};

mod examples;
use examples::{BRANCH, MATERIALS, PIER, PLANK};

const PROJECT: &str = include_str!("../../../schema/null-schema-project.json");

fn schema() -> &'static Schema {
    static S: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
    S.get_or_init(|| Schema::from_json(PROJECT).unwrap())
}

/// `texts` checked as written (clean, unless `clean` is false), then expanded: the files and
/// the expansion's problems, rendered.
fn run(texts: &[(&str, &str)], clean: bool) -> (Vec<Expanded>, String) {
    let mut map = SourceMap::new();
    let parses: Vec<(FileId, Parse)> = texts
        .iter()
        .map(|(name, text)| (map.add(*name, *text), parse(text)))
        .collect();
    let files: Vec<_> = parses.iter().map(|(i, p)| (*i, p)).collect();
    if clean {
        let found = check(schema(), &files);
        assert!(found.is_empty(), "{}", map.render_all(&found));
    }
    let (out, problems) = expand(schema(), &files);
    (out, map.render_all(&problems))
}

/// The expanded files' problems, rendered against the expanded texts.
fn expanded_problems(files: &[Expanded]) -> String {
    let mut map = SourceMap::new();
    let ids: Vec<Expanded> = files
        .iter()
        .map(|e| Expanded {
            file: map.add(format!("expanded-{}", e.file.0), e.text.clone()),
            text: e.text.clone(),
        })
        .collect();
    let found: Vec<Diagnostic> = check_expanded(schema(), &ids);
    map.render_all(&found)
}

fn pier() -> Vec<(&'static str, &'static str)> {
    vec![
        ("materials.noll", MATERIALS),
        ("components/plank.noll", PLANK),
        ("objects/pier.noll", PIER),
    ]
}

/// The block of the datablock `head` (`object @plank.7`) in `text`.
fn block<'t>(text: &'t str, head: &str) -> &'t str {
    let at = text
        .find(&format!("\n{head} "))
        .unwrap_or_else(|| panic!("no `{head}` in\n{text}"));
    let rest = &text[at + 1..];
    &rest[..rest.find("\n}\n").map_or(rest.len(), |e| e + 3)]
}

#[test]
fn the_pier_expands_to_twenty_planks_with_their_handles() {
    let (out, problems) = run(&pier(), true);
    assert_eq!(problems, "");
    let text = &out[2].text;
    // Twenty members, `@plank.1` to `@plank.20`, in order, and nothing else is a plank object.
    let members: Vec<&str> = text
        .lines()
        .filter_map(|l| l.strip_prefix("  object "))
        .collect();
    let want: Vec<String> = (1..=20).map(|n| format!("@plank.{n}")).collect();
    assert_eq!(members, want);
    assert_eq!(text.matches("\nobject @plank.").count(), 20, "{text}");
    assert_eq!(text.matches("\nmesh @plank.").count(), 20, "{text}");
    assert!(
        !text.contains("use ") && !text.contains("override "),
        "{text}"
    );
    // Each at n*25cm, its own mesh 2.4 m long.
    assert_eq!(
        block(text, "object @plank.1"),
        "object @plank.1 \"Plank\" {\n  data mesh @plank.1/@plank-mesh\n  material @weathered-oak\n  location [0m 0.25m 0.35m]\n}\n"
    );
    assert!(block(text, "object @plank.20").contains("location [0m 5m 0.35m]"));
    assert_eq!(
        block(text, "mesh @plank.7/@plank-mesh"),
        "mesh @plank.7/@plank-mesh \"Plank\" {\n  primitive cube size=1m scale=[2.4 0.2 0.04]\n}\n"
    );
    // Plank 7 is liquid: water, and the fluid modifier.
    assert_eq!(
        block(text, "object @plank.7"),
        "object @plank.7 \"Plank\" {\n  data mesh @plank.7/@plank-mesh\n  material @water\n  modifier @fluid fluid {\n    fluid-type flow\n  }\n  location [0m 1.75m 0.35m]\n}\n"
    );
    // Components are gone; the other files are as written.
    assert_eq!(out[0].text, MATERIALS);
    assert_eq!(out[1].text, "");
}

#[test]
fn the_expanded_project_checks_clean() {
    let (out, _) = run(&pier(), true);
    assert_eq!(expanded_problems(&out), "");
}

#[test]
fn expanding_twice_gives_the_same_tree() {
    let (first, _) = run(&pier(), true);
    let (second, _) = run(&pier(), true);
    assert_eq!(first, second);
    // And the expanded project, expanded again, is itself.
    let texts: Vec<(&str, &str)> = first.iter().map(|e| ("f.noll", e.text.as_str())).collect();
    let (again, problems) = run(&texts, false);
    assert_eq!(problems, "");
    let a: Vec<&str> = again.iter().map(|e| e.text.as_str()).collect();
    let b: Vec<&str> = first.iter().map(|e| e.text.as_str()).collect();
    assert_eq!(a, b);
}

#[test]
fn the_tree_counts_down_to_fifteen_branches() {
    let woods = "collection @woods \"Woods\" {\n  use @branch @tree { depth 3 }\n}\n";
    let files = [
        ("materials.noll", MATERIALS),
        ("components/branch.noll", BRANCH),
        ("objects/woods.noll", woods),
    ];
    let (out, problems) = run(&files, true);
    assert_eq!(problems, "");
    let text = &out[2].text;
    // 1 + 2 + 4 + 8 copies, each a branch and its segment.
    assert_eq!(text.matches("\nobject @tree").count(), 15, "{text}");
    assert_eq!(text.matches("\nmesh @tree").count(), 15, "{text}");
    assert!(
        text.contains("\n  object @tree/@left/@right/@left\n"),
        "{text}"
    );
    // Each copy's values are its own: the length shrinks by 0.7 a level, the parent is the copy
    // that placed it.
    assert_eq!(
        block(text, "object @tree/@left/@right"),
        "object @tree/@left/@right \"Branch\" {\n  data mesh @tree/@left/@right/@segment\n  parent @tree/@left\n  location [0m 0m 0.7m]\n  rotation-euler [0deg -35deg 0deg]\n}\n"
    );
    assert!(
        block(text, "mesh @tree/@left/@right/@segment").contains("radius=0.0245m depth=0.49m"),
        "{text}"
    );
    assert_eq!(expanded_problems(&out), "");
}

#[test]
fn a_recursion_that_does_not_count_down_is_an_error() {
    // The checker refuses it as written; the expansion refuses it too, and ends.
    let branch = BRANCH.replace("depth \"depth - 1\"", "depth depth");
    let woods = "collection @woods \"Woods\" {\n  use @branch @tree { depth 3 }\n}\n";
    let (_, problems) = run(
        &[
            ("materials.noll", MATERIALS),
            ("components/branch.noll", &branch),
            ("objects/woods.noll", woods),
        ],
        false,
    );
    assert!(problems.contains("does not count down"), "{problems}");
    // A component with no countdown placing itself.
    let ring = "component @ring \"Ring\" {\n  object @ring \"Ring\" {}\n  use @ring @inner {}\n}\ncollection @c \"C\" {\n  use @ring @r {}\n}\n";
    let (_, problems) = run(&[("ring.noll", ring)], false);
    assert!(problems.contains("does not count down"), "{problems}");
}

#[test]
fn an_expansion_past_a_million_datablocks_is_an_error() {
    let pier = PIER.replace("count=20", "count=500001");
    let (_, problems) = run(
        &[
            ("materials.noll", MATERIALS),
            ("components/plank.noll", PLANK),
            ("objects/pier.noll", &pier),
        ],
        false,
    );
    assert!(problems.contains("past 1000000 datablocks"), "{problems}");
    assert!(problems.contains("objects/pier.noll:2"), "{problems}");
}

#[test]
fn patterns_over_lists_overrides_of_paths_and_parameters_from_parameters() {
    let post = r#"component @post "Post" {
  param height length
  param finish material @weathered-oak
  param kind round { one-of [round square] }
  object @post "Post" {
    data mesh @post-mesh
    material finish
    if "kind == square" { scale [1 1 height/1m] }
    else { scale [0.5 0.5 height/1m] }
  }
  mesh @post-mesh "Post" { primitive cylinder radius=height*0.05 depth=height }
}

component @fence "Fence" {
  param posts 4
  object @fence "Fence" {}
  use @post @row count=posts { height "(n - 1) * 0.5m + 1m"; location [n*2m 0m 0m]; parent @fence }
}
"#;
    let yard = r#"collection @yard "Yard" {
  use @fence @north { posts 3 }
  use @post @gate each=[1.2m 1.5m] as=size { height size; kind square }
  override @north/@row.2 { finish @water }
  override @gate.1/@post-mesh { primitive cylinder radius=10cm }
}
"#;
    let (out, problems) = run(
        &[
            ("materials.noll", MATERIALS),
            ("post.noll", post),
            ("yard.noll", yard),
        ],
        true,
    );
    assert_eq!(problems, "");
    let text = &out[2].text;
    let members: Vec<&str> = text
        .lines()
        .filter_map(|l| l.strip_prefix("  object "))
        .collect();
    assert_eq!(
        members,
        [
            "@north",
            "@north/@row.1",
            "@north/@row.2",
            "@north/@row.3",
            "@gate.1",
            "@gate.2"
        ]
    );
    assert_eq!(
        block(text, "object @north/@row.3"),
        "object @north/@row.3 \"Post\" {\n  data mesh @north/@row.3/@post-mesh\n  material @weathered-oak\n  scale [0.5 0.5 2]\n  location [6m 0m 0m]\n  parent @north\n}\n"
    );
    assert!(
        block(text, "object @north/@row.2").contains("material @water"),
        "{text}"
    );
    assert!(
        block(text, "object @gate.2").contains("scale [1 1 1.5]"),
        "{text}"
    );
    assert_eq!(
        block(text, "mesh @gate.1/@post-mesh"),
        "mesh @gate.1/@post-mesh \"Post\" {\n  primitive cylinder radius=10cm\n}\n"
    );
    assert_eq!(expanded_problems(&out), "");
}

#[test]
fn a_mistake_in_the_expanded_project_is_found_at_its_copy() {
    // The expanded project is checked as any other: a reference a copy's line makes to nothing
    // is an error, for each copy that makes it.
    let woods = "collection @woods \"Woods\" {\n  use @branch @tree { depth 1 }\n}\n";
    let (out, _) = run(
        &[("m.noll", MATERIALS), ("b.noll", BRANCH), ("w.noll", woods)],
        true,
    );
    let bad = out[2].text.replace("parent @tree\n", "parent @oak-tree\n");
    let found = expanded_problems(&[Expanded {
        file: FileId(0),
        text: bad,
    }]);
    assert_eq!(
        found.matches("no object is called `@oak-tree`").count(),
        2,
        "{found}"
    );
}

/// A small component with a length and a tilt, placed by `yard`.
const SIGN: &str = r#"component @sign "Sign" {
  param length 1m
  param tilt 0deg
  object @sign "Sign" {
    if "length*3 == 1m" { hide-render }
    if "tilt > 20deg" { hide-viewport }
    rotation-euler [0deg tilt 0deg]
    scale [length/1m 1 1]
  }
}
"#;

fn signs(yard: &str) -> (String, String) {
    let (out, problems) = run(&[("sign.noll", SIGN), ("yard.noll", yard)], false);
    (out[1].text.clone(), problems)
}

#[test]
fn a_parameter_stays_exact_through_its_placement() {
    // A third of a metre, given by the placement, is a third, not 60 digits of it.
    let (text, problems) =
        signs("collection @yard \"Yard\" {\n  use @sign @a { length \"1m/3\" }\n}\n");
    assert_eq!(problems, "");
    let a = block(&text, "object @a");
    assert!(a.contains("  hide-render\n"), "{a}");
    assert!(
        a.contains(&format!("scale [0.{} 1 1]", "3".repeat(60))),
        "{a}"
    );
}

#[test]
fn a_condition_compares_one_kind_in_one_unit() {
    let (text, problems) = signs("collection @yard \"Yard\" {\n  use @sign @a { tilt 30deg }\n}\n");
    assert_eq!(problems, "");
    assert!(
        block(&text, "object @a").contains("hide-viewport"),
        "{text}"
    );
    // Degrees against radians is no exact comparison.
    let sign = SIGN.replace("tilt > 20deg", "tilt > 1rad");
    let (_, problems) = run(
        &[
            ("sign.noll", &sign),
            (
                "yard.noll",
                "collection @yard \"Yard\" {\n  use @sign @a { tilt 30deg }\n}\n",
            ),
        ],
        false,
    );
    assert!(
        problems.contains("degrees and radians do not compare exactly"),
        "{problems}"
    );
}

#[test]
fn degrees_survive_a_step_that_is_no_angle() {
    let (text, problems) =
        signs("collection @yard \"Yard\" {\n  use @sign @a { tilt 35deg*2m/1m }\n}\n");
    assert_eq!(problems, "");
    assert!(
        block(&text, "object @a").contains("rotation-euler [0deg 70deg 0deg]"),
        "{text}"
    );
}

#[test]
fn overrides_are_laid_in_layers_the_last_replacing() {
    // The component's line, then the placement's, then each override's in order: one line.
    let yard = r#"collection @yard "Yard" {
  use @sign @a count=2 { rotation-euler [1deg 0deg 0deg] }
  override @a.2 { rotation-euler [2deg 0deg 0deg] }
  override @a.2 { rotation-euler [3deg 0deg 0deg] }
}
"#;
    let (out, problems) = run(&[("sign.noll", SIGN), ("yard.noll", yard)], true);
    assert_eq!(problems, "");
    let text = &out[1].text;
    let one = block(text, "object @a.1");
    let two = block(text, "object @a.2");
    assert_eq!(one.matches("rotation-euler").count(), 1, "{one}");
    assert!(one.contains("rotation-euler [1deg 0deg 0deg]"), "{one}");
    assert_eq!(two.matches("rotation-euler").count(), 1, "{two}");
    assert!(two.contains("rotation-euler [3deg 0deg 0deg]"), "{two}");
    assert_eq!(expanded_problems(&out), "");
}

#[test]
fn a_value_beyond_exact_reach_is_an_error_never_a_panic() {
    // A denominator over u128::MAX / 10 prints; a product beyond i128 is an error.
    let (text, problems) = signs(
        "collection @yard \"Yard\" {\n  use @sign @a { length \"1m/40000000000000000000000000000000000001\" }\n}\n",
    );
    assert_eq!(problems, "");
    assert!(
        block(&text, "object @a").contains("scale [0.0000000000000000000000000000000000000"),
        "{text}"
    );
    let (_, problems) = signs(
        "collection @yard \"Yard\" {\n  use @sign @a { length \"1m/40000000000000000000000000000000000001 / 40000000000000000000000000000000000001\" }\n}\n",
    );
    assert!(
        problems.contains("too large to compute exactly"),
        "{problems}"
    );
}
