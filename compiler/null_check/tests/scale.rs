//! The scene's unit scale (task 1c-01d): a value written with a unit is divided by
//! `scale-length`, inside a scene by the scene's, outside one by the project's; never with the
//! system `none`.

use null_check::units::{self, Outside};
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

/// A camera with two lengths in a unit: in range at any scale.
const TWO: &str = "camera @wide \"Wide\" {\n  clip-start 1mm\n  clip-end 10m\n}\n";

fn codes_of(found: &[Diagnostic]) -> Vec<&'static str> {
    found.iter().map(|d| d.code).collect()
}

#[test]
fn with_no_project_units_values_outside_a_scene_use_the_scale_the_scenes_agree_on() {
    // From the 1c-01d review: two scenes at a hundredth agree, so outside them it is a
    // hundredth too.
    let agree = errors(&[
        "scene @a \"A\" {\n  units metric scale=0.01\n}\n",
        "scene @b \"B\" {\n  units { scale-length 0.01 }\n}\n",
        CAMERA,
    ]);
    assert!(agree.is_empty(), "{agree:#?}");
    // A scene with no `units`, or only a system other than `none`, is at 1.
    let at_one = errors(&[
        "scene @a \"A\" {\n  units metric\n}\n",
        "scene @b \"B\"\n",
        CAMERA,
    ]);
    assert_eq!(codes_of(&at_one), [codes::RANGE], "{at_one:#?}");
    // A scene whose system is `none` is at 1 whatever its scale, so it disagrees with one at a
    // hundredth: the first value in a scaled unit outside a scene says so, once.
    let disagree = errors(&[
        "scene @a \"A\" {\n  units none scale=0.01\n}\n",
        "scene @b \"B\" {\n  units metric scale=0.01\n}\n",
        TWO,
    ]);
    assert_eq!(codes_of(&disagree), [codes::UNIT], "{disagree:#?}");
    let d = &disagree[0];
    assert!(d.message.contains("@a 1, @b 0.01"), "{}", d.message);
    assert_eq!(&TWO[d.primary.span.start..d.primary.span.end], "1mm");
    // Bare numbers, and values inside a scene, are not scaled by the outside scale.
    let bare = errors(&[
        "scene @a \"A\" {\n  units scale=2\n}\n",
        "scene @b \"B\" {\n  units scale=0.01\n  time-jump-delta 0.5s\n}\n",
        "camera @c \"C\" {\n  clip-start 0.001\n  lens 35mm\n}\n",
    ]);
    assert!(bare.is_empty(), "{bare:#?}");
}

#[test]
fn a_project_units_line_holds_outside_every_scene_even_when_empty() {
    // `units {}` leaves the project at a new scene's settings, scale 1, whatever the scenes say.
    let empty = errors(&[
        "project \"P\" {\n  units {}\n}\n",
        "scene @a \"A\" {\n  units scale=0.01\n}\n",
        CAMERA,
    ]);
    assert_eq!(codes_of(&empty), [codes::RANGE], "{empty:#?}");
    // It also settles a disagreement.
    let settled = errors(&[
        "project \"P\" {\n  units scale=0.01\n}\n",
        "scene @a \"A\" {\n  units scale=2\n}\n",
        "scene @b \"B\"\n",
        TWO,
        CAMERA,
    ]);
    assert!(settled.is_empty(), "{settled:#?}");
}

#[test]
fn only_the_first_units_line_and_the_first_of_each_setting_count() {
    let lines = errors(&[
        "project \"P\" {\n  units scale=0.01\n  units scale=1\n}\n",
        CAMERA,
    ]);
    assert_eq!(codes_of(&lines), [codes::REPEATED], "{lines:#?}");
    let settings = errors(&[
        "project \"P\" {\n  units { scale 0.01; scale-length 1 }\n}\n",
        CAMERA,
    ]);
    assert_eq!(codes_of(&settings), [codes::REPEATED], "{settings:#?}");
    let system = errors(&[
        "project \"P\" {\n  units metric { system none; scale 0.01 }\n}\n",
        CAMERA,
    ]);
    assert_eq!(codes_of(&system), [codes::REPEATED], "{system:#?}");
}

#[test]
fn the_shared_reader_says_what_the_checker_reads() {
    let parses = [
        parse("project \"P\" {\n  units { scale 0.01; scale-length 1; length mm }\n}\n"),
        parse("scene @a \"A\" {\n  units none scale=2 rotation=deg\n}\n"),
    ];
    let project = units::project(&parses).unwrap();
    assert_eq!(project.scale(), 0.01);
    assert_eq!(project.length.as_deref(), Some("mm"));
    assert_eq!(units::outside(&parses), Outside::Scale(0.01));
    let scenes = units::scenes(&parses[1..]);
    assert_eq!(scenes[0].0, "@a");
    assert_eq!(scenes[0].1.scale(), 1.0);
    assert_eq!(
        units::outside(&[parses[1].clone(), parse("scene @b \"B\"\n")]),
        Outside::Scale(1.0)
    );
    assert_eq!(
        units::outside(&[
            parse("project \"P\"\n"),
            parse("scene @b \"B\" { units scale=3 }\n")
        ]),
        Outside::Scale(3.0)
    );
}

/// The project schema, which has drivers' structs.
fn project_errors(texts: &[&str]) -> Vec<Diagnostic> {
    let schema =
        Schema::from_json(include_str!("../../../schema/null-schema-project.json")).unwrap();
    let parses: Vec<_> = texts.iter().map(|t| parse(t)).collect();
    let files: Vec<_> = parses
        .iter()
        .enumerate()
        .map(|(i, p)| (FileId(i as u32), p))
        .collect();
    check(&schema, &files)
}

#[test]
fn a_drivers_scaled_unit_outside_every_scene_needs_the_scenes_to_agree() {
    // Task 1d-07f: a number in a driver is read as a value is, so where the scenes' unit scales
    // differ and the project sets no `units`, a length in a driver outside them is the same
    // error as a value's, reported once, at the number.
    let scenes = "scene @a \"A\" {\n  units metric scale=1\n}\n\nscene @b \"B\" {\n  units metric scale=0.01\n}\n";
    let lights = "object @d \"D\"\n\nlight @l \"L\" {\n  type point\n  energy driver=\"spin * 1m\" {\n    var spin transforms { target @d transform-type=rot-z }\n  }\n}\n\nlight @k \"K\" {\n  type point\n  energy driver=\"spin * 2m\" {\n    var spin transforms { target @d transform-type=rot-z }\n  }\n}\n";
    let found = project_errors(&[scenes, lights]);
    let units: Vec<&Diagnostic> = found.iter().filter(|d| d.code == codes::UNIT).collect();
    assert_eq!(units.len(), 1, "{found:#?}");
    assert!(units[0].message.contains("scales differ"), "{found:#?}");
    // At `1m`, in the first driver's string.
    let at = lights.find("1m").unwrap();
    assert_eq!(units[0].primary.span.start, at, "{found:#?}");
    // A unit the scale does not touch is read as written, and agreeing scenes settle it.
    let degrees = lights.replace("1m", "90deg").replace("2m", "45deg");
    assert!(project_errors(&[scenes, &degrees]).is_empty());
    let agreeing = scenes.replace("scale=0.01", "scale=1");
    assert!(project_errors(&[&agreeing, lights]).is_empty());
}

#[test]
fn a_drivers_seconds_on_a_frame_property_read_through_its_scene() {
    // Task 1d-07f: a property that counts frames reads a driver's seconds as frames through its
    // scene's frame rate, as a value's (task 1c-01h); outside every scene there is none, so
    // seconds are refused there, as a value's are.
    let outside = "object @d \"D\"\n\ncurve @c \"C\" {\n  eval-time driver=\"spin * 2s\" {\n    var spin transforms { target @d transform-type=rot-z }\n  }\n}\n";
    let found = project_errors(&[outside]);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].code, codes::UNIT);
    assert!(
        found[0]
            .message
            .contains("`2s` is in seconds, and this driver's property counts frames"),
        "{found:#?}"
    );
    assert!(project_errors(&[&outside.replace("spin * 2s", "spin * 48")]).is_empty());
    // Inside a scene, its frame rate: the scene's own driver checks clean.
    let inside = "object @d \"D\"\n\nscene @s \"S\" {\n  fps 25\n  frame-float driver=\"spin * 2s\" {\n    var spin transforms { target @d transform-type=rot-z }\n  }\n}\n";
    assert!(
        project_errors(&[inside]).is_empty(),
        "{:#?}",
        project_errors(&[inside])
    );
}

#[test]
fn a_datablocks_place_is_the_checkers() {
    // Task 1d-07f, round 2: `driver::place` gives Sau the context the checker reads a driver in.
    use null_check::driver::{compile_in, place};
    let a = parse("project \"P\" {}\n\nscene @a \"A\" {\n  units metric scale=1\n  fps 25\n}\n");
    let b = parse("scene @b \"B\" {\n  units metric scale=0.01\n}\n");
    let files = [&a, &b];
    // Outside every scene: the scenes disagree, so a length is the checker's error, word for word.
    let outside = place(&files, None);
    assert!(!outside.in_scene);
    let message = outside.disagreement.clone().expect("the scenes disagree");
    let e = compile_in("t * 1m", &["t"], &outside.context(false)).unwrap_err();
    assert_eq!(e.message, message);
    let lights = "object @d \"D\"\n\nlight @l \"L\" {\n  type point\n  energy driver=\"spin * 1m\" {\n    var spin transforms { target @d transform-type=rot-z }\n  }\n}\n";
    let found = project_errors(&[a.print().as_str(), b.print().as_str(), lights]);
    assert!(found.iter().any(|d| d.message == message), "{found:#?}");
    // Inside a scene: its scale and its frame rate.
    let scene_a = a.document().nodes().find(|n| n.name() == "scene").unwrap();
    let inside = place(&files, Some(scene_a));
    assert!(inside.in_scene && inside.disagreement.is_none());
    assert_eq!((inside.scene.scale_length, inside.scene.fps), (1.0, 25));
    assert_eq!(
        compile_in("t * 2s", &["t"], &inside.context(true))
            .unwrap()
            .expression,
        "t * 50"
    );
    let scene_b = b.document().nodes().find(|n| n.name() == "scene").unwrap();
    assert_eq!(place(&files, Some(scene_b)).scene.scale_length, 0.01);
}
