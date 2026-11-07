//! Frames and seconds (task 1c-01c): a float frame property takes seconds through its scene's
//! frame rate, `fps` / `fps-base`; an integer frame is always bare.

use null_check::{Schema, check, codes};
use null_syntax::{Diagnostic, FileId, parse};
use null_value::{Dimension, Scene, read_in};

fn schema() -> Schema {
    Schema::from_json(include_str!("../../../schema/null-schema.json")).unwrap()
}

fn errors(text: &str) -> Vec<Diagnostic> {
    let p = parse(text);
    assert!(null_syntax::diagnostics(&p, FileId(0)).is_empty());
    check(&schema(), &[(FileId(0), &p)])
}

#[test]
fn seconds_on_a_float_frame_read_through_the_scenes_frame_rate() {
    // Clean at a new scene's 24 fps and at 30000/1001 fps (29.97).
    for text in [
        "scene @main \"Main\" {\n  frame-float 1.5s\n  time-jump-delta 0.5s\n}\n",
        "scene @main \"Main\" {\n  fps 30000\n  fps-base 1001\n  frame-float 1.5s\n}\n",
        "scene @main \"Main\" {\n  frame-float 36\n  frame-subframe 0.5\n}\n",
    ] {
        let found = errors(text);
        assert!(found.is_empty(), "{text}: {found:#?}");
    }
    // What the checker reads them as: 1.5 s is 36 frames at 24 fps, and at 30000/1001 fps the
    // exact 1.5 × 30000 / 1001, rounded once.
    let at = |fps, fps_base| Scene {
        fps,
        fps_base,
        ..Scene::default()
    };
    assert_eq!(
        read_in::<f32>("1.5s", Dimension::Frames, &at(24, 1.0)),
        Ok(36.0)
    );
    let ntsc: f32 = read_in("1.5s", Dimension::Frames, &at(30000, 1001.0)).unwrap();
    assert_eq!(ntsc, (1.5f64 * 30000.0 / 1001.0) as f32);
}

#[test]
fn an_integer_frame_is_bare_and_a_frame_rate_is_checked() {
    let found = errors("scene @main \"Main\" {\n  frame-start 1.5s\n  fps 0\n  fps-base 0\n}\n");
    let codes: Vec<_> = found.iter().map(|d| d.code).collect();
    assert_eq!(found.len(), 3, "{found:#?}");
    // `frame-start` is an integer: seconds are refused, never rounded to a frame.
    assert!(matches!(codes[0], codes::UNIT | codes::TYPE), "{found:#?}");
    assert_eq!(codes[1..], [codes::RANGE, codes::RANGE]);
}
