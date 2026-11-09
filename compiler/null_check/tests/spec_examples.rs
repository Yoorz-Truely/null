//! Every example of the spec's forms of modifiers, constraints and pose bones (section 8),
//! node trees (section 10) and animation (section 11) checks clean against the schema of all
//! project data, checked together as one project, since they name each other (task 1d-07a).
//! A construct the sections describe without an example fails the test.

use null_check::{Schema, check};
use null_syntax::{FileId, Parse, SourceMap, parse};

const SPEC: &str = include_str!("../../../docs/spec/null-v0.md");
const PROJECT: &str = include_str!("../../../schema/null-schema-project.json");

/// The text from the heading starting `from` to the next heading of its level or above.
fn section(from: &str) -> &'static str {
    let start = SPEC
        .find(from)
        .unwrap_or_else(|| panic!("no section {from}"));
    let level = from.chars().take_while(|c| *c == '#').count();
    let rest = &SPEC[start + from.len()..];
    let end = rest
        .match_indices("\n#")
        .find(|(i, _)| {
            let hashes = rest[i + 1..].chars().take_while(|c| *c == '#').count();
            hashes <= level
        })
        .map_or(rest.len(), |(i, _)| i);
    &SPEC[start..start + from.len() + end]
}

/// The ```null blocks of `text`.
fn examples(text: &str) -> Vec<&str> {
    text.split("```null\n")
        .skip(1)
        .map(|b| b.split("\n```").next().unwrap())
        .collect()
}

#[test]
fn every_example_of_sections_8_10_and_11_checks_clean() {
    let schema = Schema::from_json(PROJECT).unwrap();
    let parts = [
        section("### Modifiers, constraints and pose bones"),
        section("## 10. Node trees"),
        section("## 11. Animation and drivers"),
    ];
    let blocks: Vec<String> = parts
        .iter()
        .flat_map(|p| examples(p))
        .map(|b| format!("{b}\n"))
        .collect();
    assert!(blocks.len() >= 7, "{} examples", blocks.len());
    let mut map = SourceMap::new();
    let parses: Vec<(FileId, Parse)> = blocks
        .iter()
        .enumerate()
        .map(|(i, b)| (map.add(format!("example-{}.noll", i + 1), b), parse(b)))
        .collect();
    for (id, p) in &parses {
        let syntax = null_syntax::diagnostics(p, *id);
        assert!(syntax.is_empty(), "{}", map.render_all(&syntax));
    }
    let files: Vec<_> = parses.iter().map(|(i, p)| (*i, p)).collect();
    let found = check(&schema, &files);
    assert!(found.is_empty(), "{}", map.render_all(&found));

    // Every construct the sections name is in an example.
    let all = blocks.join("\n");
    for construct in [
        "modifier @",
        " nodes \"",
        "node-group @",
        "constraint @",
        "bone @",
        "data armature",
        "nodes {",
        "layout {",
        "frame \"",
        "parent @",
        "[@",
        "input @",
        "output @",
        "panel @",
        " group \"",
        "node-tree @",
        "group-input",
        "group-output",
        "zone @",
        "repeat-item @",
        "output {",
        "compositing-node-group",
        "slot @",
        "fcurve ",
        "index=",
        "key ",
        "interpolation=",
        "handle-left=",
        "layer @",
        "strip @",
        "keyframe",
        "channels @",
        "action @",
        "slot=@",
        "track @",
        "driver=",
        "var ",
        "target @",
    ] {
        assert!(all.contains(construct), "no example writes `{construct}`");
    }
}

/// The examples' blocks, each changed by `edit` where it holds its first argument.
fn check_edited(from: &str, to: &str) -> Vec<String> {
    let schema = Schema::from_json(PROJECT).unwrap();
    let parts = [
        section("### Modifiers, constraints and pose bones"),
        section("## 10. Node trees"),
        section("## 11. Animation and drivers"),
    ];
    let blocks: Vec<String> = parts
        .iter()
        .flat_map(|p| examples(p))
        .map(|b| format!("{b}\n"))
        .collect();
    assert!(
        blocks.iter().any(|b| b.contains(from)),
        "no example holds `{from}`"
    );
    let blocks: Vec<String> = blocks.iter().map(|b| b.replacen(from, to, 1)).collect();
    let mut map = SourceMap::new();
    let parses: Vec<(FileId, Parse)> = blocks
        .iter()
        .enumerate()
        .map(|(i, b)| (map.add(format!("example-{}.noll", i + 1), b), parse(b)))
        .collect();
    let files: Vec<_> = parses.iter().map(|(i, p)| (*i, p)).collect();
    check(&schema, &files)
        .iter()
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

#[test]
fn each_form_catches_its_mistakes() {
    // (the example's text, written wrong, a word of the error expected)
    let cases = [
        // Modifiers, constraints, pose bones.
        (
            "width 4mm",
            "wdth 4mm",
            "null-unknown-property a bevel modifier has no `wdth`; did you mean `width`?",
        ),
        (
            "@bevel bevel",
            "@bevel bevl",
            "null-kind `bevl` is not a kind of modifier",
        ),
        (
            "track-axis track-negative-z",
            "track-axs track-negative-z",
            "did you mean `track-axis`?",
        ),
        ("chain-count 2", "chain-count 2.5", "null-type"),
        (
            "rotation-mode xyz",
            "rotaton-mode xyz",
            "did you mean `rotation-mode`?",
        ),
        (
            "    count 3\n",
            "    cont 3\n",
            "a nodes modifier has no `cont`; did you mean `count`?",
        ),
        // Node trees.
        (
            "node @noise tex-noise",
            "node @noise tex-noyse",
            "null-kind `tex-noyse` is not a node kind a shader tree takes",
        ),
        (
            "{ scale 4 }",
            "{ scal 4 }",
            "a tex-noise node has no `scal`; did you mean `scale`?",
        ),
        (
            "fac @noise.fac",
            "fac @noise.fax",
            "a tex-noise node has no output `fax`",
        ),
        (
            "surface @bsdf.bsdf",
            "surface 0.5",
            "`surface` takes a link",
        ),
        (
            "roughness 0.6",
            "roughness 0.6\n      location [0 0]",
            "written in the tree's `layout` block",
        ),
        (
            "parent @box",
            "parent @ramp",
            "`parent` names a `frame` node",
        ),
        (
            "input @count int",
            "input @count innt",
            "null-kind `innt` is not a socket type",
        ),
        (
            "default-value 4",
            "default-valeu 4",
            "did you mean `default-value`?",
        ),
        (
            "{ default-value 10cm }",
            "{ default-value 10deg }",
            "null-unit",
        ),
        (
            "zone @loop repeat",
            "zone @loop repet",
            "null-kind `repet` is not a zone this tree takes",
        ),
        (
            "geometry @loop.geometry\n        translation",
            "geometry @loop.geomtry\n        translation",
            "the repeat zone `@loop` has no output `geomtry` here",
        ),
        (
            "repeat-item @geometry geometry",
            "repeat-item @geometry geomtry",
            "`geomtry`",
        ),
        (
            "output { geometry @join.geometry }",
            "output { geometri @join.geometry }",
            "a zone's output has no `geometri`",
        ),
        (
            "@loop [-300 0] [400 0]",
            "@loop [-300 0]",
            "a zone's layout line is `@zone [x y] [x y]`",
        ),
        (
            "@bsdf [-100 0] width=240",
            "@bsdf [-100 0] height=240",
            "only a frame's layout line takes `height=`",
        ),
        (
            "width=300 height=200",
            "width=300 depth=200",
            "a layout line takes `width=`, and `height=` for a frame, not `depth=`",
        ),
        (
            "@loop [-300 0] [400 0]",
            "@loop [-300 0] [400 0] width=200",
            "a zone's layout line holds its two positions only",
        ),
        (
            "@bsdf [-100 0] width=240",
            "@bsdf [-100 0] width=wide",
            "null-type",
        ),
        (
            "count 6",
            "cuont 6",
            "a group node has no `cuont`; did you mean `count`?",
        ),
        (
            "tower-out @stack.stacked",
            "tower-out @stack.stacker",
            "a group node has no output `stacker`",
        ),
        (
            "node-tree @stack-up",
            "node-tree @stack-upp",
            "no node-tree is called `@stack-upp`",
        ),
        (
            "geometry [@loop.geometry @lift.geometry]",
            "geometry [@loop.geometry @lift.geom]",
            "a transform node has no output `geom`",
        ),
        (
            "contrast 10",
            "contrst 10",
            "a bright-contrast node has no `contrst`",
        ),
        // Animation.
        (
            "slot @lamp light",
            "slot @lamp lihgt",
            "null-enum `lihgt` is not a kind of datablock a slot animates",
        ),
        (
            "fcurve energy {",
            "fcurve enrgy {",
            "a light has no `enrgy` to animate",
        ),
        (
            "fcurve location index=2",
            "fcurve location",
            "`location` is an array",
        ),
        (
            "fcurve location index=2",
            "fcurve location index=3",
            "`index` is an item of `location`, 0 to 2",
        ),
        ("key 6 620W", "key 6 620m", "null-unit"),
        (
            "interpolation=sine",
            "interpolaton=sine",
            "a key has no `interpolaton`",
        ),
        ("strength 40", "strenght 40", "did you mean `strength`?"),
        (
            "channels @ball",
            "channels @bal",
            "this action has no slot `@bal`",
        ),
        (
            "slot @ball object \"Ball\"\n",
            "slot @ball object \"Ball\" {\n    fcurve location index=0 { key 1 0m }\n  }\n",
            "holds its f-curves in its strips' `channels` blocks",
        ),
        (
            "action @flicker slot=@lamp",
            "action @flicker slot=@lamb",
            "`@flicker` has no slot `@lamb`",
        ),
        (
            "action @hop slot=@ball",
            "action @flicker slot=@lamp",
            "slot `@lamp` animates a light, not an object",
        ),
        (
            "blend-type add",
            "blend-typ add",
            "an NLA strip has no `blend-typ`",
        ),
        (
            "var spin transforms",
            "var spin transform",
            "`transform` is not a kind of driver variable",
        ),
        (
            "var spin transforms",
            "var spin-it transforms",
            "not a variable's name",
        ),
        (
            "var spin transforms",
            "var spin loc-diff",
            "a loc-diff variable has 2 `target` lines, not 1",
        ),
        ("transform-type=rot-z", "transform-type=rot-q", "null-enum"),
        (
            "energy driver=\"spin * 100W\"",
            "energy 100W driver=\"spin * 100W\"",
            "takes no value of its own",
        ),
        // A driver's expression (task 1d-07f).
        (
            "energy driver=\"spin * 100W\"",
            "energy driver=\"spn * 100W\"",
            "null-type this driver has no variable `spn`",
        ),
        (
            "energy driver=\"spin * 100W\"",
            "energy driver=\"self.energy * 2\"",
            "null-python this driver is Python",
        ),
    ];
    let mut failures = Vec::new();
    for (from, to, want) in cases {
        let found = check_edited(from, to);
        if !found.iter().any(|f| f.contains(want)) {
            failures.push(format!("`{to}`: expected `{want}`, got {found:#?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn every_example_in_the_spec_checks_clean_as_one_project() {
    // Task 1d-01a: section 4 says every example checks clean, so every ```null block of the
    // whole spec is one project here, its handles distinct across sections.
    let schema = Schema::from_json(PROJECT).unwrap();
    // A Null file that is not a datablock file (the handle map `ids { … }`, `settings.noll`,
    // `view.noll`) is parsed and formatted by the other tests, but not checked as a project's.
    let blocks: Vec<String> = examples(SPEC)
        .iter()
        .filter(|b| {
            let p = parse(b);
            !p.document()
                .nodes()
                .any(|n| matches!(n.name().as_ref(), "ids" | "settings" | "view"))
        })
        .map(|b| format!("{b}\n"))
        .collect();
    assert!(blocks.len() >= 14, "{} examples", blocks.len());
    let mut map = SourceMap::new();
    let parses: Vec<(FileId, Parse)> = blocks
        .iter()
        .enumerate()
        .map(|(i, b)| (map.add(format!("example-{}.noll", i + 1), b), parse(b)))
        .collect();
    for (id, p) in &parses {
        let syntax = null_syntax::diagnostics(p, *id);
        assert!(syntax.is_empty(), "{}", map.render_all(&syntax));
    }
    let files: Vec<_> = parses.iter().map(|(i, p)| (*i, p)).collect();
    let found = check(&schema, &files);
    assert!(found.is_empty(), "{}", map.render_all(&found));
    // An example of a form not built yet is fenced `null (<task>)`, and names the task.
    for fence in SPEC.split("```null (").skip(1) {
        let task = fence.split(')').next().unwrap_or_default();
        let is_task =
            task.len() >= 5 && task.starts_with(|c: char| c.is_ascii_digit()) && task.contains('-');
        assert!(is_task, "a fence `null ({task})` names no task");
        // Not checked yet, but written in Null's syntax already.
        let body = fence.split_once('\n').map_or("", |x| x.1);
        let body = format!("{}\n", body.split("\n```").next().unwrap_or_default());
        let parsed = parse(&body);
        let mut map = SourceMap::new();
        let id = map.add(format!("unbuilt ({task})"), &body);
        let syntax = null_syntax::diagnostics(&parsed, id);
        assert!(syntax.is_empty(), "{}", map.render_all(&syntax));
    }
}

/// Checks one file alone against the project's schema; its diagnostics as `code message`.
fn check_alone(text: &str) -> Vec<String> {
    let schema = Schema::from_json(PROJECT).unwrap();
    let parse = parse(text);
    check(&schema, &[(FileId(0), &parse)])
        .iter()
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

#[test]
fn a_texture_s_nodes_are_a_texture_tree_of_any_texture_type() {
    // Section 10 (task 1d-07b): a texture's `nodes` block, whatever its type, and a
    // `texture-node-tree` group. The texture family's node kinds join the schema with 1d-07k, so
    // until then a node there is named as a texture tree's, not a shader tree's.
    for kind in ["texture", "noise", "wood"] {
        let text = format!("{kind} @t \"T\" {{\n  use-nodes\n  nodes {{\n  }}\n}}\n");
        assert!(
            check_alone(&text).is_empty(),
            "{kind}: {:?}",
            check_alone(&text)
        );
        let text =
            format!("{kind} @t \"T\" {{\n  nodes {{\n    node @c bsdf-principled\n  }}\n}}\n");
        let found = check_alone(&text);
        assert!(
            found
                .iter()
                .any(|f| f.contains("not a node kind a texture tree takes")),
            "{kind}: {found:?}"
        );
    }
    let group = "texture-node-tree @g \"G\" {\n  nodes {\n  }\n}\n";
    assert!(check_alone(group).is_empty(), "{:?}", check_alone(group));
}
