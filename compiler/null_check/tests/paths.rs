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
    node @bsdf bsdf-principled { roughness 0.62 }
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
fn line(index: &Index, schema: &Schema, word: &str, within: Option<(&str, &str)>) -> String {
    let at = within
        .map(|(k, h)| format!(" (in {k} {h})"))
        .unwrap_or_default();
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
        ok("@bsdf.roughness", Some(("material", "@weathered-oak")))
            .socket
            .as_deref(),
        Some("roughness")
    );
    assert_eq!(ok("@bevel", Some(("object", "@pier"))).kinds, ["modifier"]);
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
    let cases: &[(&str, Option<(&str, &str)>)] = &[
        ("@peir/@bevel", None),
        ("@pier/@bevle", None),
        ("@pier/@tarck", None),
        ("@weathered-oak/@bsfd.roughness", None),
        ("@weathered-oak/@bsdf.rougness", None),
        ("@bsdf.metalic", Some(("material", "@weathered-oak"))),
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

#[test]
fn inside_its_block_a_path_starts_at_the_owners_own_sub_objects() {
    // From the 1b-02c review: a datablock with the same handle as one of the owner's
    // sub-objects must not shadow it, and the owner is one datablock, its kind and handle.
    let text = "\
material @oak \"Oak\" {
  nodes {
    node @bsdf bsdf-principled { roughness 0.5 }
  }
}

object @bsdf \"Bsdf\" {
  modifier @edge bevel
}

object @pier \"Pier\" {
  modifier @bevel bevel
}

mesh @pier \"Pier\"
";
    let p = parse(text);
    let index = Index::build(&[(FileId(0), &p)]);
    let schema = schema();
    let r = index
        .resolve(&schema, "@bsdf.roughness", Some(("material", "@oak")))
        .unwrap();
    assert_eq!(
        (r.owner_kind.as_str(), r.kinds.as_slice()),
        ("material", &["node".to_string()][..])
    );
    assert_eq!(r.socket.as_deref(), Some("roughness"));
    let r = index
        .resolve(&schema, "@bsdf", Some(("material", "@oak")))
        .unwrap();
    assert_eq!(r.kinds, ["node"]);
    // Outside the block, `@bsdf` is the object.
    assert_eq!(
        index.resolve(&schema, "@bsdf", None).unwrap().owner_kind,
        "object"
    );
    // Inside `object @pier` the modifier resolves; inside `mesh @pier` it is not the mesh's,
    // and the error names that owner rather than calling `@pier` ambiguous.
    let r = index
        .resolve(&schema, "@bevel", Some(("object", "@pier")))
        .unwrap();
    assert_eq!(r.kinds, ["modifier"]);
    let e = index
        .resolve(&schema, "@bevel", Some(("mesh", "@pier")))
        .unwrap_err();
    assert!(
        e.message.contains("`@pier` has no sub-object `@bevel`"),
        "{}",
        e.message
    );
    // A path that starts at another datablock is still read as written.
    let r = index
        .resolve(&schema, "@pier/@bevel", Some(("material", "@oak")))
        .unwrap();
    assert_eq!(r.owner_kind, "object");
    // From the second 1b-02c review: once the first step is the owner's own sub-object, a
    // later error stays the owner's, with its fix, and lists the owner's sub-objects.
    let e = index
        .resolve(&schema, "@bsdf.rougness", Some(("material", "@oak")))
        .unwrap_err();
    assert_eq!(e.fix.as_deref(), Some("@bsdf.roughness"), "{}", e.message);
    let e = index
        .resolve(&schema, "@bsdf/@x", Some(("material", "@oak")))
        .unwrap_err();
    assert!(
        e.message
            .contains("`@bsdf` has no sub-object `@x`: it has none"),
        "{}",
        e.message
    );
    assert!(!e.message.contains("@edge"), "{}", e.message);
}

#[test]
fn a_trees_links_and_frames_may_be_written_from_their_datablock() {
    // Short form 8 written in full (task 1d-03j): `@clay/@noise.fac` in `material @clay` is
    // `@noise.fac`, and `parent @clay/@box` is `parent @box`; another datablock's node is not
    // this tree's.
    let schema =
        Schema::from_json(include_str!("../../../schema/null-schema-project.json")).unwrap();
    let errors = |text: &str| -> Vec<String> {
        let p = parse(text);
        check(&schema, &[(FileId(0), &p)])
            .iter()
            .map(|d| format!("{} {}", d.code, d.message))
            .collect()
    };
    let full = "material @clay \"Clay\" {\n  nodes {\n    node @noise tex-noise { scale 4 }\n    node @box frame \"Box\"\n    node @bsdf bsdf-principled {\n      roughness @clay/@noise.fac\n      parent @clay/@box\n    }\n  }\n}\n";
    assert_eq!(errors(full), Vec::<String>::new());
    let other = full.replace("@clay/@noise.fac", "@oak/@noise.fac");
    let found = errors(&other);
    assert!(
        found.len() == 1 && found[0].starts_with(codes::HANDLE),
        "{found:?}"
    );
}
