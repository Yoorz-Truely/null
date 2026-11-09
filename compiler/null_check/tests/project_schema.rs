//! The checker on the schema of all project data (task 1b-03e): `schema/null-schema-project.json`,
//! which Sau's build makes (task 1b-03b), loads fast, checks the slice's project as the slice's
//! schema does, and suggests the right name for a misspelt property of any datablock type.

use std::path::Path;
use std::time::Instant;

use null_check::{Schema, check, codes};
use null_syntax::{Diagnostic, FileId, parse};

const PROJECT: &str = include_str!("../../../schema/null-schema-project.json");
const SLICE: &str = include_str!("../../../schema/null-schema.json");

fn errors(schema: &Schema, texts: &[&str]) -> Vec<Diagnostic> {
    let parses: Vec<_> = texts.iter().map(|t| parse(t)).collect();
    let files: Vec<_> = parses
        .iter()
        .enumerate()
        .map(|(i, p)| (FileId(i as u32), p))
        .collect();
    check(schema, &files)
}

#[test]
fn the_project_schema_loads_in_under_50_ms() {
    // The best of five, as a language server loads it once at start. The budget holds for an
    // optimised build (`just verify` runs the tests with `--release`); a debug build only reports.
    let mut best = f64::INFINITY;
    let mut schema = None;
    for _ in 0..5 {
        let t = Instant::now();
        schema = Some(Schema::from_json(PROJECT).unwrap());
        best = best.min(t.elapsed().as_secs_f64() * 1000.0);
    }
    let schema = schema.unwrap();
    eprintln!(
        "the project schema: {} structs, {} datablock kinds, loaded in {best:.1} ms",
        schema.structs.len(),
        schema.datablock_kinds().count()
    );
    assert!(schema.structs.len() > 1000);
    if !cfg!(debug_assertions) {
        assert!(best < 50.0, "loaded in {best:.1} ms");
    }
}

fn slice_project() -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/slice");
    let mut texts: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    texts.sort();
    texts
        .iter()
        .map(|p| std::fs::read_to_string(p).unwrap())
        .collect()
}

#[test]
fn the_slice_project_checks_clean_on_the_project_schema() {
    let schema = Schema::from_json(PROJECT).unwrap();
    let texts = slice_project();
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    assert_eq!(refs.len(), 3);
    let found = errors(&schema, &refs);
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn the_slice_schemas_errors_are_the_project_schemas() {
    // Within the slice's structs the two schemas are the same, so every mistake gives the same
    // error and fix; only a misspelt datablock kind may find more kinds to suggest.
    let text = "\
object @cube \"Cube\" {
  locaton [0m 0m 1m]
  rotation-mode XYZ
  scale [1 1]
}

camera @cam \"Cam\" {
  lens 35deg
  clip-start -1m
}

material @m \"M\" {
  roughnes 0.5
}
";
    let slice = errors(&Schema::from_json(SLICE).unwrap(), &[text]);
    let project = errors(&Schema::from_json(PROJECT).unwrap(), &[text]);
    assert_eq!(slice.len(), 6, "{slice:#?}");
    assert_eq!(format!("{slice:?}"), format!("{project:?}"));
}

#[test]
fn a_misspelt_property_of_any_datablock_type_gets_a_suggestion() {
    let schema = Schema::from_json(PROJECT).unwrap();
    let kinds: Vec<_> = schema.datablock_kinds().collect();
    assert!(kinds.len() >= 55, "{}", kinds.len());
    let mut tried = 0;
    let mut missed = Vec::new();
    for s in &kinds {
        let props = schema.properties_of(&s.rna);
        // A writable property of its own with a name long enough to misspell by one letter.
        let Some(p) = s
            .properties
            .iter()
            .find(|p| !p.readonly && p.name.len() >= 6 && !p.name.contains("--"))
        else {
            continue;
        };
        let mut wrong = p.name.clone();
        wrong.remove(2);
        if props.iter().any(|q| q.name == wrong) {
            continue;
        }
        tried += 1;
        let text = format!("{} @x \"X\" {{\n  {wrong} 1\n}}\n", s.kind);
        let found = errors(&schema, &[&text]);
        let fix = found
            .iter()
            .find(|d| d.code == codes::UNKNOWN_PROPERTY)
            .and_then(|d| d.fixes.first())
            .map(|f| f.edits[0].replacement.clone());
        match fix {
            Some(name) if props.iter().any(|q| q.name == name) => {}
            _ => missed.push(format!("{}: `{wrong}` for `{}`: {found:?}", s.kind, p.name)),
        }
    }
    assert!(tried >= 50, "{tried}");
    assert!(missed.is_empty(), "{}", missed.join("\n"));
}

#[test]
fn every_name_is_kebab_case_and_every_unit_is_one_null_reads() {
    let schema = Schema::from_json(PROJECT).unwrap();
    let mut bad = Vec::new();
    let mut units = std::collections::BTreeSet::new();
    for s in &schema.structs {
        if !null_syntax::names::is_kebab_case(&s.kind) {
            bad.push(format!("kind {}", s.kind));
        }
        for p in &s.properties {
            if !null_syntax::names::is_kebab_case(&p.name) {
                bad.push(format!("{}.{}", s.rna, p.name));
            }
            units.extend(p.unit.clone());
        }
    }
    assert!(bad.is_empty(), "{bad:?}");
    // Each unit RNA uses here is one of section 6's: a value with a unit is read in it.
    for unit in &units {
        let p = format!("{{\"name\": \"x\", \"type\": \"float32\", \"unit\": \"{unit}\"}}");
        let p: null_check::Property = serde_json::from_str(&p).unwrap();
        assert!(null_check::unit_dimension(&p).is_some(), "{unit}");
    }
    assert!(units.len() >= 10, "{units:?}");
}

#[test]
fn matrices_are_rows_of_lists_and_run_time_enums_take_unlisted_items() {
    let schema = Schema::from_json(PROJECT).unwrap();
    let object = |body: &str| format!("object @x \"X\" {{\n{body}\n}}\n");
    let identity = "  matrix-world [[1 0 0 0] [0 1 0 0] [0 0 1 0] [0 0 0 1]]";
    assert!(errors(&schema, &[&object(identity)]).is_empty());
    for wrong in [
        "  matrix-world [1 0 0 0]",
        "  matrix-world [[1 0 0 0] [0 1 0 0] [0 0 1 0]]",
        "  matrix-world [[1 0 0] [0 1 0] [0 0 1] [0 0 0]]",
    ] {
        let found = errors(&schema, &[&object(wrong)]);
        // One error, or one for each row that is the wrong length.
        assert!(!found.is_empty(), "{wrong}");
        assert!(
            found.iter().all(|d| d.code == codes::TYPE),
            "{wrong}: {found:#?}"
        );
    }
    // Section 6's own example: a 4×4 transform's translation column takes lengths, read through
    // the scene's unit scale; every other item is bare (the 1b-03e review, the lead's call).
    let spec = "  matrix-basis [[1 0 0 2m] [0 1 0 0m] [0 0 1 0m] [0 0 0 1]]";
    assert!(errors(&schema, &[&object(spec)]).is_empty());
    let outside = errors(
        &schema,
        &[&object(
            "  matrix-basis [[1m 0 0 2m] [0 1 0 0m] [0 0 1 0m] [0 0 0 1m]]",
        )],
    );
    assert_eq!(outside.len(), 2, "{outside:#?}");
    assert!(outside.iter().all(|d| d.code == codes::UNIT));
    let angle = errors(
        &schema,
        &[&object(
            "  matrix-basis [[1 0 0 90deg] [0 1 0 0] [0 0 1 0] [0 0 0 1]]",
        )],
    );
    assert_eq!(angle.len(), 1, "{angle:#?}");
    // `instance-type`'s items come from a function: a listed one is taken, a misspelt one is
    // refused with its fix, and one far from every listed item is left to Blender.
    assert!(errors(&schema, &[&object("  instance-type faces")]).is_empty());
    let misspelt = errors(&schema, &[&object("  instance-type facess")]);
    assert_eq!(misspelt.len(), 1, "{misspelt:#?}");
    assert_eq!(misspelt[0].fixes[0].edits[0].replacement, "faces");
    assert!(errors(&schema, &[&object("  instance-type made-at-run-time")]).is_empty());
}

#[test]
fn a_material_takes_the_node_kinds_a_shader_tree_takes() {
    // Node kinds by S67, read in their tree (task 1b-03c): a material's tree is a shader tree.
    let schema = Schema::from_json(PROJECT).unwrap();
    let material =
        |node: &str| format!("material @m \"M\" {{\n  nodes {{\n    node @n {node}\n  }}\n}}\n");
    for kind in ["bsdf-principled", "output-material", "math", "tex-noise"] {
        let found = errors(&schema, &[&material(kind)]);
        assert!(found.is_empty(), "{kind}: {found:#?}");
    }
    // A compositor's own node, and the geometry switch's compositor-tree name, are not a
    // shader tree's; the slice's old spelling is gone.
    for kind in ["r-layers", "geometry-switch", "principled-bsdf"] {
        let found = errors(&schema, &[&material(kind)]);
        assert_eq!(found.len(), 1, "{kind}: {found:#?}");
        assert_eq!(found[0].code, codes::KIND, "{kind}");
    }
    let shader = schema.node_kinds(null_check::SHADER_TREE);
    assert!(shader.len() > 100, "{}", shader.len());
    let switch = schema.nodes.iter().find(|n| {
        n.trees.as_ref().is_some_and(|t| {
            t.get("CompositorNodeTree").map(String::as_str) == Some("geometry-switch")
        })
    });
    assert!(switch.is_some());
}

#[test]
fn cycles_settings_are_their_own_section_and_check_as_blocks() {
    // Task 1b-03g: Cycles' settings, from its own section, written as blocks of the structs a
    // scene, object, light or world holds (spec, section 6, "Pointer").
    let schema = Schema::from_json(PROJECT).unwrap();
    let cycles = &schema.addons["cycles"];
    assert_eq!(cycles.module, "cycles");
    assert!(
        cycles
            .attached
            .iter()
            .any(|a| a.owner == "Scene" && a.property.name == "cycles")
    );
    let render = "\
scene @main \"Main\" {
  cycles {
    samples 64
    device gpu
    use-denoising
    time-limit 30s
  }
  frame-start 1
}

object @cube \"Cube\" {
  cycles { use-motion-blur off }
}

light @lamp \"Lamp\" {
  type point
  cycles { max-bounces 8 }
}

world @sky \"Sky\" {
  cycles { sampling-method automatic }
}
";
    let found = errors(&schema, &[render]);
    assert!(found.is_empty(), "{found:#?}");
    // A misspelt Cycles setting is found, with its fix.
    let misspelt = errors(
        &schema,
        &["scene @main \"Main\" {\n  cycles { sampels 64 }\n}\n"],
    );
    assert_eq!(misspelt.len(), 1, "{misspelt:#?}");
    assert_eq!(misspelt[0].code, codes::UNKNOWN_PROPERTY);
    assert_eq!(misspelt[0].fixes[0].edits[0].replacement, "samples");
    // A value out of Cycles' range is refused.
    let range = errors(
        &schema,
        &["scene @main \"Main\" {\n  cycles { samples 0 }\n}\n"],
    );
    assert_eq!(range.len(), 1, "{range:#?}");
    assert_eq!(range[0].code, codes::RANGE);
    // Without the section (the slice's schema), `cycles` is no property of a scene.
    let slice = Schema::from_json(SLICE).unwrap();
    let without = errors(
        &slice,
        &["scene @main \"Main\" {\n  cycles { samples 64 }\n}\n"],
    );
    assert_eq!(without.len(), 1, "{without:#?}");
    assert_eq!(without[0].code, codes::UNKNOWN_PROPERTY);
}

#[test]
fn a_property_with_a_flat_form_is_an_error_in_a_block() {
    // S70: one spelling per property. A block is for a struct Null does not flatten; `fps` is a
    // scene's own line, so `render { fps 30 }` is an error whose fix moves the line out.
    let schema = Schema::from_json(PROJECT).unwrap();
    let clean = errors(
        &schema,
        &["scene @s \"S\" {\n  fps 24\n  render { resolution-x 1920 }\n}\n"],
    );
    assert!(clean.is_empty(), "{clean:#?}");
    let text = "scene @s \"S\" {\n  render { fps 30; resolution-x 1920 }\n}\n";
    let found = errors(&schema, &[text]);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].code, codes::REPEATED);
    let edits: Vec<_> = found[0].fixes[0]
        .edits
        .iter()
        .map(|e| (e.span, e.replacement.as_str()))
        .collect();
    let fixed = null_syntax::apply_edits(text, &edits);
    assert!(fixed.contains("  fps 30\n"), "{fixed}");
    let after = errors(&schema, &[&fixed]);
    assert!(after.is_empty(), "{fixed}: {after:#?}");
    // The unit settings are written `units`, never as their block.
    let units = errors(
        &schema,
        &["scene @s \"S\" {\n  unit-settings { system metric }\n}\n"],
    );
    assert_eq!(units.len(), 1, "{units:#?}");
    assert!(units[0].message.contains("`units"), "{}", units[0].message);
}

#[test]
fn the_old_principled_bsdf_gets_the_new_name_as_its_fix() {
    let schema = Schema::from_json(PROJECT).unwrap();
    let text = "material @m \"M\" {\n  nodes {\n    node @b principled-bsdf\n  }\n}\n";
    let found = errors(&schema, &[text]);
    assert_eq!(found[0].fixes[0].edits[0].replacement, "bsdf-principled");
}

/// The project schema with `ID.name`'s field length: Blender's, `MAX_ID_NAME - 2` (256 bytes,
/// the closing NUL included), as Sau's build writes it (task 1b-03h). A schema made before that
/// task has none, so the test adds it; once the schema carries it, it must be that.
fn schema_with_name_length() -> Schema {
    let mut json: serde_json::Value = serde_json::from_str(PROJECT).unwrap();
    let id = json["structs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["rna"] == "ID")
        .unwrap();
    let name = id["properties"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p["rna"] == "name")
        .unwrap();
    match name.get("max-length") {
        Some(given) => assert_eq!(given, 256, "ID.name's field is MAX_ID_NAME - 2"),
        None => name["max-length"] = 256.into(),
    }
    Schema::from_json(&json.to_string()).unwrap()
}

fn range_errors(schema: &Schema, text: &str) -> Vec<Diagnostic> {
    errors(schema, &[text])
        .into_iter()
        .filter(|d| d.code == codes::RANGE)
        .collect()
}

#[test]
fn a_name_one_byte_too_long_for_blenders_field_is_an_error() {
    // Spec, section 6: a string longer than Blender's field for it is an error, never cut; the
    // field counts bytes, its closing NUL included (task 1b-03h).
    let schema = schema_with_name_length();
    let fits = "a".repeat(255);
    assert_eq!(
        range_errors(&schema, &format!("object @o \"{fits}\"\n")),
        vec![]
    );
    let too_long = "a".repeat(256);
    let found = range_errors(&schema, &format!("object @o \"{too_long}\"\n"));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0]
            .message
            .contains("`name` holds at most 255 bytes, not 256"),
        "{}",
        found[0].message
    );
    // Bytes, not characters: 127 two-byte letters and one more byte fit; 128 do not.
    let wide = "é".repeat(127);
    assert_eq!(
        range_errors(&schema, &format!("object @o \"{wide}a\"\n")),
        vec![]
    );
    let wider = "é".repeat(128);
    assert_eq!(
        range_errors(&schema, &format!("object @o \"{wider}\"\n")).len(),
        1
    );
    // A collection defined in a scene is held to it too.
    let scene = format!("scene @s \"S\" {{\n  collection @c \"{too_long}\" {{\n  }}\n}}\n");
    assert_eq!(range_errors(&schema, &scene).len(), 1);
}

#[test]
fn a_string_with_no_field_length_takes_any_length() {
    // An unlimited string (no `max-length`) is not held to a length.
    let mut json: serde_json::Value = serde_json::from_str(PROJECT).unwrap();
    for s in json["structs"].as_array_mut().unwrap() {
        for p in s["properties"].as_array_mut().unwrap() {
            p.as_object_mut().unwrap().remove("max-length");
        }
    }
    let schema = Schema::from_json(&json.to_string()).unwrap();
    let long = "a".repeat(5000);
    assert_eq!(
        range_errors(&schema, &format!("object @o \"{long}\"\n")),
        vec![]
    );
}

/// Every enum property in `json`, at any depth: (where, property).
fn enums<'a>(
    json: &'a serde_json::Value,
    at: &str,
    out: &mut Vec<(String, &'a serde_json::Value)>,
) {
    match json {
        serde_json::Value::Object(map) => {
            if map.get("type").and_then(|t| t.as_str()) == Some("enum") {
                out.push((at.to_string(), json));
            }
            let here = map
                .get("rna")
                .and_then(|r| r.as_str())
                .map_or(at.to_string(), |r| format!("{at}/{r}"));
            for v in map.values() {
                enums(v, &here, out);
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|v| enums(v, at, out)),
        _ => {}
    }
}

#[test]
fn every_enum_default_is_one_of_its_items_as_a_file_writes_it() {
    // Task 1b-03j: a default is written as its item is, so the formatter can leave it out: a
    // free-text item keeps its case (`Linear`), and a flag set is in the items' order. An enum
    // whose items are made when it is used (`items-at-run-time`, or the slice's items that
    // depend on another setting, `when`) may default to one the schema does not list.
    for (file, text) in [
        ("null-schema-project.json", PROJECT),
        (
            "null-schema.json",
            include_str!("../../../schema/null-schema.json"),
        ),
    ] {
        let json: serde_json::Value = serde_json::from_str(text).unwrap();
        let mut found = Vec::new();
        enums(&json, "", &mut found);
        assert!(found.len() > 30, "{file}: {} enums", found.len());
        let mut wrong = Vec::new();
        for (at, p) in found {
            let Some(default) = p.get("default") else {
                continue;
            };
            let items: Vec<&serde_json::Value> = p
                .get("items")
                .and_then(|i| i.as_array())
                .map(|i| i.iter().collect())
                .unwrap_or_default();
            let at_run_time = p.get("items-at-run-time").is_some()
                || items.iter().any(|i| i.get("when").is_some());
            let names: Vec<&str> = items.iter().filter_map(|i| i["name"].as_str()).collect();
            let ok = match default {
                serde_json::Value::String(d) => names.contains(&d.as_str()) || at_run_time,
                serde_json::Value::Array(ds) => {
                    let ds: Vec<&str> = ds.iter().filter_map(|d| d.as_str()).collect();
                    let listed: Vec<&str> =
                        names.iter().copied().filter(|n| ds.contains(n)).collect();
                    (ds.iter().all(|d| names.contains(d)) || at_run_time) && ds.starts_with(&listed)
                }
                _ => false,
            };
            if !ok {
                wrong.push(format!(
                    "{at}: {} defaults to {default}, items {names:?}",
                    p["name"]
                ));
            }
        }
        assert!(wrong.is_empty(), "{file}:\n{}", wrong.join("\n"));
    }
}

#[test]
fn an_enums_free_text_item_is_a_string_and_its_fix_is_quoted() {
    // Section 6 (S16): an item whose name is free text, not an identifier (an image node's
    // `Cubic`), is written as a string. Bare, in any case, it is an error whose fix is the
    // quoted item (task 1d-01a, found by checking section 4's example).
    let schema = Schema::from_json(PROJECT).unwrap();
    let material = |value: &str| {
        format!(
            "material @m \"M\" {{\n  nodes {{\n    node @tex tex-image {{ interpolation {value} }}\n  }}\n}}\n"
        )
    };
    assert_eq!(errors(&schema, &[&material("\"Cubic\"")]), vec![]);
    for bare in ["cubic", "Cubic"] {
        let found = errors(&schema, &[&material(bare)]);
        assert_eq!(found.len(), 1, "{bare}: {found:#?}");
        assert!(
            found[0].message.contains("free text"),
            "{}",
            found[0].message
        );
        assert_eq!(found[0].fixes[0].edits[0].replacement, "\"Cubic\"");
    }
    // A misspelling near a free-text item gets the quoted item too.
    let found = errors(&schema, &[&material("cubik")]);
    assert_eq!(
        found[0].fixes[0].edits[0].replacement, "\"Cubic\"",
        "{found:#?}"
    );
}

#[test]
fn none_empties_a_pointer_blender_lets_be_empty() {
    // Sau's task 1c-02j: `none` empties a pointer (section 6), as write-back writes one, where the
    // schema does not mark the pointer `never-none`; where it does, `none` is an error.
    let text =
        "object @lamp \"Lamp\" {\n  parent none\n}\n\nscene @main \"Main\" {\n  camera none\n}\n";
    for (file, json) in [("project", PROJECT), ("slice", SLICE)] {
        let schema = Schema::from_json(json).unwrap();
        assert_eq!(errors(&schema, &[text]), vec![], "{file}");
    }
    // A pointer the schema marks `never-none`: Object.parent marked so here, in a copy of the
    // slice's schema, since no settable pointer of the slice's is.
    let mut json: serde_json::Value = serde_json::from_str(SLICE).unwrap();
    for s in json["structs"].as_array_mut().unwrap() {
        if s["rna"] == "Object" {
            for p in s["properties"].as_array_mut().unwrap() {
                if p["name"] == "parent" {
                    p["never-none"] = serde_json::Value::Bool(true);
                }
            }
        }
    }
    let schema = Schema::from_json(&json.to_string()).unwrap();
    let found = errors(&schema, &[text]);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].code, codes::TYPE);
    assert!(
        found[0].message.contains("never empty") && found[0].message.contains("parent"),
        "{}",
        found[0].message
    );
    // Any other word is still a handle, or its error.
    let found = errors(
        &Schema::from_json(SLICE).unwrap(),
        &["object @lamp \"Lamp\" {\n  parent nothing\n}\n"],
    );
    assert!(found[0].message.contains("takes a handle"), "{found:#?}");
}

#[test]
fn a_curve_and_a_lattice_take_shape_keys_as_a_mesh_does() {
    // RFC 0005 (S88, 1d-07q's review): `shape-key` and `shape-keys` in a curve's and a lattice's
    // blocks, a curve's Bézier handles too; a text curve, which has no shape keys, refuses them.
    let schema = Schema::from_json(PROJECT).unwrap();
    let clean = [
        "curve @path \"Path\" {\n  shape-key @basis \"Basis\"\n  shape-key @bent \"Bent\" {\n    positions [[0m 0m 0m] [1m 0m 1m]]\n    handles-left [[-0.5m 0m 0m] [0.5m 0m 1m]]\n    handles-right [[0.5m 0m 0m] [1.5m 0m 1m]]\n    relative-key @basis\n    value 0.5\n  }\n  shape-keys { use-relative }\n}\n",
        "lattice @cage \"Cage\" {\n  shape-key @basis \"Basis\"\n  shape-key @squash \"Squash\" { positions [[0m 0m 0m]] }\n}\n",
    ];
    for text in clean {
        let found = errors(&schema, &[text]);
        assert!(found.is_empty(), "{text}\n{found:?}");
    }
    // A lattice has no Bézier handles; a shape key's relative key is the owner's own.
    let found = errors(
        &schema,
        &[
            "lattice @cage \"Cage\" {\n  shape-key @squash \"Squash\" { handles-left [[0m 0m 0m]]; relative-key @nope }\n}\n",
        ],
    );
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found.iter().any(|d| d.code == codes::UNKNOWN_PROPERTY),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|d| d.code == codes::HANDLE && d.message.contains("a lattice's shape keys")),
        "{found:?}"
    );
    let found = errors(
        &schema,
        &["text-curve @t \"T\" {\n  shape-key @k \"K\"\n}\n"],
    );
    assert!(!found.is_empty(), "a text curve takes no shape keys");
}
