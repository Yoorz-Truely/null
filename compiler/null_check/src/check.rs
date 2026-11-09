//! The walk over a project's files: definitions first, then every node against the schema.

use std::collections::{HashMap, HashSet};

use components::{Component, Placed, Scope};

use null_syntax::ast::{Entry, Node, Value};
use null_syntax::names::{is_kebab_case, to_kebab_case};
use null_syntax::{Diagnostic, FileId, Parse, Span, SyntaxToken, TextEdit, closest};

use crate::codes;
use crate::forms;
use crate::schema::{Property, Schema};
use crate::units::{self, Outside, Units};
use crate::values::{
    Out, check_bool, check_bool_array, check_display_name, check_enum, check_float,
    check_float_array, check_float_matrix, check_int, check_string, dimension, list, list_near,
};

mod anim;
pub(crate) mod components;
pub(crate) mod expr;
mod level0;
mod rig;
mod subs;
mod trees;

/// Checks every file of a project together, since handles are visible project-wide
/// (spec, section 7), and returns the problems in each file's text order. Syntax and
/// name problems are `null_syntax::diagnostics`'s, not repeated here.
pub fn check(schema: &Schema, files: &[(FileId, &Parse)]) -> Vec<Diagnostic> {
    run(schema, files, false)
}

/// [`check`] on expanded files ([`crate::expand`]): a copy's handles (`@plank.7`,
/// `@plank.7/@plank-mesh`) are the handles of the datablocks it made, defined and named as any
/// other, and there are no components or placements left to reach them through.
pub(crate) fn check_flat(schema: &Schema, files: &[(FileId, &Parse)]) -> Vec<Diagnostic> {
    run(schema, files, true)
}

fn run(schema: &Schema, files: &[(FileId, &Parse)], expanded: bool) -> Vec<Diagnostic> {
    let mut cx = Checker {
        schema,
        defs: HashMap::new(),
        out: Vec::new(),
        scene: None,
        scale: 1.0,
        project: Units::default(),
        outside: Outside::Scale(1.0),
        disagreement: None,
        groups: HashMap::new(),
        actions: HashMap::new(),
        refs: Vec::new(),
        components: HashMap::new(),
        placements: HashMap::new(),
        cycles: HashSet::new(),
        scope: None,
        expanded,
        owner: None,
        outside_place: crate::driver::Place::default(),
        scene_places: HashMap::new(),
        scene_place: None,
    };
    let parses = files.iter().map(|(_, p)| *p);
    cx.project = units::project(parses.clone()).unwrap_or_default();
    cx.outside = units::outside(parses);
    cx.scale = cx.outside.scale();
    cx.disagreement = disagreement(&cx.outside);
    let all: Vec<&Parse> = files.iter().map(|(_, p)| *p).collect();
    cx.outside_place = crate::driver::place(&all, None);
    for (file, parse) in files {
        for node in parse.document().nodes().filter(|n| n.name() == "scene") {
            cx.scene_places.insert(
                (file.0, node.span().start),
                crate::driver::place(&all, Some(node)),
            );
        }
    }
    for (file, parse) in files {
        for node in parse.document().nodes() {
            cx.collect(*file, node, true);
            // Node groups' interfaces and actions' slots, which other files name.
            let kind = node.name();
            if kind == "action" {
                anim::slots(node, &mut cx.actions);
            } else if cx.tree_of(&kind).is_some_and(|(_, group)| group) {
                trees::interfaces(node, &mut cx.groups);
            } else if kind == "component" {
                // Components and placements, which other files reach (task 1d-03b).
                cx.collect_component(node);
            } else if kind == "scene" || kind == "collection" {
                cx.collect_placements(node);
            }
        }
    }
    cx.find_cycles();
    for (file, parse) in files {
        for node in parse.document().nodes() {
            if node.name() != "array" {
                cx.top(*file, node);
            }
        }
    }
    // Level 0 data files, each against the line that names it (section 9, task 1d-06b).
    for (file, parse) in files {
        for node in parse.document().nodes() {
            if node.name() == "array" {
                cx.data_file(*file, node);
            }
        }
    }
    cx.unmatched_refs();
    let mut out = cx.out;
    // Sub-object handles are unique within their owner, whatever their kinds (section 7).
    out.extend(crate::paths::Index::build(files).duplicates());
    out.sort_by_key(|d| (d.primary.file.0, d.primary.span.start, d.primary.span.end));
    out
}

/// One attribute of a node: a `key=value` on its line, or a child node.
struct Attr<'a> {
    name: String,
    name_span: Span,
    span: Span,
    args: Vec<Value<'a>>,
    props: Vec<(SyntaxToken<'a>, Value<'a>)>,
    node: Option<Node<'a>>,
}

/// A node's line: its handle, display name and any other arguments.
struct Header<'a> {
    handle: Option<(String, Span)>,
    name: Option<String>,
    /// The display name as written: a string, or the list of its bytes (S65).
    name_value: Option<Value<'a>>,
    rest: Vec<Value<'a>>,
}

struct Checker<'s> {
    schema: &'s Schema,
    /// (kind, handle) to where it is defined.
    defs: HashMap<(String, String), (FileId, Span)>,
    out: Vec<Diagnostic>,
    /// The scene whose block is being checked, with its frame rate (task 1c-01c).
    scene: Option<null_value::Scene>,
    /// The unit scale values are read at: the project's outside a scene, the scene's inside
    /// one (task 1c-01d).
    scale: f32,
    /// The project's `units`, which a scene's own are read over.
    project: Units,
    /// The unit scale outside any scene: the project's, or the one the scenes agree on.
    outside: Outside,
    /// Where the scenes disagree and the project sets no `units`: the message for the first
    /// value written with a scaled unit outside a scene, until it is reported.
    disagreement: Option<String>,
    /// Every node group's interface, by handle (task 1d-07a).
    groups: HashMap<String, trees::Interface>,
    /// Every action's slots, by handle (task 1d-07a).
    actions: HashMap<String, anim::Slots>,
    /// The lines that name a level 0 data file, until the file is read (task 1d-06b).
    refs: Vec<level0::ArrayRef>,
    /// Every component, by handle (task 1d-03b).
    components: HashMap<String, Component>,
    /// Every placement in a scene or a collection, by its handle (task 1d-03b).
    placements: HashMap<String, Placed>,
    /// (a, b) for components a and b that place each other, directly or through others.
    cycles: HashSet<(String, String)>,
    /// Inside a component or a placement's block: what names mean (task 1d-03b).
    scope: Option<Scope>,
    /// The files are expanded (task 1d-03c): copy handles are plain handles.
    expanded: bool,
    /// The datablock whose block is being checked, and its own sub-objects, (kind, handle):
    /// inside it a handle names one of them first (section 7; task 1b-01q).
    owner: Option<Own>,
    /// Where drivers are read (task 1d-07f): outside every scene, and inside each scene node, by
    /// its file and place, as `driver::place` gives them to the checker and to Sau alike.
    outside_place: crate::driver::Place,
    scene_places: HashMap<(u32, usize), crate::driver::Place>,
    /// The place of the scene whose block is being checked.
    scene_place: Option<crate::driver::Place>,
}

/// A datablock being checked, with its own sub-objects.
#[derive(Clone, Debug, Default)]
struct Own {
    kind: String,
    handle: String,
    subs: Vec<(String, String)>,
}

fn args_of(node: Node<'_>) -> Vec<Value<'_>> {
    node.entries()
        .flat_map(|e| match e {
            Entry::Arg(v) => vec![v],
            // Scowr's keyword attributes (`to @x`) are two arguments in Null.
            Entry::Keyword { key, value } => vec![Value::Word(key), value],
            Entry::Prop { .. } => vec![],
        })
        .collect()
}

fn props_of(node: Node<'_>) -> Vec<(SyntaxToken<'_>, Value<'_>)> {
    node.entries()
        .filter_map(|e| match e {
            Entry::Prop { key, value } => Some((key, value)),
            _ => None,
        })
        .collect()
}

fn header(node: Node<'_>) -> Header<'_> {
    let mut handle = None;
    let mut name = None;
    let mut name_value = None;
    let mut rest = Vec::new();
    for (i, v) in args_of(node).into_iter().enumerate() {
        match v {
            Value::Word(t) if i == 0 && t.text.starts_with('@') => {
                handle = Some((t.text.to_string(), t.span))
            }
            Value::Str(_) | Value::Raw(_) if name_value.is_none() => {
                name = v.as_string().map(|s| s.into_owned());
                name_value = Some(v);
            }
            // A name that is not UTF-8, as the list of its bytes (S65, task 1c-02g).
            Value::List(_)
                if name_value.is_none() && handle.is_some() && i == 1 && is_byte_list(&v) =>
            {
                name_value = Some(v);
            }
            _ => rest.push(v),
        }
    }
    Header {
        handle,
        name,
        name_value,
        rest,
    }
}

/// Whether `v` is a list of whole numbers, a byte list (S65).
fn is_byte_list(v: &Value<'_>) -> bool {
    let items = v.items();
    !items.is_empty()
        && items.iter().all(|i| {
            i.as_word()
                .is_some_and(|w| w.bytes().all(|b| b.is_ascii_digit()))
        })
}

/// The span that takes attribute `a`'s line out of its file: a line of its own with its
/// indentation and its line feed, or one of a `{ a; b }` line as far as its `;`. None for a
/// `key=value` on its owner's line.
fn removal(a: &Attr<'_>) -> Option<Span> {
    let node = a.node?;
    let text = node.0.file_text();
    let span = node.span();
    let before = &text[..span.start];
    let indent_start = before.trim_end_matches([' ', '\t']).len();
    let own_line = (indent_start == 0 || before[..indent_start].ends_with('\n'))
        && text[..span.end].ends_with('\n');
    if own_line {
        return Some(Span::new(indent_start, span.end));
    }
    Some(span)
}

/// The attributes of `node`: its line's properties, then its child nodes.
fn attrs(node: Node<'_>) -> Vec<Attr<'_>> {
    let mut out: Vec<Attr<'_>> = props_of(node)
        .into_iter()
        .map(|(key, value)| Attr {
            name: key.text.to_string(),
            name_span: key.span,
            span: key.span.cover(value.span()),
            args: vec![value],
            props: Vec::new(),
            node: None,
        })
        .collect();
    for child in node.children() {
        let Some(t) = child.name_token() else {
            continue;
        };
        out.push(Attr {
            name: child.name().into_owned(),
            name_span: t.span,
            span: child.code_span(),
            args: args_of(child),
            props: props_of(child),
            node: Some(child),
        });
    }
    out
}

impl<'s> Checker<'s> {
    fn out(&mut self, file: FileId) -> Out<'_> {
        Out {
            file,
            scope: self.scope.as_ref(),
            list: &mut self.out,
            scene: self.scene,
            scale: self.scale,
            disagreement: &mut self.disagreement,
        }
    }

    fn is_datablock(&self, kind: &str) -> bool {
        self.schema.datablock_kinds().any(|s| s.kind == kind)
    }

    /// The handle a datablock node defines: written, or made from its display name (S18).
    fn handle_of(&self, node: Node<'_>) -> Option<(String, Span)> {
        let h = header(node);
        h.handle.or_else(|| {
            let name = h.name?;
            let span = node.name_token()?.span;
            Some((format!("@{}", to_kebab_case(&name)?), span))
        })
    }

    fn collect(&mut self, file: FileId, node: Node<'_>, top: bool) {
        let kind = node.name().into_owned();
        let defining =
            (top && (self.is_datablock(&kind) || kind == forms::PROJECT)) || kind == "collection";
        if defining && let Some((handle, span)) = self.handle_of(node) {
            let key = (kind.clone(), handle.clone());
            if let Some(&(f0, s0)) = self.defs.get(&key) {
                let d = Diagnostic::error(
                    codes::HANDLE,
                    file,
                    span,
                    format!(
                        "`{handle}` is defined twice as {}; a handle is unique within its kind",
                        article(&kind)
                    ),
                )
                .with_label(f0, s0, "first defined here");
                self.out.push(d);
            } else {
                self.defs.insert(key, (file, span));
            }
        }
        // Collections defined inside a scene or collection.
        if kind == "scene" || kind == "collection" {
            for child in node.children() {
                if child.name() == "collection" && defines(child) {
                    self.collect(file, child, false);
                }
            }
        }
    }

    fn top(&mut self, file: FileId, node: Node<'_>) {
        let Some(name) = node.name_token() else {
            return;
        };
        let kind = node.name().into_owned();
        if kind == forms::PROJECT {
            return self.project(file, node);
        }
        if kind == "component" {
            return self.component(file, node);
        }
        let Some(s) = self.schema.datablock_kinds().find(|s| s.kind == kind) else {
            let kinds: Vec<&str> = [forms::PROJECT, "component"]
                .into_iter()
                .chain(self.schema.datablock_kinds().map(|s| s.kind.as_str()))
                .collect();
            let suggestion = closest(&kind, kinds.iter().copied());
            self.out(file).error_fix(
                codes::KIND,
                name.span,
                format!(
                    "`{kind}` is not a kind of datablock the schema knows: {}",
                    list_near(&kind, &kinds)
                ),
                suggestion.map(|k| (format!("write `{k}`"), name.span, k.to_string())),
            );
            return;
        };
        let rna = s.rna.clone();
        self.header_check(file, node, &kind, Some(&rna));
        // Inside its block, its own sub-objects come first (section 7, task 1b-01q).
        let h = header(node);
        let handle = h.handle.map(|(h, _)| h).or_else(|| {
            h.name
                .as_deref()
                .and_then(to_kebab_case)
                .map(|k| format!("@{k}"))
        });
        self.owner = handle.map(|handle| Own {
            kind: kind.clone(),
            handle,
            subs: crate::paths::own_sub_objects(node),
        });
        if kind == "scene" {
            let mut rate = frame_rate(node);
            self.scale = Units::of(node)
                .unwrap_or_default()
                .over(&self.project)
                .scale();
            rate.scale_length = self.scale;
            self.scene = Some(rate);
            self.scene_place = self.scene_places.get(&(file.0, node.span().start)).cloned();
        }
        match kind.as_str() {
            "light" => self.light(file, node),
            _ => self.body(file, node, &kind, &rna),
        }
        self.owner = None;
        self.scene = None;
        self.scene_place = None;
        self.scale = self.outside.scale();
    }

    /// The handle and display name on a datablock's line, and nothing else. The display name is
    /// struct `rna`'s `name`, so it must fit Blender's field for it (task 1b-03h).
    fn header_check(&mut self, file: FileId, node: Node<'_>, kind: &str, rna: Option<&str>) {
        let h = header(node);
        let name_property = rna.and_then(|rna| {
            self.schema
                .properties_of(rna)
                .into_iter()
                .find(|p| p.name == "name" && p.ty == "string")
                .cloned()
        });
        let expanded = self.expanded;
        let mut out = self.out(file);
        if let Some(v) = &h.name_value {
            check_display_name(v, name_property.as_ref(), &mut out);
        }
        match &h.handle {
            Some((handle, span)) => {
                if !(expanded && is_copy_handle(handle)) {
                    kebab_handle(handle, *span, &mut out);
                }
            }
            None if h.name.as_deref().and_then(to_kebab_case).is_none() => {
                let span = node.name_token().map_or(node.span(), |t| t.span);
                out.error(
                    codes::HANDLE,
                    span,
                    format!("this {kind} needs a handle: write `{kind} @name \"Name\"`"),
                );
            }
            None => {}
        }
        for v in h.rest {
            out.error(
                codes::TYPE,
                v.span(),
                format!(
                    "{}'s line holds its handle and name only; `{}` belongs on a line of its own",
                    article(kind),
                    v.text()
                ),
            );
        }
    }

    fn project(&mut self, file: FileId, node: Node<'_>) {
        let names = ["units", "scene"];
        let mut seen = HashSet::new();
        for a in attrs(node) {
            if a.name == "units" && !seen.insert(a.name.clone()) {
                self.out(file).error(
                    codes::REPEATED,
                    a.name_span,
                    "`units` is given twice; a project has one",
                );
                continue;
            }
            match a.name.as_str() {
                "units" => self.units(file, &a),
                "scene" => self.reference(file, &a, "scene"),
                _ => self.unknown(file, &a, "a project", &names, None),
            }
        }
    }

    /// The body of a datablock of `kind`, struct `rna`: RNA's properties and Null's forms.
    fn body(&mut self, file: FileId, node: Node<'_>, kind: &str, rna: &str) {
        let props = self.schema.properties_of(rna);
        if kind == "action" {
            return self.action_body(file, node, &props);
        }
        self.attributes(file, node, kind, &props, &[]);
    }

    fn light(&mut self, file: FileId, node: Node<'_>) {
        let all = attrs(node);
        let ty = all
            .iter()
            .find(|a| a.name == "type")
            .and_then(|a| a.args.first())
            .and_then(|v| v.as_word())
            .unwrap_or("point")
            .to_string();
        let sub = format!("{ty}-light");
        let Some(sub_rna) = self.schema.struct_by_kind(&sub).map(|s| s.rna.clone()) else {
            // An unknown type is reported by the `type` enum itself.
            let props = self.schema.properties_of("Light");
            return self.attributes(file, node, "light", &props, &[]);
        };
        let props = self.schema.properties_of(&sub_rna);
        // Properties of the other kinds of light, to say whose they are.
        let others: Vec<(String, String)> = self
            .schema
            .structs
            .iter()
            .filter(|s| s.base.as_deref() == Some("Light") && s.kind != sub)
            .flat_map(|s| {
                s.properties
                    .iter()
                    .map(|p| (p.name.clone(), s.kind.clone()))
            })
            .collect();
        let power = forms::POWER_LIGHTS.contains(&sub_rna.as_str());
        let power_names: &[&str] = if power { &["energy"] } else { &[] };
        self.attributes_with(file, node, "light", &props, power_names, &others, &ty);
    }

    fn attributes(
        &mut self,
        file: FileId,
        node: Node<'_>,
        kind: &str,
        props: &[&Property],
        power: &[&str],
    ) {
        self.attributes_with(file, node, kind, props, power, &[], "");
    }

    #[allow(clippy::too_many_arguments)]
    fn attributes_with(
        &mut self,
        file: FileId,
        node: Node<'_>,
        kind: &str,
        props: &[&Property],
        power: &[&str],
        others: &[(String, String)],
        light_type: &str,
    ) {
        let tree_kind = self.tree_of(kind);
        let mut forms: Vec<&str> = match kind {
            "scene" => vec![
                "object",
                "collection",
                "units",
                "fps",
                "fps-base",
                "use",
                "override",
                "marker",
            ],
            "collection" => vec!["object", "collection", "use", "override"],
            "object" => vec!["data", "material", "modifier", "constraint", "bone"],
            "armature" => vec!["bone", "bone-collection"],
            "mesh" => vec![
                "primitive",
                "vertices",
                "edges",
                "polygons",
                "attribute",
                "uv-map",
                "vertex-group",
                "shape-key",
                "shape-keys",
            ],
            // A curve's, a surface's and a lattice's shape keys too (RFC 0005, S88).
            "curve" | "surface" | "lattice" => vec!["shape-key", "shape-keys"],
            "pose-bone" => vec!["constraint"],
            _ => vec![],
        };
        if let Some((_, group)) = tree_kind {
            forms.extend(["nodes", "layout"]);
            if group {
                forms.extend(["input", "output", "panel"]);
            }
        }
        // Animation on any datablock that has it (section 11).
        let animated = props.iter().any(|p| p.name == "animation-data");
        if animated {
            forms.extend(["action", "track"]);
        }
        let mut names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
        names.extend(forms.iter().copied());
        // `shape-keys` is the read-only pointer of every curve kind; where the block form is not
        // taken (a text curve), it is no name to suggest for `shape-key` (1d-07q's review).
        if !forms.contains(&"shape-keys") {
            names.retain(|n| *n != "shape-keys");
        }
        let mut seen: HashSet<String> = HashSet::new();
        let mut vertex_count = None;
        let all = attrs(node);
        // Node handles first, so layout and links may name nodes defined later.
        let tree = tree_kind.map(|(rna, group)| {
            let interface = group
                .then(|| {
                    header(node)
                        .handle
                        .and_then(|(h, _)| self.groups.get(&h).cloned())
                })
                .flatten();
            trees::Tree::new(
                rna,
                all.iter().find(|a| a.name == "nodes").and_then(|a| a.node),
                interface,
                self.handle_of(node).map(|(h, _)| h),
            )
        });
        for a in &all {
            let repeatable = matches!(
                (kind, a.name.as_str()),
                (
                    "scene" | "collection",
                    "object" | "collection" | "use" | "override"
                ) | (_, "if" | "else")
                    | ("object", "material" | "modifier" | "constraint" | "bone")
                    | ("pose-bone", "constraint")
                    | (
                        "mesh",
                        "attribute" | "uv-map" | "vertex-group" | "shape-key"
                    )
                    | ("curve" | "surface" | "lattice", "shape-key")
                    | ("armature", "bone" | "bone-collection")
                    | ("scene", "marker")
            ) || (tree.is_some()
                && matches!(a.name.as_str(), "input" | "output" | "panel"))
                || (animated && a.name == "track");
            if !repeatable && !seen.insert(a.name.clone()) {
                self.out(file).error(
                    codes::REPEATED,
                    a.name_span,
                    format!("`{}` is given twice; {} has one", a.name, article(kind)),
                );
                continue;
            }
            match (kind, a.name.as_str()) {
                // Placements and overrides (task 1d-03b).
                ("scene" | "collection", "use") => self.placement(file, a.node.unwrap(), None),
                ("scene" | "collection", "override") => self.override_line(file, a.node.unwrap()),
                // Conditions in a component's datablock: their lines are the datablock's.
                (_, "if" | "else") if self.scope.is_some() && a.node.is_some() => {
                    let n = a.node.unwrap();
                    if a.name == "if" {
                        self.condition(file, n);
                    } else {
                        let at = all.iter().position(|b| b.span == a.span).unwrap_or(0);
                        if at == 0 || all[at - 1].name != "if" {
                            self.out(file).error(
                                codes::TYPE,
                                a.name_span,
                                "`else` follows an `if` directly",
                            );
                        }
                    }
                    self.attributes_with(file, n, kind, props, power, others, light_type);
                }
                ("scene" | "collection", "object") => self.reference(file, a, "object"),
                ("scene" | "collection", "collection") => {
                    if a.node.is_some_and(defines) {
                        let n = a.node.unwrap();
                        self.header_check(file, n, "collection", Some("Collection"));
                        let props = self.schema.properties_of("Collection");
                        self.attributes(file, n, "collection", &props, &[]);
                    } else {
                        self.reference(file, a, "collection")
                    }
                }
                ("scene", "units") => self.units(file, a),
                ("scene", "fps") => self.frame_rate_part(file, a, true),
                ("scene", "fps-base") => self.frame_rate_part(file, a, false),
                ("object", "data") => self.data(file, a),
                ("object", "material") => self.reference(file, a, "material"),
                ("mesh", "primitive") => self.primitive(file, a),
                ("mesh", "vertices") if a.props.iter().any(|(k, _)| k.text == "file") => {}
                ("mesh", "vertices") => vertex_count = self.vertices(file, a),
                // After the loop, against the vertex count and the other arrays (task 1d-06b).
                ("mesh", "polygons" | "edges" | "attribute" | "uv-map" | "vertex-group") => {}
                ("object", "modifier") => self.family_member(file, a, "Modifier"),
                ("object" | "pose-bone", "constraint") => self.family_member(file, a, "Constraint"),
                ("object", "bone") => self.pose_bone(file, a),
                // RFC 0005 (S88, task 1d-07q).
                ("armature", "bone") => self.armature_bone(file, a, &all),
                ("armature", "bone-collection") => self.bone_collection(file, a, &all),
                ("mesh" | "curve" | "surface" | "lattice", "shape-key") => {
                    let owner = header(node).handle.map(|(h, _)| (kind.to_string(), h));
                    self.shape_key(file, a, &all, kind, owner)
                }
                ("mesh" | "curve" | "surface" | "lattice", "shape-keys") => {
                    self.shape_keys(file, a)
                }
                ("scene", "marker") => self.marker(file, a),
                (_, "nodes") if tree.is_some() => self.nodes(file, a, tree.as_ref().unwrap()),
                (_, "layout") if tree.is_some() => self.layout(file, a, tree.as_ref().unwrap()),
                (_, "input" | "output" | "panel") if tree_kind.is_some_and(|(_, g)| g) => {
                    self.interface_line(file, a)
                }
                (_, "raw") => self.raw_block(file, a, kind),
                (_, "action") if animated => self.plays(file, a, kind),
                (_, "track") if animated => self.track(file, a, kind),
                _ => match props.iter().find(|p| p.name == a.name) {
                    Some(p) if a.props.iter().any(|(k, _)| k.text == "driver") => {
                        self.driver(file, a, p)
                    }
                    Some(p) => {
                        let power = power.contains(&a.name.as_str());
                        self.property(file, a, p, power)
                    }
                    None => {
                        let whose = others.iter().find(|(n, _)| *n == a.name).map(|(_, k)| {
                            format!(
                                "{}'s, and this light's `type` is `{light_type}`",
                                article(&k.replace('-', " "))
                            )
                        });
                        self.unknown(file, a, &article(kind), &names, whose)
                    }
                },
            }
        }
        if let Some(a) = all.iter().find(|a| {
            kind == "mesh" && a.name == "polygons" && !a.props.iter().any(|(k, _)| k.text == "file")
        }) {
            self.polygons(file, a, vertex_count);
        }
        if kind == "mesh"
            && let Some((handle, _)) = header(node).handle
        {
            self.level0(file, ("mesh".to_string(), handle), &all);
        }
    }

    /// The DNA struct whose raw fields a datablock of `kind` takes: its RNA struct, or the first
    /// struct up its chain the raw table lists (DNA's name for a datablock's struct is RNA's).
    fn raw_struct(&self, kind: &str) -> String {
        let table = self.schema.raw.as_ref();
        let mut at = self.schema.struct_by_kind(kind);
        let first = at.map(|s| s.rna.clone()).unwrap_or_default();
        while let Some(s) = at {
            if table.is_some_and(|t| t.structs.contains_key(&s.rna)) {
                return s.rna.clone();
            }
            at = s.base.as_deref().and_then(|b| self.schema.struct_by_rna(b));
        }
        first
    }

    /// A `raw` block (section 19, Sau's task 1d-05b): DNA fields RNA does not reach, each a flat
    /// path of DNA's own names with a value of its DNA type, as the raw table lists them. Without
    /// a raw table its lines are not checked against DNA.
    fn raw_block(&mut self, file: FileId, a: &Attr<'_>, kind: &str) {
        let Some(block) = a.node.filter(|n| n.block().is_some()) else {
            return self.out(file).error(
                codes::TYPE,
                a.span,
                "`raw` is a block of DNA fields, such as `raw { transflag 4 }`".to_string(),
            );
        };
        if !a.args.is_empty() || !a.props.is_empty() {
            self.out(file).error(
                codes::TYPE,
                a.span,
                "`raw` takes no value of its own, only its block of DNA fields".to_string(),
            );
        }
        let Some(table) = self.schema.raw.clone() else {
            return;
        };
        let owner = self.raw_struct(kind);
        let fields = table.fields(&owner);
        let mut seen = HashSet::new();
        for line in attrs(block) {
            if line.node.is_some_and(|n| n.block().is_some()) {
                self.out(file).error(
                    codes::TYPE,
                    line.span,
                    format!(
                        "a raw block's lines are flat DNA paths (`{}.flag 1`), not blocks",
                        line.name
                    ),
                );
                continue;
            }
            let resolved = table.resolve(&owner, &line.name);
            let path = resolved.map_or(line.name.clone(), |(f, _)| f.path.clone());
            if !seen.insert(path.clone()) {
                self.out(file).error(
                    codes::REPEATED,
                    line.name_span,
                    format!("`{}` is given twice in this raw block", line.name),
                );
                continue;
            }
            // A v0 path without an embedded struct array's index reads as element 0 (S94).
            if let Some((_, true)) = resolved {
                self.out(file).warning_fix(
                    codes::UNKNOWN_PROPERTY,
                    line.name_span,
                    format!(
                        "`{}` leaves out the index of the element it names, and reads as element 0: write `{path}`",
                        line.name
                    ),
                    (format!("write `{path}`"), line.name_span, path.clone()),
                );
            }
            let Some((field, _)) = resolved else {
                let paths = fields.iter().map(|f| f.path.as_str());
                let suggestion = closest(&line.name, paths);
                let message = match (fields.is_empty(), suggestion) {
                    (true, _) => format!(
                        "`{owner}` has no raw fields in this build's raw table, so no `{}`",
                        line.name
                    ),
                    (false, Some(s)) => format!(
                        "`{owner}` has no raw field `{}`; did you mean `{s}`?",
                        line.name
                    ),
                    (false, None) => format!("`{owner}` has no raw field `{}`", line.name),
                };
                self.out(file).error_fix(
                    codes::UNKNOWN_PROPERTY,
                    line.name_span,
                    message,
                    suggestion.map(|s| (format!("write `{s}`"), line.name_span, s.to_string())),
                );
                continue;
            };
            match (line.args.as_slice(), line.props.is_empty()) {
                ([v], true) => {
                    if let Err(e) = crate::raw::read_value(v, field) {
                        let at = e.fix.as_ref().map_or(v.span(), |(s, _)| *s);
                        let fix = e.fix.map(|(s, w)| (format!("write `{w}`"), s, w));
                        self.out(file).error_fix(codes::TYPE, at, e.message, fix);
                    }
                }
                _ => self.out(file).error(
                    codes::TYPE,
                    line.span,
                    format!("`{}` takes one value, {}", line.name, field.describe()),
                ),
            }
        }
    }

    /// An unknown attribute: the nearest name as the fix, or Null's own name for it.
    fn unknown(
        &mut self,
        file: FileId,
        a: &Attr<'_>,
        owner: &str,
        names: &[&str],
        whose: Option<String>,
    ) {
        if let Some((_, null, why)) = forms::ALIASES
            .iter()
            .find(|(w, n, _)| *w == a.name && names.contains(n))
        {
            return self.out(file).error_fix(
                codes::UNKNOWN_PROPERTY,
                a.name_span,
                format!("{owner} has no `{}`: write `{null}`, as {why}", a.name),
                Some((format!("write `{null}`"), a.name_span, null.to_string())),
            );
        }
        if let Some(whose) = whose {
            return self.out(file).error(
                codes::UNKNOWN_PROPERTY,
                a.name_span,
                format!("`{}` is {whose}", a.name),
            );
        }
        let kebab = to_kebab_case(&a.name);
        // A known property written in another case (`pass_index`): the syntax's `null-name`
        // already says so, with the same fix, so one mistake gives one error (1d-02c's review).
        if kebab
            .as_deref()
            .is_some_and(|k| k != a.name && names.contains(&k))
        {
            return;
        }
        let suggestion = kebab
            .as_deref()
            .filter(|k| names.contains(k))
            .or_else(|| closest(&a.name, names.iter().copied()));
        let message = match suggestion {
            Some(s) => format!("{owner} has no `{}`; did you mean `{s}`?", a.name),
            None => format!("{owner} has no `{}`", a.name),
        };
        self.out(file).error_fix(
            codes::UNKNOWN_PROPERTY,
            a.name_span,
            message,
            suggestion.map(|s| (format!("write `{s}`"), a.name_span, s.to_string())),
        );
    }

    /// A property a flat form of Null's sets, written in a block of its struct instead (S70):
    /// an error, with a fix that moves the line to the flat form's place, before the block's own
    /// line (`render { fps 30 }` in a scene is `fps 30` at the scene).
    fn flat_in_block(&mut self, file: FileId, a: &Attr<'_>, block: Node<'_>, target: &str) {
        for child in block.children() {
            let name = child.name();
            let Some((_, _, flat)) = forms::FLAT_FORMS
                .iter()
                .find(|(t, inner, _)| *t == target && *inner == name)
            else {
                continue;
            };
            let code = child.code_span();
            let base = child.span().start;
            let text = &child.0.text()[code.start - base..code.end - base];
            let moved = text.replacen(name.as_ref(), flat, 1);
            let line_start = a.span.start;
            let d = Diagnostic::error(
                codes::REPEATED,
                file,
                code,
                format!(
                    "`{name}` is written at the {} as `{flat}` (S70: one spelling per property), not in `{}`",
                    "scene", a.name
                ),
            )
            .with_fix(
                format!("move it out of `{}`", a.name),
                vec![
                    TextEdit {
                        file,
                        span: child.span(),
                        replacement: String::new(),
                    },
                    TextEdit {
                        file,
                        span: Span::new(line_start, line_start),
                        replacement: format!("{moved}\n  "),
                    },
                ],
            );
            self.out.push(d);
        }
    }

    /// One RNA property's value.
    fn property(&mut self, file: FileId, a: &Attr<'_>, p: &Property, power: bool) {
        // A struct the owner holds, not a datablock (`cycles` on a scene, task 1b-03g), is written
        // as a block of its own properties, checked as that struct's.
        if p.ty == "pointer"
            && let Some(block) = a.node.filter(|n| n.block().is_some())
            && let Some(target) = p.target.as_deref()
            && !self.schema.is_datablock_struct(target)
            && let Some(s) = self.schema.struct_by_rna(target)
        {
            // One spelling per property (S70): the block form is for a struct Null does not
            // flatten. `units` writes a scene's unit settings, so a block of them is an error.
            if target == "UnitSettings" {
                self.out(file).error(
                    codes::TYPE,
                    a.name_span,
                    format!(
                        "a scene's unit settings are written `units …` (section 8), not as a `{}` block",
                        a.name
                    ),
                );
                return;
            }
            self.flat_in_block(file, a, block, target);
            let schema = self.schema;
            let props = schema.properties_of(target);
            self.attributes(file, block, &s.kind.clone(), &props, &[]);
            return;
        }
        let mut out = self.out(file);
        let name = a.name.as_str();
        if p.readonly || p.ty == "collection" {
            out.error_fix(
                codes::READONLY,
                a.name_span,
                format!(
                    "`{name}` is read-only in Blender, so a file cannot set it; remove the line"
                ),
                removal(a).map(|span| (format!("remove `{name}`"), span, String::new())),
            );
            return;
        }
        if let Some((key, _)) = a.props.first() {
            out.error(
                codes::TYPE,
                key.span,
                format!("`{name}` takes no properties"),
            );
            return;
        }
        if a.node.is_some_and(|n| n.block().is_some()) {
            out.error(codes::TYPE, a.span, format!("`{name}` takes no block"));
            return;
        }
        if p.ty == "bool" && p.dimensions().is_empty() && !p.is_dynamic() {
            return check_bool(&a.args, name, a.span, &mut out);
        }
        let [value] = a.args.as_slice() else {
            let message = if a.args.is_empty() {
                format!("`{name}` needs a value")
            } else {
                format!("`{name}` takes one value; write a list as `[ … ]`")
            };
            return out.error(codes::TYPE, a.span, message);
        };
        let dim = dimension(p, power);
        match p.ty.as_str() {
            "float32" | "float64" => match p.dimensions().as_slice() {
                [] => check_float(value, name, dim, p, &mut out),
                [len] => check_float_array(value, *len, name, dim, p, &mut out),
                [rows, columns, ..] => {
                    check_float_matrix(value, *rows, *columns, name, dim, p, &mut out);
                }
            },
            "int" => match p.fixed_len() {
                None if !p.is_dynamic() => {
                    check_int(value, name, p, &mut out);
                }
                _ => {
                    for item in value.items() {
                        check_int(&item, name, p, &mut out);
                    }
                }
            },
            "bool" => check_bool_array(value, name, p, &mut out),
            "enum" => check_enum(value, name, p, &mut out),
            "string" => check_string(value, name, p, &mut out),
            "pointer" => {
                let target = p.target.as_deref().unwrap_or("ID");
                let kind = self
                    .schema
                    .struct_by_rna(target)
                    .map(|s| s.kind.clone())
                    .unwrap_or_default();
                // `none` empties a pointer (section 6), as write-back writes one (`parent none`),
                // where Blender lets it be empty (Sau's task 1c-02j).
                if value.as_word() == Some("none") {
                    if p.never_none {
                        self.out(file).error(
                            codes::TYPE,
                            value.span(),
                            format!(
                                "`{name}` is never empty in Blender, so it takes a handle of a {kind}, not `none`"
                            ),
                        );
                    }
                    return;
                }
                self.handle_ref(file, value, &kind, name);
            }
            other => out.error(
                codes::TYPE,
                a.name_span,
                format!("`{name}` is of a type Null does not write yet ({other})"),
            ),
        }
    }

    /// A scene's `fps` (a whole number, 1 to 32767) or `fps-base` (a number from 0.00001 to
    /// 1000000): its render settings' frame rate, written on the scene (spec, section 8).
    fn frame_rate_part(&mut self, file: FileId, a: &Attr<'_>, fps: bool) {
        let mut out = self.out(file);
        let [value] = a.args.as_slice() else {
            return out.error(
                codes::TYPE,
                a.span,
                format!("`{}` takes one number", a.name),
            );
        };
        let (lo, hi) = if fps {
            forms::FPS_RANGE
        } else {
            forms::FPS_BASE_RANGE
        };
        let p = Property::ranged(if fps { "int" } else { "float32" }, lo, hi);
        if fps {
            check_int(value, &a.name, &p, &mut out);
        } else {
            check_float(value, &a.name, null_value::Dimension::None, &p, &mut out);
        }
    }

    /// `object @h`, `scene @h`, `material @h`: one handle of `kind`.
    fn reference(&mut self, file: FileId, a: &Attr<'_>, kind: &str) {
        match a.args.as_slice() {
            [v] if a.props.is_empty() => self.handle_ref(file, v, kind, &a.name),
            _ => self.out(file).error(
                codes::TYPE,
                a.span,
                format!("`{}` takes one handle, such as `{} @name`", a.name, a.name),
            ),
        }
    }

    /// Whether a datablock defined as `defined` is a `wanted`: the same kind, or one whose
    /// struct extends it (a `geometry` node group is a `node-tree`; any datablock is an `id`).
    fn kind_fits(&self, defined: &str, wanted: &str) -> bool {
        if defined == wanted {
            return true;
        }
        let Some(want) = self.schema.struct_by_kind(wanted).map(|s| s.rna.as_str()) else {
            return false;
        };
        let mut at = self
            .schema
            .datablock_kinds()
            .find(|s| s.kind == defined)
            .and_then(|s| s.base.as_deref());
        while let Some(rna) = at {
            if rna == want {
                return true;
            }
            at = self
                .schema
                .struct_by_rna(rna)
                .and_then(|s| s.base.as_deref());
        }
        false
    }

    /// A handle that must name a datablock of `kind`.
    fn handle_ref(&mut self, file: FileId, v: &Value<'_>, kind: &str, what: &str) {
        // Inside a component: a handle parameter by its name, and the component's own handles
        // (task 1d-03b).
        if let Some(scope) = self.scope.clone()
            && let Some(word) = v.as_word()
        {
            if let Some(expr::Kind::Handle(k)) = scope.names.get(word) {
                if !self.kind_fits(k, kind) {
                    self.out(file).error(
                        codes::HANDLE,
                        v.span(),
                        format!("`{word}` is a handle of a {k}, and `{what}` takes a {kind}"),
                    );
                }
                return;
            }
            if scope
                .locals
                .iter()
                .any(|(k, h)| h == word && self.kind_fits(k, kind))
            {
                return;
            }
        }
        // A copy, or a datablock a copy made (`@plank.7`, `@plank.7/@plank-mesh`, task 1d-03b).
        if let Some(word) = v.as_word()
            && !self.expanded
            && (components::is_copy_path(word)
                || (word.contains('/')
                    && self
                        .placements
                        .contains_key(word.split('/').next().unwrap_or_default())))
        {
            match self.copy_path(word) {
                Ok(components::Reached::Copy(_)) if self.kind_fits("object", kind) => {}
                Ok(components::Reached::Datablock(k)) if self.kind_fits(&k, kind) => {}
                Ok(components::Reached::Sub { .. }) => {}
                Ok(components::Reached::Copy(_)) => self.out(file).error(
                    codes::HANDLE,
                    v.span(),
                    format!("`{word}` is a copy, an object, and `{what}` takes a {kind}"),
                ),
                Ok(components::Reached::Datablock(k)) => self.out(file).error(
                    codes::HANDLE,
                    v.span(),
                    format!("`{word}` is a {k}, and `{what}` takes a {kind}"),
                ),
                Err(message) => self.out(file).error(codes::HANDLE, v.span(), message),
            }
            return;
        }
        let Some(word) = v.as_word().filter(|w| w.starts_with('@')) else {
            return self.out(file).error(
                codes::TYPE,
                v.span(),
                format!(
                    "`{what}` takes a handle of a {kind}, such as `@name`, not `{}`",
                    v.text()
                ),
            );
        };
        // Inside its owner's block, a handle names the owner's own sub-object first, so a
        // datablock elsewhere never shadows it (section 7): one of a datablock's kind is a clash.
        if let Some(own) = &self.owner
            && let Some((sub, _)) = own.subs.iter().find(|(_, h)| h == word)
        {
            let message = format!(
                "inside {} `{}`, `{word}` names its {sub} `{word}` (a handle names its owner's own sub-object first, section 7), and `{what}` takes a {kind}; give the {sub} or the {kind} another handle",
                own.kind, own.handle
            );
            return self.out(file).error(codes::HANDLE, v.span(), message);
        }
        if !(self.expanded && is_copy_handle(word))
            && !kebab_handle(word, v.span(), &mut self.out(file))
        {
            // Respelt, it may name one: then the name's own fix is the whole fix.
            let kebab = to_kebab_case(word.trim_start_matches('@')).map(|k| format!("@{k}"));
            if kebab.is_some_and(|k| self.defs.contains_key(&(kind.to_string(), k))) {
                return;
            }
        }
        if self
            .defs
            .keys()
            .any(|(k, h)| h == word && self.kind_fits(k, kind))
        {
            return;
        }
        let other = self
            .defs
            .keys()
            .find(|(_, h)| h == word)
            .map(|(k, _)| k.clone());
        if let Some(other) = other {
            return self.out(file).error(
                codes::HANDLE,
                v.span(),
                format!("`{word}` is a {other}, and `{what}` takes a {kind}"),
            );
        }
        let candidates: Vec<&str> = self
            .defs
            .keys()
            .filter(|(k, _)| self.kind_fits(k, kind))
            .map(|(_, h)| h.as_str())
            .collect();
        let near = closest(word, candidates.iter().copied()).map(str::to_string);
        let message = match &near {
            Some(n) => format!("no {kind} is called `{word}`; did you mean `{n}`?"),
            None => format!("no {kind} is called `{word}`"),
        };
        self.out(file).error_fix(
            codes::HANDLE,
            v.span(),
            message,
            near.map(|n| (format!("write `{n}`"), v.span(), n)),
        );
    }

    /// `data <kind> @h`: an object's data, with its kind written.
    fn data(&mut self, file: FileId, a: &Attr<'_>) {
        match a.args.as_slice() {
            [k, v] if k.as_word().is_some_and(|w| forms::OBJECT_DATA.contains(&w)) => {
                let kind = k.as_word().unwrap().to_string();
                self.handle_ref(file, v, &kind, "data");
            }
            [k, _] => {
                let w = k.text();
                let near = closest(w, forms::OBJECT_DATA.iter().copied());
                self.out(file).error_fix(
                    codes::KIND,
                    k.span(),
                    format!(
                        "`{w}` is not a kind of object data: it is one of {}",
                        list(&forms::OBJECT_DATA)
                    ),
                    near.map(|n| (format!("write `{n}`"), k.span(), n.to_string())),
                );
            }
            [v] if v.as_word().is_some_and(|w| w.starts_with('@')) => {
                let word = v.as_word().unwrap();
                let kind = self
                    .defs
                    .keys()
                    .find(|(k, h)| h == word && forms::OBJECT_DATA.contains(&k.as_str()))
                    .map(|(k, _)| k.clone());
                self.out(file).error_fix(
                    codes::TYPE,
                    v.span(),
                    "`data` names the data's kind before its handle, such as `data mesh @name`",
                    kind.map(|k| {
                        (
                            format!("write `{k} {word}`"),
                            v.span(),
                            format!("{k} {word}"),
                        )
                    }),
                );
            }
            _ => self.out(file).error(
                codes::TYPE,
                a.span,
                "`data` takes a kind and a handle, such as `data mesh @name`",
            ),
        }
    }

    /// `units metric scale=1 length=m rotation=deg`: a scene's unit settings, short.
    fn units(&mut self, file: FileId, a: &Attr<'_>) {
        let props = self.schema.properties_of("UnitSettings");
        let find = |n: &str| props.iter().copied().find(|p| p.name == n);
        let mut entries: Vec<(String, Span, Value<'_>)> = Vec::new();
        let mut args = a.args.iter();
        if let Some(v) = args.next() {
            entries.push(("system".into(), v.span(), *v));
        }
        for v in args {
            self.out(file).error(
                codes::TYPE,
                v.span(),
                "`units` takes the system, then `scale=`, `length=` and `rotation=`",
            );
        }
        // A child node's attributes include its line's `key=value`s.
        match a.node {
            Some(n) => {
                for c in attrs(n) {
                    if let [v] = c.args.as_slice() {
                        entries.push((c.name.clone(), c.name_span, *v));
                    }
                }
            }
            None => {
                for (key, v) in &a.props {
                    entries.push((key.text.to_string(), key.span, *v));
                }
            }
        }
        let mut names: Vec<&str> = forms::UNITS_SHORT.iter().map(|(s, _)| *s).collect();
        names.extend(props.iter().map(|p| p.name.as_str()));
        let mut seen = HashSet::new();
        for (key, span, v) in entries {
            let full = forms::UNITS_SHORT
                .iter()
                .find(|(s, _)| *s == key)
                .map_or(key.as_str(), |(_, f)| *f);
            if !seen.insert(full.to_string()) {
                self.out(file).error(
                    codes::REPEATED,
                    span,
                    format!("`{full}` is given twice in `units`; only the first counts"),
                );
                continue;
            }
            let Some(p) = find(full) else {
                let pseudo = Attr {
                    name: key.clone(),
                    name_span: span,
                    span,
                    args: vec![v],
                    props: Vec::new(),
                    node: None,
                };
                self.unknown(file, &pseudo, "`units`", &names, None);
                continue;
            };
            let short: &[(&str, &str)] = match full {
                "length-unit" => &forms::LENGTH_UNITS,
                "system-rotation" => &forms::ROTATION_UNITS,
                _ => &[],
            };
            if v.as_word()
                .is_some_and(|w| short.iter().any(|(u, _)| *u == w))
            {
                continue;
            }
            let pseudo = Attr {
                name: key.clone(),
                name_span: span,
                span,
                args: vec![v],
                props: Vec::new(),
                node: None,
            };
            self.property(file, &pseudo, p, false);
        }
    }

    /// `primitive <kind> settings…`: a mesh made by one of Blender's add operators.
    fn primitive(&mut self, file: FileId, a: &Attr<'_>) {
        let kinds: Vec<&str> = self
            .schema
            .primitives
            .iter()
            .map(|p| p.kind.as_str())
            .collect();
        let Some(k) = a.args.first() else {
            return self.out(file).error(
                codes::TYPE,
                a.span,
                format!("`primitive` takes a kind: {}", list(&kinds)),
            );
        };
        let word = k.text();
        let Some(prim) = self.schema.primitives.iter().find(|p| p.kind == word) else {
            let near = closest(word, kinds.iter().copied());
            return self.out(file).error_fix(
                codes::KIND,
                k.span(),
                format!("`{word}` is not a primitive: {}", list_near(word, &kinds)),
                near.map(|n| (format!("write `{n}`"), k.span(), n.to_string())),
            );
        };
        let props: Vec<&Property> = prim.properties.iter().collect();
        let names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
        for v in &a.args[1..] {
            self.out(file).error(
                codes::TYPE,
                v.span(),
                "a primitive's settings are written as `name=value`",
            );
        }
        // A child node's attributes include its line's `key=value`s.
        let entries: Vec<Attr<'_>> = match a.node {
            Some(n) => attrs(n),
            None => a
                .props
                .iter()
                .map(|(key, v)| Attr {
                    name: key.text.to_string(),
                    name_span: key.span,
                    span: key.span.cover(v.span()),
                    args: vec![*v],
                    props: Vec::new(),
                    node: None,
                })
                .collect(),
        };
        for e in &entries {
            match props.iter().find(|p| p.name == e.name) {
                Some(p) => self.property(file, e, p, false),
                None => {
                    // A setting other primitives take: say which, rather than guess a name.
                    let others: Vec<&str> = self
                        .schema
                        .primitives
                        .iter()
                        .filter(|p| p.properties.iter().any(|q| q.name == e.name))
                        .map(|p| p.kind.as_str())
                        .collect();
                    let whose = (!others.is_empty()).then(|| {
                        format!(
                            "not a {word}'s: Blender's {word} operator does not take it; {} {}",
                            list(&others),
                            if others.len() == 1 { "does" } else { "do" }
                        )
                    });
                    self.unknown(file, e, &format!("a {word} primitive"), &names, whose)
                }
            }
        }
    }

    /// `vertices [[x y z] …]`: positions, lengths. Returns how many.
    fn vertices(&mut self, file: FileId, a: &Attr<'_>) -> Option<usize> {
        let co = self
            .schema
            .properties_of("MeshVertex")
            .into_iter()
            .find(|p| p.name == "co")?
            .clone();
        let mut out = self.out(file);
        let [list @ Value::List(_)] = a.args.as_slice() else {
            out.error(
                codes::TYPE,
                a.span,
                "`vertices` takes a list of positions, such as [[0m 0m 0m] [1m 0m 0m]]",
            );
            return None;
        };
        let items = list.items();
        for item in &items {
            check_float_array(item, 3, "vertices", dimension(&co, false), &co, &mut out);
        }
        Some(items.len())
    }

    /// `polygons [[0 1 2] …]`: each polygon's vertex indices, three or more.
    fn polygons(&mut self, file: FileId, a: &Attr<'_>, vertex_count: Option<usize>) {
        let Some(index) = self
            .schema
            .properties_of("MeshPolygon")
            .into_iter()
            .find(|p| p.name == "vertices")
            .cloned()
        else {
            return;
        };
        let min = index.min_length.unwrap_or(3);
        let mut out = self.out(file);
        let [list @ Value::List(_)] = a.args.as_slice() else {
            out.error(
                codes::TYPE,
                a.span,
                "`polygons` takes a list of polygons, each a list of vertex indices, such as [[0 1 2]]",
            );
            return;
        };
        for poly in list.items() {
            let Value::List(_) = poly else {
                out.error(
                    codes::TYPE,
                    poly.span(),
                    format!(
                        "each polygon is a list of {min} or more vertex indices, such as [0 1 2]"
                    ),
                );
                continue;
            };
            let indices = poly.items();
            if indices.len() < min {
                out.error(
                    codes::TYPE,
                    poly.span(),
                    format!(
                        "a polygon has {min} or more vertices; this one has {}",
                        indices.len()
                    ),
                );
            }
            for i in &indices {
                if let Some(n) = check_int(i, "polygons", &index, &mut out)
                    && let Some(count) = vertex_count
                    && n as usize >= count
                {
                    out.error(
                        codes::RANGE,
                        i.span(),
                        format!(
                            "vertex {n} does not exist: the mesh has {count} vertices, 0 to {}",
                            count.saturating_sub(1)
                        ),
                    );
                }
            }
        }
    }
}

/// Whether a `collection` line inside a scene or collection defines one (it has a
/// display name or a block) rather than naming one.
fn defines(node: Node<'_>) -> bool {
    node.block().is_some() || header(node).name.is_some()
}

/// The message for a value written with a scaled unit outside a scene when the scenes' scales
/// disagree and the project sets no `units` (spec, section 6, "Scene scale").
pub(crate) fn disagreement(outside: &Outside) -> Option<String> {
    let Outside::Disagree(scenes) = outside else {
        return None;
    };
    let each: Vec<String> = scenes
        .iter()
        .map(|(name, scale)| format!("{name} {}", null_value::print_bare(*scale)))
        .collect();
    Some(format!(
        "this value is outside every scene, and the scenes' unit scales differ ({}); give the \
         project a `units` line to say which scale holds outside them, or write a bare number",
        each.join(", ")
    ))
}

/// A scene's frame rate from its `fps` and `fps-base` lines, a new scene's (24 and 1) where one
/// is missing or does not read; those lines' own errors are reported where they are checked.
pub(crate) fn frame_rate(scene: Node<'_>) -> null_value::Scene {
    let mut rate = null_value::Scene::default();
    for a in attrs(scene) {
        let Some(word) = a.args.first().and_then(|v| v.as_word()) else {
            continue;
        };
        match a.name.as_str() {
            "fps" => {
                if let Ok(n) = null_value::read_int(word).map(u32::try_from) {
                    rate.fps = n.unwrap_or(rate.fps).max(1);
                }
            }
            "fps-base" => {
                if let Ok(x) = null_value::read::<f32>(word, null_value::Dimension::None)
                    && x > 0.0
                    && x.is_finite()
                {
                    rate.fps_base = x;
                }
            }
            _ => {}
        }
    }
    rate
}

/// The handles of a node tree's nodes and their kinds.
/// A handle an expansion writes: kebab-case handles joined by `/`, each with a copy's number
/// or not (`@plank.7`, `@tree/@left/@right`, `@plank.7/@plank-mesh`; task 1d-03c).
fn is_copy_handle(handle: &str) -> bool {
    handle.split('/').all(|step| {
        let bare = step.strip_prefix('@').unwrap_or("");
        let name = match bare.rsplit_once('.') {
            Some((n, k)) if !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit()) => n,
            _ => bare,
        };
        is_kebab_case(name)
    })
}

/// A handle must be `@` and kebab-case (S16, S18).
fn kebab_handle(handle: &str, span: Span, out: &mut Out<'_>) -> bool {
    let bare = handle.trim_start_matches('@');
    if is_kebab_case(bare) {
        return true;
    }
    let fix = to_kebab_case(bare).map(|k| format!("@{k}"));
    out.error_fix(
        codes::NAME,
        span,
        format!("`{handle}` is not a kebab-case handle: `@`, then lower-case words joined by `-`"),
        fix.map(|f| (format!("write `{f}`"), span, f)),
    );
    false
}

/// `kind` with its article: "a scene", "an object".
fn article(kind: &str) -> String {
    let an = kind.starts_with(['a', 'e', 'i', 'o', 'u']);
    format!("{} {kind}", if an { "an" } else { "a" })
}
