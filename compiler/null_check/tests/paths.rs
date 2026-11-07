//! Sub-object paths (task 1b-02c): every path form of the spec's section 7 resolves, and a
//! misspelt owner, modifier, node or socket is an error with the path written right.

use null_check::paths::{Index, Path, Segment};
use null_check::{Schema, check, codes};
use null_syntax::{FileId, Parse, parse};

fn schema() -> Schema {
    Schema::from_json(include_str!("../../../schema/null-schema.json")).unwrap()
}

const PROJECT: &str = "\
object @pier \"Pier\" {
  data mesh @pier-deck
  modifier @array array { count 12 }
  modifier @bevel bevel { width 4mm; segments 3 }
  constraint @track track-to { target @cam }
  material @weathered-oak
}

mesh @pier-deck \"PierDeck\" {
  vertex-group @left \"Left\"
  shape-key @basis \"Basis\"
  uv-map @uv-map \"UVMap\"
}

material @weathered-oak \"Weathered Oak\" {
  nodes {
    node @bsdf principled-bsdf { roughness 0.62 }
    node @out output-material { surface @bsdf.bsdf }
  }
}

armature @rig-bones \"RigBones\" {
  bone @spine-002 \"Spine.002\"
}

object @rig \"Rig\" {
  data armature @rig-bones
  bone @spine-002 {
    constraint @ik ik { chain-count 2 }
  }
}

scene @main \"Main\" {
  view-layer @beauty \"Beauty\"
}

object @cam \"Cam\"
";

fn index() -> Index {
    let p: Parse = parse(PROJECT);
    Index::build(&[(FileId(0), &p)])
}

/// Each case resolved, as one line: what it names, or its error and fix.
fn line(index: &Index, schema: &Schema, word: &str, within: Option<&str>) -> String {
    let at = within.map(|w| format!(" (in {w})")).unwrap_or_default();
    match index.resolve(schema, word, within) {
        Ok(r) => format!(
            "{word}{at}: a {}{}{}",
            r.owner_kind,
            r.kinds
                .iter()
                .map(|k| format!(" > {k}"))
                .collect::<String>(),
            r.socket
                .map(|s| format!(", socket {s}"))
                .unwrap_or_default()
        ),
        Err(e) => format!(
            "{word}{at}: error: {}{}",
            e.message,
            e.fix.map(|f| format!(" [fix: {f}]")).unwrap_or_default()
        ),
    }
}

#[test]
fn every_path_form_of_the_spec_resolves() {
    let (index, schema) = (index(), schema());
    let ok = |word, within| index.resolve(&schema, word, within).unwrap();
    assert_eq!(ok("@pier/@bevel", None).kinds, ["modifier"]);
    assert_eq!(ok("@pier/@track", None).kinds, ["constraint"]);
    let socket = ok("@weathered-oak/@bsdf.roughness", None);
    assert_eq!(
        (socket.kinds, socket.socket.as_deref()),
        (vec!["node".to_string()], Some("roughness"))
    );
    assert_eq!(
        ok("@weathered-oak/@bsdf.bsdf", None).socket.as_deref(),
        Some("bsdf")
    );
    assert_eq!(ok("@rig-bones/@spine-002", None).kinds, ["bone"]);
    assert_eq!(
        ok("@rig/@spine-002/@ik", None).kinds,
        ["bone", "constraint"]
    );
    assert_eq!(ok("@pier-deck/@left", None).kinds, ["vertex-group"]);
    assert_eq!(ok("@pier-deck/@basis", None).kinds, ["shape-key"]);
    assert_eq!(ok("@main/@beauty", None).kinds, ["view-layer"]);
    // Inside its owner's block, a path may leave the owner out.
    assert_eq!(
        ok("@bsdf.roughness", Some("@weathered-oak"))
            .socket
            .as_deref(),
        Some("roughness")
    );
    assert_eq!(ok("@bevel", Some("@pier")).kinds, ["modifier"]);
    // A datablock alone is a path of one step.
    assert_eq!(ok("@pier", None).kinds, Vec::<String>::new());
}

#[test]
fn a_path_parses_into_its_steps() {
    let p = Path::parse("@rig/@spine-002/@ik").unwrap();
    assert_eq!(p.segments.len(), 3);
    assert_eq!(p.socket, None);
    let p = Path::parse("@weathered-oak/@bsdf.roughness").unwrap();
    assert_eq!(p.socket.as_deref(), Some("roughness"));
    let p = Path::parse("@plank.7").unwrap();
    assert_eq!(
        p.segments,
        [Segment {
            handle: "@plank".into(),
            copy: Some(7)
        }]
    );
    let p = Path::parse("@cosmosis-materials:oak").unwrap();
    assert_eq!(p.package.as_deref(), Some("cosmosis-materials"));
    assert_eq!(p.segments[0].handle, "@oak");
    let p = Path::parse("@pier.7/@bevel").unwrap();
    assert_eq!(p.segments[0].copy, Some(7));
    assert!(Path::parse("@bsdf.roughness/@x").is_err());
    assert!(Path::parse("pier/@bevel").is_err());
    assert!(Path::parse("@pier/").is_err());
}

#[test]
fn misspelt_paths_are_errors_with_the_path_written_right() {
    let (index, schema) = (index(), schema());
    let cases: &[(&str, Option<&str>)] = &[
        ("@peir/@bevel", None),
        ("@pier/@bevle", None),
        ("@pier/@tarck", None),
        ("@weathered-oak/@bsfd.roughness", None),
        ("@weathered-oak/@bsdf.rougness", None),
        ("@bsdf.metalic", Some("@weathered-oak")),
        ("@rig/@spine-002/@lk", None),
        ("@rig/@spine-02/@ik", None),
        ("@pier/@bevel.width", None),
        ("@pier/@nothing", None),
        ("@cam/@x", None),
        ("@plank.7", None),
        ("@cosmosis-materials:oak", None),
        ("@pier//@bevel", None),
    ];
    let lines: Vec<String> = cases
        .iter()
        .map(|(w, within)| line(&index, &schema, w, *within))
        .collect();
    insta::assert_snapshot!("misspelt", lines.join("\n"));
}

#[test]
fn a_handle_shared_by_two_sub_objects_of_one_owner_is_an_error_whatever_their_kinds() {
    let text = "\
object @pier \"Pier\" {
  modifier @bevel bevel
  constraint @bevel limit-location
}

object @lamp \"Lamp\" {
  modifier @bevel bevel
}
";
    let p = parse(text);
    let found = check(&schema(), &[(FileId(0), &p)]);
    let handles: Vec<_> = found.iter().filter(|d| d.code == codes::HANDLE).collect();
    assert_eq!(handles.len(), 1, "{found:#?}");
    assert!(
        handles[0]
            .message
            .contains("`@bevel` names two sub-objects of `@pier` (a modifier and a constraint)")
    );
    // Two owners may each have their own `@bevel`.
    assert!(!handles[0].message.contains("@lamp"));
}
