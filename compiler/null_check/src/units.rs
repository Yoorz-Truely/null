//! Reading a project's or scene's `units` line for the unit scale (spec, section 6, "Scene
//! scale"; task 1c-01d). The one reader the checker, write-back (`null_print`) and Sau's apply
//! use, so they agree on every file.

use null_syntax::Parse;
use null_syntax::ast::{Entry, Node};

use crate::forms;

/// What a `units` line says: its system (`metric`, `imperial`, `none`), `scale-length`, and
/// the display units `length-unit` and `system-rotation` as written, where it says them. What
/// it leaves out is a new scene's: metric, scale 1.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Units {
    /// The unit system, if the line gives one.
    pub system: Option<String>,
    /// `scale-length`, if the line gives one that reads as a number.
    pub scale: Option<f32>,
    /// `length-unit` (`length=`), as written: `mm` or `millimeters`.
    pub length: Option<String>,
    /// `system-rotation` (`rotation=`), as written: `deg` or `degrees`.
    pub rotation: Option<String>,
}

impl Units {
    /// The `units` line of a project or scene node, if it has one: `units metric scale=0.01`,
    /// `scale-length=`, or `system` and `scale` / `scale-length` lines of one value in its
    /// block. Only the first `units` line counts and, within it, the first setting of each;
    /// the checker reports the others as given twice. An empty `units {}` is a `units` line
    /// that leaves everything as a new scene's.
    pub fn of(holder: Node<'_>) -> Option<Units> {
        let units = holder.children().find(|c| c.name() == "units")?;
        let mut settings: Vec<(String, String)> = Vec::new();
        if let Some(w) = units.args().next().and_then(|v| v.as_word()) {
            settings.push(("system".into(), w.to_string()));
        }
        for entry in units.entries() {
            if let Entry::Prop { key, value } = entry
                && let Some(w) = value.as_word()
            {
                settings.push((key.text.to_string(), w.to_string()));
            }
        }
        for line in units.children() {
            let mut args = line.args();
            if let (Some(v), None) = (args.next(), args.next())
                && let Some(w) = v.as_word()
            {
                settings.push((line.name().into_owned(), w.to_string()));
            }
        }
        let mut out = Units::default();
        let mut scale_seen = false;
        for (key, word) in settings {
            let first = |slot: &mut Option<String>| {
                if slot.is_none() {
                    *slot = Some(word.clone());
                }
            };
            match full_name(&key) {
                "system" => first(&mut out.system),
                "length-unit" => first(&mut out.length),
                "system-rotation" => first(&mut out.rotation),
                "scale-length" if !scale_seen => {
                    scale_seen = true;
                    out.scale = null_value::read::<f32>(&word, null_value::Dimension::None).ok();
                }
                _ => {}
            }
        }
        Some(out)
    }

    /// These settings, with `other`'s where these say nothing.
    pub fn over(self, other: &Units) -> Units {
        Units {
            system: self.system.or_else(|| other.system.clone()),
            scale: self.scale.or(other.scale),
            length: self.length.or_else(|| other.length.clone()),
            rotation: self.rotation.or_else(|| other.rotation.clone()),
        }
    }

    /// The unit scale values are divided by: `scale-length`, 1 when unset or not a positive
    /// number, and 1 whatever it is when the system is `none`, as in Blender.
    pub fn scale(&self) -> f32 {
        if self.system.as_deref() == Some("none") {
            return 1.0;
        }
        self.scale
            .filter(|s| *s > 0.0 && s.is_finite())
            .unwrap_or(1.0)
    }
}

/// A `units` key's full name: `scale` is `scale-length`.
fn full_name(key: &str) -> &str {
    forms::UNITS_SHORT
        .iter()
        .find(|(s, _)| *s == key)
        .map_or(key, |(_, f)| f)
}

/// The unit scale for values outside any scene.
#[derive(Clone, Debug, PartialEq)]
pub enum Outside {
    /// The one scale: the project's, or the one every scene has.
    Scale(f32),
    /// No project `units` and the scenes' scales differ: each scene's handle (or name, as
    /// written) and scale, in file order. A value written with a scaled unit outside a scene
    /// has no one scale to be read at.
    Disagree(Vec<(String, f32)>),
}

impl Outside {
    /// The scale to read at: the one scale, or 1 where the scenes disagree (the checker reports
    /// the disagreement; write-back writes bare numbers there).
    pub fn scale(&self) -> f32 {
        match self {
            Outside::Scale(s) => *s,
            Outside::Disagree(_) => 1.0,
        }
    }
}

/// The project's `units`: the `units` line of the first project node that has one. Inside a
/// scene, the scene's own are read over it.
pub fn project<'a>(files: impl IntoIterator<Item = &'a Parse>) -> Option<Units> {
    files
        .into_iter()
        .flat_map(|parse| parse.document().nodes())
        .filter(|node| node.name() == forms::PROJECT)
        .find_map(Units::of)
}

/// Each scene's handle or name, as written, with its `units` (a new scene's where it has none),
/// in file order.
pub fn scenes<'a>(files: impl IntoIterator<Item = &'a Parse>) -> Vec<(String, Units)> {
    files
        .into_iter()
        .flat_map(|parse| parse.document().nodes())
        .filter(|node| node.name() == "scene")
        .map(|node| {
            let name = node
                .args()
                .next()
                .map_or_else(|| "a scene".to_string(), |v| v.text().to_string());
            (name, Units::of(node).unwrap_or_default())
        })
        .collect()
}

/// The scale values outside any scene are read at (spec, section 6, "Scene scale"): the
/// project's `units` where it has a `units` line; else the scale every scene has (a scene with
/// no `units` has 1), or 1 with no scenes; else the scenes disagree.
pub fn outside<'a>(files: impl IntoIterator<Item = &'a Parse> + Clone) -> Outside {
    if let Some(project) = project(files.clone()) {
        return Outside::Scale(project.scale());
    }
    let scales: Vec<(String, f32)> = scenes(files)
        .into_iter()
        .map(|(name, units)| (name, units.scale()))
        .collect();
    match scales.first() {
        None => Outside::Scale(1.0),
        Some((_, s)) if scales.iter().all(|(_, t)| t.to_bits() == s.to_bits()) => {
            Outside::Scale(*s)
        }
        Some(_) => Outside::Disagree(scales),
    }
}
