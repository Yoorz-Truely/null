//! Level 0 arrays (spec, section 9; task 1d-06b): a mesh's `edges`, attributes by domain and
//! type, UV maps and vertex-group weights, inline or in data files named with a checked
//! `count=` and `bounds=` (Sau's decision S74).

use null_check::{Schema, check, codes};
use null_syntax::{Diagnostic, FileId, parse};

const PROJECT: &str = include_str!("../../../schema/null-schema-project.json");

fn schema() -> &'static Schema {
    static S: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
    S.get_or_init(|| Schema::from_json(PROJECT).unwrap())
}

fn errors(texts: &[&str]) -> Vec<Diagnostic> {
    let parses: Vec<_> = texts.iter().map(|t| parse(t)).collect();
    let files: Vec<_> = parses
        .iter()
        .enumerate()
        .map(|(i, p)| (FileId(i as u32), p))
        .collect();
    check(schema(), &files)
}

fn messages(texts: &[&str]) -> Vec<String> {
    errors(texts)
        .into_iter()
        .map(|d| format!("{}: {}", d.code, d.message))
        .collect()
}

/// A square of two triangles: 4 points, 5 edges, 2 faces, 6 corners.
const SQUARE: &str = "\
mesh @square \"Square\" {
  vertices [[0m 0m 0m] [1m 0m 0m] [1m 1m 0m] [0m 1m 0m]]
  edges [[0 1] [1 2] [2 3] [3 0] [0 2]]
  polygons [[0 1 2] [0 2 3]]
  attribute @height point float \"height\" [0 0.5 1 0.25]
  attribute @crease-edge edge float \"crease_edge\" [0 0 0 0 1]
  attribute @shade face boolean \"sharp_face\" [on off]
  attribute @tint corner float-color \"Tint\" [[1 0 0 1] [0 1 0 1] [0 0 1 1] [1 0 0 1] [0 0 1 1] [1 1 1 1]]
  attribute @ids point int32-2d \"ids\" [[1 2] [3 4] [5 6] [7 8]]
  attribute @frame face float4x4 \"frame\" [[[1 0 0 0] [0 1 0 0] [0 0 1 0] [0 0 0 1]] [[1 0 0 0] [0 1 0 0] [0 0 1 0] [0 0 0 1]]]
  attribute @label point string \"label\" [\"a\" \"b\" \"c\" \"d\"]
  uv-map @uv-map \"UVMap\" [[0 0] [1 0] [1 1] [0 0] [1 1] [0 1]]
  vertex-group @left \"Left\" { weights [[0 1] [3 0.5]] }
}
";

#[test]
fn every_level0_line_inline_checks_clean() {
    assert_eq!(messages(&[SQUARE]), Vec::<String>::new());
}

#[test]
fn an_array_of_another_length_than_its_domain_is_an_error() {
    let wrong = SQUARE
        .replace("[0 0.5 1 0.25]", "[0 0.5 1]")
        .replace("[[0 0] [1 0] [1 1] [0 0] [1 1] [0 1]]", "[[0 0] [1 0]]");
    let found = messages(&[&wrong]);
    assert!(
        found
            .iter()
            .any(|m| m.contains("one value per point: 4, not 3")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|m| m.contains("one value per corner: 6, not 2")),
        "{found:?}"
    );
}

#[test]
fn a_wrong_domain_type_value_or_index_is_an_error() {
    for (from, to, what) in [
        (
            "point float \"height\"",
            "vertex float \"height\"",
            "not a domain",
        ),
        (
            "point float \"height\"",
            "point floot \"height\"",
            "not an attribute's data type",
        ),
        ("[on off]", "[on maybe]", "on or off"),
        (
            "[[0 1] [1 2] [2 3] [3 0] [0 2]]",
            "[[0 1] [1 2] [2 3] [3 0] [0 9]]",
            "edges",
        ),
        (
            "weights [[0 1] [3 0.5]]",
            "weights [[3 1] [0 0.5]]",
            "increasing vertex index",
        ),
        (
            "weights [[0 1] [3 0.5]]",
            "weights [[0 1] [3 1.5]]",
            "weights",
        ),
    ] {
        let found = messages(&[&SQUARE.replace(from, to)]);
        assert!(found.iter().any(|m| m.contains(what)), "{to}: {found:?}");
    }
}

const ROCK: &str = "\
mesh @rock \"Rock\" {
  vertices file=\"rock.vertices.noll\" count=3 bounds=[[0m 0m 0m] [1m 2m 0.5m]]
  polygons [[0 1 2]]
  attribute @heat point float \"heat\" file=\"rock.heat.noll\" count=3 bounds=[0.25 4]
}
";

const ROCK_VERTICES: &str = "\
array mesh @rock vertices [
  [0m 0m 0m]
  [1m 0m 0.5m]
  [0m 2m 0m]
]
";

const ROCK_HEAT: &str = "\
array mesh @rock @heat [
  0.25
  4
  1
]
";

#[test]
fn a_data_file_named_with_its_summary_checks_clean() {
    assert_eq!(
        messages(&[ROCK, ROCK_VERTICES, ROCK_HEAT]),
        Vec::<String>::new()
    );
}

#[test]
fn a_summary_that_disagrees_with_its_data_file_is_an_error() {
    // The count.
    let found = errors(&[
        &ROCK.replace("count=3 bounds=[[", "count=4 bounds=[["),
        ROCK_VERTICES,
        ROCK_HEAT,
    ]);
    assert!(
        found
            .iter()
            .any(|d| d.code == codes::TYPE
                && d.message.contains("`count=4`, but the data file holds 3")),
        "{found:?}"
    );
    // The bounds.
    let found = errors(&[
        &ROCK.replace("[1m 2m 0.5m]", "[1m 2m 0.4m]"),
        ROCK_VERTICES,
        ROCK_HEAT,
    ]);
    assert!(
        found
            .iter()
            .any(|d| d.code == codes::RANGE && d.message.contains("`bounds=` disagrees")),
        "{found:?}"
    );
    let found = errors(&[
        &ROCK.replace("bounds=[0.25 4]", "bounds=[0 4]"),
        ROCK_VERTICES,
        ROCK_HEAT,
    ]);
    assert!(found.iter().any(|d| d.code == codes::RANGE), "{found:?}");
    // A wrong element in the data file is reported there.
    let found = errors(&[
        ROCK,
        &ROCK_VERTICES.replace("[0m 2m 0m]", "[0m 2m]"),
        ROCK_HEAT,
    ]);
    assert!(
        found.iter().any(|d| d.primary.file == FileId(1)),
        "{found:?}"
    );
}

#[test]
fn a_line_without_its_data_file_and_a_data_file_without_its_line_are_errors() {
    let found = messages(&[ROCK, ROCK_VERTICES]);
    assert!(
        found
            .iter()
            .any(|m| m.contains("no data file holds `@heat` of mesh @rock")),
        "{found:?}"
    );
    let stray = "array mesh @rock normals [\n  [0 0 1]\n]\n";
    let found = messages(&[ROCK, ROCK_VERTICES, ROCK_HEAT, stray]);
    assert!(
        found
            .iter()
            .any(|m| m.contains("no line names this data file")),
        "{found:?}"
    );
    // A data file's line needs its count (S74 (1)).
    let found = messages(&[
        &ROCK.replace(" count=3 bounds=[0.25 4]", ""),
        ROCK_VERTICES,
        ROCK_HEAT,
    ]);
    assert!(
        found.iter().any(|m| m.contains("gives its `count=`")),
        "{found:?}"
    );
}

/// A mesh whose float attribute's data file is `values`, named with `bounds`.
fn heat(values: &str, bounds: &str) -> (String, String) {
    let mesh = format!(
        "mesh @rock \"Rock\" {{\n  vertices [[0m 0m 0m] [1m 0m 0m] [0m 1m 0m]]\n  polygons [[0 1 2]]\n  attribute @heat point float \"heat\" file=\"rock.heat.noll\" count=3{bounds}\n}}\n"
    );
    let data = format!("array mesh @rock @heat [\n{values}]\n");
    (mesh, data)
}

#[test]
fn bounds_order_minus_zero_below_zero_in_any_order_and_leave_nans_out() {
    // Sau's decision S75: each component's smallest and largest by `total_cmp`.
    for values in ["  0\n  -0\n  1\n", "  -0\n  0\n  1\n", "  1\n  0\n  -0\n"] {
        let (mesh, data) = heat(values, " bounds=[-0 1]");
        assert_eq!(messages(&[&mesh, &data]), Vec::<String>::new(), "{values}");
        let (mesh, data) = heat(values, " bounds=[0 1]");
        let found = messages(&[&mesh, &data]);
        assert!(
            found.iter().any(|m| m.contains("`bounds=` disagrees")),
            "{values}: {found:?}"
        );
    }
    // A NaN is left out of the bounds.
    let (mesh, data) = heat("  0.5\n  nan\n  2\n", " bounds=[0.5 2]");
    assert_eq!(messages(&[&mesh, &data]), Vec::<String>::new());
    // An all-NaN array has no bounds.
    let (mesh, data) = heat("  nan\n  nan\n  nan\n", " bounds=[0 0]");
    let found = messages(&[&mesh, &data]);
    assert!(found.iter().any(|m| m.contains("only NaNs")), "{found:?}");
    let (mesh, data) = heat("  nan\n  nan\n  nan\n", "");
    assert_eq!(messages(&[&mesh, &data]), Vec::<String>::new());
}

#[test]
fn a_second_data_file_for_one_line_says_so() {
    let found = errors(&[ROCK, ROCK_VERTICES, ROCK_HEAT, ROCK_HEAT]);
    let second: Vec<_> = found
        .iter()
        .filter(|d| {
            d.message
                .contains("a second data file for `@heat` of mesh @rock")
        })
        .collect();
    assert_eq!(second.len(), 1, "{found:?}");
    assert!(
        !found.iter().any(|d| d.message.contains("no line names")),
        "{found:?}"
    );
}

#[test]
fn a_key_files_keys_are_checked_at_their_own_lines_against_the_f_curves_property() {
    // Section 11, S86 (task 1d-07t): a key file named with its summary checks clean, in the
    // full form as in the short; a key's mistake is reported in the key file, at its line, by the
    // property its f-curve animates (a light's energy, a power).
    let action = "action @flicker \"Flicker\" {\n  slot @lamp light \"Lamp\" {\n    fcurve energy {\n      keys file=\"flicker.lamp.energy.keys.noll\" count=2 frames=[1 6]\n    }\n  }\n}\n";
    let keys = "keys @flicker @lamp energy {\n  key 1 800W\n  key 6 620W interpolation=linear\n}\n";
    assert_eq!(messages(&[action, keys]), Vec::<String>::new());
    let layered = "action @hop \"Hop\" {\n  slot @ball object \"Ball\"\n  layer @base \"Base\" {\n    strip @keys keyframe {\n      channels @ball {\n        fcurve location index=2 {\n          keys file=\"hop.ball.location-2.keys.noll\" count=1 frames=[12 12]\n        }\n      }\n    }\n  }\n}\n";
    let hop_keys = "keys @hop @ball location index=2 {\n  key 12 1.5m\n}\n";
    assert_eq!(messages(&[layered, hop_keys]), Vec::<String>::new());
    let bad = "keys @flicker @lamp energy {\n  key 1 800m\n  key 6 620W interpolation=lineer\n}\n";
    let found = errors(&[action, bad]);
    let codes_found: Vec<&str> = found.iter().map(|d| d.code).collect();
    assert_eq!(codes_found, [codes::UNIT, codes::ENUM], "{found:#?}");
    assert!(
        found.iter().all(|d| d.primary.file == FileId(1)),
        "{found:#?}"
    );
}
