//! The checker against the slice schema: a clean project, and every kind of error with its fix.

use std::path::Path;

use null_check::{Schema, check, codes};
use null_syntax::{Diagnostic, FileId, Parse, SourceMap, apply_edits, parse};

fn schema() -> Schema {
    Schema::from_json(include_str!("../../../schema/null-schema.json")).unwrap()
}

/// Checks `texts` as one project; returns the map, for rendering, and the problems.
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
    let found = check(&schema(), &files);
    (map, found)
}

#[test]
fn the_slice_project_checks_clean() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/slice");
    let mut texts = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap().to_str().unwrap().to_string();
        texts.push((name, std::fs::read_to_string(&p).unwrap()));
    }
    texts.sort();
    assert_eq!(texts.len(), 3);
    let refs: Vec<_> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let (map, found) = run(&refs);
    assert!(found.is_empty(), "{}", map.render_all(&found));
}

/// One broken line per case, inside a project that is otherwise clean.
const CASES: &str = "\
scene @main \"Main\" {
  object @cube
  object @lamp
  object @cam
  frame-strat 1
}

object @cube \"Cube\" {
  locaton [0m 0m 1m]
  location 1m
  rotation-euler [0m 0m 45deg]
  rotation-mode XYZ
  scale [1 1]
  data mesh @cube-mesh
  material @steel
  parent @Cube
}

object @cam \"Cam\" {
  data @lens
  rotation-mode spherical
}

object @lamp \"Lamp\" { data light @lamp-light; location [0MM 0m 2m] }

mesh @cube-mesh \"Cube\" {
  primitive cube size=2m segments=4
  faces [[0 1 2]]
}

mesh @tri \"Tri\" {
  vertices [[0m 0m 0m] [1m 0m 0m] [0m 1m 0m]]
  polygons [[0 1 3] [0 1]]
}

camera @lens \"Lens\" {
  lens 35deg
  clip-start 0
  type perspective
}

light @lamp-light \"Lamp\" {
  type point
  energy 800lm
  angle 1deg
  use-shadow yes
}

material @paint \"Paint\" {
  roughness 1.5
  roughness 0.5
  name-full \"Paint\"
  nodes {
    node @bsdf bsdf-principled { base-colour [1 1 1 1]; normal @bsdf.out }
    node @mix mix-shader
  }
}

lamp @x \"X\"
";

#[test]
fn each_mistake_gives_an_error_with_a_suggestion() {
    let (map, found) = run(&[("objects/cases.noll", CASES)]);
    let codes_found: Vec<&str> = found.iter().map(|d| d.code).collect();
    for code in [
        codes::UNKNOWN_PROPERTY,
        codes::TYPE,
        codes::UNIT,
        codes::RANGE,
        codes::ENUM,
        codes::HANDLE,
        codes::READONLY,
        codes::REPEATED,
        codes::NAME,
        codes::KIND,
    ] {
        assert!(codes_found.contains(&code), "no {code}");
    }
    insta::assert_snapshot!("errors", map.render_all(&found));
}

#[test]
fn the_three_errors_of_the_task_each_come_with_a_fix() {
    // 1a-06's "Done when": a misspelt name, a wrong type and a wrong unit.
    let text = "object @cube \"Cube\" {\n  locaton [0m 0m 1m]\n  scale 2\n  rotation-euler [0deg 0deg 45dgr]\n}\n";
    let (_, found) = run(&[("cube.noll", text)]);
    let by_code = |c| found.iter().find(|d| d.code == c).unwrap();
    let name = by_code(codes::UNKNOWN_PROPERTY);
    assert_eq!(name.fixes[0].edits[0].replacement, "location");
    let ty = by_code(codes::TYPE);
    assert_eq!(
        ty.message,
        "`scale` takes 3 numbers, as a list such as [0 0 0]"
    );
    let unit = by_code(codes::UNIT);
    assert_eq!(unit.fixes[0].edits[0].replacement, "deg");
    // Applying the fixes leaves only the wrong type, which needs the author.
    let edits: Vec<_> = found
        .iter()
        .flat_map(|d| d.fixes.first())
        .flat_map(|f| &f.edits)
        .map(|e| (e.span, e.replacement.as_str()))
        .collect();
    let fixed = apply_edits(text, &edits);
    let (_, again) = run(&[("cube.noll", &fixed)]);
    assert_eq!(again.len(), 1, "{again:?}");
    assert_eq!(again[0].code, codes::TYPE);
}

#[test]
fn handles_resolve_across_files() {
    let a = "object @pier \"Pier\" { data mesh @deck }\n";
    let b = "mesh @deck \"Deck\" { primitive plane size=4m }\n";
    let (_, found) = run(&[("a.noll", a), ("b.noll", b)]);
    assert!(found.is_empty(), "{found:?}");
    let (_, found) = run(&[("a.noll", a)]);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, codes::HANDLE);
    let twice = "mesh @deck \"A\" { primitive plane }\nmesh @deck \"B\" { primitive plane }\n";
    let (_, found) = run(&[("a.noll", twice)]);
    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains("defined twice"));
}

#[test]
fn a_handle_may_come_from_the_display_name() {
    // S18: `object "Pier"` gives `@pier`; `Cube.001` gives `@cube-001`.
    let text = "object \"Cube.001\" { data mesh @m }\nmesh \"M\" { primitive cube }\nscene @s { object @cube-001 }\n";
    let (_, found) = run(&[("a.noll", text)]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_lights_properties_follow_its_type() {
    let point = "light @l \"L\" { type point; energy 100W; shadow-soft-size 1cm }\n";
    let (_, found) = run(&[("a.noll", point)]);
    assert!(found.is_empty(), "{found:?}");
    let sun = "light @l \"L\" { type sun; energy 3; angle 1deg }\n";
    let (_, found) = run(&[("a.noll", sun)]);
    assert!(found.is_empty(), "{found:?}");
    // A sun's strength is an irradiance, so no watts.
    let (_, found) = run(&[("a.noll", "light @l \"L\" { type sun; energy 3W }\n")]);
    assert_eq!(found[0].code, codes::UNIT);
    // A spot's size belongs to spots.
    let (_, found) = run(&[("a.noll", "light @l \"L\" { type point; spot-size 30deg }\n")]);
    assert_eq!(
        found[0].message,
        "`spot-size` is a spot light's, and this light's `type` is `point`"
    );
}

#[test]
fn scene_units_take_short_and_full_forms() {
    for units in [
        "units metric scale=1 length=m rotation=deg",
        "units imperial length=ft rotation=rad",
        "units { system metric; length-unit centimeters; scale-length 0.01 }",
    ] {
        let text = format!("project \"P\" {{\n  {units}\n  scene @s\n}}\nscene @s \"S\"\n");
        let (map, found) = run(&[("p.noll", &text)]);
        assert!(found.is_empty(), "{units}: {}", map.render_all(&found));
    }
    let (_, found) = run(&[("p.noll", "project \"P\" { units metric length=yd }\n")]);
    assert_eq!(found[0].code, codes::ENUM);
}

/// The spec's pier (section 4), as far as the slice schema reaches: its object and its deck.
/// The schema has no modifiers yet, so the `modifier` nodes are read here, not checked.
fn spec_pier() -> String {
    let spec = include_str!("../../../docs/spec/null-v0.md");
    let start = spec.find("// objects/pier.noll").expect("the pier example");
    let body = &spec[start..];
    body[..body.find("```").unwrap()].to_string()
}

#[test]
fn the_pier_is_proportioned_by_its_primitive_not_its_object() {
    // Task 1a-06b, after Sau's 1a-05c: `primitive cube size=1m scale=[1.2 3 0.08]` shapes the
    // deck, and the object's scale stays 1, so its 4mm bevel is 4mm in the world. Measured in
    // Sau (agent/null-a's report): a 4mm bevel insets every face by 0.004 m this way, and by
    // 0.012, 0.0048 and 0.0048 m when the object is scaled instead.
    let pier = spec_pier();
    let file = parse(&pier);
    let object = file
        .document()
        .nodes()
        .find(|n| n.name() == "object")
        .unwrap();
    let words = |node: &null_syntax::ast::Node<'_>, attr: &str| -> Vec<String> {
        node.children()
            .find(|c| c.name() == attr)
            .and_then(|c| c.args().next())
            .map(|v| v.items().iter().map(|i| i.text().to_string()).collect())
            .unwrap()
    };
    let scale: Vec<f32> = words(&object, "scale")
        .iter()
        .map(|w| null_value::read(w, null_value::Dimension::None).unwrap())
        .collect();
    assert_eq!(scale, [1.0, 1.0, 1.0], "the object is not scaled");
    let bevel = object
        .children()
        .find(|c| {
            c.name() == "modifier" && c.args().nth(1).and_then(|v| v.as_word()) == Some("bevel")
        })
        .unwrap();
    let width = bevel.children().find(|c| c.name() == "width").unwrap();
    let width: f32 = null_value::read(
        width.args().next().unwrap().text(),
        null_value::Dimension::Length,
    )
    .unwrap();
    assert_eq!(width, 0.004, "4mm, and nothing scales it");

    // Without the modifier nodes (not in the slice schema yet) and with its material defined,
    // the pier checks clean against the schema: the primitive's `scale` is the operator's.
    let kept: String = pier
        .lines()
        .scan(0usize, |skip, line| {
            let t = line.trim_start();
            let opens = t.matches('{').count();
            let closes = t.matches('}').count();
            if *skip > 0 {
                *skip = *skip + opens - closes;
                return Some(None);
            }
            if t.starts_with("modifier ") {
                *skip = opens - closes;
                return Some(None);
            }
            Some(Some(line))
        })
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");
    let material = "material @weathered-oak \"Weathered Oak\" { roughness 0.62 }\n";
    let (map, found) = run(&[
        ("objects/pier.noll", &kept),
        ("materials/oak.noll", material),
    ]);
    assert!(found.is_empty(), "{}\n{kept}", map.render_all(&found));
    assert!(kept.contains("primitive cube size=1m scale=[1.2 3 0.08]"));
}

#[test]
fn only_primitives_whose_operator_takes_scale_accept_it() {
    let (_, found) = run(&[(
        "a.noll",
        "mesh @a \"A\" { primitive uv-sphere radius=1m scale=[1 1 2] }\n",
    )]);
    assert!(found.is_empty(), "{found:?}");
    let (_, found) = run(&[(
        "a.noll",
        "mesh @a \"A\" { primitive plane size=2m scale=[1 2 1] }\n",
    )]);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, codes::UNKNOWN_PROPERTY);
    assert_eq!(
        found[0].message,
        "`scale` is not a plane's: Blender's plane operator does not take it; \
         `cube`, `uv-sphere`, `ico-sphere`, `cylinder`, `cone` do"
    );
    // Unitless: a length is not a scale.
    let (_, found) = run(&[(
        "a.noll",
        "mesh @a \"A\" { primitive cube scale=[1m 1m 1m] }\n",
    )]);
    assert!(found.iter().all(|d| d.code == codes::UNIT), "{found:?}");
    assert_eq!(found.len(), 3);
}

#[test]
fn a_handle_defined_twice_says_which_kind_with_its_article() {
    // "as an object", not "as a object" (found reviewing Cosmósis's 1d-04b).
    let (_, found) = run(&[
        ("a.noll", "object @a \"A\"\n"),
        (
            "b.noll",
            "object @a \"A2\"\nmesh @m \"M\"\nmesh @m \"M2\"\n",
        ),
    ]);
    let messages: Vec<_> = found.iter().map(|d| d.message.as_str()).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("defined twice as an object")),
        "{messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("defined twice as a mesh")),
        "{messages:?}"
    );
}

#[test]
fn a_node_line_may_give_the_nodes_blender_name_in_quotes() {
    // Sau's task 1b-02d: a node renamed in Sau keeps its handle, and its line gains the name.
    let material = |line: &str| {
        format!(
            "material @clay \"Clay\" {{\n  nodes {{\n    {line} {{\n      roughness 0.7\n    }}\n    \
             node @out output-material {{\n      surface @bsdf.bsdf\n    }}\n  }}\n}}\n"
        )
    };
    let named = material("node @bsdf bsdf-principled \"Skin\"");
    let (map, found) = run(&[("clay.noll", &named)]);
    assert!(found.is_empty(), "{}", map.render_all(&found));
    for wrong in [
        "node @bsdf bsdf-principled skin",
        "node @bsdf bsdf-principled \"Skin\" \"Again\"",
        "node @bsdf bsdf-principled 0.7",
    ] {
        let text = material(wrong);
        let (map, found) = run(&[("clay.noll", &text)]);
        assert_eq!(found.len(), 1, "{wrong}: {}", map.render_all(&found));
        assert_eq!(found[0].code, codes::TYPE, "{wrong}");
    }
}
