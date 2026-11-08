//! Defaults left out (spec, section 15; task 1d-02b): an attribute whose value is its property's
//! default, the value in a new datablock (section 8: the schema's `new` where it has one, else
//! RNA's `default`), is written by nobody and read as nothing, so the formatter takes its line
//! out.
//!
//! Only what is surely the default goes:
//! - a line of its own (not one of a `{ a; b }` line), with one value, no `key=value`, no block,
//!   and no comment on it or just above it, since a commented value was written on purpose;
//! - of a property of the struct the block is (a datablock's, a struct block's such as
//!   `cycles { … }`, a modifier's or a constraint's), not a pointer or a collection;
//! - whose value reads, by section 6's rule, to exactly the default's bits;
//! - and whose unit does not depend on the scene (a length, an area, a mass, a time in seconds
//!   and the rest scale with the scene's units or frame rate, which this pass does not read).
//!
//! Settings a project pins (section 5) are kept once pinning exists; nothing pins yet.

use null_check::{Property, Schema};
use null_syntax::ast::{Entry, Node};
use null_syntax::{NodeKind, SyntaxElement, TokenKind, parse};
use null_value::Scene;
use serde_json::Value as Json;

use crate::typed::{Typed, read_value};

/// `text` with every attribute line that holds its property's default taken out. Text that does
/// not parse cleanly is returned as it is.
pub fn without_defaults(text: &str, schema: &Schema) -> String {
    let file = parse(text);
    if !file.errors().is_empty() {
        return text.to_string();
    }
    let mut cut: Vec<(usize, usize)> = Vec::new();
    for node in file.document().nodes() {
        if let Some(rna) = datablock_rna(schema, node) {
            block(schema, node, &rna, text, &mut cut);
        }
    }
    cut.sort_unstable();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for (start, end) in cut {
        if start < at {
            continue;
        }
        out.push_str(&text[at..start]);
        at = end;
    }
    out.push_str(&text[at..]);
    out
}

/// The struct a top-level node is: its kind's, a light's by its `type`.
fn datablock_rna(schema: &Schema, node: Node<'_>) -> Option<String> {
    let kind = node.name();
    let s = schema.struct_by_kind(&kind)?;
    if !schema.is_datablock_struct(&s.rna) {
        return None;
    }
    if kind == "light" {
        let ty = node
            .children()
            .find(|c| c.name() == "type")
            .and_then(|c| c.args().next())
            .and_then(|v| v.as_word().map(str::to_string))
            .unwrap_or_else(|| "point".into());
        if let Some(sub) = schema.struct_by_kind(&format!("{ty}-light")) {
            return Some(sub.rna.clone());
        }
    }
    Some(s.rna.clone())
}

/// Collects the default lines of `node`'s block, read as struct `rna`, and of the struct blocks
/// and family members inside it.
fn block(schema: &Schema, node: Node<'_>, rna: &str, text: &str, cut: &mut Vec<(usize, usize)>) {
    let props = schema.properties_of(rna);
    for child in node.children() {
        let name = child.name();
        let Some(p) = props.iter().find(|p| p.name == name) else {
            // A modifier or a constraint: its block by its kind's struct.
            let base = match (rna, name.as_ref()) {
                (_, "modifier") if schema.is_datablock_struct(rna) => Some("Modifier"),
                (_, "constraint") => Some("Constraint"),
                _ => None,
            };
            if let Some(base) = base
                && let Some(kind) = child
                    .args()
                    .nth(1)
                    .and_then(|v| v.as_word().map(str::to_string))
                && let Some(s) = schema
                    .structs
                    .iter()
                    .find(|s| s.base.as_deref() == Some(base) && s.kind == kind)
            {
                block(schema, child, &s.rna, text, cut);
            }
            continue;
        };
        if p.ty == "pointer" {
            // A struct block (`cycles { … }`, `render { … }`): its lines by that struct.
            if child.block().is_some()
                && let Some(target) = p.target.as_deref()
                && !schema.is_datablock_struct(target)
            {
                block(schema, child, target, text, cut);
            }
            continue;
        }
        if let Some(span) = default_line(child, p, text) {
            cut.push(span);
        }
    }
}

/// The span to cut, from the start of its line through its line feed, when `node` is a line of
/// its own holding `p`'s default and nothing else.
fn default_line(node: Node<'_>, p: &Property, text: &str) -> Option<(usize, usize)> {
    if p.readonly || p.ty == "collection" || scene_dependent(p) {
        return None;
    }
    let entries: Vec<Entry<'_>> = node.entries().collect();
    let [Entry::Arg(value)] = entries.as_slice() else {
        return None;
    };
    if node.block().is_some() {
        return None;
    }
    // No comment on the line, nor a `;` that puts another line beside it.
    let has_comment_or_semicolon = node.0.children().any(|e| {
        matches!(e, SyntaxElement::Token(t)
            if matches!(t.kind, TokenKind::LineComment | TokenKind::BlockComment | TokenKind::Semicolon))
    }) || node
        .0
        .children()
        .any(|e| matches!(e, SyntaxElement::Node(n) if n.kind() == NodeKind::Block));
    if has_comment_or_semicolon {
        return None;
    }
    let span = node.span();
    // A line of its own: only indentation before it on its line, and it ends at a line feed.
    let line_start = text[..span.start].rfind('\n').map_or(0, |i| i + 1);
    if !text[line_start..span.start]
        .chars()
        .all(|c| c == ' ' || c == '\t')
    {
        return None;
    }
    if !text[..span.end].ends_with('\n') {
        return None;
    }
    // No comment on the line just above, which would be about this value.
    let above = text[..line_start].trim_end_matches('\n');
    let prev_line = &above[above.rfind('\n').map_or(0, |i| i + 1)..];
    if prev_line.trim_start().starts_with("//") || prev_line.trim_end().ends_with("*/") {
        return None;
    }
    let default = default_of(p)?;
    let written = read_value(value.text(), p, &Scene::default()).ok()?;
    same(&written, &default).then_some((line_start, span.end))
}

/// Whether a value of `p` reads through the scene's unit scale or frame rate.
fn scene_dependent(p: &Property) -> bool {
    matches!(
        p.unit.as_deref(),
        Some(
            "length" | "area" | "volume" | "mass" | "velocity" | "acceleration" | "power" | "time"
        )
    )
}

/// `p`'s default as a typed value: the schema's `new`, else its `default`.
fn default_of(p: &Property) -> Option<Typed> {
    let json = p.new.as_ref().or(p.default.as_ref())?;
    typed(json, p)
}

fn typed(json: &Json, p: &Property) -> Option<Typed> {
    Some(match (p.ty.as_str(), json) {
        ("bool", Json::Bool(b)) => Typed::Bool(*b),
        ("int", Json::Number(n)) => Typed::Int(n.as_i64()?),
        ("float32", Json::Number(n)) => Typed::F32(n.as_f64()? as f32),
        ("float64", Json::Number(n)) => Typed::F64(n.as_f64()?),
        ("enum", Json::String(s)) if !p.flags => Typed::Enum(s.clone()),
        ("enum", Json::Array(items)) if p.flags => Typed::Flags(
            items
                .iter()
                .map(|i| i.as_str().map(str::to_string))
                .collect::<Option<_>>()?,
        ),
        ("string", Json::String(s)) => Typed::Text(s.as_bytes().to_vec()),
        (_, Json::Array(items)) => {
            let single = Property {
                array: None,
                ..p.clone()
            };
            Typed::List(
                items
                    .iter()
                    .map(|i| typed(i, &single))
                    .collect::<Option<_>>()?,
            )
        }
        _ => return None,
    })
}

/// Equal bit for bit: `-0` is not `0`.
fn same(a: &Typed, b: &Typed) -> bool {
    match (a, b) {
        (Typed::F32(x), Typed::F32(y)) => x.to_bits() == y.to_bits(),
        (Typed::F64(x), Typed::F64(y)) => x.to_bits() == y.to_bits(),
        (Typed::List(x), Typed::List(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same(a, b))
        }
        _ => a == b,
    }
}
