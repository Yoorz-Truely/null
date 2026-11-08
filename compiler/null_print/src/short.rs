//! Short forms (Sau's decision S35; spec, section 8, "Short forms"; task 1d-03d): the shorter
//! ways of writing a thing that has a full form, read as their full forms, and kept by
//! write-back.
//!
//! - [`expand`] writes a project's files with every short form in its full form: what the
//!   checker and Sau's apply read the short form as. It is a view of the meaning, so comments
//!   and lines commented out with `/-` are not in it, and it is laid out by [`crate::format`].
//! - [`write_through`] is write-back's edit of one value, addressed as the full form spells it
//!   (the datablock, the lines down to the value, the value). It finds the value however the
//!   file writes it, short or full, and changes only that value, so a short form stays short:
//!   writing a value through a file and then expanding it gives what writing the same value
//!   through the expanded file gives.
//!
//! The short forms, each with its full form:
//! 1. **A datablock's handle from its name** (section 7): `object "Pier"` is `object @pier "Pier"`.
//! 2. **A sub-object's name from its handle** (sections 8, 9 and 10): `modifier @bevel bevel` is
//!    `modifier @bevel bevel "bevel"`, for a modifier, constraint, pose bone, node or attribute.
//! 3. **Settings on their owner's line** (section 5): `modifier @bevel bevel width=4mm` is
//!    `modifier @bevel bevel { width 4mm }`, for a datablock, modifier, constraint or pose bone.
//! 4. **A bare boolean** (section 5): `use-cyclic` is `use-cyclic on`.
//! 5. **`units`' short form** (section 8): `units metric scale=1 length=m rotation=deg` is
//!    `units { system metric; scale-length 1; length-unit meters; system-rotation degrees }`.
//! 6. **`slot=` left out** (section 11) where the action has one slot: `action @flicker` is
//!    `action @flicker slot=@lamp`.
//! 7. **An action's one layer and strip** (section 11, S71 (3)): f-curves in a slot's block are
//!    `layer @layer "Layer" { strip @strip keyframe { channels @slot { … } } }`, Blender's own
//!    name for the first layer, and the strip, which has no name, by its kind.
//! 8. **A path's owner left out in its own block** (section 7): `@bsdf.roughness` in
//!    `material @oak` is `@oak/@bsdf.roughness`. Lines name nodes and sub-objects of their own
//!    datablock the short way, so [`expand`] leaves the text as it is; the index of paths reads
//!    both the same (`null_check::paths::Index::resolve`), and write-back writes the short way
//!    in the owner's block.

use std::collections::HashMap;

use null_check::Schema;
use null_check::forms::{LENGTH_UNITS, ROTATION_UNITS, UNITS_SHORT};
use null_check::paths::{Index, Path};
use null_check::units::{self, Outside};
use null_syntax::ast::{Entry, Node, Value};
use null_syntax::names::to_kebab_case;
use null_syntax::{FileId, Parse, Span, parse};
use null_value::{Dimension, Scene, Unit};

use crate::format::format;
use crate::write::{Edit, Values, Word, WriteError, handle_of, insert_text, replace};

/// The sub-object lines whose quoted name may be left out (the handle without `@` then), with
/// the number of arguments before the name: `modifier @h kind "Name"`, `bone @h "Name"`,
/// `attribute @h domain type "Name"`.
const NAMED: [(&str, usize); 5] = [
    ("modifier", 2),
    ("constraint", 2),
    ("bone", 1),
    ("node", 2),
    ("attribute", 3),
];

/// The sub-object lines whose settings may stand on their line, as a datablock's may.
const SETTINGS_ON_LINE: [&str; 3] = ["modifier", "constraint", "bone"];

/// Blender's name for an action's first layer (`layer_default_name`, `animrig/intern/action.cc`).
const LAYER: &str = "Layer";

/// What one file's expansion needs to know of the others: each action's slots, in order.
struct Project {
    slots: HashMap<String, Vec<String>>,
}

impl Project {
    fn of(parses: &[Parse]) -> Project {
        let mut slots = HashMap::new();
        for p in parses {
            for n in p.document().nodes().filter(|n| n.name() == "action") {
                let Some(h) = handle_of(n) else { continue };
                let list: Vec<String> = n
                    .children()
                    .filter(|c| c.name() == "slot")
                    .filter_map(|c| {
                        c.args()
                            .next()
                            .and_then(|v| v.as_word())
                            .map(str::to_string)
                    })
                    .collect();
                slots.insert(h, list);
            }
        }
        Project { slots }
    }

    /// The action's one slot, if it has exactly one.
    fn only_slot(&self, action: &str) -> Option<&str> {
        match self.slots.get(action).map(Vec::as_slice) {
            Some([one]) => Some(one),
            _ => None,
        }
    }
}

/// Every file of a project with its short forms written in full, laid out by the formatter.
/// A file that does not parse cleanly is given back as it is, and so is one holding a lone `\`
/// word, which only the whitespace written after it keeps a word.
pub fn expand(texts: &[&str], schema: &Schema) -> Vec<String> {
    let parses: Vec<Parse> = texts.iter().map(|t| parse(t)).collect();
    let project = Project::of(&parses);
    let ex = Expander {
        schema,
        project: &project,
    };
    texts
        .iter()
        .zip(&parses)
        .map(|(text, p)| {
            // A lone `\` word is kept a word only by the unusual whitespace after it (a `\` before
            // a line's end is a continuation), which a file written afresh does not keep.
            let lone_backslash = null_syntax::lex(text)
                .0
                .iter()
                .scan(0, |at, t| {
                    let s = &text[*at..*at + t.len as usize];
                    *at += t.len as usize;
                    Some(t.kind == null_syntax::TokenKind::Word && s == "\\")
                })
                .any(|b| b);
            if !p.errors().is_empty() || lone_backslash {
                return (*text).to_string();
            }
            let mut out = String::new();
            for n in p.document().nodes().filter(|n| !n.is_commented_out()) {
                ex.node(n, true, 0, &mut out);
            }
            format(&out)
        })
        .collect()
}

struct Expander<'s, 'p> {
    schema: &'s Schema,
    project: &'p Project,
}

impl Expander<'_, '_> {
    fn is_datablock(&self, kind: &str) -> bool {
        self.schema.datablock_kinds().any(|s| s.kind == kind)
    }

    /// Whether some struct has a boolean property of this name (a socket's `boolean` too).
    fn is_boolean(&self, name: &str) -> bool {
        self.schema.structs.iter().any(|s| {
            s.properties
                .iter()
                .any(|p| p.name == name && matches!(p.ty.as_str(), "bool" | "boolean"))
        })
    }

    /// One node and its block, written in full at `depth`.
    fn node(&self, n: Node<'_>, top: bool, depth: usize, out: &mut String) {
        let name = n.name().into_owned();
        if name == "units" && !top {
            return self.units(n, depth, out);
        }
        let mut words: Vec<String> = Vec::new();
        let mut moved: Vec<String> = Vec::new();
        let datablock = top && self.is_datablock(&name);
        let settings_down = datablock || (!top && SETTINGS_ON_LINE.contains(&name.as_str()));
        let args: Vec<Value<'_>> = n.args().collect();
        // 1. A datablock's handle from its name.
        if datablock
            && let Some(first) = args.first()
            && let Some(s) = first.as_string()
            && let Some(h) = to_kebab_case(&s)
        {
            words.push(format!("@{h}"));
        }
        // 2. A sub-object's name from its handle.
        let name_at = NAMED
            .iter()
            .find(|(k, _)| !top && *k == name)
            .map(|(_, at)| *at)
            .filter(|at| {
                args.len() >= *at
                    && args[0].as_word().is_some_and(|w| w.starts_with('@'))
                    && !args
                        .iter()
                        .any(|v| matches!(v, Value::Str(_) | Value::Raw(_)))
            });
        let mut arg_i = 0;
        for e in n.entries() {
            match e {
                Entry::Arg(v) => {
                    if name_at == Some(arg_i) {
                        words.push(quoted(&args[0].text()[1..]));
                    }
                    words.push(v.text().to_string());
                    arg_i += 1;
                }
                Entry::Keyword { key, value } => {
                    words.push(format!("{} {}", key.text, value.text()));
                    arg_i += 1;
                }
                // 3. Settings on their owner's line.
                Entry::Prop { key, value } if settings_down => {
                    moved.push(format!("{} {}", key.text, value.text()));
                }
                Entry::Prop { key, value } => {
                    words.push(format!("{}={}", key.text, value.text()));
                }
            }
        }
        if name_at == Some(arg_i) {
            words.push(quoted(&args[0].text()[1..]));
        }
        // 6. `slot=` left out.
        if !top
            && name == "action"
            && let [Value::Word(t)] = args.as_slice()
            && !n
                .entries()
                .any(|e| matches!(e, Entry::Prop { key, .. } if key.text == "slot"))
            && let Some(slot) = self.project.only_slot(t.text)
        {
            words.push(format!("slot={slot}"));
        }
        // 4. A bare boolean.
        if !top && n.entries().next().is_none() && n.block().is_none() && self.is_boolean(&name) {
            words.push("on".into());
        }
        let pad = "  ".repeat(depth);
        // The name as written: a quoted one stays quoted (`"("` is no word).
        let written = n
            .name_token()
            .map_or_else(|| name.clone(), |t| t.text.to_string());
        let head = std::iter::once(written)
            .chain(words)
            .collect::<Vec<_>>()
            .join(" ");
        let children: Vec<Node<'_>> = n.children().collect();
        if moved.is_empty() && n.block().is_none() {
            out.push_str(&format!("{pad}{head}\n"));
            return;
        }
        out.push_str(&format!("{pad}{head} {{\n"));
        for m in moved {
            out.push_str(&format!("{pad}  {m}\n"));
        }
        // 7. An action's one layer and strip.
        let short_action = top
            && name == "action"
            && !children.iter().any(|c| c.name() == "layer")
            && children
                .iter()
                .any(|c| c.name() == "slot" && c.children().next().is_some());
        let mut channels: Vec<Node<'_>> = Vec::new();
        for c in &children {
            if short_action && c.name() == "slot" && c.children().next().is_some() {
                // The slot's line here, its f-curves in the strip's channels.
                let line: Vec<&str> = c.args().map(|v| v.text()).collect();
                out.push_str(&format!("{pad}  slot {}\n", line.join(" ")));
                channels.push(*c);
                continue;
            }
            self.node(*c, false, depth + 1, out);
        }
        if short_action {
            let taken: Vec<String> = children
                .iter()
                .filter_map(|c| {
                    c.args()
                        .next()
                        .and_then(|v| v.as_word())
                        .map(str::to_string)
                })
                .collect();
            let layer = free("@layer", &taken);
            let strip = free("@strip", &taken);
            out.push_str(&format!(
                "{pad}  layer {layer} {} {{\n{pad}    strip {strip} keyframe {{\n",
                quoted(LAYER)
            ));
            for s in channels {
                let slot = s.args().next().map_or("", |v| v.text());
                out.push_str(&format!("{pad}      channels {slot} {{\n"));
                for f in s.children() {
                    self.node(f, false, depth + 4, out);
                }
                out.push_str(&format!("{pad}      }}\n"));
            }
            out.push_str(&format!("{pad}    }}\n{pad}  }}\n"));
        }
        out.push_str(&format!("{pad}}}\n"));
    }

    /// `units`' short form (form 5): the system, then each setting by its full name, a unit word
    /// as its item.
    fn units(&self, n: Node<'_>, depth: usize, out: &mut String) {
        let pad = "  ".repeat(depth);
        let mut lines: Vec<String> = Vec::new();
        if let Some(v) = n.args().next() {
            lines.push(format!("system {}", v.text()));
        }
        let mut settings: Vec<(String, String)> = Vec::new();
        for e in n.entries() {
            if let Entry::Prop { key, value } = e {
                settings.push((key.text.to_string(), value.text().to_string()));
            }
        }
        for c in n.children() {
            let words: Vec<&str> = c.args().map(|v| v.text()).collect();
            let written = c
                .name_token()
                .map_or_else(|| c.name().into_owned(), |t| t.text.to_string());
            settings.push((written, words.join(" ")));
        }
        for (key, value) in settings {
            let full = full_unit_setting(&key);
            let value = unit_item(full, &value).map_or(value, str::to_string);
            lines.push(format!("{full} {value}"));
        }
        if lines.is_empty() {
            out.push_str(&format!("{pad}units {{}}\n"));
            return;
        }
        out.push_str(&format!("{pad}units {{\n"));
        for l in lines {
            out.push_str(&format!("{pad}  {l}\n"));
        }
        out.push_str(&format!("{pad}}}\n"));
    }
}

fn quoted(s: &str) -> String {
    null_syntax::escape(s)
}

/// `want`, or with `-2`, `-3`, … where a sub-object of the owner already has it.
fn free(want: &str, taken: &[String]) -> String {
    if !taken.iter().any(|t| t == want) {
        return want.to_string();
    }
    (2..)
        .map(|k| format!("{want}-{k}"))
        .find(|h| !taken.iter().any(|t| t == h))
        .unwrap_or_default()
}

/// A `units` setting's full name: `scale` is `scale-length`.
fn full_unit_setting(key: &str) -> &str {
    UNITS_SHORT
        .iter()
        .find(|(s, _)| *s == key)
        .map_or(key, |(_, f)| f)
}

/// A `units` setting's short name, if it has one: `scale-length` is `scale`.
fn short_unit_setting(full: &str) -> Option<&'static str> {
    UNITS_SHORT
        .iter()
        .find(|(_, f)| *f == full)
        .map(|(s, _)| *s)
}

/// The item a unit word stands for in `length-unit` or `system-rotation` (`m` is `meters`).
fn unit_item(setting: &str, word: &str) -> Option<&'static str> {
    unit_table(setting)
        .iter()
        .find(|(u, _)| *u == word)
        .map(|(_, i)| *i)
}

fn unit_table(setting: &str) -> &'static [(&'static str, &'static str)] {
    match setting {
        "length-unit" => &LENGTH_UNITS,
        "system-rotation" => &ROTATION_UNITS,
        _ => &[],
    }
}

/// One line on the way from a datablock to a value, as the full form writes it: its name and
/// its first arguments (`fcurve` and `["energy"]`, `key` and `["6"]`, `modifier` and
/// `["@bevel"]`).
#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    /// The line's name.
    pub name: &'a str,
    /// The words its line starts with, after its name.
    pub args: &'a [&'a str],
}

/// Which value of the last line (or of the datablock, with no lines) is written.
#[derive(Clone, Copy, Debug)]
pub enum Attr<'a> {
    /// A setting by its name: a child line's value, or `name=value` on the line.
    Named(&'a str),
    /// The line's own argument at this index (a key's value is its argument 1).
    Arg(usize),
}

/// A value as the full form addresses it.
#[derive(Clone, Copy, Debug)]
pub struct Address<'a> {
    /// The datablock's kind.
    pub kind: &'a str,
    /// The datablock's handle.
    pub handle: &'a str,
    /// The lines from the datablock down, in full form.
    pub lines: &'a [Line<'a>],
    /// The value.
    pub attr: Attr<'a>,
}

/// A new value.
#[derive(Clone, Copy, Debug)]
pub enum New<'a> {
    /// Numbers of a dimension, written in the unit their word already uses (S21), and in
    /// `display` when new.
    Number(Values<'a>, Dimension, Option<Unit>),
    /// `on` or `off`.
    Bool(bool),
    /// An enum item or a handle.
    Word(&'a str),
    /// A path in full, from its datablock (`@oak/@bsdf`).
    Path(&'a str),
}

/// Where the address leads in a file.
struct Found<'a> {
    /// The datablock's node.
    datablock: Node<'a>,
    /// The node the value is on, if the file writes it (an action's slot for its short form's
    /// channels).
    node: Node<'a>,
}

/// The edits that set the value at `at` in file `file` of the project `texts` to `new`, finding
/// it through the short forms: only the value changes, so a short form stays short.
pub fn write_through(
    texts: &[&str],
    file: usize,
    at: &Address<'_>,
    new: New<'_>,
) -> Result<Vec<Edit>, WriteError> {
    let parses: Vec<Parse> = texts.iter().map(|t| parse(t)).collect();
    let project = Project::of(&parses);
    let p = parses.get(file).ok_or(WriteError::NoSuchNode)?;
    let text = p.text();
    let found = find(p, at)?;
    // Outside every scene: the project's scale, or the one the scenes agree on (section 6); where
    // they disagree, scaled values are written bare, as `write_transform` writes them.
    let scene = match units::outside(&parses) {
        Outside::Scale(scale_length) => Some(Scene {
            scale_length,
            ..Scene::default()
        }),
        Outside::Disagree(_) => None,
    };
    let w = Writer {
        text,
        found: &found,
        parses: &parses,
        scene,
    };
    let node = found.node;
    match at.attr {
        Attr::Arg(i) => {
            let v = node.args().nth(i).ok_or(WriteError::NoSuchNode)?;
            Ok(w.value(v, new))
        }
        Attr::Named(name) => {
            // `units`: the system is its argument, settings by their short or full names.
            if node.name() == "units" {
                return Ok(w.units(node, name, new));
            }
            // `slot=` left out where the action has one slot.
            if name == "slot"
                && node.name() == "action"
                && prop(node, "slot").is_none()
                && let (New::Word(slot), Some(a)) =
                    (new, node.args().next().and_then(|v| v.as_word()))
                && project.only_slot(a) == Some(slot)
            {
                return Ok(Vec::new());
            }
            if let Some(child) = node.children().find(|c| c.name() == name) {
                return match child.args().next() {
                    Some(v) => Ok(w.value(v, new)),
                    // A bare boolean: on as it stands, `off` after its name.
                    None if child.block().is_none() => Ok(match new {
                        New::Bool(true) => Vec::new(),
                        _ => vec![Edit {
                            span: Span::empty(child.code_span().end),
                            replacement: format!(" {}", w.printed(new)),
                        }],
                    }),
                    None => Err(WriteError::NoSuchNode),
                };
            }
            if let Some(v) = prop(node, name) {
                return Ok(w.value(v, new));
            }
            Ok(vec![w.insert(node, name, new)])
        }
    }
}

/// The node an address leads to, through an action's short form.
fn find<'a>(p: &'a Parse, at: &Address<'_>) -> Result<Found<'a>, WriteError> {
    let datablock = p
        .document()
        .nodes()
        .find(|n| n.name() == at.kind && handle_of(*n).as_deref() == Some(at.handle))
        .ok_or(WriteError::NoSuchNode)?;
    let mut node = datablock;
    let mut virtual_strip = false;
    for (i, line) in at.lines.iter().enumerate() {
        let short_action =
            i == 0 && at.kind == "action" && !datablock.children().any(|c| c.name() == "layer");
        if short_action && line.name == "layer" {
            virtual_strip = true;
            continue;
        }
        if virtual_strip && line.name == "strip" {
            continue;
        }
        let name = if virtual_strip && line.name == "channels" {
            virtual_strip = false;
            "slot"
        } else {
            line.name
        };
        node = node
            .children()
            .find(|c| {
                c.name() == name && {
                    let words: Vec<&str> = c.args().map(|v| v.text()).collect();
                    words.len() >= line.args.len()
                        && words.iter().zip(line.args).all(|(a, b)| a == b)
                }
            })
            .ok_or(WriteError::NoSuchNode)?;
    }
    Ok(Found { datablock, node })
}

fn prop<'a>(node: Node<'a>, name: &str) -> Option<Value<'a>> {
    node.entries().find_map(|e| match e {
        Entry::Prop { key, value } if key.text == name => Some(value),
        _ => None,
    })
}

/// What a write needs of its file and project.
struct Writer<'w, 'a> {
    text: &'a str,
    found: &'w Found<'a>,
    parses: &'w [Parse],
    scene: Option<Scene>,
}

impl Writer<'_, '_> {
    /// A new value as written where nothing says how: a number in `display` where it fits, a
    /// boolean `on` or `off`.
    fn printed(&self, new: New<'_>) -> String {
        match new {
            New::Number(values, _, display) => {
                crate::write::printed(&Word::all(values, self.scene), display)
            }
            New::Bool(b) => if b { "on" } else { "off" }.to_string(),
            New::Word(w) | New::Path(w) => w.to_string(),
        }
    }

    /// The edits that make written value `v` the new value, in the form it is written in.
    fn value(&self, v: Value<'_>, new: New<'_>) -> Vec<Edit> {
        let replace_with = |s: String| {
            vec![Edit {
                span: v.span(),
                replacement: s,
            }]
        };
        match new {
            New::Number(values, dimension, _) => {
                replace(v, &Word::all(values, self.scene), dimension)
            }
            New::Bool(b) => {
                let now = match v.as_word() {
                    Some("on") => Some(true),
                    Some("off") => Some(false),
                    _ => None,
                };
                if now == Some(b) {
                    Vec::new()
                } else {
                    replace_with(self.printed(new))
                }
            }
            New::Word(w) if v.as_word() == Some(w) => Vec::new(),
            New::Word(w) => replace_with(w.to_string()),
            New::Path(full) => self.path(v, full).map_or_else(Vec::new, replace_with),
        }
    }

    /// A path's new word (form 8), or none where `v` already names its target. Within its
    /// owner's block a path whose first step is one of the owner's sub-objects is the owner's
    /// (section 7), so a full path to one of them is written from there.
    fn path(&self, v: Value<'_>, full: &str) -> Option<String> {
        let owner = self.found.datablock;
        let kind = owner.name().into_owned();
        let Some(handle) = handle_of(owner) else {
            return Some(full.to_string());
        };
        let files: Vec<(FileId, &Parse)> = self
            .parses
            .iter()
            .enumerate()
            .map(|(i, p)| (FileId(u32::try_from(i).unwrap_or(u32::MAX)), p))
            .collect();
        let index = Index::build(&files);
        let own: Vec<String> = index
            .owners(&handle)
            .iter()
            .filter(|o| o.kind == kind)
            .flat_map(|o| o.subs.iter().map(|s| s.handle.clone()))
            .collect();
        // The path a word in the owner's block stands for.
        let first_own = |w: &str| {
            Path::parse(w).is_ok_and(|p| {
                p.package.is_none() && p.segments.first().is_some_and(|s| own.contains(&s.handle))
            })
        };
        let in_full = |w: &str| {
            if first_own(w) {
                format!("{handle}/{w}")
            } else {
                w.to_string()
            }
        };
        if v.as_word().is_some_and(|w| in_full(w) == full) {
            return None;
        }
        let short = full
            .strip_prefix(&format!("{handle}/"))
            .filter(|rest| first_own(rest));
        Some(short.unwrap_or(full).to_string())
    }

    /// A setting the line does not have yet: a line in its block (a bare one for `on`), or
    /// `name=value` on its line.
    fn insert(&self, node: Node<'_>, name: &str, new: New<'_>) -> Edit {
        if matches!(new, New::Bool(true)) && node.block().is_some() {
            return insert_text(self.text, node, name, None);
        }
        insert_text(self.text, node, name, Some(&self.printed(new)))
    }

    /// The edits for one `units` setting, by its full name. The system is the line's argument;
    /// a setting is found by its short name or its full one, and is new on the line by its short
    /// one; a unit word stays a unit word.
    fn units(&self, node: Node<'_>, full: &str, new: New<'_>) -> Vec<Edit> {
        let short = short_unit_setting(full);
        let written: Option<Value<'_>> = if full == "system" {
            node.args().next().or_else(|| child_value(node, "system"))
        } else {
            [Some(full), short]
                .into_iter()
                .flatten()
                .find_map(|k| prop(node, k).or_else(|| child_value(node, k)))
        };
        match (written, new) {
            (Some(v), New::Word(item)) => {
                let word = v.as_word().unwrap_or_default();
                // A unit word stands for its item, and stays a unit word where one stands for
                // the new item.
                let as_unit = unit_item(full, word);
                if as_unit.unwrap_or(word) == item {
                    return Vec::new();
                }
                let write = unit_table(full)
                    .iter()
                    .find(|(_, i)| *i == item)
                    .filter(|_| as_unit.is_some())
                    .map_or(item, |(u, _)| *u);
                vec![Edit {
                    span: v.span(),
                    replacement: write.to_string(),
                }]
            }
            (Some(v), _) => self.value(v, new),
            (None, _) if full == "system" && node.block().is_none() => {
                // `units` with no system: the system is its first argument.
                let end = node
                    .name_token()
                    .map_or(node.code_span().end, |t| t.span.end);
                vec![Edit {
                    span: Span::empty(end),
                    replacement: format!(" {}", self.printed(new)),
                }]
            }
            (None, _) => {
                let key = if node.block().is_some() {
                    full
                } else {
                    short.unwrap_or(full)
                };
                vec![insert_text(self.text, node, key, Some(&self.printed(new)))]
            }
        }
    }
}

fn child_value<'a>(node: Node<'a>, name: &str) -> Option<Value<'a>> {
    node.children()
        .find(|c| c.name() == name)
        .and_then(|c| c.args().next())
}
