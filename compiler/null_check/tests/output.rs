//! A slice material connected to its output checks clean against the schema Sau generates
//! (Sau task 1a-05b): the `output-material` node and its `surface` input are in the schema.

use null_check::{Schema, check};
use null_syntax::{Diagnostic, FileId, Parse, SourceMap, parse};

fn schema() -> Schema {
    Schema::from_json(include_str!("../../../schema/null-schema.json")).unwrap()
}

/// Checks `text` as a one-file project; returns the map, for rendering, and the problems.
fn run(text: &str) -> (SourceMap, Vec<Diagnostic>) {
    let mut map = SourceMap::new();
    let id: FileId = map.add("materials/oak.noll", text);
    let p: Parse = parse(text);
    let syntax = null_syntax::diagnostics(&p, id);
    assert!(syntax.is_empty(), "{}", map.render_all(&syntax));
    let found = check(&schema(), &[(id, &p)]);
    (map, found)
}

const CONNECTED: &str = "\
material @oak \"Weathered Oak\" {
  nodes {
    node @bsdf principled-bsdf {
      base-color [0.8 0.62 0.41 1]
      roughness 0.62
    }
    node @out output-material {
      surface @bsdf.bsdf
    }
  }
  layout { @bsdf [-160 0]; @out [120 0] }
}
";

#[test]
fn a_material_connected_to_its_output_checks_clean() {
    let (map, found) = run(CONNECTED);
    assert!(found.is_empty(), "{}", map.render_all(&found));
}

#[test]
fn the_output_is_a_node_kind_with_a_surface_input() {
    let schema = schema();
    let out = schema
        .nodes
        .iter()
        .find(|n| n.kind == "output-material")
        .expect("output-material in the schema");
    assert!(out.inputs.iter().any(|i| i.name == "surface"));
}

#[test]
fn a_link_to_an_output_the_bsdf_lacks_is_an_error() {
    let (map, found) = run(&CONNECTED.replace("@bsdf.bsdf", "@bsdf.bdsf"));
    let text = map.render_all(&found);
    assert_eq!(found.len(), 1, "{text}");
    assert!(text.contains("has no output `bdsf`"), "{text}");
}
