//! A matrix written row by row, a 4×4 transform's translation in a length unit (spec, section 6;
//! task 1b-03e after review): printed by `print_matrix`, read back exactly, and clean to the
//! checker on the schema of all project data.

use null_check::{Schema, check};
use null_print::print_matrix;
use null_syntax::{FileId, parse};
use null_value::{Dimension, Float, Scene, Unit, read_in};
use proptest::prelude::*;

const PROJECT: &str = include_str!("../../../schema/null-schema-project.json");

/// RNA's flat array of a 4×4 matrix: column by column.
fn column_major(rows: [[f32; 4]; 4]) -> Vec<f32> {
    (0..4)
        .flat_map(|j| (0..4).map(move |i| rows[i][j]))
        .collect()
}

#[test]
fn the_specs_example_prints_as_written() {
    let m = column_major([
        [1.0, 0.0, 0.0, 2.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);
    let text = print_matrix(&m, 4, 4, true, Some(Unit::M), &Scene::default()).unwrap();
    assert_eq!(text, "[[1 0 0 2m] [0 1 0 0m] [0 0 1 0m] [0 0 0 1]]");
    // With no display unit the translation is bare; a matrix that is not a transform is bare
    // throughout.
    assert_eq!(
        print_matrix(&m, 4, 4, true, None, &Scene::default()).unwrap(),
        "[[1 0 0 2] [0 1 0 0] [0 0 1 0] [0 0 0 1]]"
    );
    assert_eq!(
        print_matrix(&m, 4, 4, false, Some(Unit::M), &Scene::default()).unwrap(),
        "[[1 0 0 2] [0 1 0 0] [0 0 1 0] [0 0 0 1]]"
    );
    assert_eq!(
        print_matrix(&m[..15], 4, 4, true, None, &Scene::default()),
        None
    );
    // At unit scale 0.01, 200 Blender units are 2 m.
    let scaled = column_major([
        [1.0, 0.0, 0.0, 200.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);
    let s = Scene {
        scale_length: 0.01,
        ..Scene::default()
    };
    assert_eq!(
        print_matrix(&scaled, 4, 4, true, Some(Unit::M), &s).unwrap(),
        "[[1 0 0 2m] [0 1 0 0m] [0 0 1 0m] [0 0 0 1]]"
    );
}

fn matrix_rows(text: &str) -> Vec<Vec<String>> {
    let file = parse(&format!("m {text}\n"));
    let node = file.document().nodes().next().unwrap();
    let value = node.args().next().unwrap();
    value
        .items()
        .iter()
        .map(|row| row.items().iter().map(|c| c.text().to_string()).collect())
        .collect()
}

proptest! {
    #[test]
    fn a_transform_reads_back_to_the_same_bits(
        cells in proptest::array::uniform16(any::<f32>().prop_filter("finite", |x| x.is_finite())),
        unit in prop_oneof![Just(Unit::M), Just(Unit::Cm), Just(Unit::Mm), Just(Unit::Ft)],
        scale in prop_oneof![Just(1.0f32), Just(0.01), Just(2.5)],
    ) {
        let scene = Scene { scale_length: scale, ..Scene::default() };
        let text = print_matrix(&cells, 4, 4, true, Some(unit), &scene).unwrap();
        let rows = matrix_rows(&text);
        for i in 0..4 {
            for j in 0..4 {
                let dimension = if i < 3 && j == 3 { Dimension::Length } else { Dimension::None };
                let back: f32 = read_in(&rows[i][j], dimension, &scene).unwrap();
                prop_assert_eq!(back.bits(), cells[j * 4 + i].bits(), "{} at {},{}", text, i, j);
            }
        }
    }
}

#[test]
fn the_checker_takes_the_printed_matrix() {
    let schema = Schema::from_json(PROJECT).unwrap();
    let m = column_major([
        [0.5, 0.0, 0.0, 1.25],
        [0.0, 0.5, 0.0, -3.0],
        [0.0, 0.0, 0.5, 0.004],
        [0.0, 0.0, 0.0, 1.0],
    ]);
    let text = print_matrix(&m, 4, 4, true, Some(Unit::Cm), &Scene::default()).unwrap();
    assert_eq!(
        text,
        "[[0.5 0 0 125cm] [0 0.5 0 -300cm] [0 0 0.5 0.4cm] [0 0 0 1]]"
    );
    let file = parse(&format!("object @x \"X\" {{\n  matrix-basis {text}\n}}\n"));
    let found = check(&schema, &[(FileId(0), &file)]);
    assert!(found.is_empty(), "{found:#?}");
}
