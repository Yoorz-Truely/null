//! Write-back across a project: find the datablock in whichever file holds it, edit its transform.
//!
//! This is Null's side of Sau's write-back (spec, section 17; task 1a-14a). Sau's side
//! (1a-14) calls it from C++ through its Rust core, so everything crossing is plain data:
//! file paths and texts in; for each edit, a path, a byte range of that file's text, and
//! the new text out. No syntax tree or other Null type crosses.

use std::fmt;

use null_syntax::parse;
use null_value::{Dimension, Unit};

use crate::write::{Values, WriteError, write};

/// One file of the project: its path, as the caller names it, and its whole text.
#[derive(Clone, Copy, Debug)]
pub struct ProjectFile<'a> {
    /// The file's path; returned unchanged in each [`FileEdit`].
    pub path: &'a str,
    /// The file's text.
    pub text: &'a str,
}

/// One edit to one file: replace bytes `start..end` of its text with `replacement`.
///
/// One call can give several edits to one file (three values changed, three edits). Every
/// range is in the file's text as given, before any edit, and no two overlap, so apply them
/// from the last to the first (or keep an offset).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEdit {
    /// The file's path, as given in its [`ProjectFile`].
    pub path: String,
    /// The first byte replaced.
    pub start: usize,
    /// One past the last byte replaced; equal to `start` for an insertion.
    pub end: usize,
    /// The new text.
    pub replacement: String,
}

/// A transform attribute Sau writes back from a drag (spec, section 17).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transform {
    /// `location`, three lengths.
    Location,
    /// `rotation-euler`, three angles.
    RotationEuler,
    /// `scale`, three factors.
    Scale,
}

impl Transform {
    /// The attribute's Null name.
    pub fn name(self) -> &'static str {
        match self {
            Transform::Location => "location",
            Transform::RotationEuler => "rotation-euler",
            Transform::Scale => "scale",
        }
    }

    /// What its values measure.
    fn dimension(self) -> Dimension {
        match self {
            Transform::Location => Dimension::Length,
            Transform::RotationEuler => Dimension::Angle,
            Transform::Scale => Dimension::None,
        }
    }

    /// The attribute by its Null name.
    pub fn from_name(name: &str) -> Option<Transform> {
        [
            Transform::Location,
            Transform::RotationEuler,
            Transform::Scale,
        ]
        .into_iter()
        .find(|t| t.name() == name)
    }
}

/// Why nothing could be written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteBackError {
    /// No file defines an object with that handle.
    NoSuchObject,
    /// More than one file defines it; the project does not check.
    DefinedTwice,
    /// Not three values.
    NotThreeValues,
    /// A value is NaN or infinite, which no file can hold as a number.
    NotFinite,
}

impl fmt::Display for WriteBackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WriteBackError::NoSuchObject => write!(f, "no file defines that object"),
            WriteBackError::DefinedTwice => write!(f, "more than one file defines that object"),
            WriteBackError::NotThreeValues => write!(f, "a transform takes three values"),
            WriteBackError::NotFinite => write!(f, "a value is not a finite number"),
        }
    }
}

impl std::error::Error for WriteBackError {}

/// The edits that set `transform` of the object `handle` (`@pier`) to `values`, in
/// whichever file of `files` defines it.
///
/// Each value that changed is rewritten in the unit its word already uses (S21); an
/// attribute the object does not have yet is added in the project's display unit, read
/// from `units` in the project or a scene, in any form the checker takes (`length=cm`,
/// `length=centimeters`, a `length-unit` line), and bare when there is none. An object
/// defined twice, in one file or two, is refused, and so is a value that is NaN or infinite. Every other
/// character of every file is kept: comments, layout and line endings.
pub fn write_transform(
    files: &[ProjectFile<'_>],
    handle: &str,
    transform: Transform,
    values: [f32; 3],
) -> Result<Vec<FileEdit>, WriteBackError> {
    if values.iter().any(|v| !v.is_finite()) {
        return Err(WriteBackError::NotFinite);
    }
    // Every definition counts, in one file or across several (the 1a-14a review).
    let mut found = None;
    let mut definitions = 0;
    for (i, f) in files.iter().enumerate() {
        let file = parse(f.text);
        let here = file
            .document()
            .nodes()
            .filter(|n| n.name() == "object" && defines(*n, handle))
            .count();
        definitions += here;
        if here > 0 && found.is_none() {
            found = Some((i, file));
        }
    }
    if definitions > 1 {
        return Err(WriteBackError::DefinedTwice);
    }
    let (i, file) = found.ok_or(WriteBackError::NoSuchObject)?;
    let display = display_unit(files, transform);
    let edits = write(
        &file,
        "object",
        handle,
        transform.name(),
        Values::F32(&values),
        transform.dimension(),
        display,
    )
    .map_err(|e| match e {
        WriteError::NoSuchNode => WriteBackError::NoSuchObject,
        WriteError::NoValues => WriteBackError::NotThreeValues,
    })?;
    Ok(edits
        .into_iter()
        .map(|e| FileEdit {
            path: files[i].path.to_string(),
            start: e.span.start,
            end: e.span.end,
            replacement: e.replacement,
        })
        .collect())
}

/// Whether the object node defines `handle`: written, or made from its display name (S18).
fn defines(node: null_syntax::ast::Node<'_>, handle: &str) -> bool {
    let Some(first) = node.args().next() else {
        return false;
    };
    match first.as_word() {
        Some(w) if w.starts_with('@') => w == handle,
        _ => first.as_string().is_some_and(|s| {
            null_syntax::names::to_kebab_case(&s).map(|k| format!("@{k}"))
                == Some(handle.to_string())
        }),
    }
}

/// The project's display unit for `transform`: `units length=…` for a location,
/// `rotation=…` for a rotation (spec, section 8), from the `project` node of any file.
fn display_unit(files: &[ProjectFile<'_>], transform: Transform) -> Option<Unit> {
    // The short name and the full one, and the enum items each unit may be written as.
    let (short, full, items): (&str, &str, &[(&str, Unit)]) = match transform {
        Transform::Location => (
            "length",
            "length-unit",
            &[
                ("meters", Unit::M),
                ("centimeters", Unit::Cm),
                ("millimeters", Unit::Mm),
                ("kilometers", Unit::Km),
                ("inches", Unit::In),
                ("feet", Unit::Ft),
            ],
        ),
        Transform::RotationEuler => (
            "rotation",
            "system-rotation",
            &[("degrees", Unit::Deg), ("radians", Unit::Rad)],
        ),
        Transform::Scale => return None,
    };
    let unit = |word: &str| {
        Unit::from_suffix(word, transform.dimension())
            .filter(|u| u.dimension() == transform.dimension())
            .or_else(|| items.iter().find(|(w, _)| *w == word).map(|(_, u)| *u))
    };
    for f in files {
        let file = parse(f.text);
        // `units` in the project, or in a scene, as `key=value` on its line or as a child line.
        let holders = file
            .document()
            .nodes()
            .filter(|n| n.name() == "project" || n.name() == "scene");
        for holder in holders {
            for units in holder.children().filter(|c| c.name() == "units") {
                for entry in units.entries() {
                    if let null_syntax::ast::Entry::Prop { key, value } = entry
                        && (key.text == short || key.text == full)
                    {
                        return value.as_word().and_then(unit);
                    }
                }
                for line in units.children() {
                    if line.name() == short || line.name() == full {
                        return line.args().next().and_then(|v| v.as_word()).and_then(unit);
                    }
                }
            }
        }
    }
    None
}
