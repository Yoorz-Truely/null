//! The minimal-edit writer: new values for one attribute, as the fewest characters changed.

use std::fmt;

use null_syntax::ast::{Entry, Node, Value};
use null_syntax::names::to_kebab_case;
use null_syntax::{Parse, Span};
use null_value::{Dimension, Float, Scene, Unit, print_bare, print_in_scene, read_in, unit_of};

/// One text edit: replace `span` with `replacement`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    /// What to replace; empty for an insertion.
    pub span: Span,
    /// The new text.
    pub replacement: String,
}

/// An attribute's new values, in the type Blender stores them.
#[derive(Clone, Copy, Debug)]
pub enum Values<'v> {
    /// float32 values (every float RNA reads and writes).
    F32(&'v [f32]),
    /// float64 values.
    F64(&'v [f64]),
}

/// Why nothing could be written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteError {
    /// No datablock of that kind has that handle in this file.
    NoSuchNode,
    /// No values were given.
    NoValues,
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WriteError::NoSuchNode => write!(f, "no such datablock in this file"),
            WriteError::NoValues => write!(f, "no values to write"),
        }
    }
}

impl std::error::Error for WriteError {}

/// The edits that set attribute `attr` of the `kind` datablock `handle` in `parse` to
/// `values`, a value of `dimension` each.
///
/// - A value that already reads back to the same bits is left as written.
/// - A changed value is printed in the unit its word already uses, when a decimal in
///   that unit reads back exactly, and bare in the internal unit otherwise (S21).
/// - A list of another length, or a word where a list was, is replaced whole.
/// - An attribute the datablock does not have yet is added as a line in its block, in
///   `display` (the project's unit), or as `attr=value` on its line if it has no block.
pub fn write(
    parse: &Parse,
    kind: &str,
    handle: &str,
    attr: &str,
    values: Values<'_>,
    dimension: Dimension,
    display: Option<Unit>,
) -> Result<Vec<Edit>, WriteError> {
    write_in(
        parse,
        kind,
        handle,
        attr,
        values,
        dimension,
        display,
        &Scene::default(),
    )
}

/// [`write`] in `scene`: a value is read and printed through its frame rate, so a frame
/// written in seconds keeps its seconds (task 1c-01c), and through its unit scale.
#[allow(clippy::too_many_arguments)]
pub fn write_in(
    parse: &Parse,
    kind: &str,
    handle: &str,
    attr: &str,
    values: Values<'_>,
    dimension: Dimension,
    display: Option<Unit>,
    scene: &Scene,
) -> Result<Vec<Edit>, WriteError> {
    let words: Vec<Word> = match values {
        Values::F32(v) => v.iter().map(|&x| Word::new(x, *scene)).collect(),
        Values::F64(v) => v.iter().map(|&x| Word::new(x, *scene)).collect(),
    };
    if words.is_empty() {
        return Err(WriteError::NoValues);
    }
    let node = parse
        .document()
        .nodes()
        .find(|n| n.name() == kind && handle_of(*n).as_deref() == Some(handle))
        .ok_or(WriteError::NoSuchNode)?;
    let text = parse.text();
    match find_value(node, attr) {
        Some(value) => Ok(replace(value, &words, dimension)),
        None => Ok(vec![insert(text, node, attr, &words, dimension, display)]),
    }
}

/// How a value prints in a unit, if it fits exactly.
type InUnit = Box<dyn Fn(Unit) -> Option<String>>;
/// Whether a word of a dimension reads back to the value.
type Same = Box<dyn Fn(&str, Dimension) -> bool>;

/// A value to write, with its type erased: how it prints and whether a word reads to it.
struct Word {
    bare: String,
    in_unit: InUnit,
    same: Same,
}

impl Word {
    fn new<F: Float>(x: F, scene: Scene) -> Word {
        Word {
            bare: print_bare(x),
            in_unit: Box::new(move |u| print_in_scene(x, u, &scene)),
            same: Box::new(move |w, d| {
                read_in::<F>(w, d, &scene).is_ok_and(|y| y.bits() == x.bits())
            }),
        }
    }

    /// The word for this value, in `unit` if one is given and fits exactly.
    fn print(&self, unit: Option<Unit>) -> String {
        unit.and_then(|u| (self.in_unit)(u))
            .unwrap_or_else(|| self.bare.clone())
    }
}

/// The handle a datablock node has: written, or made from its display name (S18).
fn handle_of(node: Node<'_>) -> Option<String> {
    let mut args = node.args();
    let first = args.next()?;
    if let Some(w) = first.as_word().filter(|w| w.starts_with('@')) {
        return Some(w.to_string());
    }
    let name = first.as_string()?;
    Some(format!("@{}", to_kebab_case(&name)?))
}

/// The value of attribute `attr`: a child line's first argument, or `attr=value`.
fn find_value<'a>(node: Node<'a>, attr: &str) -> Option<Value<'a>> {
    if let Some(child) = node.children().find(|c| c.name() == attr) {
        return child.args().next();
    }
    node.entries().find_map(|e| match e {
        Entry::Prop { key, value } if key.text == attr => Some(value),
        _ => None,
    })
}

fn replace(value: Value<'_>, words: &[Word], dimension: Dimension) -> Vec<Edit> {
    match (value, words) {
        (Value::Word(t), [w]) => edit_word(t.text, t.span, w, dimension)
            .into_iter()
            .collect(),
        (Value::List(_), _) if value.items().len() == words.len() => value
            .items()
            .iter()
            .zip(words)
            .filter_map(|(item, w)| match item {
                Value::Word(t) => edit_word(t.text, t.span, w, dimension),
                other => Some(Edit {
                    span: other.span(),
                    replacement: w.print(None),
                }),
            })
            .collect(),
        _ => {
            // Another shape: the whole value, in the unit of its first word, if any.
            let first = match value {
                Value::Word(t) => Some(t.text),
                _ => value.items().first().and_then(|v| v.as_word()),
            };
            let unit = first.and_then(|w| unit_of(w, dimension));
            vec![Edit {
                span: value.span(),
                replacement: printed(words, unit),
            }]
        }
    }
}

/// The edit for one word, or none if it already reads back to the value.
fn edit_word(old: &str, span: Span, w: &Word, dimension: Dimension) -> Option<Edit> {
    if (w.same)(old, dimension) {
        return None;
    }
    Some(Edit {
        span,
        replacement: w.print(unit_of(old, dimension)),
    })
}

fn printed(words: &[Word], unit: Option<Unit>) -> String {
    match words {
        [w] => w.print(unit),
        _ => {
            let all: Vec<_> = words.iter().map(|w| w.print(unit)).collect();
            format!("[{}]", all.join(" "))
        }
    }
}

/// The file's line ending: CRLF if most of its line feeds follow a carriage return.
fn line_ending(text: &str) -> &'static str {
    let feeds = text.matches('\n').count();
    let crlf = text.matches("\r\n").count();
    if feeds > 0 && crlf * 2 > feeds {
        "\r\n"
    } else {
        "\n"
    }
}

/// The leading whitespace of the line `at` is on.
fn indent_at(text: &str, at: usize) -> &str {
    let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let line = &text[start..];
    &line[..line.len() - line.trim_start_matches([' ', '\t']).len()]
}

fn insert(
    text: &str,
    node: Node<'_>,
    attr: &str,
    words: &[Word],
    dimension: Dimension,
    display: Option<Unit>,
) -> Edit {
    let unit = display.filter(|u| u.dimension() == dimension);
    let value = printed(words, unit);
    let Some(block) = node.block() else {
        // No block: `attr=value` at the end of the node's line, before any comment.
        return Edit {
            span: Span::empty(node.code_span().end),
            replacement: format!(" {attr}={value}"),
        };
    };
    let line = format!("{attr} {value}");
    let nl = line_ending(text);
    let children: Vec<_> = node.children().collect();
    let inner = format!("{}  ", indent_at(text, node.span().start));
    match children.last() {
        Some(last) => {
            let end = last.span().end;
            if text[..end].ends_with('\n') {
                // The last child ends its own line: a new line after it, indented as it is.
                Edit {
                    span: Span::empty(end),
                    replacement: format!("{}{line}{nl}", indent_at(text, last.span().start)),
                }
            } else {
                // The last child shares its line (`{ a; b }`): one more after a `;`.
                Edit {
                    span: Span::empty(last.code_span().end),
                    replacement: format!("; {line}"),
                }
            }
        }
        None => {
            let open = block.span().start + 1;
            if let Some(feed) = text[block.span().range()].find('\n') {
                // At the start of the line after `{`, so a comment on the brace's line stays
                // the brace's (the 1a-07 review).
                Edit {
                    span: Span::empty(block.span().start + feed + 1),
                    replacement: format!("{inner}{line}{nl}"),
                }
            } else {
                Edit {
                    span: Span::new(open, block.span().end - 1),
                    replacement: format!(" {line} "),
                }
            }
        }
    }
}
