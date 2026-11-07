//! The walk over a project's files: definitions first, then every node against the schema.

use std::collections::{HashMap, HashSet};

use null_syntax::ast::{Entry, Node, Value};
use null_syntax::names::{is_kebab_case, to_kebab_case};
use null_syntax::{Diagnostic, FileId, Parse, Span, SyntaxToken, closest};

use crate::codes;
use crate::forms;
use crate::schema::{Property, Schema};
use crate::units::{self, Outside, Units};
use crate::values::{
    Out, check_bool, check_enum, check_float, check_float_array, check_float_matrix, check_int,
    dimension, list,
};

/// Checks every file of a project together, since handles are visible project-wide
/// (spec, section 7), and returns the problems in each file's text order. Syntax and
/// name problems are `null_syntax::diagnostics`'s, not repeated here.
pub fn check(schema: &Schema, files: &[(FileId, &Parse)]) -> Vec<Diagnostic> {
    let mut cx = Checker {
        schema,
        defs: HashMap::new(),
        out: Vec::new(),
        scene: None,
        scale: 1.0,
        project: Units::default(),
        outside: Outside::Scale(1.0),
        disagreement: None,
    };
    let parses = files.iter().map(|(_, p)| *p);
    cx.project = units::project(parses.clone()).unwrap_or_default();
    cx.outside = units::outside(parses);
    cx.scale = cx.outside.scale();
    cx.disagreement = disagreement(&cx.outside);
    for (file, parse) in files {
        for node in parse.document().nodes() {
            cx.collect(*file, node, true);
        }
    }
    for (file, parse) in files {
        for node in parse.document().nodes() {
            cx.top(*file, node);
        }
    }
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
    let mut rest = Vec::new();
    for (i, v) in args_of(node).into_iter().enumerate() {
        match v {
            Value::Word(t) if i == 0 && t.text.starts_with('@') => {
                handle = Some((t.text.to_string(), t.span))
            }
            Value::Str(_) | Value::Raw(_) if name.is_none() => {
                name = v.as_string().map(|s| s.into_owned())
            }
            _ => rest.push(v),
        }
    }
    Header { handle, name, rest }
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
                    format!("`{handle}` is defined twice as a {kind}; a handle is unique within its kind"),
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
        let Some(s) = self.schema.datablock_kinds().find(|s| s.kind == kind) else {
            let kinds: Vec<&str> = std::iter::once(forms::PROJECT)
                .chain(self.schema.datablock_kinds().map(|s| s.kind.as_str()))
                .collect();
            let suggestion = closest(&kind, kinds.iter().copied());
            self.out(file).error_fix(
                codes::KIND,
                name.span,
                format!(
                    "`{kind}` is not a kind of datablock the schema knows: {}",
                    list(&kinds)
                ),
                suggestion.map(|k| (format!("write `{k}`"), name.span, k.to_string())),
            );
            return;
        };
        let rna = s.rna.clone();
        self.header_check(file, node, &kind);
        if kind == "scene" {
            let mut rate = frame_rate(node);
            self.scale = Units::of(node)
                .unwrap_or_default()
                .over(&self.project)
                .scale();
            rate.scale_length = self.scale;
            self.scene = Some(rate);
        }
        match kind.as_str() {
            "light" => self.light(file, node),
            _ => self.body(file, node, &kind, &rna),
        }
        self.scene = None;
        self.scale = self.outside.scale();
    }

    /// The handle and display name on a datablock's line, and nothing else.
    fn header_check(&mut self, file: FileId, node: Node<'_>, kind: &str) {
        let h = header(node);
        let mut out = self.out(file);
        match &h.handle {
            Some((handle, span)) => {
                kebab_handle(handle, *span, &mut out);
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
        let forms: &[&str] = match kind {
            "scene" => &["object", "collection", "units", "fps", "fps-base"],
            "collection" => &["object", "collection"],
            "object" => &["data", "material"],
            "mesh" => &["primitive", "vertices", "polygons"],
            "material" => &["nodes", "layout"],
            _ => &[],
        };
        let mut names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
        names.extend(forms.iter().copied());
        let mut seen: HashSet<String> = HashSet::new();
        let mut vertex_count = None;
        let all = attrs(node);
        // Node handles first, so layout and links may name nodes defined later.
        let node_handles = all
            .iter()
            .find(|a| a.name == "nodes")
            .and_then(|a| a.node)
            .map(node_tree)
            .unwrap_or_default();
        for a in &all {
            let repeatable = matches!(
                (kind, a.name.as_str()),
                ("scene" | "collection", "object" | "collection") | ("object", "material")
            );
            if !repeatable && !seen.insert(a.name.clone()) {
                self.out(file).error(
                    codes::REPEATED,
                    a.name_span,
                    format!("`{}` is given twice; {} has one", a.name, article(kind)),
                );
                continue;
            }
            match (kind, a.name.as_str()) {
                ("scene" | "collection", "object") => self.reference(file, a, "object"),
                ("scene" | "collection", "collection") => {
                    if a.node.is_some_and(defines) {
                        let n = a.node.unwrap();
                        self.header_check(file, n, "collection");
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
                ("mesh", "vertices") => vertex_count = self.vertices(file, a),
                ("mesh", "polygons") => {} // after the loop, against the vertex count
                ("material", "nodes") => self.nodes(file, a, &node_handles),
                ("material", "layout") => self.layout(file, a, &node_handles),
                _ => match props.iter().find(|p| p.name == a.name) {
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
        if let Some(a) = all.iter().find(|a| kind == "mesh" && a.name == "polygons") {
            self.polygons(file, a, vertex_count);
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

    /// One RNA property's value.
    fn property(&mut self, file: FileId, a: &Attr<'_>, p: &Property, power: bool) {
        let mut out = self.out(file);
        let name = a.name.as_str();
        if p.readonly || p.ty == "collection" {
            out.error(
                codes::READONLY,
                a.name_span,
                format!("`{name}` is read-only in Blender, so it is not written"),
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
        if p.ty == "bool" {
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
            "enum" => check_enum(value, name, p, &mut out),
            "string" => {
                if value.as_string().is_none() {
                    out.error(
                        codes::TYPE,
                        value.span(),
                        format!(
                            "`{name}` takes a string, such as \"Name\", not `{}`",
                            value.text()
                        ),
                    );
                }
            }
            "pointer" => {
                let target = p.target.as_deref().unwrap_or("ID");
                let kind = self
                    .schema
                    .struct_by_rna(target)
                    .map(|s| s.kind.clone())
                    .unwrap_or_default();
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

    /// A handle that must name a datablock of `kind`.
    fn handle_ref(&mut self, file: FileId, v: &Value<'_>, kind: &str, what: &str) {
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
        if !kebab_handle(word, v.span(), &mut self.out(file)) {
            // Respelt, it may name one: then the name's own fix is the whole fix.
            let kebab = to_kebab_case(word.trim_start_matches('@')).map(|k| format!("@{k}"));
            if kebab.is_some_and(|k| self.defs.contains_key(&(kind.to_string(), k))) {
                return;
            }
        }
        if self
            .defs
            .contains_key(&(kind.to_string(), word.to_string()))
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
            .filter(|(k, _)| k == kind)
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
                format!("`{word}` is not a primitive: {}", list(&kinds)),
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

    /// `nodes { node @h <kind> { inputs } … }`: a material's shader nodes.
    fn nodes(&mut self, file: FileId, a: &Attr<'_>, handles: &HashMap<String, String>) {
        let Some(tree) = a.node.filter(|n| n.block().is_some()) else {
            return self.out(file).error(
                codes::TYPE,
                a.span,
                "`nodes` takes a block of `node @handle <kind>` lines",
            );
        };
        let kinds: Vec<&str> = self.schema.nodes.iter().map(|n| n.kind.as_str()).collect();
        let mut seen = HashSet::new();
        for child in tree.children() {
            let Some(name) = child.name_token() else {
                continue;
            };
            if child.name() != "node" {
                self.out(file).error(
                    codes::UNKNOWN_PROPERTY,
                    name.span,
                    format!("a node tree holds `node` lines, not `{}`", child.name()),
                );
                continue;
            }
            let args = args_of(child);
            let (Some(h), Some(k)) = (args.first(), args.get(1)) else {
                self.out(file).error(
                    codes::TYPE,
                    child.code_span(),
                    "a node is written `node @handle <kind>`, such as `node @bsdf principled-bsdf`",
                );
                continue;
            };
            let handle = h.text();
            if !handle.starts_with('@') {
                self.out(file).error(
                    codes::TYPE,
                    h.span(),
                    "a node's handle comes first, such as `@bsdf`",
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
            let kind = k.text();
            let Some(def) = self.schema.nodes.iter().find(|n| n.kind == kind) else {
                let near = closest(kind, kinds.iter().copied());
                self.out(file).error_fix(
                    codes::KIND,
                    k.span(),
                    format!(
                        "`{kind}` is not a node kind the schema knows: {}",
                        list(&kinds)
                    ),
                    near.map(|n| (format!("write `{n}`"), k.span(), n.to_string())),
                );
                continue;
            };
            let inputs: Vec<&Property> = def.inputs.iter().collect();
            let names: Vec<&str> = inputs.iter().map(|p| p.name.as_str()).collect();
            for v in &args[2..] {
                self.out(file).error(
                    codes::TYPE,
                    v.span(),
                    "a node's inputs are written as `name value` lines or `name=value`",
                );
            }
            let mut given = HashSet::new();
            for input in attrs(child) {
                if !given.insert(input.name.clone()) {
                    self.out(file).error(
                        codes::REPEATED,
                        input.name_span,
                        format!("`{}` is given twice", input.name),
                    );
                    continue;
                }
                let Some(p) = inputs.iter().find(|p| p.name == input.name) else {
                    self.unknown(file, &input, &format!("a {kind} node"), &names, None);
                    continue;
                };
                match input.args.as_slice() {
                    [v] if v.as_word().is_some_and(|w| w.starts_with('@')) => {
                        self.link(file, v, handles)
                    }
                    _ => self.property(file, &input, p, false),
                }
            }
        }
    }

    /// `@node.socket`: a link from another node's output.
    fn link(&mut self, file: FileId, v: &Value<'_>, handles: &HashMap<String, String>) {
        let word = v.as_word().unwrap_or_default();
        let Some((node, socket)) = word.split_once('.') else {
            return self.out(file).error(
                codes::TYPE,
                v.span(),
                format!("a link names a node and its output, such as `{word}.bsdf`"),
            );
        };
        let Some(kind) = handles.get(node) else {
            let near = closest(node, handles.keys().map(String::as_str)).map(str::to_string);
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
        if let Some(def) = self.schema.nodes.iter().find(|n| &n.kind == kind) {
            let outs: Vec<&str> = def.outputs.iter().map(|o| o.name.as_str()).collect();
            if !outs.contains(&socket) {
                let near = closest(socket, outs.iter().copied());
                self.out(file).error_fix(
                    codes::HANDLE,
                    v.span(),
                    format!(
                        "a {kind} node has no output `{socket}`: it has {}",
                        list(&outs)
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
    }

    /// `layout { @h [x y]; … }`: where the editor draws each node.
    fn layout(&mut self, file: FileId, a: &Attr<'_>, handles: &HashMap<String, String>) {
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
            if !handles.contains_key(handle) {
                out.error(
                    codes::HANDLE,
                    name.span,
                    format!("no node of this tree is called `{handle}`"),
                );
            }
            let args = args_of(child);
            match args.as_slice() {
                [v @ Value::List(_)] if v.items().len() == 2 => {
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
                [_, rest @ ..] if !rest.is_empty() => out.error(
                    codes::TYPE,
                    rest[0].span(),
                    "one node per layout line: end each with a line break or `;`",
                ),
                _ => out.error(
                    codes::TYPE,
                    child.code_span(),
                    "a layout line is `@node [x y]`",
                ),
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
fn disagreement(outside: &Outside) -> Option<String> {
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
fn frame_rate(scene: Node<'_>) -> null_value::Scene {
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
fn node_tree(tree: Node<'_>) -> HashMap<String, String> {
    tree.children()
        .filter(|c| c.name() == "node")
        .filter_map(|c| {
            let args = args_of(c);
            Some((
                args.first()?.text().to_string(),
                args.get(1)?.text().to_string(),
            ))
        })
        .collect()
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
