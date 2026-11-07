//! The writer, over random edits of the slice files: only the changed values' characters change.

use std::path::Path;

use null_check::{Schema, check};
use null_print::{Values, write};
use null_syntax::{FileId, TokenKind, apply_edits, ast::Value, lex, parse};
use null_value::{Dimension, Unit, read, unit_of};
use proptest::prelude::*;

/// The checker's clean slice project.
fn files() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../null_check/tests/slice");
    let mut out: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            let name = p.file_name().unwrap().to_str().unwrap().to_string();
            (name, std::fs::read_to_string(&p).unwrap())
        })
        .collect();
    out.sort();
    out
}

/// Every object in the files: (file index, handle).
fn objects(files: &[(String, String)]) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (i, (_, text)) in files.iter().enumerate() {
        for n in parse(text).document().nodes() {
            if n.name() == "object"
                && let Some(h) = n
                    .args()
                    .next()
                    .and_then(|v| v.as_word().map(str::to_string))
            {
                out.push((i, h));
            }
        }
    }
    out
}

const ATTRS: [(&str, Dimension); 3] = [
    ("location", Dimension::Length),
    ("rotation-euler", Dimension::Angle),
    ("scale", Dimension::None),
];

/// The words of `attr` on object `handle`, if it has it.
fn words(text: &str, handle: &str, attr: &str) -> Option<Vec<String>> {
    let file = parse(text);
    let node = file.document().nodes().find(|n| {
        n.name() == "object" && n.args().next().and_then(|v| v.as_word()) == Some(handle)
    })?;
    let value = node
        .children()
        .find(|c| c.name() == attr)
        .and_then(|c| c.args().next())
        .or_else(|| {
            node.entries().find_map(|e| match e {
                null_syntax::ast::Entry::Prop { key, value } if key.text == attr => Some(value),
                _ => None,
            })
        })?;
    Some(match value {
        Value::List(_) => value.items().iter().map(|v| v.text().to_string()).collect(),
        v => vec![v.text().to_string()],
    })
}

/// A value of each float kind: any finite bits, or a short decimal, or the old value.
fn value() -> impl Strategy<Value = Option<f32>> {
    prop_oneof![
        any::<u32>()
            .prop_map(f32::from_bits)
            .prop_filter("finite", |x| x.is_finite())
            .prop_map(Some),
        (-999i32..999, 0u32..4).prop_map(|(m, e)| Some(m as f32 / 10f32.powi(e as i32))),
        Just(None),
    ]
}

proptest! {
    #[test]
    fn an_edit_changes_only_its_values_characters(
        pick in any::<prop::sample::Index>(),
        attr in 0..ATTRS.len(),
        new in prop::collection::vec(value(), 3),
    ) {
        let files = files();
        let objects = objects(&files);
        let (i, handle) = &objects[pick.index(objects.len())];
        let (attr, dim) = ATTRS[attr];
        let text = &files[*i].1;
        let old = words(text, handle, attr);
        // None keeps the old value, where there is one.
        let values: Vec<f32> = new
            .iter()
            .enumerate()
            .map(|(k, v)| v.unwrap_or_else(|| {
                old.as_ref().map_or(0.0, |w| read::<f32>(&w[k], dim).unwrap())
            }))
            .collect();
        let file = parse(text);
        let edits = write(&file, "object", handle, attr, Values::F32(&values), dim, Some(display(dim))).unwrap();
        let pairs: Vec<_> = edits.iter().map(|e| (e.span, e.replacement.as_str())).collect();
        let after = apply_edits(text, &pairs);

        // The values read back to the bits written.
        let got = words(&after, handle, attr).expect("the attribute is there");
        prop_assert_eq!(got.len(), 3);
        for (w, v) in got.iter().zip(&values) {
            prop_assert_eq!(read::<f32>(w, dim).unwrap().to_bits(), v.to_bits(), "{}", w);
        }

        match &old {
            Some(old) => {
                // Only changed words were touched, each keeping its unit where it fits.
                let changed: Vec<usize> = (0..3).filter(|&k| read::<f32>(&old[k], dim).unwrap().to_bits() != values[k].to_bits()).collect();
                prop_assert_eq!(edits.len(), changed.len());
                for k in 0..3 {
                    if !changed.contains(&k) {
                        prop_assert_eq!(&got[k], &old[k]);
                    } else if let Some(u) = unit_of(&old[k], dim)
                        && null_value::print_in(values[k], u).is_some()
                    {
                        prop_assert_eq!(unit_of(&got[k], dim), Some(u), "{} became {}", old[k], got[k]);
                    }
                }
                // Every other token is as it was: same kinds, same text, comments too.
                let (a, _) = lex(text);
                let (b, _) = lex(&after);
                prop_assert_eq!(a.len(), b.len());
                let mut pa = 0;
                let mut pb = 0;
                let mut differ = 0;
                for (x, y) in a.iter().zip(&b) {
                    let tx = &text[pa..pa + x.len as usize];
                    let ty = &after[pb..pb + y.len as usize];
                    prop_assert_eq!(x.kind, y.kind);
                    if tx != ty {
                        prop_assert_eq!(x.kind, TokenKind::Word);
                        differ += 1;
                    }
                    pa += x.len as usize;
                    pb += y.len as usize;
                }
                prop_assert_eq!(differ, changed.len());
            }
            None => {
                // A new attribute: one insertion, and everything before and after it kept.
                prop_assert_eq!(edits.len(), 1);
                let at = edits[0].span.start;
                prop_assert!(edits[0].span.is_empty());
                prop_assert_eq!(&after[..at], &text[..at]);
                prop_assert_eq!(&after[at + edits[0].replacement.len()..], &text[at..]);
            }
        }

        // The edited project still parses and checks clean.
        let mut all: Vec<(String, String)> = files.clone();
        all[*i].1 = after.clone();
        let parses: Vec<_> = all.iter().map(|(_, t)| parse(t)).collect();
        for p in &parses {
            prop_assert!(null_syntax::diagnostics(p, FileId(0)).is_empty(), "{}", after);
        }
        let refs: Vec<_> = parses.iter().enumerate().map(|(k, p)| (FileId(k as u32), p)).collect();
        let schema = Schema::from_json(include_str!("../../../schema/null-schema.json")).unwrap();
        let found = check(&schema, &refs);
        prop_assert!(found.is_empty(), "{:?}\n{}", found, after);
    }
}

fn display(dim: Dimension) -> Unit {
    match dim {
        Dimension::Angle => Unit::Deg,
        _ => Unit::M,
    }
}

#[test]
fn the_reviews_unit_is_kept() {
    // 1a-07's note: a 17-digit feet value must not lose `ft` on the way back.
    let text = "object @a \"A\" { location [-18750715569787871e-5ft 0m 0m] }\n";
    let x: f64 = read("-187507155697.87872ft", Dimension::Length).unwrap();
    let file = parse(text);
    let edits = write(
        &file,
        "object",
        "@a",
        "location",
        Values::F64(&[x, 0.0, 0.0]),
        Dimension::Length,
        None,
    )
    .unwrap();
    let pairs: Vec<_> = edits
        .iter()
        .map(|e| (e.span, e.replacement.as_str()))
        .collect();
    let after = apply_edits(text, &pairs);
    assert!(after.contains("ft 0m 0m]"), "{after}");
}

#[test]
fn a_new_attribute_goes_where_the_block_has_room() {
    let cases = [
        (
            "object @a \"A\" {\n  data mesh @m\n}\n",
            "object @a \"A\" {\n  data mesh @m\n  scale [2 2 2]\n}\n",
        ),
        (
            "object @a \"A\" { data mesh @m }\n",
            "object @a \"A\" { data mesh @m; scale [2 2 2] }\n",
        ),
        (
            "object @a \"A\" {\n}\n",
            "object @a \"A\" {\n  scale [2 2 2]\n}\n",
        ),
        (
            "object @a \"A\" {}\n",
            "object @a \"A\" { scale [2 2 2] }\n",
        ),
        (
            "object @a \"A\" // the a\n",
            "object @a \"A\" scale=[2 2 2] // the a\n",
        ),
    ];
    for (text, want) in cases {
        let file = parse(text);
        let edits = write(
            &file,
            "object",
            "@a",
            "scale",
            Values::F32(&[2.0; 3]),
            Dimension::None,
            None,
        )
        .unwrap();
        let pairs: Vec<_> = edits
            .iter()
            .map(|e| (e.span, e.replacement.as_str()))
            .collect();
        assert_eq!(apply_edits(text, &pairs), want);
    }
}

#[test]
fn a_value_of_another_shape_is_replaced_whole() {
    let text = "object @a \"A\" { location 1m }\n";
    let file = parse(text);
    let edits = write(
        &file,
        "object",
        "@a",
        "location",
        Values::F32(&[1.0, 2.0, 3.0]),
        Dimension::Length,
        None,
    )
    .unwrap();
    let pairs: Vec<_> = edits
        .iter()
        .map(|e| (e.span, e.replacement.as_str()))
        .collect();
    assert_eq!(
        apply_edits(text, &pairs),
        "object @a \"A\" { location [1m 2m 3m] }\n"
    );
}

#[test]
fn a_frame_written_in_seconds_keeps_its_seconds() {
    // Task 1c-01c: a float frame written in seconds is read and printed through the scene's frame
    // rate, so a new value goes back in seconds where a decimal in seconds reads back exactly.
    use null_print::write_in;
    use null_value::{Dimension, Scene, read_in};
    let text = "scene @main \"Main\" {\n  fps 30000\n  fps-base 1001\n  frame-float 1.5s\n}\n";
    let file = parse(text);
    let scene = Scene {
        fps: 30000,
        fps_base: 1001.0,
        ..Scene::default()
    };
    let two: f32 = read_in("2s", Dimension::Frames, &scene).unwrap();
    let edits = write_in(
        &file,
        "scene",
        "@main",
        "frame-float",
        Values::F32(&[two]),
        Dimension::Frames,
        None,
        &scene,
    )
    .unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].replacement, "2s");
    // The same value again changes nothing.
    let same: f32 = read_in("1.5s", Dimension::Frames, &scene).unwrap();
    let none = write_in(
        &file,
        "scene",
        "@main",
        "frame-float",
        Values::F32(&[same]),
        Dimension::Frames,
        None,
        &scene,
    )
    .unwrap();
    assert!(none.is_empty(), "{none:?}");
}
