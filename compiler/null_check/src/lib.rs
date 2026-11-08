//! Null's checker: every name, type, unit and handle of a project against the schema from RNA.
//!
//! Implements Sau's decision S24, strict type safety "with no compromises": a name
//! not in the schema, a value of the wrong type or a unit a property cannot take is
//! always an error, with a suggestion wherever one is clear. The schema is generated from RNA
//! by Sau: `schema/null-schema-project.json` holds all project data (tasks 1b-03b and 1b-03e),
//! `schema/null-schema.json` phase 1a's slice (task 1a-05), and either loads; the few forms
//! where RNA is not enough (project, units, membership, materials, mesh data,
//! primitives, node trees) are Null's own, listed in the spec, section 8 ("Null's
//! forms over RNA"), and in [`forms`].
//!
//! # Use
//!
//! ```
//! use null_check::{Schema, check};
//! use null_syntax::{FileId, parse};
//!
//! let schema = Schema::from_json(include_str!("../../../schema/null-schema.json")).unwrap();
//! let file = parse("object @cube \"Cube\" {\n  locaton [0m 0m 1m]\n}\n");
//! let found = check(&schema, &[(FileId(0), &file)]);
//! assert_eq!(found[0].code, "null-unknown-property");
//! assert_eq!(found[0].fixes[0].edits[0].replacement, "location");
//! ```

mod check;
pub mod expand;
pub mod forms;
pub mod level0;
pub mod paths;
pub mod raw;
mod schema;
pub mod units;
mod values;

pub use check::check;
pub use expand::{Expanded, check_expanded, expand};
pub use schema::{
    Datablock, Item, NodeDef, Primitive, Property, Range, SHADER_TREE, Schema, Struct,
};
pub use values::unit_dimension;

/// The checker's diagnostic codes (spec, section 18). They are stable.
pub mod codes {
    /// A datablock kind, node kind or primitive the schema does not know.
    pub const KIND: &str = "null-kind";
    /// A property the kind does not have.
    pub const UNKNOWN_PROPERTY: &str = "null-unknown-property";
    /// A value of the wrong type or shape.
    pub const TYPE: &str = "null-type";
    /// A unit the property cannot take, or no unit at all.
    pub const UNIT: &str = "null-unit";
    /// A value outside the property's hard range.
    pub const RANGE: &str = "null-range";
    /// An enum value the property does not list.
    pub const ENUM: &str = "null-enum";
    /// A handle that resolves to nothing, to the wrong kind, or is defined twice.
    pub const HANDLE: &str = "null-handle";
    /// A property Blender does not let be set.
    pub const READONLY: &str = "null-readonly";
    /// A property given twice.
    pub const REPEATED: &str = "null-repeated";
    /// A handle or enum value that is not kebab-case (the syntax checks names, `null_syntax`).
    pub const NAME: &str = "null-name";
    /// Text that does not fit the node syntax (`null_syntax`'s).
    pub const SYNTAX: &str = null_syntax::codes::SYNTAX;

    /// Every code of Null's diagnostics, the one list of them (spec, section 18; task 1d-02c), in
    /// the spec's order. A test fails when the spec's list and this one differ, and when a code
    /// has no snapshot test.
    pub const ALL: [&str; 11] = [
        SYNTAX,
        NAME,
        KIND,
        UNKNOWN_PROPERTY,
        TYPE,
        UNIT,
        RANGE,
        ENUM,
        HANDLE,
        READONLY,
        REPEATED,
    ];
}
