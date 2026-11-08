//! Expansion (spec, section 8; Sau's task 1d-03c): placements made into the datablocks they
//! make, each copy with its handle; the expanded project checks clean, expands the same every
//! time, and a recursion that does not count down or an expansion past the limit is an error.

use null_check::{Expanded, Schema, check, check_expanded, expand};
use null_syntax::{Diagnostic, FileId, Parse, SourceMap, parse};

use crate::examples::{BRANCH, MATERIALS, PIER, PLANK};

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

/// The expanded files' problems, each as `file:start..end code: message`, with its notes.
fn expanded_problems(files: &[Expanded]) -> String {
    let found: Vec<Diagnostic> = check_expanded(schema(), files);
    found
        .iter()
        .map(|d| {
            format!(
                "{}:{}..{} {}: {} {:?}\n",
                d.primary.file.0,
                d.primary.span.start,
                d.primary.span.end,
                d.code,
                d.message,
                d.notes
            )
        })
        .collect()
}

/// `texts` expanded and the expanded project checked, its problems rendered where they are
/// written in `texts` (task 1d-03h).
fn checked_through(texts: &[(&str, &str)]) -> String {
    let mut map = SourceMap::new();
    let parses: Vec<(FileId, Parse)> = texts
        .iter()
        .map(|(name, text)| (map.add(*name, *text), parse(text)))
        .collect();
    let files: Vec<_> = parses.iter().map(|(i, p)| (*i, p)).collect();
    let (out, problems) = expand(schema(), &files);
    assert_eq!(map.render_all(&problems), "");
    map.render_all(&check_expanded(schema(), &out))
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
    let found = expanded_problems(&[Expanded::unmapped(FileId(0), bad)]);
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

/// A rig whose spine bone reaches for a target: a sub-object (the constraint) of a sub-object
/// (the pose bone) of each copy (task 1d-03i).
const RIG: &str = r#"component @rig "Rig" {
  object @rig "Rig" {
    data armature @rig-bones
    bone @spine "Spine" {
      rotation-mode xyz
      constraint @reach kinematic "IK" {
        target @lid
        chain-count 2
      }
    }
  }
  armature @rig-bones "Rig"
}

object @lid "Lid"
"#;

fn rigs(overrides: &str) -> (Vec<Expanded>, String) {
    let yard =
        format!("collection @yard \"Yard\" {{\n  use @rig @rig count=2 {{}}\n{overrides}}}\n");
    run(&[("rig.noll", RIG), ("yard.noll", &yard)], false)
}

/// The checker's problems on the rigs as written, rendered.
fn rig_problems(overrides: &str) -> String {
    let yard =
        format!("collection @yard \"Yard\" {{\n  use @rig @rig count=2 {{}}\n{overrides}}}\n");
    let mut map = SourceMap::new();
    let texts = [("rig.noll", RIG), ("yard.noll", yard.as_str())];
    let parses: Vec<(FileId, Parse)> = texts
        .iter()
        .map(|(name, text)| (map.add(*name, *text), parse(text)))
        .collect();
    let files: Vec<_> = parses.iter().map(|(i, p)| (*i, p)).collect();
    map.render_all(&check(schema(), &files))
}

#[test]
fn an_override_of_a_bone_s_constraint_lands_on_that_constraint_alone() {
    let o = "  override @rig.2/@spine/@reach { chain-count 3 }\n";
    assert_eq!(rig_problems(o), "");
    let (out, problems) = rigs(o);
    assert_eq!(problems, "");
    let text = &out[1].text;
    let two = block(text, "object @rig.2");
    assert_eq!(
        two,
        "object @rig.2 \"Rig\" {\n  data armature @rig.2/@rig-bones\n  bone @spine \"Spine\" {\n    rotation-mode xyz\n    constraint @reach kinematic \"IK\" {\n      target @lid\n      chain-count 3\n    }\n  }\n}\n"
    );
    assert!(
        block(text, "object @rig.1").contains("chain-count 2"),
        "{text}"
    );
    // The bone itself, overridden, keeps its constraint; and the expanded rigs check clean.
    let (out, _) = rigs("  override @rig.1/@spine { rotation-mode zyx }\n");
    let one = block(&out[1].text, "object @rig.1");
    assert!(
        one.contains("rotation-mode zyx") && one.contains("chain-count 2"),
        "{one}"
    );
    assert_eq!(expanded_problems(&out), "");
}

#[test]
fn a_misspelt_step_of_a_sub_object_path_is_reported_with_the_nearest() {
    let p = rig_problems("  override @rig.2/@spine/@reech { chain-count 3 }\n");
    assert!(
        p.contains("the bone `@spine` has no sub-object `@reech`; did you mean `@reach`?"),
        "{p}"
    );
    let p = rig_problems("  override @rig.2/@spin/@reach { chain-count 3 }\n");
    assert!(p.contains("did you mean `@spine`?"), "{p}");
    // The override's lines are the constraint's own.
    let p = rig_problems("  override @rig.2/@spine/@reach { chain-cont 3 }\n");
    assert!(p.contains("`chain-cont`"), "{p}");
    assert!(p.contains("chain-count"), "{p}");
}

#[test]
fn a_component_s_datablocks_may_take_their_handles_from_their_names() {
    // S18 inside a component: `object "Plank"` is `@plank`, `mesh "Plank Mesh"` `@plank-mesh`,
    // and a datablock under a condition is the copy's too.
    let plank = r#"component @plank "Plank" {
  param state dry { one-of [dry wet] }
  object "Plank" {
    data mesh @plank-mesh
    if "state == wet" { material @water }
  }
  mesh "Plank Mesh" { primitive cube size=1m }
  if "state == wet" {
    material "Puddle" {}
  }
}
"#;
    let pier = "collection @pier \"Pier\" {\n  use @plank @plank count=2 {}\n  override @plank.2 { state wet }\n}\n";
    let (out, problems) = run(
        &[
            ("m.noll", MATERIALS),
            ("plank.noll", plank),
            ("pier.noll", pier),
        ],
        true,
    );
    assert_eq!(problems, "");
    let text = &out[2].text;
    assert!(
        text.contains("object @plank.1 \"Plank\" {\n  data mesh @plank.1/@plank-mesh\n}\n"),
        "{text}"
    );
    assert!(
        text.contains("mesh @plank.2/@plank-mesh \"Plank Mesh\""),
        "{text}"
    );
    assert!(
        text.contains("material @plank.2/@puddle \"Puddle\""),
        "{text}"
    );
    assert!(!text.contains("@plank.1/@puddle"), "{text}");
    assert_eq!(expanded_problems(&out), "");
}

#[test]
fn a_mistake_in_a_component_is_reported_once_at_its_line_naming_the_copies() {
    // Every plank names a material no file defines: one error, at the component's line.
    let plank = PLANK.replace("material @weathered-oak", "material @weathered-oaks");
    let found = checked_through(&[
        ("materials.noll", MATERIALS),
        ("components/plank.noll", &plank),
        ("objects/pier.noll", PIER),
    ]);
    assert_eq!(found.matches("error[").count(), 1, "{found}");
    assert!(
        found.starts_with(
            "components/plank.noll:6:36 error[null-handle]: no material is called `@weathered-oaks`"
        ),
        "{found}"
    );
    // Plank 7 is liquid, so 19 copies make the line.
    assert!(
        found.contains(
            "in 19 copies: `@plank.1`, `@plank.2`, `@plank.3`, `@plank.4`, `@plank.5` and 14 more"
        ),
        "{found}"
    );
    assert!(found.contains("fix: write `@weathered-oak`"), "{found}");
}

#[test]
fn a_mistake_is_mapped_exactly_where_it_is_written_and_a_computed_value_to_its_expression() {
    // A key written as it stands is found at its letters; a computed value, at its expression.
    let plank = PLANK
        .replace("primitive cube size=1m", "primitive cube edge=1m")
        .replace(
            "    data mesh @plank-mesh\n",
            "    data mesh @plank-mesh\n    pass-index \"length/1m\"\n",
        );
    let found = checked_through(&[
        ("materials.noll", MATERIALS),
        ("components/plank.noll", &plank),
        ("objects/pier.noll", PIER),
    ]);
    assert!(
        found.contains("components/plank.noll:14:20 error[null-unknown-property]"),
        "{found}"
    );
    assert!(found.contains("in 20 copies"), "{found}");
    assert!(
        found.contains("components/plank.noll:6:16 error[null-type]"),
        "{found}"
    );
    assert_eq!(found.matches("error[").count(), 2, "{found}");
}

#[test]
fn a_mistake_in_one_copy_s_override_or_outside_every_copy_is_found_where_written() {
    let pier = PIER.replace(
        "override @plank.7 { state liquid }",
        "override @plank.7 { state liquid; pass-index 2.5 }",
    );
    let found = checked_through(&[
        (
            "materials.noll",
            "material @weathered-oak \"Weathered Oak\" { roughnes 1 }\nmaterial @water \"Water\"\n",
        ),
        ("components/plank.noll", PLANK),
        ("objects/pier.noll", &pier),
    ]);
    // The override's value, in plank 7 alone.
    assert!(
        found.contains("objects/pier.noll:5:48 error[null-type]"),
        "{found}"
    );
    assert!(found.contains("in copy `@plank.7`"), "{found}");
    // A file's own datablock, as it stands.
    assert!(
        found.contains("materials.noll:1:43 error[null-unknown-property]"),
        "{found}"
    );
    assert_eq!(found.matches("error[").count(), 2, "{found}");
}

#[test]
fn the_expansion_s_own_problem_in_a_component_is_reported_once() {
    // A condition that cannot be decided, met in every copy: one problem, naming them.
    let plank = PLANK.replace("if \"state == solid\"", "if \"state > length\"");
    let (_, problems) = run(
        &[
            ("materials.noll", MATERIALS),
            ("components/plank.noll", &plank),
            ("objects/pier.noll", PIER),
        ],
        false,
    );
    assert_eq!(problems.matches("error[").count(), 1, "{problems}");
    assert!(problems.contains("components/plank.noll:6:5"), "{problems}");
    assert!(problems.contains("in 20 copies"), "{problems}");
}

/// A post with a stack of three modifiers, two copies of it in a yard.
const STACKED: &str = r#"component @post "Post" {
  object @post "Post" {
    data mesh @post-mesh
    modifier @a bevel { width 2mm }
    modifier @b array { count 3 }
    modifier @c subsurf { levels 1 }
  }
  mesh @post-mesh "Post" { primitive cube size=1m }
}
"#;

fn stacked(yard_lines: &str) -> (Vec<Expanded>, String) {
    let yard =
        format!("collection @yard \"Yard\" {{\n  use @post @x count=2 {{}}\n{yard_lines}}}\n");
    run(&[("post.noll", STACKED), ("yard.noll", &yard)], true)
}

#[test]
fn a_root_override_replaces_a_modifier_in_its_place_and_checks_clean() {
    // `run` asserts the project as written checks clean.
    let (out, problems) = stacked(
        "  override @x.2 { modifier @b array { count 7 } }\n  override @x.1 { modifier @d weld {} }\n",
    );
    assert_eq!(problems, "");
    let text = &out[1].text;
    let two = block(text, "object @x.2");
    let order: Vec<&str> = two
        .lines()
        .filter(|l| l.trim_start().starts_with("modifier "))
        .collect();
    assert_eq!(
        order,
        [
            "  modifier @a bevel {",
            "  modifier @b array {",
            "  modifier @c subsurf {"
        ],
        "{two}"
    );
    assert!(two.contains("count 7") && !two.contains("count 3"), "{two}");
    // A modifier no line replaces is added after the stack.
    let one = block(text, "object @x.1");
    assert!(
        one.contains("modifier @d weld") && one.contains("count 3"),
        "{one}"
    );
    assert!(
        one.find("modifier @c").unwrap() < one.find("modifier @d").unwrap(),
        "{one}"
    );
    assert_eq!(expanded_problems(&out), "");
}

#[test]
fn a_root_override_s_sub_object_lines_are_checked_as_those_sub_objects() {
    let yard = "collection @yard \"Yard\" {\n  use @post @x count=2 {}\n  override @x.2 { modifier @b array { cont 7 } }\n  override @x.1 { modifier @e arrays {} }\n}\n";
    let mut map = SourceMap::new();
    let texts = [("post.noll", STACKED), ("yard.noll", yard)];
    let parses: Vec<(FileId, Parse)> = texts
        .iter()
        .map(|(name, text)| (map.add(*name, *text), parse(text)))
        .collect();
    let files: Vec<_> = parses.iter().map(|(i, p)| (*i, p)).collect();
    let found = map.render_all(&check(schema(), &files));
    assert!(
        found.contains("`cont`") && found.contains("write `count`"),
        "{found}"
    );
    assert!(
        found.contains("`arrays` is not a kind of modifier"),
        "{found}"
    );
    assert!(!found.contains("has no parameter `modifier`"), "{found}");
}
