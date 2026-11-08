//! Sub-objects of a family (spec, section 8, "Modifiers and constraints"; task 1d-07a): an
//! object's `modifier` and `constraint` lines, and its pose's `bone` lines, each
//! `<word> @handle <kind> "Name" { … }` with the properties of the family's struct of that kind
//! (`modifier @bevel bevel "Bevel" { width 4mm }` is a `BevelModifier`).

use null_syntax::ast::Value;
use null_syntax::{FileId, closest};

use super::{Attr, Checker, kebab_handle};
use crate::codes;
use crate::schema::Property;
use crate::values::list_near;

/// Properties a family member's line writes, not a setting: its quoted name, its kind, and
/// what Blender keeps for itself.
const MEMBER_LINE: &[&str] = &["name", "type", "persistent-uid", "is-active", "active"];

impl Checker<'_> {
    /// `<word> @handle <kind> "Name" { … }`, a member of the family whose structs extend
    /// `base` (`Modifier`, `Constraint`).
    pub(super) fn family_member(&mut self, file: FileId, a: &Attr<'_>, base: &str) {
        let schema = self.schema;
        let word = a.name.as_str();
        let [h, k, rest @ ..] = a.args.as_slice() else {
            return self.out(file).error(
                codes::TYPE,
                a.span,
                format!(
                    "a {word} is written `{word} @handle <kind> \"Name\"`, such as `{}`",
                    example(word)
                ),
            );
        };
        if !h.as_word().is_some_and(|w| w.starts_with('@')) {
            return self.out(file).error(
                codes::TYPE,
                h.span(),
                format!("a {word}'s handle comes first, such as `{}`", example(word)),
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
                format!("a {word}'s line is `{word} @handle <kind> \"Name\"`; its settings are `name value` lines or `name=value`"),
            );
        }
        let kinds: Vec<&str> = schema
            .structs
            .iter()
            .filter(|s| s.base.as_deref() == Some(base))
            .map(|s| s.kind.as_str())
            .collect();
        let kind = k.text();
        let Some(s) = schema
            .structs
            .iter()
            .find(|s| s.kind == kind && s.base.as_deref() == Some(base))
        else {
            let near = closest(kind, kinds.iter().copied());
            return self.out(file).error_fix(
                codes::KIND,
                k.span(),
                format!(
                    "`{kind}` is not a kind of {word}: {}",
                    list_near(kind, &kinds)
                ),
                near.map(|n| (format!("write `{n}`"), k.span(), n.to_string())),
            );
        };
        let props: Vec<&Property> = schema
            .properties_of(&s.rna)
            .into_iter()
            .filter(|p| !MEMBER_LINE.contains(&p.name.as_str()))
            .collect();
        let owner = format!("{kind} {word}");
        let Some(n) = a.node else { return };
        if s.rna != "NodesModifier" {
            return self.attributes(file, n, &owner, &props, &[]);
        }
        // A geometry nodes modifier's inputs are its group's, by their handles (section 10).
        let lines = super::attrs(n);
        let group = lines
            .iter()
            .find(|l| l.name == "node-group")
            .and_then(|l| l.args.first())
            .and_then(|v| v.as_word())
            .and_then(|g| self.groups.get(g))
            .map(|i| i.inputs.clone());
        let mut names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
        if let Some(g) = &group {
            names.extend(g.iter().map(String::as_str));
        }
        for l in &lines {
            if group.as_ref().is_some_and(|g| g.contains(&l.name)) {
                continue; // the group's input: its type is the group's (section 10)
            }
            match props.iter().find(|p| p.name == l.name) {
                Some(p) => self.property(file, l, p, false),
                None => self.unknown(file, l, "a nodes modifier", &names, None),
            }
        }
    }

    /// `bone @spine "Spine" { … }` in an object: its pose bone (the armature's bone of that
    /// name, posed): `PoseBone`'s properties and its `constraint` lines.
    pub(super) fn pose_bone(&mut self, file: FileId, a: &Attr<'_>) {
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
                        "a bone's line is `bone @handle \"Name\"`",
                    );
                }
            }
            _ => {
                return self.out(file).error(
                    codes::TYPE,
                    a.span,
                    "a pose bone is written `bone @handle \"Name\" { … }`, such as `bone @spine \"Spine\"`",
                );
            }
        }
        let schema = self.schema;
        let props: Vec<&Property> = schema
            .properties_of("PoseBone")
            .into_iter()
            .filter(|p| p.name != "name")
            .collect();
        if let Some(n) = a.node {
            self.attributes(file, n, "pose-bone", &props, &[]);
        }
    }
}

fn example(word: &str) -> &'static str {
    match word {
        "modifier" => "modifier @bevel bevel \"Bevel\"",
        "constraint" => "constraint @track damped-track \"Track\"",
        _ => "f-modifier @noise noise",
    }
}
