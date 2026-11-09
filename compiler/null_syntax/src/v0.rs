//! The constructs of Null v0 in a parsed file (task 1d-02a): what each node of the shared syntax
//! is in Null, by its name and where it stands, as the spec's sections say. The syntax is Scowr's
//! (S15) and carries no meaning; this outline is the first reading of it, before the schema:
//! datablocks and their attributes, sub-objects, node trees with their nodes, zones, links and
//! `layout` (section 10), actions with their keys and key files, drivers (section 11), level 0 data files and
//! their references (section 9), components, placements, patterns, conditions and overrides
//! (section 8), and `raw` blocks (section 19).
//!
//! [`outline`] gives the constructs as a tree; [`render`] prints it, one construct per line, for
//! snapshot tests. Nothing here decides whether a construct is right: that is the checker's.

use std::fmt::Write;

use crate::Parse;
use crate::ast::{Entry, Node, Value};

/// One construct of a file: what it is, its words as written, its line, and the constructs in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// What the construct is (`datablock`, `node`, `link`, `key`, `placement`, …).
    pub what: &'static str,
    /// The node's name and arguments as written, its properties after them.
    pub words: String,
    /// The line it starts on, from 1.
    pub line: usize,
    /// The constructs inside it, in order.
    pub children: Vec<Item>,
}

/// Where a node stands, which decides what its name means.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    /// A file's top level.
    Top,
    /// Inside a datablock or anything in it not below.
    Datablock,
    /// A component's block.
    Component,
    /// A node tree's `nodes` block.
    Tree,
    /// A zone's block.
    Zone,
    /// A `layout` block.
    Layout,
    /// An f-curve's block or a driver's.
    Curve,
    /// A scene's or a collection's block, whose `collection` lines may define collections.
    Members,
}

/// Whether a `collection` line defines a collection: it has a display name or a block.
fn defines(node: Node<'_>) -> bool {
    node.block().is_some()
        || node
            .args()
            .any(|v| matches!(v, Value::Str(_) | Value::Raw(_)))
}

/// The sub-object lines (Null spec, section 7).
const SUB_OBJECTS: &[&str] = &[
    "modifier",
    "constraint",
    "bone",
    "bone-collection",
    "shape-key",
    "vertex-group",
    "uv-map",
    "attribute",
    "view-layer",
    "slot",
    "layer",
    "layer-group",
    "drawing",
    "strip",
    "track",
    "marker",
    "panel",
];

/// The constructs of `parse`, its top-level nodes first.
pub fn outline(parse: &Parse) -> Vec<Item> {
    let text = parse.print();
    let line = |at: usize| text[..at.min(text.len())].matches('\n').count() + 1;
    parse
        .document()
        .nodes()
        .filter(|n| !n.is_commented_out())
        .map(|n| item(n, Place::Top, &line))
        .collect()
}

/// `items` as text, one construct a line, indented as nested: `12 node: node @bsdf
/// bsdf-principled`.
pub fn render(items: &[Item]) -> String {
    fn go(items: &[Item], depth: usize, out: &mut String) {
        for i in items {
            let _ = writeln!(
                out,
                "{:>3} {}{}: {}",
                i.line,
                "  ".repeat(depth),
                i.what,
                i.words
            );
            go(&i.children, depth + 1, out);
        }
    }
    let mut out = String::new();
    go(items, 0, &mut out);
    out
}

fn words(node: Node<'_>) -> String {
    let mut out = vec![node.name().into_owned()];
    for e in node.entries() {
        match e {
            Entry::Arg(v) => out.push(v.text().to_string()),
            Entry::Keyword { key, value } => out.push(format!("{} {}", key.text, value.text())),
            Entry::Prop { key, value } => out.push(format!("{}={}", key.text, value.text())),
        }
    }
    out.join(" ")
}

fn prop<'a>(node: Node<'a>, name: &str) -> Option<Value<'a>> {
    node.entries().find_map(|e| match e {
        Entry::Prop { key, value } if key.text == name => Some(value),
        _ => None,
    })
}

/// Whether a value is a link to a node's output (`@tex.color`) or a list of them.
fn is_link(v: &Value<'_>) -> bool {
    match v {
        // The path's last step names the socket (`@tex.color`), also inside a copy
        // (`@plank.7/@tex.color`); a copy's own number is no socket (`@plank.7`, task 1d-03b).
        Value::Word(t) => {
            t.text.starts_with('@')
                && t.text
                    .rsplit('/')
                    .next()
                    .and_then(|step| step.rsplit_once('.'))
                    .is_some_and(|(_, socket)| {
                        socket.starts_with(|c: char| c.is_ascii_alphabetic())
                    })
        }
        Value::List(_) => {
            let items = v.items();
            !items.is_empty() && items.iter().all(is_link)
        }
        _ => false,
    }
}

/// What a node is where it stands, and the place its children stand in.
fn classify(node: Node<'_>, place: Place) -> (&'static str, Place) {
    let name = node.name();
    let name = name.as_ref();
    // Constructs that mean the same wherever they stand.
    if prop(node, "file").is_some() {
        return ("data file reference", place);
    }
    if prop(node, "driver").is_some() {
        return ("driver", Place::Curve);
    }
    match (place, name) {
        (Place::Top, "project") => ("project", Place::Datablock),
        (Place::Top, "component") => ("component", Place::Component),
        (Place::Top, "array") => ("data file", Place::Datablock),
        // A key file (section 11, S86): its keys read as an f-curve's.
        (Place::Top, "keys") => ("key file", Place::Curve),
        (Place::Top, "scene" | "collection") => ("datablock", Place::Members),
        (Place::Top, _) => ("datablock", Place::Datablock),
        (Place::Layout, _) => ("position", Place::Layout),
        (Place::Tree | Place::Zone, "node") => ("node", Place::Datablock),
        (Place::Tree | Place::Zone, "zone") => ("zone", Place::Zone),
        (Place::Zone, "output") => ("zone output", Place::Datablock),
        (Place::Zone, n) if n.ends_with("-item") => ("zone item", Place::Datablock),
        (_, "nodes") => ("node tree", Place::Tree),
        (_, "layout") => ("layout", Place::Layout),
        (_, "raw") => ("raw block", Place::Datablock),
        (_, "param") => ("parameter", Place::Datablock),
        (_, "use") => {
            if prop(node, "count").is_some() || prop(node, "each").is_some() {
                ("pattern", place)
            } else {
                ("placement", place)
            }
        }
        (_, "override") => ("override", place),
        (_, "if") => ("condition", place),
        (_, "else") => ("otherwise", place),
        (_, "fcurve") => ("f-curve", Place::Curve),
        (_, "channels") => ("channels", place),
        (Place::Curve, "key") => ("key", Place::Curve),
        (Place::Curve, "var") => ("driver variable", Place::Curve),
        (Place::Curve, "target") => ("driver target", Place::Curve),
        (Place::Component, "scene" | "collection") => ("datablock", Place::Members),
        (Place::Component, _) => ("datablock", Place::Datablock),
        // Inside a scene or a collection, a `collection` line with a display name or a block
        // defines one (section 8's members); without, it names one.
        (Place::Members, "collection") if defines(node) => ("datablock", Place::Members),
        (_, "input" | "output") => ("interface socket", Place::Datablock),
        (_, "action") if prop(node, "slot").is_some() => ("plays", place),
        (_, n) if SUB_OBJECTS.contains(&n) => ("sub-object", Place::Datablock),
        _ if node.args().next().as_ref().is_some_and(is_link) => ("link", place),
        _ => ("attribute", place),
    }
}

fn item(node: Node<'_>, place: Place, line: &dyn Fn(usize) -> usize) -> Item {
    let (what, inner) = classify(node, place);
    Item {
        what,
        words: words(node),
        line: line(node.span().start),
        children: node.children().map(|c| item(c, inner, line)).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    fn kinds(text: &str) -> Vec<&'static str> {
        fn all(items: &[Item], out: &mut Vec<&'static str>) {
            for i in items {
                out.push(i.what);
                all(&i.children, out);
            }
        }
        let mut out = Vec::new();
        all(&outline(&parse(text)), &mut out);
        out
    }

    #[test]
    fn a_raw_block_and_a_data_file_reference_are_read_in_any_datablock() {
        // Section 19's `raw` block has no example in the spec yet (1d-05b), so it is read here.
        let text = "mesh @rock \"Rock\" {\n  vertices file=\"rock.vertices.noll\" count=3\n  raw {\n    flag 2\n  }\n}\n";
        assert_eq!(
            kinds(text),
            ["datablock", "data file reference", "raw block", "attribute"]
        );
    }

    #[test]
    fn a_link_is_a_socket_value_naming_an_output_and_a_copy_is_not() {
        let text = "material @m {\n  nodes {\n    node @a mix { factor @b.fac; shader [@c.bsdf @d.bsdf]; color @plank.7/@tex.color }\n  }\n}\nobject @o { parent @plank.7 }\n";
        assert_eq!(
            kinds(text),
            [
                "datablock",
                "node tree",
                "node",
                "link",
                "link",
                "link",
                "datablock",
                "attribute"
            ]
        );
    }

    #[test]
    fn a_collection_defined_in_a_scene_is_a_datablock_and_a_member_is_not() {
        // Section 8: a `collection` line with a display name or a block defines a collection,
        // in a scene or a collection; one with neither names one (the 1d-02a review).
        let text = "scene @main \"Main\" {\n  collection @harbor \"Harbor\" {\n    object @pier\n    collection @boats \"Boats\"\n    collection @nets\n  }\n}\n";
        assert_eq!(
            kinds(text),
            [
                "datablock",
                "datablock",
                "attribute",
                "datablock",
                "attribute"
            ]
        );
    }

    #[test]
    fn the_rendering_is_one_construct_a_line() {
        let text = "scene @s {\n  use @plank @plank count=20 { location [0m n*25cm 0m] }\n}\n";
        assert_eq!(
            render(&outline(&parse(text))),
            "  1 datablock: scene @s\n  2   pattern: use @plank @plank count=20\n  2     attribute: location [0m n*25cm 0m]\n"
        );
    }
}
