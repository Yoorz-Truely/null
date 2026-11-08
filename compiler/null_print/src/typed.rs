//! Every RNA type's value, read from its written form and printed back exactly (spec, section 6,
//! "Types"; task 1c-02b). The schema's property says the type, so the same text means one value:
//! `on` and `off`; a whole number in its hard range; a float in its unit, rounded once; an enum
//! item; a flag set in the schema's item order; a string, or the list of its bytes where it is
//! not UTF-8; and arrays, nested arrays and transform matrices of any of these.
//!
//! [`print_value`] then [`read_value`] gives back the same value, bit for bit (a float's NaN
//! keeps no sign or payload, the one exception, section 6).

use std::fmt;

use null_check::Property;
use null_syntax::{escape, parse};
use null_value::{Dimension, Float, Scene, Unit, print_bare, print_in_scene, read_in, read_int};

/// A value of any RNA type, in its written order: an array's items as written, a matrix row by
/// row.
#[derive(Clone, Debug, PartialEq)]
pub enum Typed {
    /// A boolean.
    Bool(bool),
    /// An integer (RNA's are 32-bit).
    Int(i64),
    /// A float, as RNA holds it.
    F32(f32),
    /// A double.
    F64(f64),
    /// An enum item, by Null name.
    Enum(String),
    /// A flag set: its items, by Null name, in the schema's item order.
    Flags(Vec<String>),
    /// A string's bytes, without the closing NUL.
    Text(Vec<u8>),
    /// An array, or one row of a matrix.
    List(Vec<Typed>),
}

/// Why a written value is not one the property takes, or a value cannot be written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedError(pub String);

impl fmt::Display for TypedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TypedError {}

fn err<T>(message: impl Into<String>) -> Result<T, TypedError> {
    Err(TypedError(message.into()))
}

/// What a value is printed in: the scene (its unit scale and frame rate) and, for floats of a
/// unit, the display unit the project chose (`units length=cm`), used where it prints exactly.
#[derive(Clone, Copy, Debug, Default)]
pub struct Printing {
    /// The scene the value belongs to.
    pub scene: Scene,
    /// The display unit for a length.
    pub length: Option<Unit>,
    /// The display unit for an angle.
    pub angle: Option<Unit>,
}

/// One item's place in an array: its dimension, from the property's unit; a 4×4 transform's
/// translation column is length and its other items bare (section 6).
fn item_dimension(p: &Property, row: Option<usize>, column: Option<usize>) -> Dimension {
    let dims = p.dimensions();
    if p.subtype.as_deref() == Some("matrix") && dims == [4, 4] {
        return if row.is_some_and(|r| r < 3) && column == Some(3) {
            Dimension::Length
        } else {
            Dimension::None
        };
    }
    null_check::unit_dimension(p).unwrap_or(Dimension::None)
}

/// Prints `value` as property `p` writes it.
pub fn print_value(value: &Typed, p: &Property, how: &Printing) -> Result<String, TypedError> {
    match (p.dimensions().as_slice(), value) {
        ([], _) => print_scalar(value, p, how, item_dimension(p, None, None)),
        ([len], Typed::List(items)) => {
            if !p.is_dynamic() && items.len() != *len {
                return err(format!(
                    "`{}` holds {len} values, not {}",
                    p.name,
                    items.len()
                ));
            }
            let words: Result<Vec<_>, _> = items
                .iter()
                .map(|v| print_scalar(v, p, how, item_dimension(p, None, None)))
                .collect();
            Ok(format!("[{}]", words?.join(" ")))
        }
        ([rows, columns, ..], Typed::List(items)) => {
            if items.len() != *rows {
                return err(format!(
                    "`{}` holds {rows} rows, not {}",
                    p.name,
                    items.len()
                ));
            }
            let mut out = Vec::new();
            for (i, row) in items.iter().enumerate() {
                let Typed::List(cells) = row else {
                    return err(format!("`{}`'s row {} is not a list", p.name, i + 1));
                };
                if cells.len() != *columns {
                    return err(format!("`{}`'s rows hold {columns} values", p.name));
                }
                let words: Result<Vec<_>, _> = cells
                    .iter()
                    .enumerate()
                    .map(|(j, v)| print_scalar(v, p, how, item_dimension(p, Some(i), Some(j))))
                    .collect();
                out.push(format!("[{}]", words?.join(" ")));
            }
            Ok(format!("[{}]", out.join(" ")))
        }
        _ => err(format!("`{}` is an array: give a list", p.name)),
    }
}

fn print_float<F: Float>(x: F, dimension: Dimension, how: &Printing) -> String {
    let display = match dimension {
        Dimension::Length => how.length,
        Dimension::Angle => how.angle,
        _ => None,
    };
    display
        .and_then(|u| print_in_scene(x, u, &how.scene))
        .unwrap_or_else(|| print_bare(x))
}

fn print_scalar(
    value: &Typed,
    p: &Property,
    how: &Printing,
    dimension: Dimension,
) -> Result<String, TypedError> {
    match (p.ty.as_str(), value) {
        ("bool", Typed::Bool(b)) => Ok(if *b { "on" } else { "off" }.to_string()),
        ("int", Typed::Int(n)) => {
            in_range(*n as f64, p)?;
            Ok(n.to_string())
        }
        ("float32", Typed::F32(x)) => {
            if !x.is_nan() {
                in_range(f64::from(*x), p)?;
            }
            Ok(print_float(*x, dimension, how))
        }
        ("float64", Typed::F64(x)) => Ok(print_float(*x, dimension, how)),
        ("enum", Typed::Enum(name)) if !p.flags => print_item(name, p),
        ("enum", Typed::Flags(names)) if p.flags => {
            let order = flag_order(names, p)?;
            let words: Result<Vec<_>, _> = order.iter().map(|n| print_item(n, p)).collect();
            Ok(format!("[{}]", words?.join(" ")))
        }
        ("string", Typed::Text(bytes)) => print_text(bytes, p),
        (ty, v) => err(format!("`{}` is a {ty}, not {v:?}", p.name)),
    }
}

fn print_item(name: &str, p: &Property) -> Result<String, TypedError> {
    let item = p.items.iter().flatten().find(|i| i.name == name);
    match item {
        Some(i) if i.quoted => Ok(escape(&i.name)),
        Some(i) => Ok(i.name.clone()),
        None if p.items_at_run_time => Ok(name.to_string()),
        None => err(format!("`{name}` is not an item of `{}`", p.name)),
    }
}

/// The items of a flag set in the schema's item order; an item twice, or one the schema does
/// not list, is an error.
fn flag_order(names: &[String], p: &Property) -> Result<Vec<String>, TypedError> {
    let items: Vec<&str> = p.items.iter().flatten().map(|i| i.name.as_str()).collect();
    let mut seen = Vec::new();
    for n in names {
        if !items.contains(&n.as_str()) {
            return err(format!("`{n}` is not an item of `{}`", p.name));
        }
        if seen.contains(n) {
            return err(format!("`{n}` is given twice in `{}`", p.name));
        }
        seen.push(n.clone());
    }
    Ok(items
        .iter()
        .filter(|i| seen.iter().any(|s| s == *i))
        .map(|s| s.to_string())
        .collect())
}

fn print_text(bytes: &[u8], p: &Property) -> Result<String, TypedError> {
    check_text(bytes, p)?;
    Ok(match std::str::from_utf8(bytes) {
        Ok(s) => escape(s),
        Err(_) => {
            let list: Vec<String> = bytes.iter().map(u8::to_string).collect();
            format!("[{}]", list.join(" "))
        }
    })
}

/// A string Blender can hold: no NUL inside it, and within its field with the closing NUL.
fn check_text(bytes: &[u8], p: &Property) -> Result<(), TypedError> {
    if bytes.contains(&0) {
        return err(format!(
            "`{}` cannot hold a 0 byte: Blender's strings end at one",
            p.name
        ));
    }
    if let Some(max) = p.max_length
        && bytes.len() + 1 > max
    {
        return err(format!(
            "`{}` holds at most {} bytes, not {} (Blender's field is {max} with its closing NUL)",
            p.name,
            max - 1,
            bytes.len()
        ));
    }
    Ok(())
}

fn in_range(x: f64, p: &Property) -> Result<(), TypedError> {
    if let Some(r) = &p.range
        && (x < r.hard[0] || x > r.hard[1])
    {
        return err(format!(
            "`{}` takes {} to {}, not {x}",
            p.name, r.hard[0], r.hard[1]
        ));
    }
    Ok(())
}

/// Reads `text`, a value as written, as property `p` holds it, in `scene`.
pub fn read_value(text: &str, p: &Property, scene: &Scene) -> Result<Typed, TypedError> {
    let file = parse(&format!("v {text}\n"));
    let Some(node) = file.document().nodes().next() else {
        return err("no value");
    };
    let mut args = node.args();
    let (Some(value), None) = (args.next(), args.next()) else {
        return err(format!("`{}` takes one value", p.name));
    };
    read_parsed(&value, p, scene)
}

fn read_parsed(
    value: &null_syntax::ast::Value<'_>,
    p: &Property,
    scene: &Scene,
) -> Result<Typed, TypedError> {
    let dims = p.dimensions();
    fn list<'a>(
        v: &null_syntax::ast::Value<'a>,
        p: &Property,
    ) -> Result<Vec<null_syntax::ast::Value<'a>>, TypedError> {
        match v {
            null_syntax::ast::Value::List(_) => Ok(v.items()),
            _ => err(format!("`{}` takes a list, not `{}`", p.name, v.text())),
        }
    }
    match dims.as_slice() {
        [] if p.is_dynamic() => {
            let items = list(value, p)?;
            if p.min_length.is_some_and(|m| items.len() < m) {
                return err(format!(
                    "`{}` takes at least {} values",
                    p.name,
                    p.min_length.unwrap_or(0)
                ));
            }
            let read: Result<Vec<_>, _> = items
                .iter()
                .map(|v| read_scalar(v, p, scene, item_dimension(p, None, None)))
                .collect();
            Ok(Typed::List(read?))
        }
        [] => read_scalar(value, p, scene, item_dimension(p, None, None)),
        [len] => {
            let items = list(value, p)?;
            if items.len() != *len {
                return err(format!(
                    "`{}` takes {len} values, not {}",
                    p.name,
                    items.len()
                ));
            }
            let read: Result<Vec<_>, _> = items
                .iter()
                .map(|v| read_scalar(v, p, scene, item_dimension(p, None, None)))
                .collect();
            Ok(Typed::List(read?))
        }
        [rows, columns, ..] => {
            let items = list(value, p)?;
            if items.len() != *rows {
                return err(format!(
                    "`{}` takes {rows} rows, not {}",
                    p.name,
                    items.len()
                ));
            }
            let mut out = Vec::new();
            for (i, row) in items.iter().enumerate() {
                let cells = list(row, p)?;
                if cells.len() != *columns {
                    return err(format!("`{}`'s rows take {columns} values", p.name));
                }
                let read: Result<Vec<_>, _> = cells
                    .iter()
                    .enumerate()
                    .map(|(j, v)| read_scalar(v, p, scene, item_dimension(p, Some(i), Some(j))))
                    .collect();
                out.push(Typed::List(read?));
            }
            Ok(Typed::List(out))
        }
    }
}

fn word<'a>(v: &null_syntax::ast::Value<'a>, p: &Property) -> Result<&'a str, TypedError> {
    v.as_word()
        .ok_or_else(|| TypedError(format!("`{}` takes a word, not `{}`", p.name, v.text())))
}

fn read_scalar(
    v: &null_syntax::ast::Value<'_>,
    p: &Property,
    scene: &Scene,
    dimension: Dimension,
) -> Result<Typed, TypedError> {
    match p.ty.as_str() {
        "bool" => match word(v, p)? {
            "on" => Ok(Typed::Bool(true)),
            "off" => Ok(Typed::Bool(false)),
            w => err(format!("`{}` is on or off, not `{w}`", p.name)),
        },
        "int" => {
            let w = word(v, p)?;
            let n = read_int(w).map_err(|e| TypedError(format!("`{}`: {e}", p.name)))?;
            in_range(n as f64, p)?;
            Ok(Typed::Int(n))
        }
        "float32" => {
            let w = word(v, p)?;
            let x: f32 = read_in(w, dimension, scene)
                .map_err(|e| TypedError(format!("`{}`: {e}", p.name)))?;
            if !x.is_nan() {
                in_range(f64::from(x), p)?;
            }
            Ok(Typed::F32(x))
        }
        "float64" => {
            let w = word(v, p)?;
            read_in(w, dimension, scene)
                .map(Typed::F64)
                .map_err(|e| TypedError(format!("`{}`: {e}", p.name)))
        }
        "enum" if p.flags => {
            let items = match v {
                null_syntax::ast::Value::List(_) => v.items(),
                _ => {
                    return err(format!(
                        "`{}` is a set of items: write a list, `[]` for none",
                        p.name
                    ));
                }
            };
            let names: Result<Vec<_>, _> = items.iter().map(|i| read_item(i, p)).collect();
            let names = names?;
            let order = flag_order(&names, p)?;
            if order != names {
                return err(format!(
                    "`{}`'s items are written in the schema's order: [{}]",
                    p.name,
                    order.join(" ")
                ));
            }
            Ok(Typed::Flags(names))
        }
        "enum" => read_item(v, p).map(Typed::Enum),
        "string" => {
            let bytes = match v {
                null_syntax::ast::Value::List(_) => {
                    let mut bytes = Vec::new();
                    for b in v.items() {
                        let n = read_int(word(&b, p)?)
                            .ok()
                            .and_then(|n| u8::try_from(n).ok())
                            .ok_or_else(|| {
                                TypedError(format!(
                                    "`{}`'s bytes are 1 to 255, not `{}`",
                                    p.name,
                                    b.text()
                                ))
                            })?;
                        bytes.push(n);
                    }
                    if std::str::from_utf8(&bytes).is_ok() {
                        return err(format!(
                            "`{}`'s bytes are valid UTF-8: write them as a string",
                            p.name
                        ));
                    }
                    bytes
                }
                _ => v
                    .as_string()
                    .ok_or_else(|| {
                        TypedError(format!("`{}` takes a string, not `{}`", p.name, v.text()))
                    })?
                    .into_owned()
                    .into_bytes(),
            };
            check_text(&bytes, p)?;
            Ok(Typed::Text(bytes))
        }
        ty => err(format!(
            "`{}` is a {ty}, which has no value of its own here",
            p.name
        )),
    }
}

fn read_item(v: &null_syntax::ast::Value<'_>, p: &Property) -> Result<String, TypedError> {
    let name = match v.as_string() {
        Some(s) => s.into_owned(),
        None => word(v, p)?.to_string(),
    };
    let item = p.items.iter().flatten().find(|i| i.name == name);
    match item {
        Some(i) if i.quoted == v.as_string().is_some() => Ok(name),
        Some(_) => err(format!(
            "`{name}` is written {}",
            if v.as_string().is_some() {
                "bare"
            } else {
                "as a string"
            }
        )),
        None if p.items_at_run_time => Ok(name),
        None => err(format!("`{name}` is not an item of `{}`", p.name)),
    }
}
