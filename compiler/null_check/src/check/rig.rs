//! An armature's bones and bone collections, a mesh's shape keys and a scene's or an action's
//! timeline markers (RFC 0005, Sau's decision S88; task 1d-07q): sub-object lines in their
//! owner's block, their settings their RNA struct's, and the handles they name among their
//! owner's own sub-objects.
//!
//! - `bone @h "Name" { head […]; tail […]; roll …; parent @h; use-connect; collections [@h …]; … }`
//!   in an `armature`: the rest pose as the edit bones hold it, then `Bone`'s settings.
//! - `bone-collection @h "Name" { parent @h; … }` in an `armature`.
//! - `shape-key @h "Name" { positions […]; relative-key @h; vertex-group @h; value …; … }` in a
//!   mesh (its `positions` are checked with the mesh's arrays, `level0`), and
//!   `shape-keys { use-relative; action @a slot=@s; track … }` for its `Key`.
//! - `marker @h "Name" frame=N { camera @h; select }` in a scene or an action.
//! - `tile <number> { label "…"; … }` in an image, a UDIM tile's own settings (RFC 0007, S96).

use std::collections::HashMap;

use null_syntax::ast::Value;
use null_syntax::{FileId, Span};

use super::{Attr, Checker, attrs, kebab_handle};
use crate::codes;
use crate::schema::Property;

/// The handles of `all`'s lines of `kind`, as written.
pub(super) fn handles(all: &[Attr<'_>], kind: &str) -> Vec<String> {
    all.iter()
        .filter(|a| a.name == kind)
        .filter_map(|a| a.args.first()?.as_word().map(str::to_string))
        .filter(|w| w.starts_with('@'))
        .collect()
}

/// The handle `name @h` gives in `block`'s lines, with its span.
fn named(block: &[Attr<'_>], name: &str) -> Option<(String, Span)> {
    let a = block.iter().find(|a| a.name == name)?;
    let v = a.args.first()?;
    Some((v.as_word()?.to_string(), v.span()))
}

/// A struct's properties a line may set: not read-only, not a pointer or collection (those are
/// written as handles where they are written at all), and not its name.
pub(super) fn settable<'s>(props: Vec<&'s Property>, except: &[&str]) -> Vec<&'s Property> {
    props
        .into_iter()
        .filter(|p| {
            !p.readonly
                && !matches!(p.ty.as_str(), "pointer" | "collection")
                && p.name != "name"
                && !except.contains(&p.name.as_str())
        })
        .collect()
}

impl Checker<'_> {
    /// Whether a shape key's array line `b` is a list, or names a data file (`file=`); said
    /// when it is neither, with `example` (1d-07q's review: `tilts 0.5` is no list).
    pub(super) fn array_or_file(&mut self, file: FileId, b: &Attr<'_>, example: &str) -> bool {
        if b.props.iter().any(|(k, _)| k.text == "file")
            || matches!(b.args.as_slice(), [Value::List(_)])
        {
            return true;
        }
        self.out(file).error(
            codes::TYPE,
            b.span,
            format!("`{}` is a list, one per point, such as `{example}`", b.name),
        );
        false
    }

    /// A line `kind @h "Name"`, its handle first; false, said, when it is not.
    pub(super) fn sub_header(&mut self, file: FileId, a: &Attr<'_>, example: &str) -> bool {
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
                        format!(
                            "`{}`'s line is `{example}`, then its settings in a block",
                            a.name
                        ),
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

    /// `name @h` in a sub-object's block: one handle, of the owner's `kind` lines (`among`),
    /// not its own.
    fn sibling_ref(&mut self, file: FileId, b: &Attr<'_>, own: &str, among: &[String], what: &str) {
        let [v] = b.args.as_slice() else {
            return self.out(file).error(
                codes::TYPE,
                b.span,
                format!("`{}` takes one handle, of {what}", b.name),
            );
        };
        let h = v.text();
        if h == own {
            self.out(file).error(
                codes::HANDLE,
                v.span(),
                format!("`{}` names its own line", b.name),
            );
        } else if !among.iter().any(|x| x == h) {
            let near = null_syntax::closest(h, among.iter().map(String::as_str));
            self.out(file).error_fix(
                codes::HANDLE,
                v.span(),
                format!("{h} is none of {what}"),
                near.map(|n| (format!("write `{n}`"), v.span(), n.to_string())),
            );
        }
    }

    /// An armature's `bone @h "Name" { … }`; `all` is the armature's lines.
    pub(super) fn armature_bone(&mut self, file: FileId, a: &Attr<'_>, all: &[Attr<'_>]) {
        if !self.sub_header(file, a, "bone @spine \"Spine\"") {
            return;
        }
        let own = a.args[0].text().to_string();
        let bones = handles(all, "bone");
        let collections = handles(all, "bone-collection");
        let Some(block) = a.node else { return };
        let schema = self.schema;
        // The rest pose as the edit bones hold it, then the bone's own settings.
        let mut props: Vec<&Property> = schema
            .properties_of("EditBone")
            .into_iter()
            .filter(|p| matches!(p.name.as_str(), "head" | "tail" | "roll" | "use-connect"))
            .collect();
        props.extend(settable(schema.properties_of("Bone"), &[]));
        let mut names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
        names.extend(["parent", "collections"]);
        for b in attrs(block) {
            match b.name.as_str() {
                "parent" => self.sibling_ref(file, &b, &own, &bones, "the armature's bones"),
                "collections" => {
                    let items = match b.args.as_slice() {
                        [list @ Value::List(_)] => list.items(),
                        _ => {
                            self.out(file).error(
                                codes::TYPE,
                                b.span,
                                "`collections` is a list of the armature's bone collections, such as `collections [@body]`",
                            );
                            continue;
                        }
                    };
                    for v in items {
                        if !collections.iter().any(|c| c == v.text()) {
                            self.out(file).error(
                                codes::HANDLE,
                                v.span(),
                                format!("{} is none of the armature's bone collections", v.text()),
                            );
                        }
                    }
                }
                _ => match props.iter().find(|p| p.name == b.name) {
                    Some(p) => self.property(file, &b, p, false),
                    None => self.unknown(file, &b, "a bone", &names, None),
                },
            }
        }
        // A parent chain that leads back to the bone.
        let parents: HashMap<String, (String, Span)> = all
            .iter()
            .filter(|x| x.name == "bone")
            .filter_map(|x| {
                let h = x.args.first()?.as_word()?.to_string();
                Some((h, named(&attrs(x.node?), "parent")?))
            })
            .collect();
        let mut at = own.clone();
        for _ in 0..parents.len() {
            let Some((p, _)) = parents.get(&at) else {
                break;
            };
            if *p == own {
                if let Some((_, span)) = parents.get(&own) {
                    self.out(file).error(
                        codes::HANDLE,
                        *span,
                        format!("{own}'s parents lead back to it"),
                    );
                }
                break;
            }
            at = p.clone();
        }
    }

    /// An armature's `bone-collection @h "Name" { … }`.
    pub(super) fn bone_collection(&mut self, file: FileId, a: &Attr<'_>, all: &[Attr<'_>]) {
        if !self.sub_header(file, a, "bone-collection @body \"Body\"") {
            return;
        }
        let own = a.args[0].text().to_string();
        let collections = handles(all, "bone-collection");
        let Some(block) = a.node else { return };
        let props = settable(self.schema.properties_of("BoneCollection"), &[]);
        let mut names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
        names.push("parent");
        for b in attrs(block) {
            match b.name.as_str() {
                "parent" => self.sibling_ref(
                    file,
                    &b,
                    &own,
                    &collections,
                    "the armature's bone collections",
                ),
                _ => match props.iter().find(|p| p.name == b.name) {
                    Some(p) => self.property(file, &b, p, false),
                    None => self.unknown(file, &b, "a bone collection", &names, None),
                },
            }
        }
    }

    /// A `shape-key @h "Name" { … }` of a mesh, a curve, a surface or a lattice (`kind`, the
    /// owner's handle `owner`). A mesh's `positions` are checked with its arrays, one per vertex
    /// (`level0`); a curve's or a lattice's here, element by element, their count Blender's to
    /// hold; a curve's points also take `tilts` and `radii`, and its Bézier points
    /// `handles-left` and `handles-right`.
    pub(super) fn shape_key(
        &mut self,
        file: FileId,
        a: &Attr<'_>,
        all: &[Attr<'_>],
        kind: &str,
        owner: Option<(String, String)>,
    ) {
        if !self.sub_header(file, a, "shape-key @smile \"Smile\"") {
            return;
        }
        let own = a.args[0].text().to_string();
        let keys = handles(all, "shape-key");
        let groups = handles(all, "vertex-group");
        let Some(block) = a.node else { return };
        let props = settable(
            self.schema.properties_of("ShapeKey"),
            &["relative-key", "vertex-group"],
        );
        let curve = matches!(kind, "curve" | "surface");
        let mut names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
        names.extend(["positions", "relative-key", "vertex-group"]);
        if curve {
            names.extend(["handles-left", "handles-right", "tilts", "radii"]);
        }
        let whose = super::article(kind);
        for b in attrs(block) {
            match b.name.as_str() {
                "positions" if kind == "mesh" => {}
                "positions" | "handles-left" | "handles-right"
                    if b.name == "positions" || curve =>
                {
                    if let Some(owner) = &owner
                        && self.array_or_file(file, &b, &format!("{} [[0m 0m 0m] …]", b.name))
                    {
                        let array = if b.name == "positions" {
                            own.clone()
                        } else {
                            format!("{own}.{}", b.name)
                        };
                        self.array_line(
                            file,
                            &b,
                            owner,
                            &array,
                            super::level0::Elements::Positions,
                            None,
                        );
                    }
                }
                // A curve point's tilt and radius, which its shape key stores beside its position
                // (`key.cc`): one number each per point.
                "tilts" | "radii" if curve => {
                    if let Some(owner) = &owner
                        && self.array_or_file(file, &b, &format!("{} [0 0.5]", b.name))
                    {
                        let p = crate::level0::element("float").expect("float is a data type");
                        let array = format!("{own}.{}", b.name);
                        self.array_line(
                            file,
                            &b,
                            owner,
                            &array,
                            super::level0::Elements::Values(Box::new(p)),
                            None,
                        );
                    }
                }
                "relative-key" => {
                    let what = format!("{whose}'s shape keys");
                    self.sibling_ref(file, &b, &own, &keys, &what)
                }
                "vertex-group" => {
                    let what = format!("{whose}'s vertex groups");
                    self.sibling_ref(file, &b, "", &groups, &what)
                }
                _ => match props.iter().find(|p| p.name == b.name) {
                    Some(p) => self.property(file, &b, p, false),
                    None => self.unknown(file, &b, "a shape key", &names, None),
                },
            }
        }
    }

    /// The `shape-keys { … }` of a mesh, a curve, a surface or a lattice: its `Key`'s settings
    /// and what plays on it.
    pub(super) fn shape_keys(&mut self, file: FileId, a: &Attr<'_>) {
        if !a.args.is_empty() || !a.props.is_empty() {
            self.out(file).error(
                codes::TYPE,
                a.span,
                "`shape-keys` is a block of the shape keys' own settings, such as `shape-keys { use-relative }`",
            );
        }
        let Some(block) = a.node else { return };
        let props = settable(self.schema.properties_of("Key"), &[]);
        let mut names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
        names.extend(["action", "track"]);
        for b in attrs(block) {
            match b.name.as_str() {
                "action" => self.plays(file, &b, "key"),
                "track" => self.track(file, &b, "key"),
                _ => match props.iter().find(|p| p.name == b.name) {
                    Some(p) => self.property(file, &b, p, false),
                    None => self.unknown(file, &b, "the shape keys' block", &names, None),
                },
            }
        }
    }

    /// A scene's or an action's `marker @h "Name" frame=N { … }`.
    pub(super) fn marker(&mut self, file: FileId, a: &Attr<'_>) {
        if !self.sub_header(file, a, "marker @cut \"Cut\" frame=48") {
            return;
        }
        let schema = self.schema;
        let all = schema.properties_of("TimelineMarker");
        match a.props.iter().find(|(k, _)| k.text == "frame") {
            Some((_, v)) => {
                if let Some(p) = all.iter().find(|p| p.name == "frame") {
                    crate::values::check_int(v, "a marker's frame", p, &mut self.out(file));
                }
            }
            None => self.out(file).error(
                codes::TYPE,
                a.span,
                "a marker gives its frame on its line, such as `marker @cut \"Cut\" frame=48`",
            ),
        }
        for (k, _) in a.props.iter().filter(|(k, _)| k.text != "frame") {
            self.out(file)
                .error(codes::TYPE, k.span, "a marker's line takes `frame=` only");
        }
        let Some(block) = a.node else { return };
        let props = settable(all, &["frame"]);
        let mut names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
        names.push("camera");
        // The line's `frame=` reads as one of its block's lines too (section 5); checked above.
        for b in attrs(block).into_iter().filter(|b| b.name != "frame") {
            match b.name.as_str() {
                "camera" => self.reference(file, &b, "object"),
                _ => match props.iter().find(|p| p.name == b.name) {
                    Some(p) => self.property(file, &b, p, false),
                    None => self.unknown(file, &b, "a marker", &names, None),
                },
            }
        }
    }

    /// An image's `tile <number> { label "…"; … }` (Null section 12; RFC 0007, Sau's decision
    /// S96; task 1d-05m): a UDIM tile's own settings, `UDIMTile`'s, its number from 1001 to 2000
    /// and once per image (`seen`).
    pub(super) fn udim_tile(&mut self, file: FileId, a: &Attr<'_>, seen: &mut Vec<i64>) {
        let [n] = a.args.as_slice() else {
            return self.out(file).error(
                codes::TYPE,
                a.span,
                "a tile is written `tile <number> { … }`, such as `tile 1002 { label \"Back\" }`",
            );
        };
        match n.as_word().and_then(|w| w.parse::<i64>().ok()) {
            Some(x) if (1001..=2000).contains(&x) => {
                if seen.contains(&x) {
                    self.out(file).error(
                        codes::REPEATED,
                        n.span(),
                        format!("tile {x} is given twice; an image has one of each"),
                    );
                }
                seen.push(x);
            }
            Some(x) => self.out(file).error(
                codes::RANGE,
                n.span(),
                format!("tile {x} is outside UDIM's 1001 to 2000"),
            ),
            None => self.out(file).error(
                codes::TYPE,
                n.span(),
                "a tile's number is a whole number, from 1001 to 2000",
            ),
        }
        for (k, _) in &a.props {
            self.out(file).error(
                codes::TYPE,
                k.span,
                "a tile's settings are lines in its block, such as `tile 1002 { label \"Back\" }`",
            );
        }
        let Some(block) = a.node else { return };
        let props = settable(self.schema.properties_of("UDIMTile"), &["number"]);
        let names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
        for b in attrs(block) {
            match props.iter().find(|p| p.name == b.name) {
                Some(p) => self.property(file, &b, p, false),
                None => self.unknown(file, &b, "a tile", &names, None),
            }
        }
    }
}
