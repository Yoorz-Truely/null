//! Sub-object paths (spec, section 7, "Sub-object handles"): `@pier/@bevel`,
//! `@rig/@spine-002/@ik`, `@weathered-oak/@bsdf.roughness`, resolved against a project's
//! files (task 1b-02c).
//!
//! A sub-object is a line `<kind> @handle …` inside its owner's block, the kind one of
//! [`SUB_KINDS`]; a node tree's nodes are `node @handle <kind>` lines in a `nodes` block. Its
//! handle is unique within its owner across kinds. A path steps from a datablock's handle to a
//! sub-object with `/`, again where a sub-object owns others, and may end with `.` and a socket
//! of a node. A `.` followed by a number names a pattern's copy (`@plank.7`, Sau's S61 (6)), and
//! `@package:handle` a package's datablock; both parse, and resolve once patterns (1d-03) and
//! packages (1d-04) exist.

use std::collections::HashMap;

use null_syntax::ast::{Entry, Node, Value};
use null_syntax::names::to_kebab_case;
use null_syntax::{Diagnostic, FileId, Parse, Span, closest};

use crate::codes;
use crate::schema::{SHADER_TREE, Schema};

/// The kinds of sub-object line the checker indexes, besides `node`: section 7's table in the
/// words Null writes them.
pub const SUB_KINDS: &[&str] = &[
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
    "strip",
    "track",
    "marker",
    "layer-group",
    "drawing",
    "input",
    "output",
    "panel",
];

/// One step of a path: a handle, and the pattern copy it names, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    /// The handle, with its `@`.
    pub handle: String,
    /// `7` in `@plank.7`.
    pub copy: Option<u32>,
}

/// A path as written: `@package:owner.N/@sub.N/…/@node.socket`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path {
    /// `cosmosis-materials` in `@cosmosis-materials:oak`.
    pub package: Option<String>,
    /// The owner, then each sub-object.
    pub segments: Vec<Segment>,
    /// The node socket the path ends with.
    pub socket: Option<String>,
}

impl Path {
    /// Reads a path, or says what is wrong with it.
    pub fn parse(word: &str) -> Result<Path, String> {
        let parts: Vec<&str> = word.split('/').collect();
        let mut package = None;
        let mut segments = Vec::new();
        let mut socket = None;
        for (i, part) in parts.iter().enumerate() {
            let last = i + 1 == parts.len();
            if part.is_empty() {
                return Err(format!(
                    "`{word}` has an empty step: each `/` is followed by a handle"
                ));
            }
            let Some(mut rest) = part.strip_prefix('@') else {
                return Err(format!(
                    "each step of a path is a handle starting with `@`, not `{part}`"
                ));
            };
            if i == 0
                && let Some((pkg, after)) = rest.split_once(':')
            {
                package = Some(pkg.to_string());
                rest = after;
            }
            let mut pieces = rest.split('.');
            let handle = pieces.next().unwrap_or_default();
            if handle.is_empty() {
                return Err(format!("`{part}` has no handle after `@`"));
            }
            let mut copy = None;
            for piece in pieces {
                if socket.is_some() {
                    return Err(format!("`{word}` has more than one socket"));
                }
                if !piece.is_empty() && piece.bytes().all(|b| b.is_ascii_digit()) && copy.is_none()
                {
                    copy = Some(
                        piece
                            .parse()
                            .map_err(|_| format!("`{piece}` is too large a copy number"))?,
                    );
                } else if last && !piece.is_empty() {
                    socket = Some(piece.to_string());
                } else {
                    return Err(format!(
                        "a socket ends a path: `.{piece}` cannot be followed by `/`"
                    ));
                }
            }
            segments.push(Segment {
                handle: format!("@{handle}"),
                copy,
            });
        }
        Ok(Path {
            package,
            segments,
            socket,
        })
    }
}

/// A sub-object of a datablock, or of another sub-object.
#[derive(Clone, Debug)]
pub struct SubObject {
    /// Its line's kind: `modifier`, `node`, `bone`, ….
    pub kind: String,
    /// Its handle, with its `@`.
    pub handle: String,
    /// A node's kind (`bsdf-principled`), or a modifier's or constraint's type, as written.
    pub of: Option<String>,
    /// Where it is defined.
    pub at: (FileId, Span),
    /// The sub-objects it owns (a bone's constraints).
    pub subs: Vec<SubObject>,
}

/// A datablock with its sub-objects.
#[derive(Clone, Debug)]
pub struct Owner {
    /// Its kind: `object`, `material`, ….
    pub kind: String,
    /// Its handle, with its `@`.
    pub handle: String,
    /// Its sub-objects, in the file's order.
    pub subs: Vec<SubObject>,
}

/// What a path names: the owner's kind, then each sub-object's kind, and the socket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// The datablock's kind.
    pub owner_kind: String,
    /// Each sub-object's kind, in the path's order.
    pub kinds: Vec<String>,
    /// The socket, if the path ends with one.
    pub socket: Option<String>,
}

/// Why a path names nothing, and the whole path written right, when one is near.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unresolved {
    /// What is wrong.
    pub message: String,
    /// The path as it should probably be written.
    pub fix: Option<String>,
}

impl Unresolved {
    fn new(message: impl Into<String>, fix: Option<String>) -> Self {
        Unresolved {
            message: message.into(),
            fix,
        }
    }

    /// As a `null-handle` error on the path's word, with its fix.
    pub fn diagnostic(&self, file: FileId, span: Span) -> Diagnostic {
        let mut d = Diagnostic::error(codes::HANDLE, file, span, self.message.clone());
        if let Some(fix) = &self.fix {
            d = d.with_fix_replace(format!("write `{fix}`"), span, fix.clone());
        }
        d
    }
}

fn args_of(node: Node<'_>) -> Vec<Value<'_>> {
    node.entries()
        .flat_map(|e| match e {
            Entry::Arg(v) => vec![v],
            Entry::Keyword { key, value } => vec![Value::Word(key), value],
            Entry::Prop { .. } => vec![],
        })
        .collect()
}

/// A datablock's own sub-objects as (kind, handle): the sub-object lines right in `node`'s block
/// and its tree's nodes and zones. Inside the datablock's block a handle names one of these
/// first (section 7, Sau's decision S81), as the checker, rename and Sau's apply read it (task
/// 1b-01q).
pub fn own_sub_objects(node: Node<'_>) -> Vec<(String, String)> {
    subs_of(FileId(0), node)
        .into_iter()
        .map(|s| (s.kind, s.handle))
        .collect()
}

/// The sub-object lines inside `node`'s block, with theirs.
pub(crate) fn subs_of(file: FileId, node: Node<'_>) -> Vec<SubObject> {
    let mut out = Vec::new();
    for child in node.children() {
        let name = child.name();
        if name == "nodes" {
            tree_nodes(file, child, &mut out);
        } else if SUB_KINDS.contains(&name.as_ref()) {
            let args = args_of(child);
            if let Some(h) = args.first().filter(|h| h.text().starts_with('@')) {
                out.push(SubObject {
                    kind: name.into_owned(),
                    handle: h.text().to_string(),
                    of: args
                        .get(1)
                        .filter(|k| !k.text().starts_with('"'))
                        .map(|k| k.text().to_string()),
                    at: (file, h.span()),
                    subs: subs_of(file, child),
                });
            }
        }
    }
    out
}

/// A tree's `node` and `zone` lines, the nodes inside a zone's block included: one tree, one
/// set of handles (section 10).
fn tree_nodes(file: FileId, block: Node<'_>, out: &mut Vec<SubObject>) {
    for n in block.children() {
        let kind = n.name();
        if kind != "node" && kind != "zone" {
            continue;
        }
        let args = args_of(n);
        if let Some(h) = args.first().filter(|h| h.text().starts_with('@')) {
            out.push(SubObject {
                kind: kind.clone().into_owned(),
                handle: h.text().to_string(),
                of: args.get(1).map(|k| k.text().to_string()),
                at: (file, h.span()),
                subs: Vec::new(),
            });
        }
        if kind == "zone" {
            tree_nodes(file, n, out);
        }
    }
}

/// Every datablock of a project with its sub-objects, by handle.
#[derive(Clone, Debug, Default)]
pub struct Index {
    owners: HashMap<String, Vec<Owner>>,
}

impl Index {
    /// Indexes the files' top-level datablocks: a handle written, or made from the display
    /// name (S18).
    pub fn build(files: &[(FileId, &Parse)]) -> Index {
        let mut owners: HashMap<String, Vec<Owner>> = HashMap::new();
        for (file, parse) in files {
            for node in parse.document().nodes() {
                let args = args_of(node);
                let handle = match args.first() {
                    Some(v) if v.text().starts_with('@') => v.text().to_string(),
                    Some(v) => match v.as_string().and_then(|s| to_kebab_case(&s)) {
                        Some(k) => format!("@{k}"),
                        None => continue,
                    },
                    None => continue,
                };
                owners.entry(handle.clone()).or_default().push(Owner {
                    kind: node.name().into_owned(),
                    handle,
                    subs: subs_of(*file, node),
                });
            }
        }
        Index { owners }
    }

    /// The datablocks called `handle`, one per kind.
    pub fn owners(&self, handle: &str) -> &[Owner] {
        self.owners
            .get(handle)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// A `null-handle` error for every handle two sub-objects of one owner share, whatever their
    /// kinds. Two nodes or zones of one tree are left to the node tree's own check.
    pub fn duplicates(&self) -> Vec<Diagnostic> {
        fn walk(owner: &str, subs: &[SubObject], out: &mut Vec<Diagnostic>) {
            let mut seen: HashMap<&str, &SubObject> = HashMap::new();
            for s in subs {
                if let Some(first) = seen.get(s.handle.as_str()) {
                    if !(matches!(first.kind.as_str(), "node" | "zone")
                        && matches!(s.kind.as_str(), "node" | "zone"))
                    {
                        out.push(Diagnostic::error(
                            codes::HANDLE,
                            s.at.0,
                            s.at.1,
                            format!(
                                "`{}` names two sub-objects of `{owner}` (a {} and a {}): a sub-object's handle is unique within its owner, whatever its kind",
                                s.handle, first.kind, s.kind
                            ),
                        ));
                    }
                } else {
                    seen.insert(&s.handle, s);
                }
                walk(&s.handle, &s.subs, out);
            }
        }
        let mut out = Vec::new();
        for owners in self.owners.values() {
            for o in owners {
                walk(&o.handle, &o.subs, &mut out);
            }
        }
        out
    }

    /// What `word` names. `within` is the datablock (kind and handle) whose block the word is
    /// written in, where a path may leave its owner out (`@bsdf.roughness` inside
    /// `material @weathered-oak`) and starts at the owner's own sub-objects first.
    pub fn resolve(
        &self,
        schema: &Schema,
        word: &str,
        within: Option<(&str, &str)>,
    ) -> Result<Resolved, Unresolved> {
        let path = Path::parse(word).map_err(|m| Unresolved::new(m, None))?;
        if let Some(pkg) = &path.package {
            return Err(Unresolved::new(
                format!(
                    "`{word}` names a datablock of the package `{pkg}`; packages are not read yet (task 1d-04)"
                ),
                None,
            ));
        }
        if let Some(seg) = path.segments.iter().find(|s| s.copy.is_some()) {
            return Err(Unresolved::new(
                format!(
                    "`{}.{}` names copy {} of a pattern; patterns are not read yet (task 1d-03)",
                    seg.handle,
                    seg.copy.unwrap_or_default(),
                    seg.copy.unwrap_or_default()
                ),
                None,
            ));
        }
        let first = &path.segments[0].handle;
        // Inside an owner's block, a path may start at one of its own sub-objects, which come
        // first: a datablock elsewhere with the same handle never shadows them, and the owner
        // is the one datablock of that kind and handle, not every datablock so called.
        if let Some((kind, handle)) = within
            && let Some(owner) = self.owners(handle).iter().find(|o| o.kind == kind)
        {
            let mut relative = path.clone();
            relative.segments.insert(
                0,
                Segment {
                    handle: handle.to_string(),
                    copy: None,
                },
            );
            let full = format!("{handle}/{word}");
            match walk_path(schema, owner, &relative, &full) {
                Ok(r) => return Ok(r),
                // Not one of the owner's: a path that starts at a datablock is read as written,
                // but only when the owner has no sub-object of that handle.
                Err(_)
                    if self.owners.contains_key(first)
                        && !owner.subs.iter().any(|s| s.handle == *first) => {}
                Err(e) => {
                    return Err(Unresolved {
                        message: e.message,
                        fix: e.fix.and_then(|f| {
                            f.strip_prefix(&format!("{handle}/")).map(str::to_string)
                        }),
                    });
                }
            }
        }
        let owners = self.owners(first);
        if owners.is_empty() {
            let near = closest(first, self.owners.keys().map(String::as_str)).map(str::to_string);
            let fix = near.as_ref().map(|n| replace_segment(word, 0, n));
            return Err(Unresolved::new(
                format!("no datablock is called `{first}`"),
                fix,
            ));
        }
        let mut found: Vec<Resolved> = Vec::new();
        let mut errors: Vec<Unresolved> = Vec::new();
        for owner in owners {
            match walk_path(schema, owner, &path, word) {
                Ok(r) => found.push(r),
                Err(e) => errors.push(e),
            }
        }
        match found.len() {
            1 => Ok(found.remove(0)),
            0 => Err(errors.remove(0)),
            _ => Err(Unresolved::new(
                format!(
                    "`{word}` could start at the {} called `{first}`; a path's owner must be one datablock",
                    found
                        .iter()
                        .map(|r| r.owner_kind.as_str())
                        .collect::<Vec<_>>()
                        .join(" or the ")
                ),
                None,
            )),
        }
    }
}

/// `word` with its `i`-th segment's handle replaced by `with`.
fn replace_segment(word: &str, i: usize, with: &str) -> String {
    let mut parts: Vec<String> = word.split('/').map(str::to_string).collect();
    let part = &parts[i];
    let tail = part
        .find('.')
        .map(|d| part[d..].to_string())
        .unwrap_or_default();
    let head = if i == 0 {
        part.split_once(':')
            .map(|(p, _)| format!("{p}:"))
            .unwrap_or_default()
    } else {
        String::new()
    };
    let head = if head.is_empty() { String::new() } else { head };
    parts[i] = format!("{head}{with}{tail}");
    parts.join("/")
}

fn walk_path(
    schema: &Schema,
    owner: &Owner,
    path: &Path,
    word: &str,
) -> Result<Resolved, Unresolved> {
    let mut subs = &owner.subs;
    let mut kinds = Vec::new();
    let mut here = owner.handle.clone();
    let mut last: Option<&SubObject> = None;
    for (i, seg) in path.segments.iter().enumerate().skip(1) {
        let Some(sub) = subs.iter().find(|s| s.handle == seg.handle) else {
            let names: Vec<&str> = subs.iter().map(|s| s.handle.as_str()).collect();
            let near = closest(&seg.handle, names.iter().copied()).map(str::to_string);
            let has = if names.is_empty() {
                "it has none".to_string()
            } else {
                format!("it has {}", some(&names))
            };
            return Err(Unresolved::new(
                format!("`{here}` has no sub-object `{}`: {has}", seg.handle),
                near.map(|n| replace_segment(word, i, &n)),
            ));
        };
        kinds.push(sub.kind.clone());
        here = sub.handle.clone();
        subs = &sub.subs;
        last = Some(sub);
    }
    if let Some(socket) = &path.socket {
        let Some(node) = last.filter(|s| s.kind == "node" || s.kind == "zone") else {
            return Err(Unresolved::new(
                format!(
                    "`{word}` ends with a socket, which only a node has: `@owner/@node.{socket}`"
                ),
                None,
            ));
        };
        let node_kind = node.of.as_deref().unwrap_or_default();
        if let Some(def) = schema.node(SHADER_TREE, node_kind) {
            let names: Vec<&str> = def
                .inputs
                .iter()
                .chain(def.outputs.iter())
                .map(|p| p.name.as_str())
                .collect();
            if !names.contains(&socket.as_str()) {
                let near = closest(socket, names.iter().copied());
                let fix =
                    near.map(|n| format!("{}.{n}", word.rsplit_once('.').map_or(word, |(a, _)| a)));
                return Err(Unresolved::new(
                    format!(
                        "a {node_kind} node has no socket `{socket}`: it has {}",
                        some(&names)
                    ),
                    fix,
                ));
            }
        }
    }
    Ok(Resolved {
        owner_kind: owner.kind.clone(),
        kinds,
        socket: path.socket.clone(),
    })
}

/// A list of names, the first eight and how many more.
fn some(names: &[&str]) -> String {
    if names.len() <= 8 {
        return names.join(", ");
    }
    format!("{} and {} more", names[..8].join(", "), names.len() - 8)
}
