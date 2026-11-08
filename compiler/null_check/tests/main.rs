//! This crate's integration tests, in one test binary (Sau task 0-26, S69): each file beside this
//! one is a module here, not a binary of its own, so `cargo test` links one executable, not
//! 12. The files keep their names, so their snapshots and proptest regressions stay where
//! they were. A new test file must be listed here: `every_test_file_is_a_module` fails otherwise.

#[path = "check.rs"]
mod check;
#[path = "components.rs"]
mod components;
#[path = "diagnostics.rs"]
mod diagnostics;
/// The spec's component examples, shared by components.rs and expand.rs.
#[path = "examples/mod.rs"]
mod examples;
#[path = "expand.rs"]
mod expand;
#[path = "flags.rs"]
mod flags;
#[path = "frames.rs"]
mod frames;
#[path = "level0.rs"]
mod level0;
#[path = "output.rs"]
mod output;
#[path = "paths.rs"]
mod paths;
#[path = "project_schema.rs"]
mod project_schema;
#[path = "scale.rs"]
mod scale;
#[path = "spec_examples.rs"]
mod spec_examples;

/// The test files of this folder, each a module above.
const MODULES: &[&str] = &[
    "check.rs",
    "components.rs",
    "diagnostics.rs",
    "expand.rs",
    "flags.rs",
    "frames.rs",
    "level0.rs",
    "output.rs",
    "paths.rs",
    "project_schema.rs",
    "scale.rs",
    "spec_examples.rs",
];

#[test]
fn every_test_file_is_a_module() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut found: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".rs") && n != "main.rs")
        .collect();
    found.sort();
    assert_eq!(
        found, MODULES,
        "a test file in tests/ is not a module of tests/main.rs, so it would never run; add it there"
    );
}
