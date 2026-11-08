//! Node trees (spec, section 10; task 1d-07a): a material's, world's or light's `nodes` and
//! `layout`, and node groups (`shader`, `geometry`, `compositor` datablocks) with their
//! interface (`input`, `output`, `panel`). A node is `node @handle <kind> "Name" { … }`: its own
//! settings (the node struct's properties) and its inputs (the schema's sockets, a group's
//! interface for a group node), each a value or a link `@node.output`. A zone (repeat,
//! simulation, for-each, closure) is `zone @handle <kind> "Name" { … }`: its items, its input
//! node's inputs and settings, an `output { … }` block for its output node's inputs, and the
//! nodes written inside it.

use std::collections::{HashMap, HashSet};

use null_syntax::ast::{Node, Value};
use null_syntax::{FileId, closest};

use super::{Attr, Checker, args_of, attrs, header, kebab_handle};
use crate::codes;
use crate::schema::{NodeDef, Property};
use crate::values::{check_float, list_near};

/// Node properties every node has that a file may write as settings: the rest of `Node`'s are
/// the editor's or Blender's own (`location` and `width` are the `layout` block's, S45).
const NODE_SETTINGS: &[&str] = &[
    "label",
    "mute",
    "hide",
    "use-custom-color",
    "color",
    "color-tag",
    "show-options",
    "show-preview",
    "show-texture",
    "parent",
];

/// Properties of an interface socket or panel that its line writes, not a setting: its handle
/// and quoted name stand for `identifier` and `name`, its word for `socket-type`.
const INTERFACE_LINE: &[&str] = &[
    "name",
    "identifier",
    "socket-type",
    "in-out",
    "bl-socket-idname",
    "item-type",
    "parent",
    "position",
    "index",
    "interface-items",
    "persistent-uid",
    "select",
];

/// A node group's interface: its inputs' and outputs' handles, without `@`, which are the
/// sockets of a group node that uses it and of its own group input and output nodes.
#[derive(Clone, Debug, Default)]
pub(super) struct Interface {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

/// Every node group's interface in the project, by handle, read before any tree is checked
/// so a group node may use a group defined in another file.
pub(super) fn interfaces(node: Node<'_>, into: &mut HashMap<String, Interface>) {
    let Some((handle, _)) = header(node).handle else {
        return;
    };
    let mut iface = Interface::default();
    fn walk(n: Node<'_>, iface: &mut Interface) {
        for child in n.children() {
            let word = child.name();
            let first = args_of(child)
                .first()
                .and_then(|v| v.as_word())
                .filter(|w| w.starts_with('@'))
                .map(|w| w.trim_start_matches('@').to_string());
            match (word.as_ref(), first) {
                ("input", Some(h)) => iface.inputs.push(h),
                ("output", Some(h)) => iface.outputs.push(h),
                ("panel", _) => walk(child, iface),
                _ => {}
            }
        }
    }
    walk(node, &mut iface);
    into.insert(handle, iface);
}

/// One handle of a tree: a node, with its kind, or a zone.
#[derive(Clone, Debug)]
pub(super) struct Entry {
    kind: String,
    zone: bool,
    /// A zone's items' sockets, or a group node's group, by handle.
    items: Vec<String>,
    group: Option<String>,
}

/// A tree being checked: its type (`ShaderNodeTree`), its handles, and its own interface
/// where it is a node group.
pub(super) struct Tree {
    pub rna: String,
    entries: HashMap<String, Entry>,
    interface: Option<Interface>,
}

impl Tree {
    /// The handles `nodes` defines, zones and the nodes inside them included.
    pub(super) fn new(rna: &str, nodes: Option<Node<'_>>, interface: Option<Interface>) -> Tree {
        let mut entries = HashMap::new();
        fn walk(n: Node<'_>, entries: &mut HashMap<String, Entry>) {
            for child in n.children() {
                let word = child.name();
                if word != "node" && word != "zone" {
                    continue;
                }
                let args = args_of(child);
                let (Some(h), Some(k)) = (args.first(), args.get(1)) else {
                    continue;
                };
                let zone = word == "zone";
                let items = if zone {
                    child
                        .children()
                        .filter(|c| c.name().ends_with("-item"))
                        .filter_map(|c| {
                            args_of(c)
                                .first()
                                .and_then(|v| v.as_word())
                                .map(|w| w.trim_start_matches('@').to_string())
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                let group = attrs(child)
                    .iter()
                    .find(|a| a.name == "node-tree")
                    .and_then(|a| a.args.first())
                    .and_then(|v| v.as_word())
                    .map(str::to_string);
                entries.insert(
                    h.text().to_string(),
                    Entry {
                        kind: k.text().to_string(),
                        zone,
                        items,
                        group,
                    },
                );
                if zone {
                    walk(child, entries);
                }
            }
        }
        if let Some(n) = nodes {
            walk(n, &mut entries);
        }
        Tree {
            rna: rna.to_string(),
            entries,
            interface,
        }
    }
}

/// The zone kinds a tree takes, from the schema: a node `<kind>-input` with a paired output
/// node `<kind>-output` in the same tree.
fn zone_kinds<'a>(cx: &Checker<'a>, tree: &str) -> Vec<&'a str> {
    let schema = cx.schema;
    schema
        .node_kinds(tree)
        .into_iter()
        .filter_map(|k| k.strip_suffix("-input"))
        .filter(|z| {
            schema.node(tree, &format!("{z}-output")).is_some()
                && schema
                    .node(tree, &format!("{z}-input"))
                    .and_then(|d| d.rna.as_deref())
                    .and_then(|r| schema.struct_by_rna(r))
                    .is_some_and(|s| s.properties.iter().any(|p| p.name == "paired-output"))
        })
        .collect()
}

fn sockets(list: &[Property]) -> Vec<&str> {
    list.iter()
        .filter(|p| p.name != "extend")
        .map(|p| p.name.as_str())
        .collect()
}

impl<'s> Checker<'s> {
    /// The node tree kind a datablock of `kind` holds, and whether it is a group (has an
    /// interface): a material's, world's and light's are shader trees; a node group is its own.
    pub(super) fn tree_of(&self, kind: &str) -> Option<(&'s str, bool)> {
        match kind {
            "material" | "world" | "light" => Some(("ShaderNodeTree", false)),
            _ => {
                let s = self.schema.struct_by_kind(kind)?;
                (s.base.as_deref() == Some("NodeTree")).then(|| {
                    let rna: &'s str = &self.schema.struct_by_kind(kind).unwrap().rna;
                    (rna, true)
                })
            }
        }
    }

    /// `nodes { … }`: the tree's nodes and zones.
    pub(super) fn nodes(&mut self, file: FileId, a: &Attr<'_>, tree: &Tree) {
        let Some(block) = a.node.filter(|n| n.block().is_some()) else {
            return self.out(file).error(
                codes::TYPE,
                a.span,
                "`nodes` takes a block of `node @handle <kind>` lines",
            );
        };
        let mut seen = HashSet::new();
        self.node_lines(file, block, tree, None, &mut seen);
    }

    fn node_lines(
        &mut self,
        file: FileId,
        block: Node<'_>,
        tree: &Tree,
        zone: Option<&str>,
        seen: &mut HashSet<String>,
    ) {
        let zones = zone_kinds(self, &tree.rna);
        for child in block.children() {
            let Some(name) = child.name_token() else {
                continue;
            };
            let word = child.name();
            if zone.is_some() && (word.ends_with("-item") || word == "output") {
                continue; // the zone's own lines, checked with it
            }
            if word != "node" && word != "zone" {
                if zone.is_none() {
                    self.out(file).error(
                        codes::UNKNOWN_PROPERTY,
                        name.span,
                        format!("a node tree holds `node` and `zone` lines, not `{word}`"),
                    );
                }
                continue;
            }
            let args = args_of(child);
            let (Some(h), Some(k)) = (args.first(), args.get(1)) else {
                self.out(file).error(
                    codes::TYPE,
                    child.code_span(),
                    format!(
                        "a {word} is written `{word} @handle <kind>`, such as `{}`",
                        if word == "zone" {
                            "zone @loop repeat"
                        } else {
                            "node @bsdf bsdf-principled"
                        }
                    ),
                );
                continue;
            };
            let handle = h.text();
            if !handle.starts_with('@') {
                self.out(file).error(
                    codes::TYPE,
                    h.span(),
                    format!("a {word}'s handle comes first, such as `@bsdf`"),
                );
                continue;
            }
            kebab_handle(handle, h.span(), &mut self.out(file));
            if !seen.insert(handle.to_string()) {
                self.out(file).error(
                    codes::HANDLE,
                    h.span(),
                    format!("`{handle}` names two nodes of this tree"),
                );
            }
            // After the kind, the Blender name in quotes may follow (Sau's task 1b-02d).
            for (i, v) in args.iter().enumerate().skip(2) {
                if i == 2 && matches!(v, Value::Str(_) | Value::Raw(_)) {
                    continue;
                }
                self.out(file).error(
                    codes::TYPE,
                    v.span(),
                    format!(
                        "a {word}'s line is `{word} @handle kind \"Name\"`; its settings and inputs are written as `name value` lines or `name=value`"
                    ),
                );
            }
            let kind = k.text();
            if word == "zone" {
                if !zones.contains(&kind) {
                    let near = closest(kind, zones.iter().copied());
                    self.out(file).error_fix(
                        codes::KIND,
                        k.span(),
                        format!(
                            "`{kind}` is not a zone this tree takes: {}",
                            list_near(kind, &zones)
                        ),
                        near.map(|n| (format!("write `{n}`"), k.span(), n.to_string())),
                    );
                    continue;
                }
                self.zone(file, child, tree, handle, kind, seen);
                continue;
            }
            let Some(def) = self.node_def(file, tree, k) else {
                continue;
            };
            self.node_body(file, child, tree, &def, zone);
        }
    }

    /// The schema's node of the kind `k` names, or an error with the nearest kind.
    fn node_def(&mut self, file: FileId, tree: &Tree, k: &Value<'_>) -> Option<NodeDef> {
        let kind = k.text();
        if let Some(def) = self.schema.node(&tree.rna, kind) {
            return Some(def.clone());
        }
        let kinds: Vec<&str> = self.schema.node_kinds(&tree.rna);
        // The same words in another order first: S67 made `principled-bsdf` `bsdf-principled`
        // (1b-03c's review).
        let mut words: Vec<&str> = kind.split('-').collect();
        words.sort_unstable();
        let reordered = kinds.iter().copied().find(|k| {
            let mut w: Vec<&str> = k.split('-').collect();
            w.sort_unstable();
            w == words
        });
        let near = reordered.or_else(|| closest(kind, kinds.iter().copied()));
        self.out(file).error_fix(
            codes::KIND,
            k.span(),
            format!(
                "`{kind}` is not a node kind a {} tree takes: {}",
                tree_word(&tree.rna),
                list_near(kind, &kinds)
            ),
            near.map(|n| (format!("write `{n}`"), k.span(), n.to_string())),
        );
        None
    }

    /// A node's settings and inputs.
    fn node_body(
        &mut self,
        file: FileId,
        node: Node<'_>,
        tree: &Tree,
        def: &NodeDef,
        zone: Option<&str>,
    ) {
        let schema = self.schema;
        let kind = def.kind_in(&tree.rna).unwrap_or(&def.kind).to_string();
        let node_base: Vec<&str> = schema
            .struct_by_rna("Node")
            .map(|n| n.properties.iter().map(|q| q.name.as_str()).collect())
            .unwrap_or_default();
        let settings: Vec<&Property> = def
            .rna
            .as_deref()
            .map(|r| schema.properties_of(r))
            .unwrap_or_default()
            .into_iter()
            .filter(|p| {
                !node_base.contains(&p.name.as_str()) || NODE_SETTINGS.contains(&p.name.as_str())
            })
            .collect();
        // A group node's inputs are its group's; a group output's, its own group's outputs.
        let group_inputs: Option<Vec<String>> = match kind.as_str() {
            "group" => attrs(node)
                .iter()
                .find(|a| a.name == "node-tree")
                .and_then(|a| a.args.first())
                .and_then(|v| v.as_word())
                .and_then(|g| self.groups.get(g))
                .map(|i| i.inputs.clone()),
            "group-output" => tree.interface.as_ref().map(|i| i.outputs.clone()),
            _ => None,
        };
        let inputs: Vec<&Property> = def.inputs.iter().filter(|p| p.name != "extend").collect();
        let mut names: Vec<&str> = inputs.iter().map(|p| p.name.as_str()).collect();
        if let Some(g) = &group_inputs {
            names.extend(g.iter().map(String::as_str));
        }
        names.extend(settings.iter().map(|p| p.name.as_str()));
        let mut given = HashSet::new();
        for a in attrs(node) {
            if !given.insert(a.name.clone()) {
                self.out(file).error(
                    codes::REPEATED,
                    a.name_span,
                    format!("`{}` is given twice", a.name),
                );
                continue;
            }
            if a.name == "parent" {
                self.parent(file, &a, tree);
                continue;
            }
            if let Some(p) = inputs.iter().find(|p| p.name == a.name) {
                self.socket_value(file, &a, p, tree, zone);
                continue;
            }
            if group_inputs.as_ref().is_some_and(|g| g.contains(&a.name)) {
                // A group's socket: its type is the group's, checked once interfaces have
                // types the checker reads; a link is checked here.
                if let [v] = a.args.as_slice()
                    && v.as_word().is_some_and(|w| w.starts_with('@'))
                {
                    self.link(file, v, tree, zone);
                }
                continue;
            }
            if let Some(p) = settings.iter().find(|p| p.name == a.name) {
                self.property(file, &a, p, false);
                continue;
            }
            if a.name == "location" || a.name == "width" {
                self.out(file).error(
                    codes::UNKNOWN_PROPERTY,
                    a.name_span,
                    format!(
                        "a node's `{}` is written in the tree's `layout` block (S45)",
                        a.name
                    ),
                );
                continue;
            }
            self.unknown(file, &a, &format!("a {kind} node"), &names, None);
        }
    }

    /// One input socket's value: a value of its type, a link, or, for a socket that takes
    /// several links (`multi-input`), a list of links.
    fn socket_value(
        &mut self,
        file: FileId,
        a: &Attr<'_>,
        p: &Property,
        tree: &Tree,
        zone: Option<&str>,
    ) {
        match a.args.as_slice() {
            [v] if v.as_word().is_some_and(|w| w.starts_with('@')) => {
                self.link(file, v, tree, zone)
            }
            [v @ Value::List(_)]
                if p.multi_input
                    && v.items()
                        .iter()
                        .all(|i| i.as_word().is_some_and(|w| w.starts_with('@'))) =>
            {
                for item in v.items() {
                    self.link(file, &item, tree, zone);
                }
            }
            _ if matches!(
                p.ty.as_str(),
                "geometry" | "shader" | "custom" | "bundle" | "closure"
            ) =>
            {
                self.out(file).error(
                    codes::TYPE,
                    a.span,
                    format!(
                        "`{}` takes a link, such as `@node.output`; it has no value of its own",
                        a.name
                    ),
                )
            }
            _ => self.property(file, a, p, false),
        }
    }

    /// `parent @frame`: the frame node a node is drawn in.
    fn parent(&mut self, file: FileId, a: &Attr<'_>, tree: &Tree) {
        match a.args.as_slice() {
            [v] if v
                .as_word()
                .is_some_and(|w| tree.entries.get(w).is_some_and(|e| e.kind == "frame")) => {}
            [v] => self.out(file).error(
                codes::HANDLE,
                v.span(),
                format!(
                    "`parent` names a `frame` node of this tree, not `{}`",
                    v.text()
                ),
            ),
            _ => self.out(file).error(
                codes::TYPE,
                a.span,
                "`parent` takes a frame node's handle, such as `parent @group-box`",
            ),
        }
    }

    /// `@node.socket`: a link from another node's or a zone's output. Inside a zone's block,
    /// `@zone.item` is the zone's input node's output (an item as it comes into an iteration);
    /// anywhere else, its output node's (the item as it leaves).
    pub(super) fn link(&mut self, file: FileId, v: &Value<'_>, tree: &Tree, zone: Option<&str>) {
        let word = v.as_word().unwrap_or_default();
        let Some((node, socket)) = word.split_once('.') else {
            return self.out(file).error(
                codes::TYPE,
                v.span(),
                format!("a link names a node and its output, such as `{word}.bsdf`"),
            );
        };
        let Some(entry) = tree.entries.get(node) else {
            let near = closest(node, tree.entries.keys().map(String::as_str)).map(str::to_string);
            return self.out(file).error_fix(
                codes::HANDLE,
                v.span(),
                format!("no node of this tree is called `{node}`"),
                near.map(|n| {
                    (
                        format!("write `{n}.{socket}`"),
                        v.span(),
                        format!("{n}.{socket}"),
                    )
                }),
            );
        };
        let outs: Vec<String> = if entry.zone {
            let side = if zone == Some(node) {
                "input"
            } else {
                "output"
            };
            let mut outs: Vec<String> = self
                .schema
                .node(&tree.rna, &format!("{}-{side}", entry.kind))
                .map(|d| {
                    sockets(&d.outputs)
                        .into_iter()
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            outs.extend(entry.items.iter().cloned());
            outs
        } else {
            match entry.kind.as_str() {
                "group" => match entry.group.as_deref().and_then(|g| self.groups.get(g)) {
                    Some(i) => i.outputs.clone(),
                    None => return, // the group itself is reported where the node names it
                },
                "group-input" => match &tree.interface {
                    Some(i) => i.inputs.clone(),
                    None => return,
                },
                k => match self.schema.node(&tree.rna, k) {
                    Some(def) => sockets(&def.outputs)
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    None => return, // an unknown kind is reported on its own line
                },
            }
        };
        if !outs.iter().any(|o| o == socket) {
            let near = closest(socket, outs.iter().map(String::as_str)).map(str::to_string);
            let shown: Vec<&str> = outs.iter().map(String::as_str).collect();
            let what = if entry.zone {
                format!("the {} zone `{node}`", entry.kind)
            } else {
                format!("a {} node", entry.kind)
            };
            self.out(file).error_fix(
                codes::HANDLE,
                v.span(),
                format!(
                    "{what} has no output `{socket}`{}: it has {}",
                    if entry.zone { " here" } else { "" },
                    if shown.is_empty() {
                        "none".to_string()
                    } else {
                        list_near(socket, &shown)
                    }
                ),
                near.map(|n| {
                    (
                        format!("write `{node}.{n}`"),
                        v.span(),
                        format!("{node}.{n}"),
                    )
                }),
            );
        }
    }

    /// `zone @h <kind> "Name" { <items>; <input node's inputs and settings>; output { … };
    /// <nodes> }`.
    fn zone(
        &mut self,
        file: FileId,
        node: Node<'_>,
        tree: &Tree,
        handle: &str,
        kind: &str,
        seen: &mut HashSet<String>,
    ) {
        let schema = self.schema;
        let input = schema.node(&tree.rna, &format!("{kind}-input")).cloned();
        let output = schema.node(&tree.rna, &format!("{kind}-output")).cloned();
        let (Some(input), Some(output)) = (input, output) else {
            return;
        };
        // Item collections are the output node's (`repeat-items`): each is written with the
        // collection's name less its `s`, `repeat-item @h <socket-type> "Name"`.
        let out_struct = output.rna.as_deref().and_then(|r| schema.struct_by_rna(r));
        let item_words: Vec<(String, String)> = out_struct
            .into_iter()
            .flat_map(|s| s.properties.iter())
            .chain(
                input
                    .rna
                    .as_deref()
                    .and_then(|r| schema.struct_by_rna(r))
                    .into_iter()
                    .flat_map(|s| s.properties.iter()),
            )
            .filter(|p| p.ty == "collection" && p.name.ends_with("-items"))
            .filter_map(|p| Some((p.name.strip_suffix('s')?.to_string(), p.target.clone()?)))
            .collect();
        let settings: Vec<&Property> = [input.rna.as_deref(), output.rna.as_deref()]
            .into_iter()
            .flatten()
            .filter_map(|r| schema.struct_by_rna(r))
            .flat_map(|s| s.properties.iter())
            .filter(|p| p.ty != "collection" && !p.readonly && p.name != "paired-output")
            .filter(|p| !p.name.starts_with("active-"))
            .collect();
        let inputs: Vec<&Property> = input.inputs.iter().filter(|p| p.name != "extend").collect();
        let mut names: Vec<&str> = inputs.iter().map(|p| p.name.as_str()).collect();
        names.extend(settings.iter().map(|p| p.name.as_str()));
        names.extend(item_words.iter().map(|(w, _)| w.as_str()));
        names.push("output");
        let items = tree
            .entries
            .get(handle)
            .map(|e| e.items.clone())
            .unwrap_or_default();
        let mut given = HashSet::new();
        for a in attrs(node) {
            if a.name == "node" || a.name == "zone" {
                continue;
            }
            if let Some((_, target)) = item_words.iter().find(|(w, _)| *w == a.name) {
                self.zone_item(file, &a, target);
                continue;
            }
            if !given.insert(a.name.clone()) {
                self.out(file).error(
                    codes::REPEATED,
                    a.name_span,
                    format!("`{}` is given twice", a.name),
                );
                continue;
            }
            if a.name == "output" {
                self.zone_output(file, &a, tree, &output, &items, handle);
                continue;
            }
            if let Some(p) = inputs.iter().find(|p| p.name == a.name) {
                // The input node's inputs take values from outside the zone.
                self.socket_value(file, &a, p, tree, None);
                continue;
            }
            if items.contains(&a.name) {
                if let [v] = a.args.as_slice()
                    && v.as_word().is_some_and(|w| w.starts_with('@'))
                {
                    self.link(file, v, tree, None);
                }
                continue;
            }
            if let Some(p) = settings.iter().find(|p| p.name == a.name) {
                self.property(file, &a, p, false);
                continue;
            }
            self.unknown(file, &a, &format!("a {kind} zone"), &names, None);
        }
        self.node_lines(file, node, tree, Some(handle), seen);
    }

    /// `repeat-item @h <socket-type> "Name" { … }`: one item of a zone, of struct `target`.
    fn zone_item(&mut self, file: FileId, a: &Attr<'_>, target: &str) {
        let schema = self.schema;
        let Some(s) = schema.struct_by_rna(target) else {
            return;
        };
        let props = schema.properties_of(target);
        let socket_type = props.iter().copied().find(|p| p.name == "socket-type");
        match a.args.as_slice() {
            [h, t, rest @ ..] if h.as_word().is_some_and(|w| w.starts_with('@')) => {
                kebab_handle(h.text(), h.span(), &mut self.out(file));
                if let Some(p) = socket_type {
                    crate::values::check_enum(t, "socket-type", &strict(p), &mut self.out(file));
                }
                for (i, v) in rest.iter().enumerate() {
                    if i == 0 && matches!(v, Value::Str(_) | Value::Raw(_)) {
                        continue;
                    }
                    self.out(file).error(
                        codes::TYPE,
                        v.span(),
                        format!(
                            "`{}`'s line is `{} @handle <socket-type> \"Name\"`",
                            a.name, a.name
                        ),
                    );
                }
            }
            _ => {
                return self.out(file).error(
                    codes::TYPE,
                    a.span,
                    format!("`{}` is written `{} @handle <socket-type> \"Name\"`, such as `{} @geometry geometry`", a.name, a.name, a.name),
                );
            }
        }
        let rest: Vec<&Property> = props
            .into_iter()
            .filter(|p| p.name != "name" && p.name != "socket-type")
            .collect();
        if let Some(n) = a.node {
            let kind = s.kind.clone();
            self.attributes(file, n, &kind, &rest, &[]);
        }
    }

    /// A zone's `output { … }`: its output node's inputs, each a value or a link from inside.
    fn zone_output(
        &mut self,
        file: FileId,
        a: &Attr<'_>,
        tree: &Tree,
        output: &NodeDef,
        items: &[String],
        handle: &str,
    ) {
        let Some(block) = a.node.filter(|n| n.block().is_some()) else {
            return self.out(file).error(
                codes::TYPE,
                a.span,
                "`output` takes a block of the zone's output node's inputs",
            );
        };
        let inputs: Vec<&Property> = output
            .inputs
            .iter()
            .filter(|p| p.name != "extend")
            .collect();
        let mut names: Vec<&str> = inputs.iter().map(|p| p.name.as_str()).collect();
        names.extend(items.iter().map(String::as_str));
        for b in attrs(block) {
            if let Some(p) = inputs.iter().find(|p| p.name == b.name) {
                self.socket_value(file, &b, p, tree, Some(handle));
            } else if items.contains(&b.name) {
                if let [v] = b.args.as_slice()
                    && v.as_word().is_some_and(|w| w.starts_with('@'))
                {
                    self.link(file, v, tree, Some(handle));
                }
            } else {
                self.unknown(file, &b, "a zone's output", &names, None);
            }
        }
    }

    /// `layout { @h [x y]; … }`: where the editor draws each node; a zone's line holds two
    /// positions, its input node's and its output node's.
    pub(super) fn layout(&mut self, file: FileId, a: &Attr<'_>, tree: &Tree) {
        let Some(block) = a.node.filter(|n| n.block().is_some()) else {
            return self.out(file).error(
                codes::TYPE,
                a.span,
                "`layout` takes a block of `@node [x y]` lines",
            );
        };
        for child in block.children() {
            let Some(name) = child.name_token() else {
                continue;
            };
            let handle = name.text;
            let mut out = self.out(file);
            if !handle.starts_with('@') {
                out.error(
                    codes::TYPE,
                    name.span,
                    format!("a layout line starts with a node's handle, not `{handle}`"),
                );
                continue;
            }
            let zone = tree.entries.get(handle).is_some_and(|e| e.zone);
            if !tree.entries.contains_key(handle) {
                out.error(
                    codes::HANDLE,
                    name.span,
                    format!("no node of this tree is called `{handle}`"),
                );
            }
            let args = args_of(child);
            let want = if zone { 2 } else { 1 };
            let points = args
                .iter()
                .take_while(|v| matches!(v, Value::List(_)))
                .count();
            if points == want && args.len() == want {
                for v in &args {
                    if v.items().len() != 2 {
                        out.error(codes::TYPE, v.span(), "a position is `[x y]`");
                        continue;
                    }
                    for item in v.items() {
                        check_float(
                            &item,
                            "layout",
                            null_value::Dimension::None,
                            &Property::any(),
                            &mut out,
                        );
                    }
                }
            } else if args.len() > want && points >= want {
                out.error(
                    codes::TYPE,
                    args[want].span(),
                    "one node per layout line: end each with a line break or `;`",
                );
            } else if zone {
                out.error(
                    codes::TYPE,
                    child.code_span(),
                    "a zone's layout line is `@zone [x y] [x y]`: its input node's position, then its output node's",
                );
            } else {
                out.error(
                    codes::TYPE,
                    child.code_span(),
                    "a layout line is `@node [x y]`",
                );
            }
        }
    }

    /// A node group's `input`, `output` or `panel` line: `input @h <socket-type> "Name" { … }`,
    /// the socket type being an interface socket's kind (`float`, `float-distance`,
    /// `geometry`), or `panel @h "Name" { default-closed; <inputs and outputs> }`.
    pub(super) fn interface_line(&mut self, file: FileId, a: &Attr<'_>) {
        let schema = self.schema;
        if a.name == "panel" {
            match a.args.as_slice() {
                [h, rest @ ..] if h.as_word().is_some_and(|w| w.starts_with('@')) => {
                    kebab_handle(h.text(), h.span(), &mut self.out(file));
                    for (i, v) in rest.iter().enumerate() {
                        if i == 0 && matches!(v, Value::Str(_) | Value::Raw(_)) {
                            continue;
                        }
                        self.out(file).error(
                            codes::TYPE,
                            v.span(),
                            "a panel's line is `panel @handle \"Name\"`",
                        );
                    }
                }
                _ => {
                    return self.out(file).error(
                        codes::TYPE,
                        a.span,
                        "a panel is written `panel @handle \"Name\" { … }`",
                    );
                }
            }
            let Some(block) = a.node else { return };
            let props: Vec<&Property> = schema
                .properties_of("NodeTreeInterfacePanel")
                .into_iter()
                .filter(|p| !INTERFACE_LINE.contains(&p.name.as_str()))
                .collect();
            let names: Vec<&str> = props
                .iter()
                .map(|p| p.name.as_str())
                .chain(["input", "output", "panel"])
                .collect();
            for b in attrs(block) {
                match b.name.as_str() {
                    "input" | "output" | "panel" => self.interface_line(file, &b),
                    _ => match props.iter().find(|p| p.name == b.name) {
                        Some(p) => self.property(file, &b, p, false),
                        None => self.unknown(file, &b, "a panel", &names, None),
                    },
                }
            }
            return;
        }
        let kinds: Vec<&str> = schema
            .structs
            .iter()
            .filter(|s| s.base.as_deref() == Some("NodeTreeInterfaceSocket"))
            .map(|s| s.kind.as_str())
            .collect();
        let [h, t, rest @ ..] = a.args.as_slice() else {
            return self.out(file).error(
                codes::TYPE,
                a.span,
                format!("`{}` is written `{} @handle <socket-type> \"Name\"`, such as `{} @count int \"Count\"`", a.name, a.name, a.name),
            );
        };
        if !h.as_word().is_some_and(|w| w.starts_with('@')) {
            return self.out(file).error(
                codes::TYPE,
                h.span(),
                format!("an {}'s handle comes first, such as `@count`", a.name),
            );
        }
        kebab_handle(h.text(), h.span(), &mut self.out(file));
        for (i, v) in rest.iter().enumerate() {
            if i == 0 && matches!(v, Value::Str(_) | Value::Raw(_)) {
                continue;
            }
            self.out(file).error(
                codes::TYPE,
                v.span(),
                format!(
                    "an {}'s line is `{} @handle <socket-type> \"Name\"`",
                    a.name, a.name
                ),
            );
        }
        let ty = t.text();
        let Some(s) = schema
            .structs
            .iter()
            .find(|s| s.kind == ty && s.base.as_deref() == Some("NodeTreeInterfaceSocket"))
        else {
            let near = closest(ty, kinds.iter().copied());
            return self.out(file).error_fix(
                codes::KIND,
                t.span(),
                format!(
                    "`{ty}` is not a socket type: it is one of {}",
                    list_near(ty, &kinds)
                ),
                near.map(|n| (format!("write `{n}`"), t.span(), n.to_string())),
            );
        };
        let rna = s.rna.clone();
        let kind = s.kind.clone();
        let props: Vec<&Property> = schema
            .properties_of(&rna)
            .into_iter()
            .filter(|p| !INTERFACE_LINE.contains(&p.name.as_str()))
            .collect();
        if let Some(n) = a.node {
            self.attributes(file, n, &format!("{kind} socket"), &props, &[]);
        }
    }
}

/// An enum property checked strictly, as a zone item's `socket-type` is written: its listed
/// items only.
fn strict(p: &Property) -> Property {
    Property {
        items_at_run_time: false,
        ..p.clone()
    }
}

/// How a tree type is named in a message.
fn tree_word(rna: &str) -> &str {
    match rna {
        "ShaderNodeTree" => "shader",
        "GeometryNodeTree" => "geometry",
        "CompositorNodeTree" => "compositor",
        _ => "node",
    }
}
