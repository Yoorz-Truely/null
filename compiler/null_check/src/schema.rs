//! The schema Sau generates from RNA: the slice's (`schema/null-schema.json`) or that of all
//! project data (`schema/null-schema-project.json`): structs, properties, nodes.
//!
//! Read as data, never edited by hand: Sau's `sau/tools/null-schema.py`, run in the
//! built Sau, writes it (tasks 1a-05 and 1b-03b). Names are already Null's (kebab-case, S16).
//! Structs are found by RNA identifier and by kind through indexes made once at load, since the
//! project's schema has about 1,350 of them (task 1b-03e).

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value as Json;

/// The whole schema file.
#[derive(Clone, Debug, Deserialize)]
pub struct Schema {
    /// The file's own format number.
    pub format: u32,
    /// The datablock types, where the file lists them (the project's schema): RNA identifiers
    /// with their kinds. The slice's schema has none; there a datablock extends `ID` directly.
    #[serde(default)]
    pub datablocks: Option<Vec<Datablock>>,
    /// RNA structs: datablocks (`base` `ID`) and the structs inside them.
    pub structs: Vec<Struct>,
    /// Shader nodes, with their input and output sockets.
    pub nodes: Vec<NodeDef>,
    /// Mesh primitives: the add operators and their settings.
    pub primitives: Vec<Primitive>,
    /// Index into `structs` by RNA identifier, made by [`Schema::from_json`].
    #[serde(skip)]
    by_rna: HashMap<String, usize>,
    /// Index into `structs` by kind, made by [`Schema::from_json`].
    #[serde(skip)]
    by_kind: HashMap<String, usize>,
}

/// One datablock type of the project's schema.
#[derive(Clone, Debug, Deserialize)]
pub struct Datablock {
    /// Null's kind, such as `object`.
    pub kind: String,
    /// RNA's struct identifier, such as `Object`.
    pub rna: String,
}

/// One RNA struct.
#[derive(Clone, Debug, Deserialize)]
pub struct Struct {
    /// Null's kind, such as `object` or `point-light`.
    pub kind: String,
    /// RNA's struct identifier, such as `Object`.
    pub rna: String,
    /// The struct it extends (`ID` for datablocks, `Light` for point lights).
    #[serde(default)]
    pub base: Option<String>,
    /// Its properties, in RNA's order.
    pub properties: Vec<Property>,
}

/// One RNA property, or a node socket, or an operator setting.
#[derive(Clone, Debug, Deserialize)]
pub struct Property {
    /// Null's name, such as `rotation-euler`.
    pub name: String,
    /// The type: `float32`, `int`, `bool`, `enum`, `string`, `pointer`, `collection`.
    #[serde(rename = "type")]
    pub ty: String,
    /// A fixed length (`[3]`) or `"dynamic"`.
    #[serde(default)]
    pub array: Option<Json>,
    /// The shortest a dynamic array may be.
    #[serde(default, rename = "min-length")]
    pub min_length: Option<usize>,
    /// RNA's subtype, such as `color` or `translation`.
    #[serde(default)]
    pub subtype: Option<String>,
    /// RNA's unit: `length`, `rotation`, `time`, `camera` or none.
    #[serde(default)]
    pub unit: Option<String>,
    /// The hard and soft ranges.
    #[serde(default)]
    pub range: Option<Range>,
    /// The enum's items.
    #[serde(default)]
    pub items: Option<Vec<Item>>,
    /// The struct a pointer or collection refers to.
    #[serde(default)]
    pub target: Option<String>,
    /// Whether RNA lets it be set.
    #[serde(default)]
    pub readonly: bool,
    /// Whether the enum's items are computed when it is used, so `items` are only those RNA
    /// lists ahead (task 1b-03b): another value is not refused.
    #[serde(default, rename = "items-at-run-time")]
    pub items_at_run_time: bool,
}

/// A property's ranges.
#[derive(Clone, Debug, Deserialize)]
pub struct Range {
    /// What RNA refuses outside.
    pub hard: [f64; 2],
}

/// One enum item.
#[derive(Clone, Debug, Deserialize)]
pub struct Item {
    /// Null's name, such as `xyz`.
    pub name: String,
}

/// A shader node kind.
#[derive(Clone, Debug, Deserialize)]
pub struct NodeDef {
    /// Null's kind, such as `principled-bsdf`.
    pub kind: String,
    /// Its inputs.
    pub inputs: Vec<Property>,
    /// Its outputs.
    pub outputs: Vec<Property>,
}

/// A mesh primitive.
#[derive(Clone, Debug, Deserialize)]
pub struct Primitive {
    /// Null's kind, such as `cube`.
    pub kind: String,
    /// The operator's settings.
    pub properties: Vec<Property>,
}

impl Schema {
    /// Reads a schema file's text.
    pub fn from_json(text: &str) -> Result<Schema, serde_json::Error> {
        let mut schema: Schema = serde_json::from_str(text)?;
        for (i, s) in schema.structs.iter().enumerate() {
            schema.by_rna.entry(s.rna.clone()).or_insert(i);
            schema.by_kind.entry(s.kind.clone()).or_insert(i);
        }
        Ok(schema)
    }

    /// The struct with RNA identifier `rna`.
    pub fn struct_by_rna(&self, rna: &str) -> Option<&Struct> {
        self.by_rna.get(rna).map(|&i| &self.structs[i])
    }

    /// The struct of Null kind `kind`.
    pub fn struct_by_kind(&self, kind: &str) -> Option<&Struct> {
        self.by_kind.get(kind).map(|&i| &self.structs[i])
    }

    /// `rna`'s properties and those of every struct it extends, its own first.
    pub fn properties_of<'a>(&'a self, rna: &str) -> Vec<&'a Property> {
        let mut out = Vec::new();
        let mut at = self.struct_by_rna(rna);
        while let Some(s) = at {
            out.extend(s.properties.iter());
            at = s.base.as_deref().and_then(|b| self.struct_by_rna(b));
        }
        out
    }

    /// The datablock kinds: the file's list of datablock types where it has one (leaving out
    /// `ID` itself), else the structs that extend `ID` directly.
    pub fn datablock_kinds(&self) -> Box<dyn Iterator<Item = &Struct> + '_> {
        match &self.datablocks {
            Some(list) => Box::new(
                list.iter()
                    .filter(|d| d.rna != "ID")
                    .filter_map(|d| self.struct_by_rna(&d.rna)),
            ),
            None => Box::new(
                self.structs
                    .iter()
                    .filter(|s| s.base.as_deref() == Some("ID")),
            ),
        }
    }
}

impl Property {
    /// A float with no unit, no range and no other constraint.
    pub(crate) fn any() -> Property {
        Property {
            name: String::new(),
            ty: "float32".into(),
            array: None,
            min_length: None,
            subtype: None,
            unit: None,
            range: None,
            items: None,
            target: None,
            readonly: false,
            items_at_run_time: false,
        }
    }

    /// A single number of type `ty` within the hard range `lo` to `hi`: for Null's own forms
    /// that carry an RNA property's type and range (a scene's `fps`).
    pub(crate) fn ranged(ty: &str, lo: f64, hi: f64) -> Property {
        Property {
            ty: ty.into(),
            range: Some(Range { hard: [lo, hi] }),
            ..Property::any()
        }
    }

    /// The fixed length of the array, if it is one: its first dimension, the number of rows of
    /// a matrix.
    pub fn fixed_len(&self) -> Option<usize> {
        self.dimensions().first().copied()
    }

    /// A fixed array's dimensions: `[3]` for a vector, `[4, 4]` for a matrix, `[8, 3]` for a
    /// bounding box's corners; empty for a single value or a dynamic array.
    pub fn dimensions(&self) -> Vec<usize> {
        match &self.array {
            Some(Json::Array(a)) => a
                .iter()
                .filter_map(Json::as_u64)
                .map(|n| n as usize)
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Whether it is an array of any length.
    pub fn is_dynamic(&self) -> bool {
        matches!(&self.array, Some(Json::String(s)) if s == "dynamic")
    }

    /// The enum items' names.
    pub fn item_names(&self) -> Vec<&str> {
        self.items
            .iter()
            .flatten()
            .map(|i| i.name.as_str())
            .collect()
    }
}
