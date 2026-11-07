//! A matrix's written form (spec, section 6): row by row, as `mathutils.Matrix` prints it, from
//! RNA's flat array, which keeps it column by column. In a 4×4 transform (RNA subtype `MATRIX`)
//! the translation column, the first three rows' last items, holds lengths, written as
//! `location` is, in the display unit and through the scene's unit scale when a decimal in it
//! reads back to the same bits; every other item is bare (task 1b-03e, the lead's call).

use null_value::{Float, Scene, Unit, print_bare, print_in_scene};

/// `values`, RNA's flat array of a `rows` × `columns` matrix (row `i`, column `j` at
/// `j × rows + i`), written row by row: `[[1 0 0 2m] [0 1 0 0m] [0 0 1 0m] [0 0 0 1]]`.
/// `transform` marks a 4×4 transform matrix, whose translation is printed in `length` (a length
/// unit) where it fits exactly. `None` when `values` is not `rows` × `columns` long.
pub fn print_matrix<F: Float>(
    values: &[F],
    rows: usize,
    columns: usize,
    transform: bool,
    length: Option<Unit>,
    scene: &Scene,
) -> Option<String> {
    if values.len() != rows * columns || rows == 0 || columns == 0 {
        return None;
    }
    let transform = transform && rows == 4 && columns == 4;
    let rows_text: Vec<String> = (0..rows)
        .map(|i| {
            let cells: Vec<String> = (0..columns)
                .map(|j| {
                    let x = values[j * rows + i];
                    match (transform && i < 3 && j == 3, length) {
                        (true, Some(unit)) => {
                            print_in_scene(x, unit, scene).unwrap_or_else(|| print_bare(x))
                        }
                        _ => print_bare(x),
                    }
                })
                .collect();
            format!("[{}]", cells.join(" "))
        })
        .collect();
    Some(format!("[{}]", rows_text.join(" ")))
}
