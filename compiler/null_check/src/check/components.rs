//! Components, parameters, placements, patterns, conditions, overrides and recursion that
//! counts down (spec, section 8; Sau's decisions S32 to S35, S38, S61 (6) and S74; task 1d-03b):
//! read and checked, as written. Expanding them into the flat project is task 1d-03c.
//!
//! - A component's own handles are local: its datablocks are reached inside it by their
//!   handles, and from outside only through a copy (`@plank.7/@plank-mesh`).
//! - Parameters are typed by their default, or by a type's name when they have none.
//! - A placement (`use`) names a defined component, gives its parameters by name and its root
//!   object's own properties; a pattern (`count=`, `each=` with `as=`) gives `n`, and the item.
//! - A value may be an expression of the parameters in scope (`expr`), checked for its units.
//! - A condition (`if`, then `else`) compares parameters; an override names a copy, or a
//!   datablock a copy made, by its path.
//! - A component that places itself, directly or through others, does so only through a
//!   parameter marked `{ countdown }`, passed a smaller value, under an `if` that holds only
//!   while it is above 0 (S38, S74 (5)).

use std::collections::{HashMap, HashSet};

use null_syntax::ast::{Node, Value};
use null_syntax::names::is_kebab_case;
use null_syntax::{FileId, Span, closest};
use null_value::Dimension;

use super::expr::{self, Dim, Expr, Kind};
use super::{Checker, args_of, attrs, header, kebab_handle, props_of};
use crate::codes;
use crate::values::list;

/// A parameter of a component.
#[derive(Clone, Debug)]
pub(crate) struct Param {
    pub name: String,
    pub kind: Kind,
    /// No default: every placement must give it.
    pub required: bool,
    pub countdown: bool,
    /// The power of its angle in degrees where its default says (`30deg` 1, `0.5rad` and `2m`
    /// 0); `None` with no default, as a placement may give either (task 1d-03i).
    pub degrees: Option<i8>,
}

/// A placement, where it is made: the component placed, its handle, and how many copies (`None`
/// for one, `Some(None)` for a pattern whose count is not known before expansion).
#[derive(Clone, Debug)]
pub(crate) struct Placed {
    pub component: String,
    pub handle: String,
    pub copies: Option<Option<usize>>,
}

/// A component as collected before checking: what other files and placements need of it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Component {
    pub params: Vec<Param>,
    /// Its own datablocks, (kind, handle), the first `object` its root.
    pub locals: Vec<(String, String)>,
    pub root: Option<String>,
    /// The sub-objects each of its datablocks defines, by the datablock's local handle, each
    /// with its own (a bone's constraints): reached in a copy by path, the root's straight
    /// from the copy (`@plank.7/@fluid`, `@rig.2/@spine/@reach`; tasks 1d-03b and 1d-03i).
    pub subs: HashMap<String, Vec<Sub>>,
    /// The placements in its block, at any depth of `if`.
    pub places: Vec<Placed>,
}

/// A sub-object a component's datablock defines: its line's word (`modifier`, `bone`), its
/// handle, its kind where its line gives one (`kinematic`), and the sub-objects it holds.
#[derive(Clone, Debug)]
pub(crate) struct Sub {
    pub word: String,
    pub handle: String,
    pub of: Option<String>,
    pub subs: Vec<Sub>,
}

/// What names mean where values are being checked inside a component or a placement's block.
#[derive(Clone, Debug, Default)]
pub(crate) struct Scope {
    pub names: HashMap<String, Kind>,
    pub locals: Vec<(String, String)>,
    /// The power of each name's angle held in degrees, where it is known ([`Param::degrees`]).
    pub degrees: HashMap<String, i8>,
}

/// The lines of a root object's own a placement or an override may give besides its
/// properties: its modifiers, constraints and pose bones, each checked as that sub-object, and
/// its material slots (task 1d-03i).
const ROOT_LINES: [&str; 4] = ["modifier", "constraint", "bone", "material"];

/// The type names a parameter with no default may give (section 8): a unit's dimension, a
/// number, a whole number, `on-off`, or a kind of datablock.
const TYPE_NAMES: [(&str, Dimension); 6] = [
    ("length", Dimension::Length),
    ("area", Dimension::Area),
    ("volume", Dimension::Volume),
    ("mass", Dimension::Mass),
    ("angle", Dimension::Angle),
    ("time", Dimension::Seconds),
];

fn whole() -> Kind {
    Kind::Quantity {
        dim: Dim::NONE,
        whole: true,
    }
}

/// What a plain value is: a number with its unit, `on`/`off`, or `None`.
fn kind_of_word(word: &str) -> Option<Kind> {
    if matches!(word, "on" | "off") {
        return Some(Kind::Bool);
    }
    match expr::parse(word, false).ok()? {
        Expr::Number { dim, whole, .. } => Some(Kind::Quantity { dim, whole }),
        Expr::Neg(x) => match *x {
            Expr::Number { dim, whole, .. } => Some(Kind::Quantity { dim, whole }),
            _ => None,
        },
        _ => None,
    }
}

/// Whether a value of kind `got` may stand where `want` goes: the same kind; a whole number where
/// any number goes; a word of a list that is all in the wanted list.
fn fits(got: &Kind, want: &Kind) -> bool {
    match (got, want) {
        (Kind::Quantity { dim: a, whole: wa }, Kind::Quantity { dim: b, whole: wb }) => {
            a == b && (*wa || !*wb)
        }
        (Kind::Word(a), Kind::Word(b)) => a.iter().all(|w| b.contains(w)),
        (a, b) => a == b,
    }
}

impl Checker<'_> {
    /// A component's parameter line, read: the parameter, or what is wrong and where.
    fn read_param(&self, node: Node<'_>) -> Result<Param, (Span, String)> {
        let args = args_of(node);
        let at = node.name_token().map_or(node.span(), |t| t.span);
        let Some(name) = args.first().and_then(|v| v.as_word()) else {
            return Err((
                at,
                "`param` takes a name, then a default or a type: `param length 2.4m`".into(),
            ));
        };
        let name_span = args[0].span();
        if !is_kebab_case(name) {
            return Err((name_span, format!("`{name}` is not a kebab-case name")));
        }
        let mut countdown = false;
        let mut one_of: Option<Vec<String>> = None;
        for c in node.children() {
            match c.name().as_ref() {
                "countdown" => countdown = true,
                "one-of" => {
                    let items: Vec<String> = c
                        .args()
                        .next()
                        .map(|v| {
                            v.items()
                                .iter()
                                .filter_map(|i| i.as_word().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default();
                    if items.is_empty() {
                        return Err((
                            c.span(),
                            "`one-of` takes a list of words: `one-of [solid liquid]`".into(),
                        ));
                    }
                    one_of = Some(items);
                }
                other => {
                    return Err((
                        c.name_token().map_or(c.span(), |t| t.span),
                        format!(
                            "a parameter's block holds `countdown` or `one-of [...]`, not `{other}`"
                        ),
                    ));
                }
            }
        }
        let rest = &args[1..];
        let (kind, required) = match rest {
            [] => {
                return Err((
                    name_span,
                    format!(
                        "`{name}` needs a default or a type: `param {name} 2.4m`, `param {name} length`"
                    ),
                ));
            }
            // A handle of a kind: `param finish material @oak`, or with no default `param finish material`.
            [k, h] if h.as_word().is_some_and(|w| w.starts_with('@')) => {
                let kind = k.text().to_string();
                if !self.is_datablock(&kind) {
                    return Err((k.span(), format!("`{kind}` is not a kind of datablock")));
                }
                (Kind::Handle(kind), false)
            }
            [v] => {
                let word = v.text();
                if let Some(words) = &one_of {
                    if v.as_word() == Some("word") {
                        (Kind::Word(words.clone()), true)
                    } else if words.iter().any(|w| w == word) {
                        (Kind::Word(words.clone()), false)
                    } else {
                        return Err((
                            v.span(),
                            format!(
                                "`{word}` is not one of the parameter's words: {}",
                                list(&words.iter().map(String::as_str).collect::<Vec<_>>())
                            ),
                        ));
                    }
                } else if let Some(k) = kind_of_word(word) {
                    (k, false)
                } else if let Some((_, d)) = TYPE_NAMES.iter().find(|(n, _)| *n == word) {
                    (Kind::number(Dim::of(*d)), true)
                } else {
                    match word {
                        "number" => (Kind::number(Dim::NONE), true),
                        "whole-number" => (whole(), true),
                        "on-off" => (Kind::Bool, true),
                        k if self.is_datablock(k) => (Kind::Handle(k.to_string()), true),
                        _ => {
                            let mut names: Vec<&str> = TYPE_NAMES.iter().map(|(n, _)| *n).collect();
                            names.extend(["number", "whole-number", "on-off"]);
                            let near = closest(word, names.iter().copied());
                            return Err((
                                v.span(),
                                match near {
                                    Some(n) => format!(
                                        "`{word}` is neither a default nor a type; did you mean `{n}`?"
                                    ),
                                    None => format!(
                                        "`{word}` is neither a default (a number, `on`, `off`, or a word with `one-of`) nor a type ({}, or a kind of datablock)",
                                        list(&names)
                                    ),
                                },
                            ));
                        }
                    }
                }
            }
            [_, extra, ..] => {
                return Err((
                    extra.span(),
                    format!("`param {name}` takes one default or one type"),
                ));
            }
        };
        if countdown && kind != whole() {
            return Err((
                at,
                format!(
                    "`{name}` counts down, so it is a whole number, not {}",
                    kind.describe()
                ),
            ));
        }
        let degrees = args
            .get(1)
            .and_then(|v| v.as_word())
            .filter(|w| !required && kind_of_word(w).is_some())
            .map(|w| i8::from(expr::in_degrees(w)));
        Ok(Param {
            name: name.to_string(),
            kind,
            required,
            countdown,
            degrees,
        })
    }

    /// Collects a top-level `component` (before any checking), for the placements that name it.
    pub(super) fn collect_component(&mut self, node: Node<'_>) {
        let Some((handle, _)) = header(node).handle else {
            return;
        };
        let mut c = Component::default();
        for child in node.children() {
            if child.name() == "param"
                && let Ok(p) = self.read_param(child)
            {
                c.params.push(p);
            }
        }
        self.collect_component_body(node, &mut c);
        self.components.entry(handle).or_insert(c);
    }

    fn collect_component_body(&self, node: Node<'_>, c: &mut Component) {
        for child in node.children() {
            let name = child.name().into_owned();
            match name.as_str() {
                "param" | "override" => {}
                "if" | "else" => self.collect_component_body(child, c),
                "use" => {
                    if let Some(p) = placed(child) {
                        c.places.push(p);
                    }
                }
                kind if self.is_datablock(kind) => {
                    if let Some((h, _)) = self.handle_of(child) {
                        if kind == "object" && c.root.is_none() {
                            c.root = Some(h.clone());
                        }
                        let mut found = Vec::new();
                        subs(child, &mut found);
                        c.subs.entry(h.clone()).or_insert(found);
                        c.locals.push((kind.to_string(), h));
                    }
                }
                _ => {}
            }
        }
    }

    /// Collects the placements in a scene's or collection's block (and the collections defined
    /// in it), which name copies other files reach (`@plank.7`).
    pub(super) fn collect_placements(&mut self, node: Node<'_>) {
        for child in node.children() {
            match child.name().as_ref() {
                "use" => {
                    if let Some(p) = placed(child) {
                        self.placements.entry(p.handle.clone()).or_insert(p);
                    }
                }
                "collection" if super::defines(child) => self.collect_placements(child),
                _ => {}
            }
        }
    }

    /// Which components place themselves, directly or through others.
    pub(super) fn find_cycles(&mut self) {
        let graph: HashMap<&str, Vec<&str>> = self
            .components
            .iter()
            .map(|(h, c)| {
                (
                    h.as_str(),
                    c.places.iter().map(|p| p.component.as_str()).collect(),
                )
            })
            .collect();
        let reaches = |from: &str, to: &str| -> bool {
            let mut seen = HashSet::new();
            let mut todo = vec![from];
            while let Some(at) = todo.pop() {
                for next in graph.get(at).into_iter().flatten() {
                    if *next == to {
                        return true;
                    }
                    if seen.insert(*next) {
                        todo.push(next);
                    }
                }
            }
            false
        };
        let mut pairs = HashSet::new();
        for a in graph.keys() {
            for b in graph.keys() {
                if reaches(a, b) && reaches(b, a) {
                    pairs.insert((a.to_string(), b.to_string()));
                }
            }
        }
        self.cycles = pairs;
    }

    /// A top-level `component @h "Name" { … }`.
    pub(super) fn component(&mut self, file: FileId, node: Node<'_>) {
        self.header_check(file, node, "component");
        let Some((handle, _)) = header(node).handle else {
            return;
        };
        let comp = self.components.get(&handle).cloned().unwrap_or_default();
        // Parameters: each read, each name once.
        let mut seen: HashMap<String, Span> = HashMap::new();
        for child in node.children().filter(|c| c.name() == "param") {
            match self.read_param(child) {
                Ok(p) => {
                    // A handle's default names a datablock of the project (section 8).
                    if let (Kind::Handle(kind), false) = (&p.kind, p.required)
                        && let Some(h) = args_of(child).get(2).copied()
                    {
                        let outer = self.scope.take();
                        self.handle_ref(file, &h, kind, &p.name);
                        self.scope = outer;
                    }
                    let at = args_of(child)[0].span();
                    if let Some(first) = seen.insert(p.name.clone(), at) {
                        let mut d = null_syntax::Diagnostic::error(
                            codes::REPEATED,
                            file,
                            at,
                            format!("the parameter `{}` is given twice", p.name),
                        );
                        d = d.with_label(file, first, "first given here");
                        self.out.push(d);
                    }
                }
                Err((span, message)) => self.out(file).error(codes::TYPE, span, message),
            }
        }
        if comp.root.is_none() {
            let at = node.name_token().map_or(node.span(), |t| t.span);
            self.out(file).error(
                codes::KIND,
                at,
                format!("`{handle}` places nothing: a component's first `object` is its root, which a placement puts in the scene"),
            );
        }
        let names = comp
            .params
            .iter()
            .map(|p| (p.name.clone(), p.kind.clone()))
            .collect();
        let degrees = comp
            .params
            .iter()
            .filter_map(|p| Some((p.name.clone(), p.degrees?)))
            .collect();
        let outer = self.scope.replace(super::components::Scope {
            names,
            locals: comp.locals.clone(),
            degrees,
        });
        self.component_block(file, node, &handle, &comp, &mut Vec::new());
        self.scope = outer;
    }

    /// A component's block, or an `if` or `else` block in it, under the conditions `guards`.
    fn component_block(
        &mut self,
        file: FileId,
        node: Node<'_>,
        owner: &str,
        comp: &Component,
        guards: &mut Vec<expr::Condition>,
    ) {
        let mut after_if = false;
        for child in node.children() {
            let name = child.name().into_owned();
            let is_if = name == "if";
            match name.as_str() {
                "param" => {}
                "use" => self.placement(file, child, Some((owner, comp, guards.as_slice()))),
                "override" => self.override_line(file, child),
                "if" => {
                    let cond = self.condition(file, child);
                    guards.extend(cond.clone());
                    self.component_block(file, child, owner, comp, guards);
                    if cond.is_some() {
                        guards.pop();
                    }
                }
                "else" => {
                    if !after_if {
                        self.else_alone(file, child);
                    }
                    self.component_block(file, child, owner, comp, guards);
                }
                kind if self.is_datablock(kind) => self.top(file, child),
                other => {
                    let at = child.name_token().map_or(child.span(), |t| t.span);
                    let mut known = vec!["param", "use", "if", "else", "override"];
                    known.extend(self.schema.datablock_kinds().map(|s| s.kind.as_str()));
                    let near = closest(other, known.iter().copied()).map(str::to_string);
                    self.out(file).error_fix(
                        codes::KIND,
                        at,
                        format!(
                            "a component's block holds parameters, datablocks, placements (`use`), conditions and overrides; `{other}` is none of them"
                        ),
                        near.map(|n| (format!("write `{n}`"), at, n)),
                    );
                }
            }
            after_if = is_if;
        }
    }

    fn else_alone(&mut self, file: FileId, node: Node<'_>) {
        let at = node.name_token().map_or(node.span(), |t| t.span);
        self.out(file)
            .error(codes::TYPE, at, "`else` follows an `if` directly");
    }

    /// `if "<condition>" { … }`: the condition read and its names checked. Returns it if it reads.
    pub(super) fn condition(&mut self, file: FileId, node: Node<'_>) -> Option<expr::Condition> {
        let args = args_of(node);
        let at = node.name_token().map_or(node.span(), |t| t.span);
        let [v] = args.as_slice() else {
            self.out(file).error(
                codes::TYPE,
                at,
                "`if` takes one quoted condition: `if \"state == solid\" { … }`",
            );
            return None;
        };
        let Some(text) = v.as_string() else {
            self.out(file).error_fix(
                codes::TYPE,
                v.span(),
                "a condition is quoted, as it holds spaces and a comparison",
                Some(("quote it".into(), v.span(), format!("\"{}\"", v.text()))),
            );
            return None;
        };
        let cond = match expr::parse_condition(&text) {
            Ok(c) => c,
            Err(e) => {
                self.out(file)
                    .error(codes::TYPE, v.span(), format!("`{text}`: {e}"));
                return None;
            }
        };
        let scope = self.scope.clone().unwrap_or_default();
        // A word compared with a word parameter is one of its words, not a name.
        let side = |e: &Expr, other: &Expr| -> Result<Kind, String> {
            if let Expr::Name(n) = e
                && !scope.names.contains_key(n)
                && let Expr::Name(o) = other
                && let Some(Kind::Word(words)) = scope.names.get(o)
            {
                return if words.contains(n) {
                    Ok(Kind::Word(words.clone()))
                } else {
                    Err(format!(
                        "`{n}` is not one of `{o}`'s words: {}",
                        words.join(", ")
                    ))
                };
            }
            expr::kind_of(e, &|n| scope.names.get(n).cloned())
        };
        let left = side(&cond.left, &cond.right);
        let right = side(&cond.right, &cond.left);
        match (left, right) {
            (Err(e), _) | (_, Err(e)) => {
                let fix = self.name_fix(&e, &scope, v.span(), &text);
                self.out(file)
                    .error_fix(codes::TYPE, v.span(), format!("`{text}`: {e}"), fix);
                None
            }
            (Ok(a), Ok(b)) => {
                let comparable = match (&a, &b) {
                    (Kind::Quantity { dim: x, .. }, Kind::Quantity { dim: y, .. }) => x == y,
                    (Kind::Word(_), Kind::Word(_)) | (Kind::Bool, Kind::Bool) => {
                        matches!(cond.op, "==" | "!=")
                    }
                    _ => false,
                };
                if !comparable {
                    self.out(file).error(
                        codes::TYPE,
                        v.span(),
                        format!("`{text}` compares {} with {}", a.describe(), b.describe()),
                    );
                    return None;
                }
                // Degrees against radians, as the expansion refuses them (task 1d-03i).
                let lookup = |n: &str| scope.degrees.get(n).copied();
                let units = expr::degrees(&cond.left, &lookup).and_then(|x| {
                    let y = expr::degrees(&cond.right, &lookup)?;
                    match (x, y) {
                        (Some(x), Some(y)) if x != y => Err(expr::mixed_angles("compare")),
                        _ => Ok(()),
                    }
                });
                if let Err(e) = units {
                    self.out(file)
                        .error(codes::TYPE, v.span(), format!("`{text}`: {e}"));
                    return None;
                }
                Some(cond)
            }
        }
    }

    /// For "no parameter is called `x`": the nearest name in scope, as a fix of that name in `text`.
    fn name_fix(
        &self,
        error: &str,
        scope: &Scope,
        span: Span,
        text: &str,
    ) -> Option<(String, Span, String)> {
        let missing = error
            .strip_prefix("no parameter is called `")?
            .strip_suffix('`')?;
        let near = closest(missing, scope.names.keys().map(String::as_str))?;
        Some((
            format!("write `{near}`"),
            span,
            format!("\"{}\"", text.replacen(missing, near, 1)),
        ))
    }

    /// `use @component @handle [count=N | each=[…] as=name] { … }`, at the top of a scene or a
    /// collection, or inside a component (`within`: its handle, its collected form, its guards).
    pub(super) fn placement(
        &mut self,
        file: FileId,
        node: Node<'_>,
        within: Option<(&str, &Component, &[expr::Condition])>,
    ) {
        let args = args_of(node);
        let at = node.name_token().map_or(node.span(), |t| t.span);
        let [target, handle] = args.as_slice() else {
            self.out(file).error(
                codes::TYPE,
                at,
                "`use` names a component and the copy's handle: `use @plank @deck { … }`",
            );
            return;
        };
        let Some(name) = target.as_word().filter(|w| w.starts_with('@')) else {
            self.out(file).error(
                codes::TYPE,
                target.span(),
                format!(
                    "`use` names a component by its handle, not `{}`",
                    target.text()
                ),
            );
            return;
        };
        match handle.as_word().filter(|w| w.starts_with('@')) {
            Some(h) => {
                kebab_handle(h, handle.span(), &mut self.out(file));
            }
            None => self.out(file).error(
                codes::HANDLE,
                handle.span(),
                format!(
                    "the copy needs a handle, such as `@deck`, not `{}`",
                    handle.text()
                ),
            ),
        }
        // A package's component (`@cosmosis-materials:oak-plank`, S54) is read with packages (1d-04).
        if name.contains(':') {
            return;
        }
        let Some(comp) = self.components.get(name).cloned() else {
            let near =
                closest(name, self.components.keys().map(String::as_str)).map(str::to_string);
            let message = match &near {
                Some(n) => format!("no component is called `{name}`; did you mean `{n}`?"),
                None => format!("no component is called `{name}`"),
            };
            self.out(file).error_fix(
                codes::HANDLE,
                target.span(),
                message,
                near.map(|n| (format!("write `{n}`"), target.span(), n)),
            );
            return;
        };
        // The pattern: `count=` or `each=` with `as=`.
        let props = props_of(node);
        let mut extra: Vec<(String, Kind)> = Vec::new();
        let (mut count, mut each, mut item) = (None, None, None);
        for (key, value) in &props {
            match key.text {
                "count" => count = Some(*value),
                "each" => each = Some(*value),
                "as" => item = Some(*value),
                other => {
                    let near = closest(other, ["count", "each", "as"]).map(str::to_string);
                    self.out(file).error_fix(
                        codes::UNKNOWN_PROPERTY,
                        key.span,
                        format!("`use` takes `count=`, or `each=` with `as=`; not `{other}=`"),
                        near.map(|n| (format!("write `{n}`"), key.span, n)),
                    );
                }
            }
        }
        if let (Some(c), Some(_)) = (count, each) {
            self.out(file).error(
                codes::TYPE,
                c.span(),
                "a pattern gives `count=` or `each=`, not both",
            );
        }
        if let Some(c) = count {
            self.value_of(file, &c, &whole(), "count");
            extra.push(("n".into(), whole()));
        }
        match (each, item) {
            (Some(list_value), Some(as_name)) => {
                let items = list_value.items();
                if !matches!(list_value, Value::List(_)) || items.is_empty() {
                    self.out(file).error(
                        codes::TYPE,
                        list_value.span(),
                        "`each=` takes a list of one or more items: `each=[2m 2.4m]`",
                    );
                }
                let kinds: Vec<Kind> = items
                    .iter()
                    .filter_map(|i| i.as_word().and_then(kind_of_word))
                    .collect();
                let kind = if kinds.len() == items.len() && !kinds.is_empty() {
                    kinds[0].clone()
                } else {
                    Kind::Word(items.iter().map(|i| i.text().to_string()).collect())
                };
                match as_name.as_word().filter(|w| is_kebab_case(w)) {
                    Some(n) => extra.push((n.to_string(), kind)),
                    None => self.out(file).error(
                        codes::NAME,
                        as_name.span(),
                        "`as=` names each item with a kebab-case name",
                    ),
                }
                extra.push(("n".into(), whole()));
            }
            (Some(list_value), None) => {
                self.out(file).error(
                    codes::TYPE,
                    list_value.span(),
                    "`each=` needs `as=`, the name each item takes in the block",
                );
            }
            (None, Some(as_name)) => {
                self.out(file).error(
                    codes::TYPE,
                    as_name.span(),
                    "`as=` names the items of `each=`, which this `use` does not give",
                );
            }
            (None, None) => {}
        }
        // Recursion that counts down (S38, S74 (5)).
        if let Some((owner, owner_comp, guards)) = within
            && self.cycles.contains(&(owner.to_string(), name.to_string()))
        {
            self.countdown_check(
                file,
                node,
                target.span(),
                owner,
                owner_comp,
                name,
                &comp,
                guards,
            );
        }
        self.placement_block(file, node, name, &comp, &extra, true);
    }

    /// A placement's or an override's block: the component's parameters by name, else its root
    /// object's own properties; `required` asks every parameter with no default.
    fn placement_block(
        &mut self,
        file: FileId,
        node: Node<'_>,
        name: &str,
        comp: &Component,
        extra: &[(String, Kind)],
        required: bool,
    ) {
        let mut scope = self.scope.clone().unwrap_or_default();
        for (n, k) in extra {
            scope.names.insert(n.clone(), k.clone());
            // `n` is a count; an item's unit is the list's, not known here.
            if n == "n" {
                scope.degrees.insert(n.clone(), 0);
            } else {
                scope.degrees.remove(n);
            }
        }
        let outer = self.scope.replace(scope);
        let object_props = self.schema.properties_of("Object");
        let mut given: HashSet<String> = HashSet::new();
        for a in attrs(node) {
            if a.node.is_none() {
                continue; // `count=` and the rest, read above
            }
            if let Some(p) = comp.params.iter().find(|p| p.name == a.name) {
                if !given.insert(a.name.clone()) {
                    self.out(file).error(
                        codes::REPEATED,
                        a.name_span,
                        format!("`{}` is given twice", a.name),
                    );
                    continue;
                }
                match a.args.as_slice() {
                    [v] => self.value_of(file, v, &p.kind, &a.name),
                    _ => self.out(file).error(
                        codes::TYPE,
                        a.span,
                        format!("`{}` takes one value", a.name),
                    ),
                }
            } else if let Some(p) = object_props.iter().find(|p| p.name == a.name) {
                self.property(file, &a, p, false);
            } else if ROOT_LINES.contains(&a.name.as_str()) {
                // The root's own stacks and slots, laid over the copy's (task 1d-03i): a line
                // replaces the root's of its handle in its place, or is added.
                match a.name.as_str() {
                    "modifier" => self.family_member(file, &a, "Modifier"),
                    "constraint" => self.family_member(file, &a, "Constraint"),
                    "bone" => self.pose_bone(file, &a),
                    _ => self.reference(file, &a, "material"),
                }
            } else {
                let mut names: Vec<&str> = comp.params.iter().map(|p| p.name.as_str()).collect();
                names.extend(ROOT_LINES);
                names.extend(object_props.iter().map(|p| p.name.as_str()));
                let near = closest(&a.name, names.iter().copied()).map(str::to_string);
                let message = match &near {
                    Some(n) => format!(
                        "`{name}` has no parameter `{}`, nor its root object a property of that name; did you mean `{n}`?",
                        a.name
                    ),
                    None => format!(
                        "`{name}` has no parameter `{}`, nor its root object a property of that name",
                        a.name
                    ),
                };
                self.out(file).error_fix(
                    codes::UNKNOWN_PROPERTY,
                    a.name_span,
                    message,
                    near.map(|n| (format!("write `{n}`"), a.name_span, n)),
                );
            }
        }
        if required {
            let missing: Vec<&str> = comp
                .params
                .iter()
                .filter(|p| p.required && !given.contains(&p.name))
                .map(|p| p.name.as_str())
                .collect();
            if !missing.is_empty() {
                let at = node.name_token().map_or(node.span(), |t| t.span);
                self.out(file).error(
                    codes::TYPE,
                    at,
                    format!(
                        "`{name}` needs {}, which {} no default",
                        list(&missing),
                        if missing.len() == 1 { "has" } else { "have" }
                    ),
                );
            }
        }
        self.scope = outer;
    }

    /// A value given for a parameter of kind `want`: plain, or an expression of the names in scope.
    pub(super) fn value_of(&mut self, file: FileId, v: &Value<'_>, want: &Kind, what: &str) {
        if let Kind::Handle(kind) = want {
            return self.handle_ref(file, v, kind, what);
        }
        let scope = self.scope.clone().unwrap_or_default();
        let got = match v {
            Value::Word(t) if kind_of_word(t.text).is_some() => kind_of_word(t.text),
            Value::Word(t)
                if matches!(want, Kind::Word(_)) && !scope.names.contains_key(t.text) =>
            {
                let Kind::Word(words) = want else {
                    unreachable!()
                };
                if words.iter().any(|w| w == t.text) {
                    return;
                }
                let near = closest(t.text, words.iter().map(String::as_str)).map(str::to_string);
                self.out(file).error_fix(
                    codes::ENUM,
                    v.span(),
                    format!(
                        "`{what}` takes one of {}, not `{}`",
                        list(&words.iter().map(String::as_str).collect::<Vec<_>>()),
                        t.text
                    ),
                    near.map(|n| (format!("write `{n}`"), v.span(), n)),
                );
                return;
            }
            Value::Word(t) => match expr::parse(t.text, false)
                .and_then(|e| expr::kind_of(&e, &|n| scope.names.get(n).cloned()))
            {
                Ok(k) => Some(k),
                Err(e) => {
                    let fix = self
                        .name_fix(&e, &scope, v.span(), t.text)
                        .map(|(w, s, r)| (w, s, r.trim_matches('"').to_string()));
                    self.out(file).error_fix(
                        codes::TYPE,
                        v.span(),
                        format!("`{}`: {e}", t.text),
                        fix,
                    );
                    return;
                }
            },
            Value::Str(_) | Value::Raw(_) => {
                let text = v.as_string().unwrap_or_default();
                match expr::parse(&text, true)
                    .and_then(|e| expr::kind_of(&e, &|n| scope.names.get(n).cloned()))
                {
                    Ok(k) => Some(k),
                    Err(e) => {
                        let fix = self.name_fix(&e, &scope, v.span(), &text);
                        self.out(file).error_fix(
                            codes::TYPE,
                            v.span(),
                            format!("`{text}`: {e}"),
                            fix,
                        );
                        return;
                    }
                }
            }
            _ => None,
        };
        // A bare number is in the dimension's internal unit (section 6), as anywhere a value goes.
        let bare = matches!(v, Value::Word(t) if matches!(kind_of_word(t.text), Some(Kind::Quantity { dim, .. }) if dim == Dim::NONE));
        match got {
            Some(k) if fits(&k, want) => {}
            Some(Kind::Quantity { whole, .. })
                if bare && matches!(want, Kind::Quantity { whole: w, .. } if whole || !*w) => {}
            Some(k) => self.out(file).error(
                codes::TYPE,
                v.span(),
                format!(
                    "`{what}` takes {}, and `{}` is {}",
                    want.describe(),
                    v.text(),
                    k.describe()
                ),
            ),
            None => self.out(file).error(
                codes::TYPE,
                v.span(),
                format!("`{what}` takes {}, not `{}`", want.describe(), v.text()),
            ),
        }
    }

    /// A placement of `target`, inside `owner`, where the two place each other (S38, S74 (5)):
    /// it passes `target`'s countdown parameter a smaller value than `owner`'s, under an `if`
    /// that holds only while `owner`'s is above 0.
    #[allow(clippy::too_many_arguments)]
    fn countdown_check(
        &mut self,
        file: FileId,
        node: Node<'_>,
        at: Span,
        owner: &str,
        owner_comp: &Component,
        target: &str,
        target_comp: &Component,
        guards: &[expr::Condition],
    ) {
        let (Some(mine), Some(theirs)) = (
            owner_comp.params.iter().find(|p| p.countdown),
            target_comp.params.iter().find(|p| p.countdown),
        ) else {
            let who = if owner_comp.params.iter().any(|p| p.countdown) {
                target
            } else {
                owner
            };
            self.out(file).error(
                codes::TYPE,
                at,
                format!("`{owner}` places `{target}`, which places `{owner}` again: recursion counts down, so `{who}` needs a whole-number parameter marked `{{ countdown }}`"),
            );
            return;
        };
        // The value given: `"<mine> - k"`, k a positive whole number.
        let given = attrs(node)
            .into_iter()
            .find(|a| a.name == theirs.name && a.node.is_some());
        let smaller = given
            .as_ref()
            .and_then(|a| a.args.first())
            .and_then(|v| v.as_string())
            .and_then(|t| expr::parse(&t, true).ok())
            .is_some_and(|e| {
                matches!(&e, Expr::Op('-', a, b)
                if **a == Expr::Name(mine.name.clone())
                    && matches!(**b, Expr::Number { dim, whole: true, .. } if dim == Dim::NONE))
            });
        if !smaller {
            let span = given.as_ref().map_or(at, |a| a.span);
            self.out(file).error_fix(
                codes::TYPE,
                span,
                format!("`{target}` is placed again inside itself, so it counts down: give `{}` a smaller value than `{}`, such as `\"{} - 1\"`", theirs.name, mine.name, mine.name),
                given.as_ref().map(|a| (format!("write `{} \"{} - 1\"`", theirs.name, mine.name), a.span, format!("{} \"{} - 1\"", theirs.name, mine.name))),
            );
        }
        let guarded = guards.iter().any(|c| {
            let name = Expr::Name(mine.name.clone());
            let bound = |e: &Expr| matches!(e, Expr::Number { dim, .. } if *dim == Dim::NONE);
            c.left == name && bound(&c.right) && matches!(c.op, ">" | ">=")
        });
        if !guarded {
            self.out(file).error(
                codes::TYPE,
                at,
                format!("`{target}` is placed again inside itself only under a condition that ends it: put this `use` in `if \"{} > 0\" {{ … }}`", mine.name),
            );
        }
    }

    /// `override <path> { … }` (S33): the copy, or a datablock a copy made, and what it changes.
    /// An override's lines for a sub-object a copy made (`override @rig.2/@spine/@reach { … }`,
    /// task 1d-03i), checked as that sub-object's own lines: a modifier's or a constraint's by
    /// its kind's struct, a pose bone's by `PoseBone`'s. Other sub-objects' lines are left to
    /// the expanded project's check.
    fn sub_override(&mut self, file: FileId, node: Node<'_>, word: &str, of: Option<&str>) {
        let schema = self.schema;
        let (owner, rna) = match (word, of) {
            ("bone", _) => ("pose-bone".to_string(), "PoseBone".to_string()),
            ("modifier" | "constraint", Some(kind)) => {
                let base = if word == "modifier" {
                    "Modifier"
                } else {
                    "Constraint"
                };
                let Some(st) = schema
                    .structs
                    .iter()
                    .find(|st| st.kind == kind && st.base.as_deref() == Some(base))
                else {
                    return; // the copy's own line says so
                };
                (format!("{kind} {word}"), st.rna.clone())
            }
            _ => return,
        };
        let props: Vec<&crate::schema::Property> = schema
            .properties_of(&rna)
            .into_iter()
            .filter(|p| {
                !matches!(
                    p.name.as_str(),
                    "name" | "type" | "persistent-uid" | "is-active" | "active"
                )
            })
            .collect();
        self.attributes(file, node, &owner, &props, &[]);
    }

    pub(super) fn override_line(&mut self, file: FileId, node: Node<'_>) {
        let args = args_of(node);
        let at = node.name_token().map_or(node.span(), |t| t.span);
        let [target] = args.as_slice() else {
            self.out(file).error(
                codes::TYPE,
                at,
                "`override` names one copy by its path: `override @plank.7 { … }`",
            );
            return;
        };
        let word = target.text();
        match self.copy_path(word) {
            Ok(Reached::Copy(name)) => {
                let comp = self.components.get(&name).cloned().unwrap_or_default();
                self.placement_block(file, node, &name, &comp, &[], false);
            }
            Ok(Reached::Datablock(kind)) => {
                let rna = self
                    .schema
                    .struct_by_kind(&kind)
                    .map(|s| s.rna.clone())
                    .unwrap_or_default();
                let props = self.schema.properties_of(&rna);
                self.attributes(file, node, &kind, &props, &[]);
            }
            Ok(Reached::Sub { word, of }) => self.sub_override(file, node, &word, of.as_deref()),
            Err(message) => self.out(file).error(codes::HANDLE, target.span(), message),
        }
    }

    /// What a path through copies reaches: `@plank.7` a copy of `plank`, `@plank.7/@plank-mesh`
    /// a mesh it made, `@tree/@left/@left` a copy inside a copy. An error says why it reaches
    /// nothing.
    pub(super) fn copy_path(&self, word: &str) -> Result<Reached, String> {
        let steps: Vec<&str> = word.split('/').collect();
        let (first, index) = split_copy(steps[0]);
        let Some(p) = self.placements.get(first) else {
            return Err(
                match closest(first, self.placements.keys().map(String::as_str)) {
                    Some(n) => format!("no placement is called `{first}`; did you mean `{n}`?"),
                    None => format!("no placement is called `{first}`"),
                },
            );
        };
        let mut at = self.check_copy(p, first, index)?;
        let mut rest = &steps[1..];
        while let Some((step, after)) = rest.split_first() {
            let comp = self.components.get(&at).cloned().unwrap_or_default();
            let (h, index) = split_copy(step);
            if let Some(inner) = comp.places.iter().find(|q| q.handle == h) {
                at = self.check_copy(inner, h, index)?;
                rest = after;
                continue;
            }
            let root_subs = comp
                .root
                .as_ref()
                .and_then(|r| comp.subs.get(r))
                .cloned()
                .unwrap_or_default();
            if index.is_none() {
                if let Some((kind, _)) = comp.locals.iter().find(|(_, l)| l == h) {
                    let own = comp.subs.get(h).cloned().unwrap_or_default();
                    return if after.is_empty() {
                        Ok(Reached::Datablock(kind.clone()))
                    } else {
                        sub_path(&format!("{kind} `{at}/{h}`"), &own, after)
                    };
                }
                if root_subs.iter().any(|s| s.handle == h) {
                    return sub_path(&format!("a copy of `{at}`"), &root_subs, rest);
                }
            }
            let mut names: Vec<&str> = comp.locals.iter().map(|(_, l)| l.as_str()).collect();
            names.extend(comp.places.iter().map(|q| q.handle.as_str()));
            names.extend(root_subs.iter().map(|s| s.handle.as_str()));
            return Err(match closest(h, names.iter().copied()) {
                Some(n) => format!("a copy of `{at}` makes no `{h}`; did you mean `{n}`?"),
                None => format!("a copy of `{at}` makes no `{h}`"),
            });
        }
        Ok(Reached::Copy(at))
    }

    /// A step naming a placement's copy: its index where it is a pattern, none where it is not.
    fn check_copy(&self, p: &Placed, h: &str, index: Option<usize>) -> Result<String, String> {
        match (p.copies, index) {
            (None, None) => Ok(p.component.clone()),
            (None, Some(_)) => Err(format!("`{h}` is one copy, not a pattern: write `{h}`")),
            (Some(_), None) => Err(format!(
                "`{h}` is a pattern: name one of its copies, such as `{h}.1`"
            )),
            (Some(Some(n)), Some(k)) if k == 0 || k > n => Err(format!(
                "`{h}` has copies `{h}.1` to `{h}.{n}`, not `{h}.{k}`"
            )),
            (Some(_), Some(0)) => Err(format!("`{h}`'s copies count from 1: `{h}.1`")),
            (Some(_), Some(_)) => Ok(p.component.clone()),
        }
    }
}

/// What a path through copies reaches.
pub(crate) enum Reached {
    /// A copy of the component named.
    Copy(String),
    /// A datablock of the kind named, made by a copy.
    Datablock(String),
    /// A sub-object a copy made: its line's word and kind (`constraint`, `kinematic`).
    Sub { word: String, of: Option<String> },
}

/// The sub-object `steps` reach among `subs`, those of `owner` (as a message names it), each step
/// one level down (`@spine/@reach`: the bone, then its constraint); a step that names nothing
/// is an error with the nearest of that level.
fn sub_path(owner: &str, subs: &[Sub], steps: &[&str]) -> Result<Reached, String> {
    let (mut level, mut what) = (subs, owner.to_string());
    let mut reached = None;
    for step in steps {
        let Some(s) = level.iter().find(|s| s.handle == *step) else {
            let names: Vec<&str> = level.iter().map(|s| s.handle.as_str()).collect();
            return Err(match closest(step, names.iter().copied()) {
                Some(n) => format!("{what} has no sub-object `{step}`; did you mean `{n}`?"),
                None if names.is_empty() => format!("{what} has no sub-objects, so no `{step}`"),
                None => format!(
                    "{what} has no sub-object `{step}`: it has {}",
                    crate::values::list(&names)
                ),
            });
        };
        what = format!("the {} `{}`", s.word, s.handle);
        level = &s.subs;
        reached = Some(s);
    }
    let s = reached.expect("a sub-object path has a step");
    Ok(Reached::Sub {
        word: s.word.clone(),
        of: s.of.clone(),
    })
}

/// `@plank.7` as (`@plank`, 7); `@plank` as (`@plank`, none).
fn split_copy(step: &str) -> (&str, Option<usize>) {
    match step.rsplit_once('.') {
        Some((h, n)) if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) => {
            (h, n.parse().ok())
        }
        _ => (step, None),
    }
}

/// A `use` line's component, handle and copies, as written.
fn placed(node: Node<'_>) -> Option<Placed> {
    let args = args_of(node);
    let component = args.first()?.as_word()?.to_string();
    let handle = args.get(1)?.as_word()?.to_string();
    let props = props_of(node);
    let count = props.iter().find(|(k, _)| k.text == "count").map(|(_, v)| {
        v.as_word()
            .and_then(|w| null_value::read_int(w).ok())
            .and_then(|n| usize::try_from(n).ok())
    });
    let each = props
        .iter()
        .find(|(k, _)| k.text == "each")
        .map(|(_, v)| Some(v.items().len()));
    Some(Placed {
        component,
        handle,
        copies: count.or(each),
    })
}

/// The sub-objects defined in a datablock's or a sub-object's block, conditions included, each
/// with its own; a node tree's nodes are its owner's, as section 7 has them.
fn subs(node: Node<'_>, out: &mut Vec<Sub>) {
    for child in node.children() {
        let word = child.name().into_owned();
        match word.as_str() {
            "if" | "else" | "nodes" => subs(child, out),
            _ => {
                let args = args_of(child);
                let Some(h) = args
                    .first()
                    .and_then(|v| v.as_word())
                    .filter(|w| w.starts_with('@'))
                else {
                    continue;
                };
                if !(crate::paths::SUB_KINDS.contains(&word.as_str())
                    || matches!(word.as_str(), "node" | "zone"))
                {
                    continue;
                }
                let mut inner = Vec::new();
                if word == "zone" {
                    // A zone's nodes are the tree's, beside it.
                    subs(child, out);
                } else {
                    subs(child, &mut inner);
                }
                out.push(Sub {
                    of: args.get(1).and_then(|v| v.as_word()).map(str::to_string),
                    word,
                    handle: h.to_string(),
                    subs: inner,
                });
            }
        }
    }
}

/// Whether the word is a path through a copy (a `.n` step, or `/` steps from a placement).
pub(crate) fn is_copy_path(word: &str) -> bool {
    word.starts_with('@') && word.split('/').any(|s| split_copy(s).1.is_some())
}

/// An unknown name that is a known name, `-`, then more (`n-1`): subtraction written inside a
/// word, where `-` is part of a name. The message and the quoted fix.
pub(crate) fn dash_hint(missing: &str, scope: &Scope) -> Option<(String, String)> {
    let (head, tail) = missing.split_once('-')?;
    if !scope.names.contains_key(head) || tail.is_empty() {
        return None;
    }
    let fixed = format!("\"{head} - {tail}\"");
    Some((
        format!(
            "`{missing}` is one name, since `-` inside a word is part of a name; write subtraction in quotes, with spaces: {fixed}"
        ),
        fixed,
    ))
}

/// What a value checked through `values` is to be, where it may be an expression.
pub(crate) enum Want {
    /// A number of this dimension.
    Dim(Dimension),
    /// A whole number.
    Whole,
    /// `on` or `off`.
    Bool,
}

/// A value written as an expression of the names in scope (task 1d-03b), checked for the kind
/// `want`: true when it was one, whatever its errors, so the plain check is not run on it too.
pub(crate) fn expression(
    value: &Value<'_>,
    name: &str,
    want: Want,
    out: &mut crate::values::Out<'_>,
) -> bool {
    let Some(scope) = out.scope else {
        return false;
    };
    let (text, quoted) = match value {
        Value::Word(t) if expr::is_expression(t.text) => (t.text.to_string(), false),
        Value::Str(_) | Value::Raw(_) if !matches!(want, Want::Bool) => {
            (value.as_string().unwrap_or_default().into_owned(), true)
        }
        _ => return false,
    };
    if matches!(want, Want::Bool) && !scope.names.contains_key(text.as_str()) {
        return false;
    }
    let read = expr::parse(&text, quoted)
        .and_then(|e| expr::kind_of(&e, &|n| scope.names.get(n).cloned()));
    let got = match read {
        Ok(k) => k,
        Err(e) => {
            let missing = e
                .strip_prefix("no parameter is called `")
                .and_then(|m| m.strip_suffix('`'));
            if !quoted
                && missing == Some(text.as_str())
                && let Some((message, fixed)) = dash_hint(&text, scope)
            {
                out.error_fix(
                    codes::TYPE,
                    value.span(),
                    message,
                    Some((format!("write {fixed}"), value.span(), fixed)),
                );
                return true;
            }
            let fix = missing.and_then(|missing| {
                let near = closest(missing, scope.names.keys().map(String::as_str))?;
                let fixed = text.replacen(missing, near, 1);
                let fixed = if quoted {
                    format!("\"{fixed}\"")
                } else {
                    fixed
                };
                Some((format!("write `{fixed}`"), value.span(), fixed))
            });
            out.error_fix(codes::TYPE, value.span(), format!("`{text}`: {e}"), fix);
            return true;
        }
    };
    // Degrees and radians in one sum, or degrees left where no angle is, as the expansion
    // refuses them (task 1d-03i).
    let units = expr::parse(&text, quoted)
        .and_then(|e| expr::degrees(&e, &|n| scope.degrees.get(n).copied()));
    let stray = match (&units, &got) {
        (Ok(Some(d)), Kind::Quantity { dim, .. }) => {
            *d != 0 && !(*d == 1 && *dim == Dim::of(Dimension::Angle))
        }
        _ => false,
    };
    if let Err(e) = units {
        out.error(codes::TYPE, value.span(), format!("`{text}`: {e}"));
        return true;
    }
    if stray {
        out.error(
            codes::TYPE,
            value.span(),
            format!(
                "`{text}` is {} with degrees in it, which has no exact form: write its angles in one unit",
                got.describe()
            ),
        );
        return true;
    }
    let ok = match (&want, &got) {
        (Want::Dim(d), Kind::Quantity { dim, .. }) => dim.fits(*d),
        (Want::Whole, Kind::Quantity { dim, whole }) => *whole && *dim == Dim::NONE,
        (Want::Bool, Kind::Bool) => true,
        _ => false,
    };
    if !ok {
        let wanted = match want {
            Want::Dim(d) => Kind::number(Dim::of(d)).describe(),
            Want::Whole => "a whole number".into(),
            Want::Bool => "`on` or `off`".into(),
        };
        out.error(
            codes::TYPE,
            value.span(),
            format!(
                "`{name}` takes {wanted}, and `{text}` is {}",
                got.describe()
            ),
        );
    }
    true
}
