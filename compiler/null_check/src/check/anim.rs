//! Animation (spec, section 11; task 1d-07a): actions with their slots, layers, strips,
//! f-curves and keys; what plays on a datablock (`action @a slot=@s`, NLA `track` lines); and
//! drivers, written on the property they drive.

use std::collections::{HashMap, HashSet};

use null_syntax::ast::{Node, Value};
use null_syntax::{FileId, Span, closest};

use super::{Attr, Checker, args_of, article, attrs, header, kebab_handle};
use crate::codes;
use crate::schema::Property;
use crate::values::{check_enum, check_float, list, list_near};

/// An action's slots, by handle, with the kind of datablock each animates.
pub(super) type Slots = Vec<(String, String)>;

/// Every action's slots in the project, read before any datablock is checked.
pub(super) fn slots(node: Node<'_>, into: &mut HashMap<String, Slots>) {
    let Some((handle, _)) = header(node).handle else {
        return;
    };
    let list = node
        .children()
        .filter(|c| c.name() == "slot")
        .filter_map(|c| {
            let args = args_of(c);
            Some((
                args.first()?.text().to_string(),
                args.get(1)?.text().to_string(),
            ))
        })
        .collect();
    into.insert(handle, list);
}

/// Properties of an f-curve its line writes, or Blender keeps for itself.
const FCURVE_LINE: &[&str] = &[
    "driver",
    "group",
    "data-path",
    "array-index",
    "select",
    "is-valid",
    "is-empty",
    "sampled-points",
    "keyframe-points",
    "modifiers",
];

/// A key's properties a key line may give after its frame and value.
const KEY_SETTINGS: &[&str] = &[
    "handle-left-type",
    "handle-right-type",
    "interpolation",
    "type",
    "easing",
    "back",
    "amplitude",
    "period",
    "handle-left",
    "handle-right",
];

/// The struct a slot of datablock kind `id_type` animates: the datablock's, with its subtypes'
/// properties too (a light slot's `energy` is a point light's).
fn animated<'s>(cx: &Checker<'s>, id_type: &str) -> Vec<&'s Property> {
    let schema = cx.schema;
    let Some(s) = schema.datablock_kinds().find(|s| s.kind == id_type) else {
        return Vec::new();
    };
    let mut props = schema.properties_of(&s.rna);
    for sub in schema
        .structs
        .iter()
        .filter(|t| t.base.as_deref() == Some(s.rna.as_str()))
    {
        props.extend(sub.properties.iter());
    }
    props
}

impl Checker<'_> {
    /// An action's own lines: `slot`, `layer`, and `Action`'s properties.
    pub(super) fn action_body(&mut self, file: FileId, node: Node<'_>, props: &[&Property]) {
        let schema = self.schema;
        let all = attrs(node);
        let layered = all.iter().any(|a| a.name == "layer");
        let slot_items: Vec<String> = schema
            .properties_of("ActionSlot")
            .into_iter()
            .find(|p| p.name == "target-id-type")
            .map(|p| p.item_names().into_iter().map(str::to_string).collect())
            .unwrap_or_default();
        let mut slots: HashMap<String, String> = HashMap::new();
        let mut names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
        names.extend(["slot", "layer"]);
        let mut seen = HashSet::new();
        for a in &all {
            match a.name.as_str() {
                "slot" => {
                    let [h, t, rest @ ..] = a.args.as_slice() else {
                        self.out(file).error(codes::TYPE, a.span, "a slot is written `slot @handle <kind> \"Name\"`, such as `slot @lamp light \"Lamp\"`");
                        continue;
                    };
                    if !h.as_word().is_some_and(|w| w.starts_with('@')) {
                        self.out(file).error(
                            codes::TYPE,
                            h.span(),
                            "a slot's handle comes first, such as `slot @lamp light`",
                        );
                        continue;
                    }
                    kebab_handle(h.text(), h.span(), &mut self.out(file));
                    if !seen.insert(h.text().to_string()) {
                        self.out(file).error(
                            codes::HANDLE,
                            h.span(),
                            format!("`{}` names two slots of this action", h.text()),
                        );
                    }
                    let ty = t.text();
                    if !slot_items.iter().any(|i| i == ty) {
                        let near = closest(ty, slot_items.iter().map(String::as_str));
                        self.out(file).error_fix(
                            codes::ENUM,
                            t.span(),
                            format!("`{ty}` is not a kind of datablock a slot animates"),
                            near.map(|n| (format!("write `{n}`"), t.span(), n.to_string())),
                        );
                    }
                    for (i, v) in rest.iter().enumerate() {
                        if i == 0 && matches!(v, Value::Str(_) | Value::Raw(_)) {
                            continue;
                        }
                        self.out(file).error(
                            codes::TYPE,
                            v.span(),
                            "a slot's line is `slot @handle <kind> \"Name\"`",
                        );
                    }
                    slots.insert(h.text().to_string(), ty.to_string());
                    if let Some(block) = a.node {
                        let curves: Vec<Attr<'_>> = attrs(block);
                        if layered && curves.iter().any(|c| c.name == "fcurve") {
                            self.out(file).error(
                                codes::TYPE,
                                curves.iter().find(|c| c.name == "fcurve").unwrap().name_span,
                                "an action with `layer` lines holds its f-curves in its strips' `channels` blocks, not in its slots",
                            );
                            continue;
                        }
                        self.curves(file, &curves, ty, "a slot");
                    }
                }
                "layer" => {}
                _ => match props.iter().find(|p| p.name == a.name) {
                    Some(p) => self.property(file, a, p, false),
                    None => self.unknown(file, a, "an action", &names, None),
                },
            }
        }
        for a in all.iter().filter(|a| a.name == "layer") {
            self.layer(file, a, &slots);
        }
    }

    /// `layer @h "Name" { strip @s keyframe { channels @slot { fcurve … } } }`.
    fn layer(&mut self, file: FileId, a: &Attr<'_>, slots: &HashMap<String, String>) {
        if !self.member_header(file, a, "layer @base \"Base\"", false) {
            return;
        }
        let Some(block) = a.node else { return };
        let schema = self.schema;
        let props: Vec<&Property> = schema
            .properties_of("ActionLayer")
            .into_iter()
            .filter(|p| p.name != "name")
            .collect();
        let mut names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
        names.push("strip");
        for b in attrs(block) {
            if b.name != "strip" {
                match props.iter().find(|p| p.name == b.name) {
                    Some(p) => self.property(file, &b, p, false),
                    None => self.unknown(file, &b, "a layer", &names, None),
                }
                continue;
            }
            // `strip @h <type>`: the type is `ActionStrip`'s (`keyframe`).
            let types: Vec<String> = schema
                .properties_of("ActionStrip")
                .into_iter()
                .find(|p| p.name == "type")
                .map(|p| p.item_names().into_iter().map(str::to_string).collect())
                .unwrap_or_default();
            let [h, t] = b.args.as_slice() else {
                self.out(file).error(
                    codes::TYPE,
                    b.span,
                    "a strip is written `strip @handle <type>`, such as `strip @keys keyframe`",
                );
                continue;
            };
            kebab_handle(h.text(), h.span(), &mut self.out(file));
            if !types.iter().any(|x| x == t.text()) {
                let shown: Vec<&str> = types.iter().map(String::as_str).collect();
                self.out(file).error(
                    codes::ENUM,
                    t.span(),
                    format!(
                        "`{}` is not a kind of action strip: {}",
                        t.text(),
                        list_near(t.text(), &shown)
                    ),
                );
                continue;
            }
            let Some(strip) = b.node else { continue };
            for c in attrs(strip) {
                if c.name != "channels" {
                    self.unknown(file, &c, "a keyframe strip", &["channels"], None);
                    continue;
                }
                let [s] = c.args.as_slice() else {
                    self.out(file).error(
                        codes::TYPE,
                        c.span,
                        "`channels` names the slot they animate, such as `channels @lamp`",
                    );
                    continue;
                };
                let Some(ty) = slots.get(s.text()) else {
                    let near =
                        closest(s.text(), slots.keys().map(String::as_str)).map(str::to_string);
                    self.out(file).error_fix(
                        codes::HANDLE,
                        s.span(),
                        format!("this action has no slot `{}`", s.text()),
                        near.map(|n| (format!("write `{n}`"), s.span(), n)),
                    );
                    continue;
                };
                if let Some(n) = c.node {
                    let curves = attrs(n);
                    self.curves(file, &curves, &ty.clone(), "a strip's channels");
                }
            }
        }
    }

    /// A handle and an optional quoted name on a member's line; false when it has no handle.
    fn member_header(&mut self, file: FileId, a: &Attr<'_>, example: &str, kind: bool) -> bool {
        let skip = if kind { 2 } else { 1 };
        match a.args.first() {
            Some(h) if h.as_word().is_some_and(|w| w.starts_with('@')) && a.args.len() >= skip => {
                kebab_handle(h.text(), h.span(), &mut self.out(file));
                for (i, v) in a.args.iter().enumerate().skip(skip) {
                    if i == skip && matches!(v, Value::Str(_) | Value::Raw(_)) {
                        continue;
                    }
                    self.out(file).error(
                        codes::TYPE,
                        v.span(),
                        format!("`{}`'s line is `{example}`, then settings", a.name),
                    );
                }
                true
            }
            _ => {
                self.out(file).error(
                    codes::TYPE,
                    a.span,
                    format!("`{}` is written `{example}`", a.name),
                );
                false
            }
        }
    }

    /// `fcurve <path> index=N { settings; key lines; modifier lines }` for a slot of datablock
    /// kind `id_type`.
    fn curves(&mut self, file: FileId, curves: &[Attr<'_>], id_type: &str, owner: &str) {
        let schema = self.schema;
        let targets = animated(self, id_type);
        let fprops: Vec<&Property> = schema
            .properties_of("FCurve")
            .into_iter()
            .filter(|p| !FCURVE_LINE.contains(&p.name.as_str()))
            .collect();
        let kprops: Vec<&Property> = schema
            .properties_of("Keyframe")
            .into_iter()
            .filter(|p| KEY_SETTINGS.contains(&p.name.as_str()))
            .collect();
        let mut seen = HashSet::new();
        for c in curves {
            if c.name != "fcurve" {
                self.unknown(file, c, owner, &["fcurve"], None);
                continue;
            }
            // The path: a property of the animated datablock, or a sub-object path and a
            // property of it (`@bevel width`, `@bsdf.roughness`), checked when the slot's
            // users are known; their keys are written bare.
            let (target, path_end): (Option<&Property>, usize) = match c.args.as_slice() {
                [p, ..] if p.as_word().is_some_and(|w| w.starts_with('@')) => {
                    let n = if p.text().contains('.') { 1 } else { 2 };
                    (None, n)
                }
                [p, ..] => match targets.iter().copied().find(|t| t.name == p.text()) {
                    Some(t) => (Some(t), 1),
                    None => {
                        let names: Vec<&str> = targets.iter().map(|t| t.name.as_str()).collect();
                        let near = closest(p.text(), names.iter().copied());
                        self.out(file).error_fix(
                            codes::UNKNOWN_PROPERTY,
                            p.span(),
                            format!("{} has no `{}` to animate", article(id_type), p.text()),
                            near.map(|n| (format!("write `{n}`"), p.span(), n.to_string())),
                        );
                        continue;
                    }
                },
                [] => {
                    self.out(file).error(
                        codes::TYPE,
                        c.span,
                        "an f-curve names the property it animates, such as `fcurve energy`",
                    );
                    continue;
                }
            };
            for v in c.args.iter().skip(path_end) {
                self.out(file).error(
                    codes::TYPE,
                    v.span(),
                    "an f-curve's line is `fcurve <path>`, then `index=` for an array's item",
                );
            }
            let index = c
                .props
                .iter()
                .find(|(k, _)| k.text == "index")
                .map(|(_, v)| v);
            let len = target.and_then(|t| t.fixed_len());
            match (index, len) {
                (Some(v), Some(n)) => match v.text().parse::<usize>() {
                    Ok(i) if i < n => {}
                    _ => self.out(file).error(
                        codes::RANGE,
                        v.span(),
                        format!(
                            "`index` is an item of `{}`, 0 to {}",
                            c.args[0].text(),
                            n - 1
                        ),
                    ),
                },
                (None, Some(_)) => self.out(file).error(
                    codes::TYPE,
                    c.name_span,
                    format!(
                        "`{}` is an array: say which item with `index=`, such as `index=2`",
                        c.args[0].text()
                    ),
                ),
                (Some(v), None) if target.is_some() => self.out(file).error(
                    codes::TYPE,
                    v.span(),
                    format!(
                        "`{}` is not an array: it takes no `index`",
                        c.args[0].text()
                    ),
                ),
                _ => {}
            }
            for (k, _) in c.props.iter().filter(|(k, _)| k.text != "index") {
                self.out(file).error(
                    codes::TYPE,
                    k.span,
                    "an f-curve's line takes `index=` only; its settings are lines in its block",
                );
            }
            let key = (
                c.args[0].text().to_string(),
                index.map(|v| v.text().to_string()),
            );
            if !seen.insert(key) {
                self.out(file)
                    .error(codes::REPEATED, c.span, "this property is animated twice");
            }
            let Some(block) = c.node else { continue };
            let mut names: Vec<&str> = fprops.iter().map(|p| p.name.as_str()).collect();
            names.extend(["key", "modifier"]);
            // The block's lines only: `index=` on the f-curve's own line is its path's.
            for b in attrs(block).into_iter().filter(|b| b.node.is_some()) {
                match b.name.as_str() {
                    "key" => self.key(file, &b, target, &kprops),
                    "modifier" => self.family_member(file, &b, "FModifier"),
                    _ => match fprops.iter().find(|p| p.name == b.name) {
                        Some(p) => self.property(file, &b, p, false),
                        None => self.unknown(file, &b, "an f-curve", &names, None),
                    },
                }
            }
        }
    }

    /// `key <frame> <value> interpolation=… handle-left=[f v] …`: frames bare (an action is
    /// outside every scene), the value in the animated property's unit.
    fn key(&mut self, file: FileId, a: &Attr<'_>, target: Option<&Property>, kprops: &[&Property]) {
        let [frame, value, rest @ ..] = a.args.as_slice() else {
            return self.out(file).error(
                codes::TYPE,
                a.span,
                "a key is written `key <frame> <value>`, such as `key 1 800W`",
            );
        };
        for v in rest {
            self.out(file).error(
                codes::TYPE,
                v.span(),
                "a key's line is `key <frame> <value>`, then its settings as `name=value`",
            );
        }
        let mut out = self.out(file);
        check_float(
            frame,
            "a key's frame",
            null_value::Dimension::None,
            &Property::any(),
            &mut out,
        );
        match target {
            Some(t) if t.ty.starts_with("float") => {
                let single = Property {
                    array: None,
                    ..t.clone()
                };
                let dim = crate::values::dimension(&single, t.name == "energy");
                check_float(value, &t.name, dim, &single, &mut out);
            }
            _ => check_float(
                value,
                "a key's value",
                null_value::Dimension::None,
                &Property::any(),
                &mut out,
            ),
        }
        let names: Vec<&str> = kprops.iter().map(|p| p.name.as_str()).collect();
        for (k, v) in &a.props {
            let Some(p) = kprops.iter().find(|p| p.name == k.text) else {
                let near = closest(k.text, names.iter().copied());
                out.error_fix(
                    codes::UNKNOWN_PROPERTY,
                    k.span,
                    format!("a key has no `{}`", k.text),
                    near.map(|n| (format!("write `{n}`"), k.span, n.to_string())),
                );
                continue;
            };
            match p.ty.as_str() {
                "enum" => check_enum(v, &p.name, p, &mut out),
                _ if p.fixed_len().is_some() => crate::values::check_float_array(
                    v,
                    2,
                    &p.name,
                    null_value::Dimension::None,
                    &Property::any(),
                    &mut out,
                ),
                _ => check_float(v, &p.name, null_value::Dimension::None, p, &mut out),
            }
        }
        // A handle beside an automatic handle type, the type left out (`auto-clamped`, a new
        // key's) or written as `auto`, `auto-clamped` or `vector`, is placed again by Blender,
        // so it does not hold. Written handles do not make a key's handles free (S87).
        for (side, ty) in [
            ("handle-left", "handle-left-type"),
            ("handle-right", "handle-right-type"),
        ] {
            let Some((k, _)) = a.props.iter().find(|(k, _)| k.text == side) else {
                continue;
            };
            let written = a.props.iter().find(|(k, _)| k.text == ty);
            let word = match written {
                Some((_, v)) => v.as_word(),
                None => Some("auto-clamped"),
            };
            let Some(word @ ("auto" | "auto-clamped" | "vector")) = word else {
                continue;
            };
            let fix = match written {
                Some((_, v)) => (format!("write `{ty}=free`"), v.span(), "free".to_string()),
                None => (
                    format!("write `{ty}=free`"),
                    Span::new(k.span.start, k.span.start),
                    format!("{ty}=free "),
                ),
            };
            out.warning_fix(
                codes::HANDLE_PLACED,
                k.span,
                format!(
                    "Blender places a key's `{side}` itself while its `{ty}` is `{word}`, so the one written here does not hold; make the type `free` (or `aligned`) to keep it"
                ),
                fix,
            );
        }
    }

    /// `action @a slot=@s`: what plays on a datablock of `kind`, its slot one of the action's
    /// for that kind of datablock; `slot=` may be left out when the action has one slot.
    pub(super) fn plays(&mut self, file: FileId, a: &Attr<'_>, kind: &str) {
        let [v] = a.args.as_slice() else {
            return self.out(file).error(
                codes::TYPE,
                a.span,
                "`action` takes an action's handle, such as `action @walk slot=@rig`",
            );
        };
        self.handle_ref(file, v, "action", "action");
        let slot = a
            .props
            .iter()
            .find(|(k, _)| k.text == "slot")
            .map(|(_, v)| v);
        for (k, _) in a.props.iter().filter(|(k, _)| k.text != "slot") {
            self.out(file)
                .error(codes::TYPE, k.span, "`action` takes `slot=` only");
        }
        self.slot_ref(file, v, slot, kind, a.span);
    }

    fn slot_ref(
        &mut self,
        file: FileId,
        action: &Value<'_>,
        slot: Option<&Value<'_>>,
        kind: &str,
        span: Span,
    ) {
        let Some(slots) = self.actions.get(action.text()).cloned() else {
            return; // no such action: reported by the handle
        };
        let fits = |ty: &str| {
            ty == kind
                || self.schema.struct_by_kind(kind).is_some_and(|s| {
                    s.base
                        .as_deref()
                        .and_then(|b| self.schema.struct_by_rna(b))
                        .is_some_and(|b| b.kind == ty)
                })
        };
        match slot {
            Some(s) => match slots.iter().find(|(h, _)| h == s.text()) {
                Some((_, ty)) if fits(ty) => {}
                Some((_, ty)) => self.out(file).error(
                    codes::HANDLE,
                    s.span(),
                    format!(
                        "slot `{}` animates {}, not {}",
                        s.text(),
                        article(ty),
                        article(kind)
                    ),
                ),
                None => {
                    let near = closest(s.text(), slots.iter().map(|(h, _)| h.as_str()))
                        .map(str::to_string);
                    self.out(file).error_fix(
                        codes::HANDLE,
                        s.span(),
                        format!("`{}` has no slot `{}`", action.text(), s.text()),
                        near.map(|n| (format!("write `{n}`"), s.span(), n)),
                    );
                }
            },
            None if slots.len() == 1 => {
                if !fits(&slots[0].1) {
                    self.out(file).error(
                        codes::HANDLE,
                        action.span(),
                        format!(
                            "`{}`'s one slot animates a {}, not a {kind}",
                            action.text(),
                            slots[0].1
                        ),
                    );
                }
            }
            None => self.out(file).error(
                codes::TYPE,
                span,
                format!(
                    "`{}` has {} slots: say which with `slot=`",
                    action.text(),
                    slots.len()
                ),
            ),
        }
    }

    /// `track @h "Name" { strip @s "Name" { action @a slot=@s; frame-start 1; … } }`: an NLA
    /// track of the datablock, of `kind`.
    pub(super) fn track(&mut self, file: FileId, a: &Attr<'_>, kind: &str) {
        if !self.member_header(file, a, "track @base \"Base\"", false) {
            return;
        }
        let Some(block) = a.node else { return };
        let schema = self.schema;
        let tprops: Vec<&Property> = schema
            .properties_of("NlaTrack")
            .into_iter()
            .filter(|p| p.name != "name")
            .collect();
        let sprops: Vec<&Property> = schema
            .properties_of("NlaStrip")
            .into_iter()
            .filter(|p| {
                !matches!(
                    p.name.as_str(),
                    "name"
                        | "action"
                        | "action-slot"
                        | "action-slot-handle"
                        | "last-slot-identifier"
                )
            })
            .collect();
        let mut names: Vec<&str> = tprops.iter().map(|p| p.name.as_str()).collect();
        names.push("strip");
        for b in attrs(block) {
            if b.name != "strip" {
                match tprops.iter().find(|p| p.name == b.name) {
                    Some(p) => self.property(file, &b, p, false),
                    None => self.unknown(file, &b, "an NLA track", &names, None),
                }
                continue;
            }
            if !self.member_header(file, &b, "strip @walk \"Walk\"", false) {
                continue;
            }
            let Some(strip) = b.node else { continue };
            let mut snames: Vec<&str> = sprops.iter().map(|p| p.name.as_str()).collect();
            snames.push("action");
            for c in attrs(strip) {
                if c.name == "action" {
                    self.plays(file, &c, kind);
                    continue;
                }
                match sprops.iter().find(|p| p.name == c.name) {
                    Some(p) => self.property(file, &c, p, false),
                    None => self.unknown(file, &c, "an NLA strip", &snames, None),
                }
            }
        }
    }

    /// `<property> driver="<expression>" { var <name> <type> { target … } }`: a driver on the
    /// property; `driver=` a word (`sum`, `average`, `min`, `max`) for the other kinds. An
    /// array's item is `index=`.
    pub(super) fn driver(&mut self, file: FileId, a: &Attr<'_>, p: &Property) {
        let schema = self.schema;
        if p.readonly
            || !matches!(
                p.ty.as_str(),
                "float32" | "float64" | "int" | "bool" | "enum"
            )
        {
            return self.out(file).error(
                codes::TYPE,
                a.name_span,
                format!("`{}` cannot be driven: a driver gives a number", a.name),
            );
        }
        for v in &a.args {
            self.out(file).error(
                codes::TYPE,
                v.span(),
                format!(
                    "a driven `{}` takes no value of its own: the driver gives it",
                    a.name
                ),
            );
        }
        let types: Vec<String> = schema
            .properties_of("Driver")
            .into_iter()
            .find(|q| q.name == "type")
            .map(|q| q.item_names().into_iter().map(str::to_string).collect())
            .unwrap_or_default();
        // The variables the expression names: each `var <name>` line of the block.
        let vars: Vec<String> = a
            .node
            .map(attrs)
            .unwrap_or_default()
            .into_iter()
            .filter(|b| b.name == "var")
            .filter_map(|b| b.args.first().map(|v| v.text().to_string()))
            .collect();
        for (k, v) in &a.props {
            match k.text {
                "driver" => {
                    if let Some(text) = v.as_string() {
                        let frames =
                            crate::values::dimension(p, false) == null_value::Dimension::Frames;
                        self.driver_expression(file, v, &text, &vars, frames);
                    } else if !types.iter().any(|t| Some(t.as_str()) == v.as_word()) {
                        let shown: Vec<&str> = types
                            .iter()
                            .map(String::as_str)
                            .filter(|t| *t != "scripted")
                            .collect();
                        self.out(file).error(
                            codes::TYPE,
                            v.span(),
                            format!(
                                "`driver=` takes an expression in quotes, or one of {}",
                                list(&shown)
                            ),
                        );
                    }
                }
                "index" => match (v.text().parse::<usize>(), p.fixed_len()) {
                    (Ok(i), Some(n)) if i < n => {}
                    (_, Some(n)) => self.out(file).error(
                        codes::RANGE,
                        v.span(),
                        format!("`index` is an item of `{}`, 0 to {}", a.name, n - 1),
                    ),
                    (_, None) => self.out(file).error(
                        codes::TYPE,
                        v.span(),
                        format!("`{}` is not an array: it takes no `index`", a.name),
                    ),
                },
                "use-self" => {}
                other => self.out(file).error(
                    codes::TYPE,
                    k.span,
                    format!(
                        "a driver's line takes `driver=`, `index=` and `use-self=`, not `{other}`"
                    ),
                ),
            }
        }
        if p.fixed_len().is_some() && !a.props.iter().any(|(k, _)| k.text == "index") {
            self.out(file).error(
                codes::TYPE,
                a.name_span,
                format!(
                    "`{}` is an array: say which item the driver drives with `index=`",
                    a.name
                ),
            );
        }
        let Some(block) = a.node else { return };
        let var_types: Vec<String> = schema
            .properties_of("DriverVariable")
            .into_iter()
            .find(|q| q.name == "type")
            .map(|q| q.item_names().into_iter().map(str::to_string).collect())
            .unwrap_or_default();
        let tprops: Vec<&Property> = schema
            .properties_of("DriverTarget")
            .into_iter()
            .filter(|q| !matches!(q.name.as_str(), "id" | "id-type" | "data-path"))
            .collect();
        let mut seen = HashSet::new();
        // The block's lines only: `driver=` and `index=` are the property's own line's.
        for b in attrs(block).into_iter().filter(|b| b.node.is_some()) {
            if b.name != "var" {
                self.unknown(file, &b, "a driver", &["var"], None);
                continue;
            }
            let [name, ty] = b.args.as_slice() else {
                self.out(file).error(codes::TYPE, b.span, "a variable is written `var <name> <type> { target … }`, such as `var flicker single-prop`");
                continue;
            };
            let n = name.text();
            if !(n
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
            {
                self.out(file).error(codes::NAME, name.span(), format!("`{n}` is not a variable's name: letters, digits and `_`, as the expression names it"));
            }
            if !seen.insert(n.to_string()) {
                self.out(file).error(
                    codes::REPEATED,
                    name.span(),
                    format!("`{n}` names two variables of this driver"),
                );
            }
            let ty = ty.text();
            let want = match ty {
                "rotation-diff" | "loc-diff" => 2,
                _ if var_types.iter().any(|t| t == ty) => 1,
                _ => {
                    let shown: Vec<&str> = var_types.iter().map(String::as_str).collect();
                    self.out(file).error(
                        codes::ENUM,
                        b.args[1].span(),
                        format!(
                            "`{ty}` is not a kind of driver variable: {}",
                            list_near(ty, &shown)
                        ),
                    );
                    continue;
                }
            };
            let targets: Vec<Attr<'_>> = b.node.map(attrs).unwrap_or_default();
            let count = targets.iter().filter(|t| t.name == "target").count();
            if count != want {
                self.out(file).error(
                    codes::TYPE,
                    b.span,
                    format!(
                        "a {ty} variable has {want} `target` line{}, not {count}",
                        if want == 1 { "" } else { "s" }
                    ),
                );
            }
            for t in &targets {
                if t.name != "target" {
                    self.unknown(file, t, "a driver variable", &["target"], None);
                    continue;
                }
                self.driver_target(file, t, ty, &tprops);
            }
        }
    }

    /// A scripted driver's expression (task 1d-07f): in Blender's simple expressions, with
    /// Null's units, it compiles; a mistake in it is an error at its place; Python is a note.
    fn driver_expression(
        &mut self,
        file: FileId,
        v: &Value<'_>,
        text: &str,
        vars: &[String],
        frames: bool,
    ) {
        let vars: Vec<&str> = vars.iter().map(String::as_str).collect();
        // Read where `driver::place` says, as Sau reads it: in the scene, or outside every scene.
        // The scenes' disagreement is reported once in a check, as a value's is, so it is left
        // out once it has been.
        let place = self
            .scene_place
            .clone()
            .unwrap_or_else(|| self.outside_place.clone());
        let mut cx = place.context(frames);
        if self.disagreement.is_none() {
            cx.disagreement = None;
        }
        let disagreement = cx.disagreement.map(str::to_string);
        // A place in the expression, in the file: exact where the string is written as it reads
        // (no escapes), else the whole string.
        let span = v.span();
        let exact = v.text().len() == text.len() + 2;
        let place = |(s, e): (usize, usize)| {
            if exact {
                null_syntax::Span::new(span.start + 1 + s, span.start + 1 + e)
            } else {
                span
            }
        };
        match crate::driver::compile_in(text, &vars, &cx) {
            Ok(c) => {
                if let Some(why) = c.python {
                    self.out(file).note(
                        codes::PYTHON,
                        span,
                        format!(
                            "this driver is Python, not one of Blender's simple expressions ({why}): it is kept, and runs only where Blender may run Python"
                        ),
                    );
                }
            }
            Err(e) => {
                // The scenes' disagreement is reported once, as a value's is.
                if disagreement.as_deref() == Some(e.message.as_str()) {
                    self.disagreement = None;
                }
                let at = place(e.at);
                let fix = e.fix.map(|f| (format!("write `{f}`"), at, f));
                let code = if e.unit { codes::UNIT } else { codes::TYPE };
                self.out(file).error_fix(code, at, e.message, fix);
            }
        }
    }

    /// `target @handle [path] key=value…`: what a variable reads. A single property's needs a
    /// path; a context property's target is the context word (`active-scene`) and a path.
    fn driver_target(&mut self, file: FileId, t: &Attr<'_>, ty: &str, tprops: &[&Property]) {
        let schema = self.schema;
        match t.args.as_slice() {
            [c, _path, ..] if ty == "context-prop" => {
                if let Some(p) = schema
                    .properties_of("DriverTarget")
                    .into_iter()
                    .find(|q| q.name == "context-property")
                {
                    check_enum(c, "context-property", p, &mut self.out(file));
                }
            }
            [h, rest @ ..] if h.as_word().is_some_and(|w| w.starts_with('@')) => {
                let any = self.defs.keys().any(|(_, k)| k == h.text());
                if !any {
                    self.out(file).error(
                        codes::HANDLE,
                        h.span(),
                        format!("no datablock is called `{}`", h.text()),
                    );
                }
                if ty == "single-prop" && rest.is_empty() {
                    self.out(file).error(codes::TYPE, t.span, "a single property's target names its property after the handle, such as `target @lamp-light energy`");
                }
            }
            _ => self.out(file).error(
                codes::TYPE,
                t.span,
                "a target is written `target @handle`, then a property for a single property",
            ),
        }
        let names: Vec<&str> = tprops.iter().map(|p| p.name.as_str()).collect();
        for (k, v) in &t.props {
            match tprops.iter().find(|p| p.name == k.text) {
                Some(p) if p.ty == "enum" => check_enum(v, &p.name, p, &mut self.out(file)),
                Some(p) if p.ty == "bool" => {}
                Some(p) if p.ty == "string" => {
                    if v.as_string().is_none() {
                        self.out(file).error(
                            codes::TYPE,
                            v.span(),
                            format!("`{}` takes a string", p.name),
                        );
                    }
                }
                Some(p) => check_float(
                    v,
                    &p.name,
                    null_value::Dimension::None,
                    p,
                    &mut self.out(file),
                ),
                None => {
                    let near = closest(k.text, names.iter().copied());
                    self.out(file).error_fix(
                        codes::UNKNOWN_PROPERTY,
                        k.span,
                        format!("a driver target has no `{}`", k.text),
                        near.map(|n| (format!("write `{n}`"), k.span, n.to_string())),
                    );
                }
            }
        }
    }
}
