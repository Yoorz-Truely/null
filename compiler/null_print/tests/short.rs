//! Short forms (Sau's decision S35; spec, section 8, "Short forms"; task 1d-03d). For each form
//! of the spec: the short text parses and checks clean, expands to its full form, which checks
//! clean too, and a write-back through it keeps the short form, while meaning what the same
//! write through the full form means: expanding the written short text gives the written full
//! text.

use null_check::paths::Index;
use null_check::{Schema, check};
use null_print::{Address, Attr, Line, New, Values, expand, write_through};
use null_syntax::{FileId, Parse, SourceMap, apply_edits, parse};
use null_value::Dimension;

const PROJECT: &str = include_str!("../../../schema/null-schema-project.json");
const SPEC: &str = include_str!("../../../docs/spec/null-v0.md");

fn schema() -> &'static Schema {
    static S: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
    S.get_or_init(|| Schema::from_json(PROJECT).unwrap())
}

/// `texts` parse and check clean, as one project.
fn assert_clean(texts: &[&str]) {
    let mut map = SourceMap::new();
    let parses: Vec<(FileId, Parse)> = texts
        .iter()
        .enumerate()
        .map(|(i, t)| (map.add(format!("file-{}.noll", i + 1), *t), parse(t)))
        .collect();
    for (id, p) in &parses {
        let syntax = null_syntax::diagnostics(p, *id);
        assert!(syntax.is_empty(), "{}", map.render_all(&syntax));
    }
    let files: Vec<_> = parses.iter().map(|(i, p)| (*i, p)).collect();
    let found = check(schema(), &files);
    assert!(
        found.is_empty(),
        "{}\nin\n{}",
        map.render_all(&found),
        texts.join("\n")
    );
}

/// `texts` with file `file` written through at `at`.
fn written(texts: &[&str], file: usize, at: &Address<'_>, new: New<'_>) -> Vec<String> {
    let edits = write_through(texts, file, at, new).unwrap();
    let edits: Vec<_> = edits
        .iter()
        .map(|e| (e.span, e.replacement.as_str()))
        .collect();
    let mut out: Vec<String> = texts.iter().map(|t| t.to_string()).collect();
    out[file] = apply_edits(texts[file], &edits);
    out
}

fn strs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

/// One short form: `short` (a project; its form in file `file`) checks clean and expands to
/// `full`, which checks clean; writing `new` at `at` gives `short_after` in that file, and
/// expanding that gives what the same write gives through `full`.
fn form(
    short: &[&str],
    full: &[&str],
    file: usize,
    at: Address<'_>,
    new: New<'_>,
    short_after: &str,
) {
    assert_clean(short);
    let expanded = expand(short, schema());
    assert_eq!(strs(&expanded), full, "the full form");
    assert_clean(full);
    assert_eq!(
        expand(full, schema()),
        expanded,
        "the full form expands to itself"
    );

    let short_w = written(short, file, &at, new);
    assert_eq!(short_w[file], short_after, "the short form is kept");
    assert_clean(&strs(&short_w));
    let full_w = written(full, file, &at, new);
    assert_eq!(
        expand(&strs(&short_w), schema()),
        full_w,
        "a write through the short form means the write through the full form"
    );
}

fn at<'a>(kind: &'a str, handle: &'a str, lines: &'a [Line<'a>], attr: Attr<'a>) -> Address<'a> {
    Address {
        kind,
        handle,
        lines,
        attr,
    }
}

#[test]
fn a_datablocks_handle_from_its_name() {
    form(
        &["object \"Pier\" {\n  location [0m 0m 1m]\n}\n"],
        &["object @pier \"Pier\" {\n  location [0m 0m 1m]\n}\n"],
        0,
        at("object", "@pier", &[], Attr::Named("location")),
        New::Number(Values::F32(&[0.0, 0.0, 2.0]), Dimension::Length, None),
        "object \"Pier\" {\n  location [0m 0m 2m]\n}\n",
    );
}

#[test]
fn a_sub_objects_name_from_its_handle() {
    form(
        &["object @pier \"Pier\" {\n  modifier @bevel bevel {\n    width 4mm\n  }\n}\n"],
        &["object @pier \"Pier\" {\n  modifier @bevel bevel \"bevel\" {\n    width 4mm\n  }\n}\n"],
        0,
        at(
            "object",
            "@pier",
            &[Line {
                name: "modifier",
                args: &["@bevel"],
            }],
            Attr::Named("width"),
        ),
        New::Number(Values::F32(&[0.005]), Dimension::Length, None),
        "object @pier \"Pier\" {\n  modifier @bevel bevel {\n    width 5mm\n  }\n}\n",
    );
}

#[test]
fn settings_on_their_owners_line() {
    form(
        &[
            "object @pier \"Pier\" hide-render=on {\n  modifier @bevel bevel \"Bevel\" width=4mm segments=3\n}\n",
        ],
        &[
            "object @pier \"Pier\" {\n  hide-render on\n  modifier @bevel bevel \"Bevel\" {\n    width 4mm\n    segments 3\n  }\n}\n",
        ],
        0,
        at(
            "object",
            "@pier",
            &[Line {
                name: "modifier",
                args: &["@bevel"],
            }],
            Attr::Named("width"),
        ),
        New::Number(Values::F32(&[0.005]), Dimension::Length, None),
        "object @pier \"Pier\" hide-render=on {\n  modifier @bevel bevel \"Bevel\" width=5mm segments=3\n}\n",
    );
}

#[test]
fn a_bare_boolean() {
    let short = ["object @pier \"Pier\" {\n  hide-render\n}\n"];
    let address = at("object", "@pier", &[], Attr::Named("hide-render"));
    form(
        &short,
        &["object @pier \"Pier\" {\n  hide-render on\n}\n"],
        0,
        address,
        New::Bool(false),
        "object @pier \"Pier\" {\n  hide-render off\n}\n",
    );
    // On, as it already is: nothing is written.
    assert!(
        write_through(&short, 0, &address, New::Bool(true))
            .unwrap()
            .is_empty()
    );
}

const UNITS_SHORT: &str = "scene @s \"S\" {\n  units metric scale=1 length=m rotation=deg\n}\n";
const UNITS_FULL: &str = "scene @s \"S\" {\n  units {\n    system metric\n    scale-length 1\n    length-unit meters\n    system-rotation degrees\n  }\n}\n";

#[test]
fn units_short_form() {
    let lines = [Line {
        name: "units",
        args: &[],
    }];
    form(
        &[UNITS_SHORT],
        &[UNITS_FULL],
        0,
        at("scene", "@s", &lines, Attr::Named("length-unit")),
        New::Word("centimeters"),
        "scene @s \"S\" {\n  units metric scale=1 length=cm rotation=deg\n}\n",
    );
    form(
        &[UNITS_SHORT],
        &[UNITS_FULL],
        0,
        at("scene", "@s", &lines, Attr::Named("scale-length")),
        New::Number(Values::F32(&[0.01]), Dimension::None, None),
        "scene @s \"S\" {\n  units metric scale=0.01 length=m rotation=deg\n}\n",
    );
    form(
        &[UNITS_SHORT],
        &[UNITS_FULL],
        0,
        at("scene", "@s", &lines, Attr::Named("system")),
        New::Word("imperial"),
        "scene @s \"S\" {\n  units imperial scale=1 length=m rotation=deg\n}\n",
    );
}

const FLICKER: &str = r#"action @flicker "Lamp Flicker" {
  slot @lamp light "Lamp" {
    fcurve energy {
      key 1 800W interpolation=bezier
      key 6 620W
      key 11 800W
      modifier @wobble noise "Wobble" {
        strength 40
        scale 2
      }
    }
  }
}
"#;

const FLICKER_FULL: &str = r#"action @flicker "Lamp Flicker" {
  slot @lamp light "Lamp"
  layer @layer "Layer" {
    strip @strip keyframe {
      channels @lamp {
        fcurve energy {
          key 1 800W interpolation=bezier
          key 6 620W
          key 11 800W
          modifier @wobble noise "Wobble" {
            strength 40
            scale 2
          }
        }
      }
    }
  }
}
"#;

const LAMP: &str =
    "light @lamp-light \"Lamp\" {\n  type point\n  energy 800W\n  action @flicker\n}\n";
const LAMP_FULL: &str =
    "light @lamp-light \"Lamp\" {\n  type point\n  energy 800W\n  action @flicker slot=@lamp\n}\n";

#[test]
fn slot_left_out_where_the_action_has_one() {
    form(
        &[FLICKER_FULL, LAMP],
        &[FLICKER_FULL, LAMP_FULL],
        1,
        at(
            "light",
            "@lamp-light",
            &[Line {
                name: "action",
                args: &["@flicker"],
            }],
            Attr::Named("slot"),
        ),
        New::Word("@lamp"),
        LAMP,
    );
}

#[test]
fn an_actions_one_layer_and_strip() {
    let lines = [
        Line {
            name: "layer",
            args: &["@layer"],
        },
        Line {
            name: "strip",
            args: &["@strip"],
        },
        Line {
            name: "channels",
            args: &["@lamp"],
        },
        Line {
            name: "fcurve",
            args: &["energy"],
        },
        Line {
            name: "key",
            args: &["6"],
        },
    ];
    form(
        &[FLICKER, LAMP_FULL],
        &[FLICKER_FULL, LAMP_FULL],
        0,
        at("action", "@flicker", &lines, Attr::Arg(1)),
        New::Number(Values::F32(&[650.0]), Dimension::Power, None),
        &FLICKER.replace("key 6 620W", "key 6 650W"),
    );
}

const CLAY: &str = r#"material @clay "Clay" {
  nodes {
    node @noise tex-noise "Noise" { scale 4 }
    node @box frame "Surface" { label "Surface" }
    node @bsdf bsdf-principled "Skin" {
      parent @box
    }
  }
}
"#;

#[test]
fn a_paths_owner_left_out_in_its_own_block() {
    let files = [CLAY];
    assert_clean(&files);
    // The text names the frame the short way, and the index reads both ways the same.
    let full = expand(&files, schema()).join("");
    assert!(
        full.contains("parent @box\n") && !full.contains("@clay/"),
        "{full}"
    );
    let parses: Vec<Parse> = files.iter().map(|t| parse(t)).collect();
    let ids: Vec<(FileId, &Parse)> = parses.iter().map(|p| (FileId(0), p)).collect();
    let index = Index::build(&ids);
    assert_eq!(
        index
            .resolve(schema(), "@box", Some(("material", "@clay")))
            .unwrap(),
        index.resolve(schema(), "@clay/@box", None).unwrap()
    );
    // Write-back writes a path of the owner's the short way.
    let lines = [
        Line {
            name: "nodes",
            args: &[],
        },
        Line {
            name: "node",
            args: &["@bsdf"],
        },
    ];
    let address = at("material", "@clay", &lines, Attr::Named("parent"));
    assert!(
        write_through(&files, 0, &address, New::Path("@clay/@box"))
            .unwrap()
            .is_empty()
    );
    let moved = written(&files, 0, &address, New::Path("@clay/@noise"));
    assert_eq!(moved[0], CLAY.replace("parent @box", "parent @noise"));
}

/// The text from the heading starting `from` to the next heading of its level or above.
fn section(from: &str) -> &'static str {
    let start = SPEC
        .find(from)
        .unwrap_or_else(|| panic!("no section {from}"));
    let level = from.chars().take_while(|c| *c == '#').count();
    let rest = &SPEC[start + from.len()..];
    let end = rest
        .match_indices("\n#")
        .find(|(i, _)| rest[i + 1..].chars().take_while(|c| *c == '#').count() <= level)
        .map_or(rest.len(), |(i, _)| i);
    &SPEC[start..start + from.len() + end]
}

#[test]
fn the_specs_examples_expanded_check_clean_and_expand_to_themselves() {
    let blocks: Vec<String> = [
        section("### Short forms"),
        section("### Modifiers, constraints and pose bones"),
        section("## 10. Node trees"),
        section("## 11. Animation and drivers"),
    ]
    .iter()
    .flat_map(|s| s.split("```null\n").skip(1))
    .map(|b| format!("{}\n", b.split("\n```").next().unwrap()))
    .collect();
    let texts = strs(&blocks);
    assert_clean(&texts);
    let full = expand(&texts, schema());
    assert_clean(&strs(&full));
    assert_eq!(expand(&strs(&full), schema()), full);
    // Each short form is in an example, and none is left once expanded.
    let all = full.join("\n");
    for gone in [
        "\nobject \"Buoy\"",
        "modifier @round bevel width",
        "  hide-render\n",
        "units metric",
        "  action @bob\n",
        "slot @buoy object \"Buoy\" {",
    ] {
        assert!(
            blocks.iter().any(|b| b.contains(gone)),
            "no example writes `{gone}`"
        );
        assert!(!all.contains(gone), "`{gone}` is left in\n{all}");
    }
}
