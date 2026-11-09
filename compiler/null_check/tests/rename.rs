//! Renaming a handle (task 1b-01h, Sau's S17): every word that names the datablock changes, and
//! nothing else, across files; a rename that cannot be made says why.

use null_check::rename::{RenameError, rename};
use null_syntax::{FileId, Parse, apply_edits, parse};

/// The files after renaming `old` to `new`, with the kind and the reference count.
fn renamed(
    files: &[&str],
    old: &str,
    new: &str,
) -> Result<(Vec<String>, String, usize), RenameError> {
    let parses: Vec<Parse> = files.iter().map(|t| parse(t)).collect();
    let ids: Vec<(FileId, &Parse)> = parses
        .iter()
        .enumerate()
        .map(|(i, p)| (FileId(i as u32), p))
        .collect();
    let r = rename(&ids, old, new)?;
    let out = files
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let edits: Vec<_> = r
                .edits
                .iter()
                .filter(|e| e.file == FileId(i as u32))
                .map(|e| (e.span, e.replacement.as_str()))
                .collect();
            apply_edits(text, &edits)
        })
        .collect();
    Ok((out, r.kind, r.references))
}

const SCENE: &str = "\
scene @main \"Main\" {
  camera @cam
  collection \"Props\" { object @pier; object @rig }
}
";

const OBJECTS: &str = "\
object @pier \"Pier\" {
  data mesh @pier-deck
  modifier @bevel bevel { width 4mm }
  constraint @track track-to { target @cam }
  material @oak
}

object @cam \"Cam\" {
  data camera @lens
  parent @pier
}

mesh @pier-deck \"PierDeck\" {
  vertex-group @left \"Left\"
}

camera @lens \"Lens\"

material @oak \"Oak\" {
  nodes {
    node @tex image-texture { image @oak-image }
    node @bsdf bsdf-principled { base-color @tex.color }
    node @out output-material { surface @bsdf.bsdf }
  }
  layout { @tex [0 0]; @bsdf [200 0]; @out [400 0] }
}

image @oak-image \"Oak Image\"
";

#[test]
fn every_reference_in_every_file_changes_and_nothing_else() {
    let (out, kind, refs) = renamed(&[SCENE, OBJECTS], "@cam", "@camera").unwrap();
    assert_eq!(kind, "object");
    // The scene's member, the constraint's target: two references, and the definition.
    assert_eq!(refs, 2);
    assert_eq!(out[0], SCENE.replace("camera @cam", "camera @camera"));
    let want = OBJECTS
        .replace("target @cam }", "target @camera }")
        .replace("object @cam \"Cam\"", "object @camera \"Cam\"");
    assert_eq!(out[1], want);
}

#[test]
fn a_longer_handle_that_starts_the_same_is_left_alone() {
    let (out, _, refs) = renamed(&[SCENE, OBJECTS], "@pier", "@jetty").unwrap();
    // `@pier-deck` is another datablock; `parent @pier` and the scene's member are references.
    assert_eq!(refs, 2);
    assert!(out[1].contains("mesh @pier-deck \"PierDeck\""));
    assert!(out[1].contains("data mesh @pier-deck"));
    assert!(out[1].contains("parent @jetty"));
    assert!(out[1].contains("object @jetty \"Pier\""));
    assert!(out[0].contains("object @jetty; object @rig"));
}

#[test]
fn a_node_and_its_outputs_are_not_the_datablock() {
    // A material whose node has the same handle as an image: the node's line, its output and
    // the layout keep `@oak-image`; the setting that names the image changes.
    let text = "\
material @m \"M\" {
  nodes {
    node @oak-image image-texture { image @oak-image }
    node @out output-material { surface @oak-image.color }
  }
  layout { @oak-image [0 0] }
}

image @oak-image \"Oak Image\"
";
    let (out, kind, _) = renamed(&[text], "@oak-image", "@grain").unwrap();
    assert_eq!(kind, "image");
    // Inside its owner a sub-object's handle shadows the datablock's, so the material is left
    // as it was (section 7); only the image's own line changes.
    assert_eq!(
        out[0],
        text.replace(
            "image @oak-image \"Oak Image\"",
            "image @grain \"Oak Image\""
        )
    );
}

#[test]
fn a_tree_setting_that_names_a_datablock_changes() {
    let (out, _, refs) = renamed(&[OBJECTS], "@oak-image", "@grain").unwrap();
    assert_eq!(refs, 1);
    assert!(out[0].contains("image-texture { image @grain }"));
    assert!(out[0].contains("image @grain \"Oak Image\""));
}

#[test]
fn a_sub_object_with_the_handle_is_not_renamed() {
    let text = "\
object @left \"Left\" {
  data mesh @deck
}

mesh @deck \"Deck\" {
  vertex-group @left \"Left\"
}

object @other \"Other\" {
  parent @left
}
";
    let (out, kind, refs) = renamed(&[text], "@left", "@port").unwrap();
    assert_eq!(kind, "object");
    assert_eq!(refs, 1);
    assert!(out[0].contains("object @port \"Left\""));
    assert!(out[0].contains("vertex-group @left \"Left\""));
    assert!(out[0].contains("parent @port"));
}

#[test]
fn a_path_changes_its_first_handle_and_a_copy_is_its_placements() {
    // `@pier.7` is a pattern's copy, which takes its placement's handle (section 8): it never
    // names datablock `@pier`, so it stays.
    let text = "\
object @pier \"Pier\" {
  modifier @bevel bevel { width 4mm }
}

object @copy \"Copy\" {
  parent @pier.7
  constraint @c copy-location { target @pier/@bevel }
}
";
    let (out, _, refs) = renamed(&[text], "@pier", "@jetty").unwrap();
    assert_eq!(refs, 1);
    assert!(out[0].contains("parent @pier.7"));
    assert!(out[0].contains("target @jetty/@bevel"));
}

#[test]
fn the_handle_of_a_copy_outside_an_override_is_its_placements_not_the_component() {
    // The batch 3b review's case: component `@plank` placed as `@plank`, and a constraint
    // outside any override aiming at copy 7. Renaming the component changes its definition and
    // `use`'s first word; the placement and `target @plank.7` stay, so the copy still resolves.
    let plank = "component @plank \"Plank\" {\n  object @plank \"Plank\"\n}\n";
    let pier = "\
collection @pier \"Pier\" {
  use @plank @plank count=20
}

object @lamp \"Lamp\" {
  constraint @follow copy-location \"Follow\" { target @plank.7 }
}
";
    let (out, kind, refs) = renamed(&[plank, pier], "@plank", "@board").unwrap();
    assert_eq!(kind, "component");
    assert_eq!(refs, 1);
    assert_eq!(
        out[0],
        plank.replace("component @plank", "component @board")
    );
    assert_eq!(
        out[1],
        pier.replace("use @plank @plank", "use @board @plank")
    );
}

#[test]
fn a_handle_made_from_a_name_is_written_before_it() {
    let text = "\
object \"Old Pier\" {
  data mesh @deck
}

object @cam \"Cam\" {
  parent @old-pier
}
";
    let (out, _, refs) = renamed(&[text], "@old-pier", "@pier").unwrap();
    assert_eq!(refs, 1);
    assert!(out[0].contains("object @pier \"Old Pier\""), "{}", out[0]);
    assert!(out[0].contains("parent @pier"));
}

#[test]
fn a_collection_defined_inside_a_scene_is_renamed() {
    let (out, kind, _) = renamed(&[SCENE], "@props", "@set-dressing").unwrap();
    assert_eq!(kind, "collection");
    assert!(
        out[0].contains("collection @set-dressing \"Props\""),
        "{}",
        out[0]
    );
}

#[test]
fn strings_and_comments_are_never_touched() {
    let text = "\
// @cam is the camera
object @cam \"@cam\" {
  parent @pier
}

object @pier \"Pier\"
";
    let (out, _, refs) = renamed(&[text], "@cam", "@lens").unwrap();
    assert_eq!(refs, 0);
    assert_eq!(out[0], text.replace("object @cam ", "object @lens "));
}

#[test]
fn a_rename_that_cannot_be_made_says_why() {
    assert!(matches!(
        renamed(&[OBJECTS], "cam", "@x"),
        Err(RenameError::NotAHandle(h)) if h == "cam"
    ));
    assert!(matches!(
        renamed(&[OBJECTS], "@cam", "@Big"),
        Err(RenameError::NotAHandle(_))
    ));
    assert!(matches!(
        renamed(&[OBJECTS], "@ghost", "@x"),
        Err(RenameError::NoSuchHandle(_))
    ));
    // Same kind with the new handle already.
    assert!(matches!(
        renamed(&[OBJECTS], "@cam", "@pier"),
        Err(RenameError::Taken(k)) if k == "object"
    ));
    // Two kinds share the handle.
    let shared = "object @a \"A\"\nmesh @a \"A\"\n";
    assert!(matches!(
        renamed(&[shared], "@a", "@b"),
        Err(RenameError::Ambiguous(k)) if k == ["mesh", "object"]
    ));
    // A file with a syntax error.
    assert!(matches!(
        renamed(&[OBJECTS, "object @x {"], "@cam", "@y"),
        Err(RenameError::Unparsed(FileId(1)))
    ));
}

#[test]
fn another_kind_may_already_have_the_new_handle() {
    // Handles are unique per kind (S16): an object may take the handle a mesh has.
    let (out, kind, _) = renamed(&[OBJECTS], "@cam", "@lens").unwrap();
    assert_eq!(kind, "object");
    assert!(out[0].contains("object @lens \"Cam\""));
    assert!(out[0].contains("camera @lens \"Lens\""));
}

#[test]
fn the_project_line_is_not_a_datablock() {
    let text = "project \"One File\" {\n  scene @main\n}\n\nscene @main \"Main\"\n";
    assert!(matches!(
        renamed(&[text], "@one-file", "@x"),
        Err(RenameError::NoSuchHandle(_))
    ));
    let (out, kind, refs) = renamed(&[text], "@main", "@studio").unwrap();
    assert_eq!((kind.as_str(), refs), ("scene", 1));
    assert!(out[0].contains("  scene @studio\n"));
}

#[test]
fn a_components_placements_and_own_datablocks_are_not_the_component() {
    // Found assembling batch 3b: 1d-08a's corpus places component `@rig` as placement `@rig`,
    // overrides its copy's bone by a path from that placement, and defines `object @rig` inside
    // the component. Renaming the component changes its definition and `use`'s first word only;
    // the placement, the override's path and the component's own object keep their handles, and
    // a reference out of the component to a top-level datablock is still renamed.
    let component = "\
component @rig \"Rig\" {
  object @rig \"Rig\" {
    data armature @rig-bones
    bone @spine \"Spine\" {
      constraint @reach kinematic \"IK\" { target @lid }
    }
  }
  armature @rig-bones \"Rig\"
}
";
    let woods = "\
collection @woods \"Woods\" {
  use @rig @rig count=2 { }
  override @rig.2/@spine/@reach { chain-count 3 }
}
";
    let lid = "object @lid \"Lid\"\n";
    let (out, kind, refs) = renamed(&[component, woods, lid], "@rig", "@rigging").unwrap();
    assert_eq!(kind, "component");
    assert_eq!(refs, 1);
    assert_eq!(
        out[0],
        component.replace("component @rig \"Rig\"", "component @rigging \"Rig\"")
    );
    assert_eq!(out[1], woods.replace("use @rig @rig", "use @rigging @rig"));
    let (out, _, refs) = renamed(&[component, woods, lid], "@lid", "@cover").unwrap();
    assert_eq!(refs, 1);
    assert!(out[0].contains("target @cover"), "{}", out[0]);
}

#[test]
fn inside_a_component_its_own_datablock_shadows_it() {
    // 1d-08a's `@branch`: the component defines `object @branch`, and its recursive `use`'s
    // `parent @branch` is that object. Renaming the component changes its definition and each
    // `use @branch` only.
    let branch = "\
component @branch \"Branch\" {
  param depth 4 { countdown }
  object @branch \"Branch\"
  if \"depth > 0\" {
    use @branch @left {
      depth \"depth - 1\"
      parent @branch
    }
  }
}

component @fence \"Fence\" {
  object @fence \"Fence\" { }
  use @branch @row count=2 { parent @fence }
}
";
    let woods = "collection @woods \"Woods\" {\n  use @branch @tree { depth 3 }\n}\n";
    let (out, _, refs) = renamed(&[branch, woods], "@branch", "@bough").unwrap();
    // A `use` right inside another component's block is a reference too (1d-08a's `@fence`).
    assert_eq!(refs, 3);
    assert_eq!(
        out[0],
        branch
            .replace("component @branch", "component @bough")
            .replace("use @branch @left", "use @bough @left")
            .replace("use @branch @row", "use @bough @row")
    );
    assert_eq!(out[1], woods.replace("use @branch", "use @bough"));
}
