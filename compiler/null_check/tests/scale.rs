//! The scene's unit scale (task 1c-01d): a value written with a unit is divided by
//! `scale-length`, inside a scene by the scene's, outside one by the project's; never with the
//! system `none`.

use null_check::{Schema, check, codes};
use null_syntax::{Diagnostic, FileId, parse};
use null_value::{Dimension, Scene, read_in};

fn errors(texts: &[&str]) -> Vec<Diagnostic> {
    let schema = Schema::from_json(include_str!("../../../schema/null-schema.json")).unwrap();
    let parses: Vec<_> = texts.iter().map(|t| parse(t)).collect();
    let files: Vec<_> = parses
        .iter()
        .enumerate()
        .map(|(i, p)| (FileId(i as u32), p))
        .collect();
    check(&schema, &files)
}

/// A camera whose `clip-start`, 2e-8 m as written, is below RNA's 1e-6 at scale 1 and 2e-6
/// Blender units, within range, at scale 0.01.
const CAMERA: &str = "camera @lens \"Lens\" {\n  clip-start 0.00000002m\n}\n";

#[test]
fn the_projects_scale_divides_values_outside_a_scene() {
    let at_one = errors(&["project \"P\" {\n  units metric scale=1\n}\n", CAMERA]);
    assert_eq!(
        at_one.iter().map(|d| d.code).collect::<Vec<_>>(),
        [codes::RANGE],
        "{at_one:#?}"
    );
    let at_hundredth = errors(&["project \"P\" {\n  units metric scale=0.01\n}\n", CAMERA]);
    assert!(at_hundredth.is_empty(), "{at_hundredth:#?}");
    // The scale as its own line, and in a block.
    let lines = errors(&[
        "project \"P\" {\n  units { system metric; scale-length 0.01 }\n}\n",
        CAMERA,
    ]);
    assert!(lines.is_empty(), "{lines:#?}");
    // With the system `none`, Blender applies no scale, and neither does Null.
    let none = errors(&["project \"P\" {\n  units none scale=0.01\n}\n", CAMERA]);
    assert_eq!(
        none.iter().map(|d| d.code).collect::<Vec<_>>(),
        [codes::RANGE],
        "{none:#?}"
    );
    // With no project `units`, the one scene's that sets them.
    let scene = errors(&[
        "scene @main \"Main\" {\n  units metric scale=0.01\n}\n",
        CAMERA,
    ]);
    assert!(scene.is_empty(), "{scene:#?}");
}

#[test]
fn a_scenes_own_values_use_its_scale() {
    // `time-jump-delta` is a time, never scaled; the scene's frame and length values are read
    // with its own units, which override the project's field by field.
    let found = errors(&[
        "project \"P\" {\n  units metric scale=2\n}\n",
        "scene @main \"Main\" {\n  units scale=0.01\n  time-jump-delta 0.5s\n}\n",
    ]);
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn one_metre_stores_a_hundred_at_a_hundredth_and_a_half_at_two() {
    let at = |scale_length| Scene {
        scale_length,
        ..Scene::default()
    };
    assert_eq!(
        read_in::<f32>("1m", Dimension::Length, &at(0.01)),
        Ok(100.0)
    );
    assert_eq!(read_in::<f32>("1m", Dimension::Length, &at(2.0)), Ok(0.5));
}
