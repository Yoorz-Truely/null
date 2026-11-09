//! Renaming a datablock's handle in every file that names it, as VS Code renames a symbol (Sau's
//! decision S17, task 1b-01h): the edits that turn `@old` into `@new` where, and only where, it
//! names that datablock.
//!
//! A word names the datablock when it is `@old` or a path from it (`@old/@bevel`, `@old/@bsdf.base-
//! color`). A pattern's copy (`@old.7`) does not: it takes its placement's handle (section 8),
//! never a datablock's. Nor does a word where the same handle names something
//! else, which the place says:
//! - a sub-object's own line (`modifier @old bevel`), and a word or path that starts at one of the
//!   enclosing datablock's own sub-objects (section 7: inside its owner's block, a path starts
//!   at the owner's sub-objects first), by the checker's own lookup (S81): the sub-object lines
//!   right in its block and its tree's nodes, not a sub-object's own (a pose bone's constraint);
//! - inside a node tree: a node's or a zone's line, a link to a node's output (`@old.color`), a
//!   frame named by `parent`, and the `layout` block's lines;
//! - the slot an `action` line names with `slot=`, which is the action's;
//! - a component's placement: the second word of `use @component @placement`, and the path an
//!   `override` line starts at a placement (`@rig.2/@spine`); and inside a component's block,
//!   the handle of a datablock the component defines (`object @rig` in `component @rig`);
//! - a package's datablock (`@package:old`), and any string or comment.
//!
//! A definition written with only a display name (`object "Old"`, S18) gets the new handle
//! written before the name, so the name stays what it was.

use std::collections::HashSet;

use null_syntax::ast::{Entry, Node, Value};
use null_syntax::names::{is_kebab_case, to_kebab_case};
use null_syntax::{FileId, Parse, Span, TextEdit};

use crate::paths::SUB_KINDS;

/// Why a rename cannot be made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenameError {
    /// A handle is `@` and a kebab-case name.
    NotAHandle(String),
    /// No file defines the old handle.
    NoSuchHandle(String),
    /// Datablocks of more than one kind have the old handle, so its references cannot be told
    /// apart by their words alone; their kinds.
    Ambiguous(Vec<String>),
    /// A datablock of the same kind already has the new handle.
    Taken(String),
    /// A file does not parse, so its references cannot all be found; its id.
    Unparsed(FileId),
}

impl std::fmt::Display for RenameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenameError::NotAHandle(h) => {
                write!(f, "`{h}` is not a handle: `@`, then a kebab-case name")
            }
            RenameError::NoSuchHandle(h) => write!(f, "no datablock of the project is `{h}`"),
            RenameError::Ambiguous(kinds) => write!(
                f,
                "datablocks of {} kinds share the handle ({}), so the words naming each cannot be told apart; give one of them another handle by hand first",
                kinds.len(),
                kinds.join(", ")
            ),
            RenameError::Taken(kind) => write!(f, "a {kind} already has the new handle"),
            RenameError::Unparsed(_) => write!(
                f,
                "a file has syntax errors; fix them first, so that every reference can be found"
            ),
        }
    }
}

/// What a rename changes: the edits, the datablock's kind, and how many words name it.
#[derive(Clone, Debug)]
pub struct Rename {
    /// The edits, by file, in no particular order; none overlap.
    pub edits: Vec<TextEdit>,
    /// The renamed datablock's kind, such as `object`.
    pub kind: String,
    /// How many references (not counting its definition) were changed.
    pub references: usize,
}

/// The handle a top-level or defining node gives its datablock: written, or made from its
/// display name (S18).
fn handle_of(node: Node<'_>) -> Option<(String, Option<Span>, Option<Span>)> {
    let args: Vec<Value<'_>> = node
        .entries()
        .filter_map(|e| match e {
            Entry::Arg(v) => Some(v),
            _ => None,
        })
        .collect();
    match args.first() {
        Some(Value::Word(t)) if t.text.starts_with('@') => {
            Some((t.text.to_string(), Some(t.span), None))
        }
        Some(v @ (Value::Str(_) | Value::Raw(_))) => {
            let name = v.as_string()?;
            Some((format!("@{}", to_kebab_case(&name)?), None, Some(v.span())))
        }
        _ => None,
    }
}

/// Whether a `collection` line inside a scene or collection defines one (a name or a block).
fn defines(node: Node<'_>) -> bool {
    node.block().is_some()
        || node
            .args()
            .any(|v| matches!(v, Value::Str(_) | Value::Raw(_)))
}

/// Every (kind, handle) the files define, with each definition's node.
fn definitions<'a>(parses: &'a [(FileId, &'a Parse)]) -> Vec<(FileId, String, Node<'a>)> {
    let mut out = Vec::new();
    fn nested<'a>(file: FileId, node: Node<'a>, out: &mut Vec<(FileId, String, Node<'a>)>) {
        if node.name() == "scene" || node.name() == "collection" {
            for c in node.children() {
                if c.name() == "collection" && defines(c) {
                    out.push((file, "collection".into(), c));
                    nested(file, c, out);
                }
            }
        }
    }
    for (file, parse) in parses {
        // `project` is the project's settings, not a datablock.
        for node in parse.document().nodes().filter(|n| n.name() != "project") {
            out.push((*file, node.name().into_owned(), node));
            nested(*file, node, &mut out);
        }
    }
    out
}

/// The edits renaming the datablock handle `old` to `new` across `files`.
pub fn rename(files: &[(FileId, &Parse)], old: &str, new: &str) -> Result<Rename, RenameError> {
    for h in [old, new] {
        if !h.strip_prefix('@').is_some_and(is_kebab_case) {
            return Err(RenameError::NotAHandle(h.to_string()));
        }
    }
    if let Some((file, _)) = files.iter().find(|(_, p)| !p.errors().is_empty()) {
        return Err(RenameError::Unparsed(*file));
    }
    let defs = definitions(files);
    let mut kinds: Vec<String> = defs
        .iter()
        .filter(|(_, _, n)| handle_of(*n).is_some_and(|(h, _, _)| h == old))
        .map(|(_, k, _)| k.clone())
        .collect();
    kinds.sort();
    kinds.dedup();
    let kind = match kinds.as_slice() {
        [] => return Err(RenameError::NoSuchHandle(old.to_string())),
        [k] => k.clone(),
        _ => return Err(RenameError::Ambiguous(kinds)),
    };
    if defs
        .iter()
        .any(|(_, k, n)| *k == kind && handle_of(*n).is_some_and(|(h, _, _)| h == new))
    {
        return Err(RenameError::Taken(kind));
    }
    let mut edits = Vec::new();
    // The definitions: the handle written, or written now before the display name.
    let mut defined: HashSet<(FileId, usize)> = HashSet::new();
    for (file, k, node) in &defs {
        if *k != kind {
            continue;
        }
        match handle_of(*node) {
            Some((h, Some(span), _)) if h == old => {
                edits.push(TextEdit {
                    file: *file,
                    span,
                    replacement: new.to_string(),
                });
                defined.insert((*file, span.start));
            }
            Some((h, None, Some(name))) if h == old => {
                edits.push(TextEdit {
                    file: *file,
                    span: Span::new(name.start, name.start),
                    replacement: format!("{new} "),
                });
            }
            _ => {}
        }
    }
    let mut references = 0;
    for (file, parse) in files {
        for node in parse.document().nodes() {
            let subs = sub_objects(node);
            let mut walker = Walker {
                file: *file,
                old,
                new,
                subs: &subs,
                defined: &defined,
                edits: &mut edits,
                count: 0,
                shadowed: node.name() == "component" && component_defines(node, old),
            };
            walker.node(node, Context::Datablock, true, false);
            references += walker.count;
        }
    }
    Ok(Rename {
        edits,
        kind,
        references,
    })
}

/// Whether component `node` defines a datablock whose handle is `old`, at any depth of its block
/// (an `if`'s too), not counting what it places with `use`.
fn component_defines(node: Node<'_>, old: &str) -> bool {
    node.children().any(|c| {
        if c.name() == "use" {
            return false;
        }
        let handle = c
            .args()
            .next()
            .and_then(|v| v.as_word().map(str::to_string));
        (defines(c) && handle.as_deref() == Some(old)) || component_defines(c, old)
    })
}

/// The handles of a datablock's own sub-objects, by the checker's lookup (section 7, S81, task
/// 1b-01q): its sub-object lines right in its block and its tree's nodes and zones. A sub-object's
/// own sub-objects (a pose bone's constraints, a panel's sockets) are reached by a path from it,
/// so a bare handle inside the datablock never names them, and does not shadow the datablock.
fn sub_objects(node: Node<'_>) -> HashSet<String> {
    crate::paths::own_sub_objects(node)
        .into_iter()
        .map(|(_, handle)| handle)
        .collect()
}

#[derive(Clone, Copy, PartialEq)]
enum Context {
    /// A datablock's block, or something in it not below.
    Datablock,
    /// A node tree's `nodes` block, a node's or a zone's.
    Tree,
    /// A `layout` block.
    Layout,
}

struct Walker<'w> {
    file: FileId,
    old: &'w str,
    new: &'w str,
    subs: &'w HashSet<String>,
    defined: &'w HashSet<(FileId, usize)>,
    edits: &'w mut Vec<TextEdit>,
    count: usize,
    /// Inside a component that defines a datablock of the old handle: there the handle is that
    /// datablock's, except as the component `use` places.
    shadowed: bool,
}

impl Walker<'_> {
    /// `node` walked for references: in `cx`, `top` for a file's own node, `in_component` for a
    /// line right inside a component's block.
    fn node(&mut self, node: Node<'_>, cx: Context, top: bool, in_component: bool) {
        let name = node.name();
        if cx == Context::Layout {
            return; // `@node [x y]`: every handle there is a node's.
        }
        let mut args: Vec<Value<'_>> = Vec::new();
        let mut props: Vec<(String, Value<'_>)> = Vec::new();
        for e in node.entries() {
            match e {
                Entry::Arg(v) => args.push(v),
                Entry::Keyword { key, value } => {
                    args.push(Value::Word(key));
                    args.push(value);
                }
                Entry::Prop { key, value } => props.push((key.text.to_string(), value)),
            }
        }
        // A line that defines a sub-object, a node or a zone: its own handle is not a reference.
        // So does a datablock a component defines: its handle is the component's own.
        let own = !top
            && (SUB_KINDS.contains(&name.as_ref())
                || (cx == Context::Tree
                    && (name == "node" || name == "zone" || name.ends_with("-item")))
                || (in_component && defines(node) && !matches!(name.as_ref(), "use" | "override")));
        for (i, v) in args.iter().enumerate() {
            if (top || own) && i == 0 {
                continue; // The definition's own handle: renamed above, or not this datablock's.
            }
            // A placement: `use @component @placement`'s second word, and the path an
            // `override` starts at one; neither names a datablock.
            if (name == "use" && i == 1) || (name == "override" && i == 0) {
                continue;
            }
            if cx == Context::Tree && name == "parent" {
                continue; // A frame node.
            }
            if self.shadowed && !(name == "use" && i == 0) {
                continue; // The component's own datablock of that handle.
            }
            self.value(v, cx);
        }
        for (key, v) in &props {
            if key == "slot" && name == "action" {
                continue; // The action's slot, its sub-object.
            }
            if self.shadowed {
                continue;
            }
            self.value(v, cx);
        }
        for child in node.children() {
            let child_cx = match child.name().as_ref() {
                "nodes" => Context::Tree,
                "layout" => Context::Layout,
                _ => cx,
            };
            self.node(child, child_cx, false, top && name == "component");
        }
    }

    fn value(&mut self, v: &Value<'_>, cx: Context) {
        match v {
            Value::Word(t) => {
                if self.defined.contains(&(self.file, t.span.start)) {
                    return;
                }
                let word = t.text;
                let Some(rest) = word.strip_prefix(self.old) else {
                    return;
                };
                // `@old` and `@old/…` name it; `@older`, `@old.color` and `@old.7` do not: a
                // pattern's copy takes its placement's handle (section 8), never a datablock's.
                let names_it = rest.is_empty() || rest.starts_with('/');
                if !names_it {
                    return;
                }
                // Inside a node tree a bare `@x` may be a datablock (a setting's `image @oak`),
                // but never a node's output, which has `.`; inside its owner, a sub-object's
                // handle shadows the datablock's.
                if cx == Context::Tree && rest.starts_with('.') {
                    return;
                }
                if self.subs.contains(self.old) {
                    return;
                }
                self.edits.push(TextEdit {
                    file: self.file,
                    span: Span::new(t.span.start, t.span.start + self.old.len()),
                    replacement: self.new.to_string(),
                });
                self.count += 1;
            }
            Value::List(_) => {
                for item in v.items() {
                    self.value(&item, cx);
                }
            }
            _ => {}
        }
    }
}
