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
}

impl fmt::Display for WriteBackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WriteBackError::NoSuchObject => write!(f, "no file defines that object"),
            WriteBackError::DefinedTwice => write!(f, "more than one file defines that object"),
            WriteBackError::NotThreeValues => write!(f, "a transform takes three values"),
        }
    }
}

impl std::error::Error for WriteBackError {}

/// The edits that set `transform` of the object `handle` (`@pier`) to `values`, in
/// whichever file of `files` defines it.
///
/// Each value that changed is rewritten in the unit its word already uses (S21); an
/// attribute the object does not have yet is added in the project's display unit, read
/// from the project's `units … length=… rotation=…` (bare when there is none). Every other
/// character of every file is kept: comments, layout and line endings.
pub fn write_transform(
    files: &[ProjectFile<'_>],
    handle: &str,
    transform: Transform,
    values: [f32; 3],
) -> Result<Vec<FileEdit>, WriteBackError> {
    let mut found = None;
    for (i, f) in files.iter().enumerate() {
        let file = parse(f.text);
        let defines = file.document().nodes().any(|n| {
            n.name() == "object"
                && n.args()
                    .next()
                    .and_then(|v| v.as_word())
                    .is_some_and(|w| w == handle)
        }) || {
            // A handle made from the display name (S18).
            file.document().nodes().any(|n| {
                n.name() == "object"
                    && n.args()
                        .next()
                        .and_then(|v| v.as_string())
                        .is_some_and(|s| {
                            null_syntax::names::to_kebab_case(&s).map(|k| format!("@{k}"))
                                == Some(handle.to_string())
                        })
            })
        };
        if defines {
            if found.is_some() {
                return Err(WriteBackError::DefinedTwice);
            }
            found = Some((i, file));
        }
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

/// The project's display unit for `transform`: `units length=…` for a location,
/// `rotation=…` for a rotation (spec, section 8), from the `project` node of any file.
fn display_unit(files: &[ProjectFile<'_>], transform: Transform) -> Option<Unit> {
    let key = match transform {
        Transform::Location => "length",
        Transform::RotationEuler => "rotation",
        Transform::Scale => return None,
    };
    for f in files {
        let file = parse(f.text);
        for project in file.document().nodes().filter(|n| n.name() == "project") {
            for units in project.children().filter(|c| c.name() == "units") {
                for entry in units.entries() {
                    if let null_syntax::ast::Entry::Prop { key: k, value } = entry
                        && k.text == key
                    {
                        return value
                            .as_word()
                            .and_then(|w| Unit::from_suffix(w, transform.dimension()))
                            .filter(|u| u.dimension() == transform.dimension());
                    }
                }
            }
        }
    }
    None
}
