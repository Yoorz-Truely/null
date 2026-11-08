//! Null's canonical printer and minimal-edit writer, both on the lossless syntax tree.
//!
//! - [`write`] is how Sau writes an edit back (spec, section 17): given a datablock,
//!   one of its attributes and the attribute's new values, it gives the smallest text
//!   edits that make the file say so. Only the words whose values changed are
//!   rewritten, each in the unit it was already written in (Sau's decision S21);
//!   comments, layout and every other character stay as they are.
//! - [`write_transform`] is the same across a project's files, for an object's transform,
//!   with plain data in and out (paths, texts, byte ranges), for Sau to call through its
//!   Rust core (task 1a-14a).
//! - [`print_matrix`] writes a matrix row by row from RNA's column-by-column array, a 4×4
//!   transform's translation in a length unit (spec, section 6).
//! - [`print_value`] and [`read_value`] print and read a value of any RNA type, as the schema's
//!   property says, exactly (spec, section 6, "Types"; task 1c-02b).
//! - [`canonical`] puts a file's attributes in the schema's order, then lays it out
//!   (spec, section 15; task 1c-01e).
//! - [`format`] is the formatter's layout pass (spec, section 15): two-space
//!   indentation by depth, single spaces between entries, no space inside list
//!   brackets, no trailing whitespace, at most one blank line, one line feed at the
//!   end. It keeps every comment and never changes what the file means. Ordering
//!   attributes by RNA and omitting defaults need the schema and come later.
//!
//! # Use
//!
//! ```
//! use null_print::{Values, write};
//! use null_syntax::{apply_edits, parse};
//! use null_value::Dimension;
//!
//! let text = "object @pier \"Pier\" {\n  location [0m -4m 35cm] // on the water\n}\n";
//! let file = parse(text);
//! let values = Values::F32(&[0.0, -4.0, 0.5]);
//! let edits = write(&file, "object", "@pier", "location", values, Dimension::Length, None).unwrap();
//! let edits: Vec<_> = edits.iter().map(|e| (e.span, e.replacement.as_str())).collect();
//! assert_eq!(
//!     apply_edits(text, &edits),
//!     "object @pier \"Pier\" {\n  location [0m -4m 50cm] // on the water\n}\n"
//! );
//! ```

mod canonical;
mod defaults;
mod format;
mod matrix;
mod project;
mod typed;
mod write;

pub use canonical::canonical;
pub use defaults::{default_of, is_default, without_defaults};
pub use format::format;
pub use matrix::print_matrix;
pub use project::{FileEdit, ProjectFile, Transform, WriteBackError, write_transform};
pub use typed::{Printing, Typed, TypedError, print_value, read_value};
pub use write::{Edit, Values, WriteError, write, write_in};
