//! The schema Sau generates from RNA: the slice's (`schema/null-schema.json`) or that of all
//! project data (`schema/null-schema-project.json`): structs, properties, nodes.
//!
//! Read as data, never edited by hand: Sau's `sau/tools/null-schema.py`, run in the
//! built Sau, writes it (tasks 1a-05 and 1b-03b). Names are already Null's (kebab-case, S16).
//! Structs are found by RNA identifier and by kind through indexes made once at load, since the
//! project's schema has about 1,350 of them (task 1b-03e).

use std::collections::{BTreeMap, HashMap};

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
    /// Add-ons' settings, each its own section (task 1b-03g): Cycles' render, object, light and
    /// world settings. Their structs are read as Blender's are; their properties on Blender's
    /// structs (`Scene.cycles`) are added to those structs'.
    #[serde(default)]
    pub addons: BTreeMap<String, Addon>,
    /// Add-ons' properties on Blender's structs, by owner, made by [`Schema::from_json`].
    #[serde(skip)]
    attached: HashMap<String, Vec<Property>>,
    /// Index into `structs` by RNA identifier, made by [`Schema::from_json`].
    #[serde(skip)]
    by_rna: HashMap<String, usize>,
    /// Index into `structs` by kind, made by [`Schema::from_json`].
    #[serde(skip)]
    by_kind: HashMap<String, usize>,
    /// The raw table, the fields each DNA struct's `raw` block takes (section 19, Sau's task
    /// 1d-05b), when given ([`Schema::with_raw`]); without it a `raw` block's lines are not
    /// checked against DNA.
    #[serde(skip)]
    pub raw: Option<crate::raw::RawTable>,
}

/// One add-on's section of the project's schema (task 1b-03g).
#[derive(Clone, Debug, Deserialize)]
pub struct Addon {
    /// Its Python module, such as `cycles`.
    pub module: String,
    /// Its version: Blender's, for an add-on bundled with Blender.
    pub version: String,
    /// The properties it adds to Blender's structs.
    pub attached: Vec<Attached>,
    /// The structs it defines.
    pub structs: Vec<Struct>,
}

/// An add-on's property on one of Blender's structs, such as `Scene.cycles`.
#[derive(Clone, Debug, Deserialize)]
pub struct Attached {
    /// The struct it is on, by RNA identifier.
    pub owner: String,
    /// The property.
    #[serde(flatten)]
    pub property: Property,
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
    /// A node socket's identifier (`Base Color`, `A_Float`), by which Blender finds it, where
    /// the schema lists the property as a socket (Sau's task 1d-07b).
    #[serde(default)]
    pub socket: Option<String>,
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
    /// A pointer Blender never lets be empty (RNA's `PROP_NEVER_NULL`): `none` is an error there
    /// (Sau's task 1c-02j).
    #[serde(default, rename = "never-none")]
    pub never_none: bool,
    /// Whether RNA lets it be set.
    #[serde(default)]
    pub readonly: bool,
    /// Whether the enum's items are computed when it is used, so `items` are only those RNA
    /// lists ahead (task 1b-03b): another value is not refused.
    #[serde(default, rename = "items-at-run-time")]
    pub items_at_run_time: bool,
    /// Whether the enum is a flag set, any number of its items at once (task 1c-02b).
    #[serde(default)]
    pub flags: bool,
    /// The most bytes a string holds, its closing NUL included, where Blender's field is fixed
    /// (task 1c-02b); none where it is not, or the schema does not say.
    #[serde(default, rename = "max-length")]
    pub max_length: Option<usize>,
    /// Whether a node's input socket takes several links, as a list (a join's `geometry`).
    #[serde(default, rename = "multi-input")]
    pub multi_input: bool,
    /// RNA's default, where the schema gives one (task 1d-02b).
    #[serde(default)]
    pub default: Option<Json>,
    /// The value in a new datablock, where it differs from RNA's default (section 8): the default
    /// the formatter leaves out (task 1d-02b).
    #[serde(default)]
    pub new: Option<Json>,
    /// For an enum over a field's bits that names fewer states than they hold, the bits it
    /// cannot name, which a `raw` block carries masked (Sau's task 1b-03n).
    #[serde(default)]
    pub hides: Vec<Hides>,
}

/// Bits an enum cannot name (Sau's task 1b-03n): the field (by its datablock's DNA struct and
/// path), the mask, and the states that fix those bits, by Null's item name, with their value.
#[derive(Clone, Debug, Deserialize)]
pub struct Hides {
    /// The datablock's DNA struct, `Scene`.
    #[serde(rename = "struct")]
    pub owner: String,
    /// The field's path in the code's names, `flag`.
    pub path: String,
    /// The bits, as an unsigned number.
    pub mask: u64,
    /// Each state in which the bits are always one value, with that value (`frame-drop`: 8).
    #[serde(default)]
    pub states: std::collections::BTreeMap<String, u64>,
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
    /// RNA's identifier, such as `XYZ`, where the schema gives it (the project's does): what
    /// Sau sets through the bridge (task 1c-02d).
    #[serde(default)]
    pub rna: Option<String>,
    /// Whether the name is free text, not an identifier (a colour space's), so it is written
    /// as a string (S16).
    #[serde(default)]
    pub quoted: bool,
}

/// A node kind.
#[derive(Clone, Debug, Deserialize)]
pub struct NodeDef {
    /// Null's kind, such as `bsdf-principled` (Sau's decision S67: Blender's identifier with its
    /// type prefix removed, in kebab case).
    pub kind: String,
    /// Its RNA identifier, such as `ShaderNodeBsdfPrincipled`, where the schema gives it.
    #[serde(default)]
    pub rna: Option<String>,
    /// The node trees that take it, by RNA identifier (`ShaderNodeTree`), each with the kind it
    /// has there; where two kinds of one tree would share a name, the tree's own family keeps it
    /// (`geometry-switch` in a compositor tree). Absent in the slice's schema, whose nodes are all
    /// shader nodes under `kind`.
    #[serde(default)]
    pub trees: Option<std::collections::BTreeMap<String, String>>,
    /// Its inputs.
    pub inputs: Vec<Property>,
    /// Its outputs.
    pub outputs: Vec<Property>,
    /// A new node's width in the editor, its kind's default, which a `layout` line leaves out
    /// (section 10; Sau's decision S79). Absent from a schema made before Sau's task 1d-07b.
    #[serde(default)]
    pub width: Option<f32>,
    /// A new frame's height, the one node kind whose height a `layout` line writes.
    #[serde(default)]
    pub height: Option<f32>,
}

impl NodeDef {
    /// Its kind in `tree` (an RNA identifier such as `ShaderNodeTree`), if that tree takes it.
    pub fn kind_in(&self, tree: &str) -> Option<&str> {
        match &self.trees {
            Some(trees) => trees.get(tree).map(String::as_str),
            None => (tree == SHADER_TREE).then_some(self.kind.as_str()),
        }
    }
}

/// A material's node tree, the slice's one kind of tree.
pub const SHADER_TREE: &str = "ShaderNodeTree";
/// The RNA name of a texture node tree, the tree a texture holds (Null spec, section 10).
pub const TEXTURE_TREE: &str = "TextureNodeTree";

/// A mesh primitive.
#[derive(Clone, Debug, Deserialize)]
pub struct Primitive {
    /// Null's kind, such as `cube`.
    pub kind: String,
    /// The operator's settings.
    pub properties: Vec<Property>,
}

impl Schema {
    /// The schema with its raw table (`schema/null-raw.json`, section 19).
    pub fn with_raw(mut self, raw: crate::raw::RawTable) -> Schema {
        self.raw = Some(raw);
        self
    }

    /// Reads a schema file's text.
    pub fn from_json(text: &str) -> Result<Schema, serde_json::Error> {
        let mut schema: Schema = serde_json::from_str(text)?;
        let addons: Vec<Addon> = schema.addons.values().cloned().collect();
        for addon in addons {
            schema.structs.extend(addon.structs);
            for a in addon.attached {
                schema.attached.entry(a.owner).or_default().push(a.property);
            }
        }
        for (i, s) in schema.structs.iter().enumerate() {
            schema.by_rna.entry(s.rna.clone()).or_insert(i);
        }
        // A kind is unique where it is read (S67): among the datablock types, in a family, in a
        // tree. Read alone, it names the datablock type first (`scene` is `Scene`, not the node
        // socket `NodeSocketScene`), then the first struct of that kind.
        let datablock_rnas: Vec<String> = schema
            .datablocks
            .iter()
            .flatten()
            .map(|d| d.rna.clone())
            .collect();
        for rna in &datablock_rnas {
            if let Some(&i) = schema.by_rna.get(rna) {
                let kind = schema.structs[i].kind.clone();
                schema.by_kind.entry(kind).or_insert(i);
            }
        }
        for (i, s) in schema.structs.iter().enumerate() {
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

    /// `rna`'s properties and those of every struct it extends, its own first, with what add-ons
    /// add to each (`Scene.cycles`).
    pub fn properties_of<'a>(&'a self, rna: &str) -> Vec<&'a Property> {
        let mut out = Vec::new();
        let mut at = self.struct_by_rna(rna);
        while let Some(s) = at {
            out.extend(s.properties.iter());
            out.extend(self.attached.get(&s.rna).into_iter().flatten());
            at = s.base.as_deref().and_then(|b| self.struct_by_rna(b));
        }
        out
    }

    /// Whether the struct `rna` is a datablock type, which a pointer names by its handle; any
    /// other a pointer holds is written as a block of its properties.
    pub fn is_datablock_struct(&self, rna: &str) -> bool {
        let mut at = self.struct_by_rna(rna);
        while let Some(s) = at {
            if s.rna == "ID" {
                return true;
            }
            at = s.base.as_deref().and_then(|b| self.struct_by_rna(b));
        }
        rna == "ID"
    }

    /// The node of kind `kind` in a tree of type `tree` (`ShaderNodeTree`).
    pub fn node(&self, tree: &str, kind: &str) -> Option<&NodeDef> {
        self.nodes.iter().find(|n| n.kind_in(tree) == Some(kind))
    }

    /// Every node kind a tree of type `tree` takes.
    pub fn node_kinds(&self, tree: &str) -> Vec<&str> {
        self.nodes.iter().filter_map(|n| n.kind_in(tree)).collect()
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
            socket: None,
            array: None,
            min_length: None,
            subtype: None,
            unit: None,
            range: None,
            items: None,
            target: None,
            never_none: false,
            readonly: false,
            items_at_run_time: false,
            flags: false,
            max_length: None,
            multi_input: false,
            default: None,
            new: None,
            hides: Vec::new(),
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
