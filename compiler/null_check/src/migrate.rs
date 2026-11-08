//! Migrations between Null formats (Sau's decision S45, task 1d-04d; `docs/rfcs/README.md`,
//! "Versions and migrations").
//!
//! Every project names the Null format its files are written in (`null-format` in
//! `cosmosis.noll`). A change that breaks old files raises [`FORMAT`] by one and adds a
//! [`Migration`] from the old format, which rewrites one file's text into the next format.
//! Cosmósis runs the steps from a project's format up to [`FORMAT`] when it migrates a project;
//! a format newer than [`FORMAT`] is never guessed at.
//!
//! Most migrations will be generated: when a Blender release renames a property, the step is a
//! list of [`Rename`]s run by [`rename_properties`]. Format 1 is v0's, the first, so
//! [`MIGRATIONS`] is empty: a migration's tests give their own steps.
//!
//! Each step leaves a file it already rewrote unchanged, so a project written back only in
//! part (a stop between two files) migrates again to the same text.
//!
//! ```
//! use null_check::migrate::{Migration, Rename, migrate, rename_properties};
//!
//! // A made-up format 0 that called a camera's `lens` `focal-length`.
//! fn step(text: &str) -> String {
//!     rename_properties(text, &[Rename { kind: "camera", path: &[], old: "focal-length", new: "lens" }])
//! }
//! let steps = [Migration { from: 0, what: "a camera's focal-length is lens", run: step }];
//! let old = "camera @eye \"Eye\" {\n  focal-length 50mm\n}\n";
//! assert_eq!(migrate(old, 0, 1, &steps).unwrap(), "camera @eye \"Eye\" {\n  lens 50mm\n}\n");
//! ```

use null_syntax::Span;
use null_syntax::ast::{Entry, Node, scalar_text};

/// The Null format this checker reads, and the newest it migrates to: v0's.
pub const FORMAT: u32 = 1;

/// One step between formats: a file's text in format `from` written in format `from + 1`.
#[derive(Clone, Copy, Debug)]
pub struct Migration {
    /// The format it reads.
    pub from: u32,
    /// What it changes, in a few words, for the report of a migration.
    pub what: &'static str,
    /// The file's text in format `from`, as format `from + 1`. It is given only text that
    /// parses, and leaves text it already rewrote unchanged.
    pub run: fn(&str) -> String,
}

/// The migrations this Null ships, oldest first. Empty: format 1 is the first.
pub const MIGRATIONS: &[Migration] = &[];

/// Why a file or a project cannot be migrated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MigrateError {
    /// The format is newer than the one asked for: migrations only go forward.
    Newer {
        /// The file's format.
        from: u32,
        /// The format asked for.
        to: u32,
    },
    /// No migration reads this format, so the chain to the format asked for is broken here.
    NoStep {
        /// The format no step reads.
        at: u32,
        /// The oldest format the steps given migrate from.
        oldest: u32,
    },
    /// The text has syntax errors, so where its lines are is not certain.
    Unparsed,
}

impl std::fmt::Display for MigrateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrateError::Newer { from, to } => {
                write!(f, "Null format {from} is newer than format {to}")
            }
            MigrateError::NoStep { at, oldest } if *oldest == FORMAT => write!(
                f,
                "no migration reads Null format {at}: this Null reads format {FORMAT} and migrates no older format"
            ),
            MigrateError::NoStep { at, oldest } => write!(
                f,
                "no migration reads Null format {at}: this Null migrates formats {oldest} to {FORMAT}"
            ),
            MigrateError::Unparsed => {
                write!(
                    f,
                    "the file has syntax errors; fix them first, so that every line is found"
                )
            }
        }
    }
}

/// The oldest format `migrations` lift to [`FORMAT`] without a gap: [`FORMAT`] itself when none
/// do.
pub fn oldest(migrations: &[Migration]) -> u32 {
    let mut at = FORMAT;
    while let Some(m) = at
        .checked_sub(1)
        .and_then(|p| migrations.iter().find(|m| m.from == p))
    {
        at = m.from;
    }
    at
}

/// The steps from format `from` to format `to`, in order.
pub fn steps(from: u32, to: u32, migrations: &[Migration]) -> Result<Vec<Migration>, MigrateError> {
    if from > to {
        return Err(MigrateError::Newer { from, to });
    }
    (from..to)
        .map(|at| {
            migrations
                .iter()
                .find(|m| m.from == at)
                .copied()
                .ok_or(MigrateError::NoStep {
                    at,
                    oldest: oldest(migrations),
                })
        })
        .collect()
}

/// `text`, a file of format `from`, written in format `to` by `migrations`' steps.
pub fn migrate(
    text: &str,
    from: u32,
    to: u32,
    migrations: &[Migration],
) -> Result<String, MigrateError> {
    let steps = steps(from, to, migrations)?;
    let mut text = text.to_string();
    for step in steps {
        if !null_syntax::parse(&text).errors().is_empty() {
            return Err(MigrateError::Unparsed);
        }
        text = (step.run)(&text);
    }
    Ok(text)
}

/// One renamed property, as a generated migration lists it: in each datablock of `kind`, the
/// block at `path` inside it (`&["render"]` for a scene's `render { … }`, `&[]` for the
/// datablock's own), a property line `old …` or an entry `old=…` becomes `new`.
#[derive(Clone, Copy, Debug)]
pub struct Rename {
    /// The datablock's kind, a top-level line's name (`camera`).
    pub kind: &'static str,
    /// The blocks inside the datablock that hold the property, outermost first.
    pub path: &'static [&'static str],
    /// The property's old name.
    pub old: &'static str,
    /// Its new name.
    pub new: &'static str,
}

/// `text` with every property `renames` names given its new name, and nothing else changed:
/// the step of a migration generated from Blender's renames.
pub fn rename_properties(text: &str, renames: &[Rename]) -> String {
    let parse = null_syntax::parse(text);
    let mut edits: Vec<(Span, &str)> = Vec::new();
    for top in parse.document().nodes() {
        let kind = top.name();
        for r in renames.iter().filter(|r| r.kind == kind) {
            for owner in blocks_at(top, r.path) {
                // `old=` on the owner's own line.
                prop_edits(owner, r, &mut edits);
                for line in owner.children() {
                    if line.name() == r.old
                        && let Some(t) = line.name_token()
                    {
                        edits.push((t.span, r.new));
                    }
                }
            }
        }
    }
    edits.sort_by_key(|(s, _)| (s.start, s.end));
    edits.dedup_by_key(|(s, _)| (s.start, s.end));
    null_syntax::apply_edits(text, &edits)
}

/// The nodes at `path` below `node`: `node` itself for an empty path.
fn blocks_at<'a>(node: Node<'a>, path: &[&str]) -> Vec<Node<'a>> {
    let Some((first, rest)) = path.split_first() else {
        return vec![node];
    };
    node.children()
        .filter(|c| c.name() == *first)
        .flat_map(|c| blocks_at(c, rest))
        .collect()
}

/// The edits renaming `r.old=` among `node`'s entries.
fn prop_edits<'r>(node: Node<'_>, r: &'r Rename, edits: &mut Vec<(Span, &'r str)>) {
    for e in node.entries() {
        if let Entry::Prop { key, .. } = e
            && scalar_text(key) == r.old
        {
            edits.push((key.span, r.new));
        }
    }
}
