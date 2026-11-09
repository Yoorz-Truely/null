//! The `raw` block (spec, section 19; Sau's task 1d-05b): the DNA fields RNA does not reach,
//! written in DNA's own names and types, so a `.blend` converts to Null and back with every saved
//! bit.
//!
//! Which fields a datablock's `raw` block takes is the raw table, `schema/null-raw.json`, which
//! Sau's build writes (`coverage-audit.py`, `just schema-sync`): per DNA struct, each field of the
//! struct or of a struct embedded in it that no RNA property reads and no rule over DNA's facts
//! leaves out (Sau's tasks 1b-04a and 1b-04b), by its path in the code's names (`transflag`,
//! `r.mode`), its DNA type and its array dimensions.
//!
//! A value is written as its type reads:
//! - an integer in decimal, held to its type's range; `char`, `uchar` and `bool` as the byte's
//!   value, 0 to 255, so a byte reads the same on every platform;
//! - a float or double as the shortest decimal that reads back to its bits, `inf`, `-inf` or
//!   `nan`; and, in a `raw` block only, as its bits, `0x` and 8 hexadecimal digits for a float or
//!   16 for a double, which is how a NaN other than the quiet one is written, so its sign and
//!   payload survive;
//! - an array as a list of exactly its length, nested for each dimension; a `char` array as the
//!   list of every byte, those after its closing NUL included.
//!
//! Bytes are in the order of the machine that holds them, as DNA's are in memory.

use std::collections::BTreeMap;

use null_syntax::ast::Value;
use null_value::{Dimension, print_bare, read};
use serde::Deserialize;

/// One field a `raw` block may write.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct RawField {
    /// Its path from the datablock's struct, in the code's names: `flag`, `r.mode`.
    pub path: String,
    /// Its DNA type: `short`, `float`, `char`, …
    #[serde(rename = "type")]
    pub ty: String,
    /// Its array dimensions, outermost first; none for a single value.
    #[serde(default)]
    pub dims: Vec<usize>,
    /// For a whole number RNA reads in part, the bits a `raw` line carries, as an unsigned
    /// number: those an enum over its bits cannot name (`Scene.flag`'s `SCE_FRAME_DROP` under
    /// `sync-mode`'s `audio-sync`, Sau's task 1b-03n). The line holds those bits alone, and
    /// setting it leaves the field's others as they are; none for a field written whole.
    #[serde(default)]
    pub mask: Option<u64>,
}

/// The raw table: the fields each DNA struct's `raw` block may write.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RawTable {
    /// The file's own format number.
    pub format: u32,
    /// By DNA struct (`Object`, `Scene`), in the table's order.
    #[serde(default)]
    pub structs: BTreeMap<String, Vec<RawField>>,
}

impl RawTable {
    /// Reads a raw table file's text.
    pub fn from_json(text: &str) -> Result<RawTable, serde_json::Error> {
        serde_json::from_str(text)
    }

    /// The fields of struct `name`; none for a struct the table does not list.
    pub fn fields(&self, name: &str) -> &[RawField] {
        self.structs
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Field `path` of struct `name`.
    pub fn field(&self, name: &str, path: &str) -> Option<&RawField> {
        self.fields(name).iter().find(|f| f.path == path)
    }
}

/// A DNA scalar type, as a raw block reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scalar {
    /// A byte: `char`, `uchar`, `uint8_t`, `bool`, written 0 to 255.
    U8,
    /// `int8_t`.
    I8,
    /// `short`, `int16_t`.
    I16,
    /// `ushort`, `uint16_t`.
    U16,
    /// `int`, `int32_t`.
    I32,
    /// `uint`, `uint32_t`.
    U32,
    /// `int64_t`.
    I64,
    /// `uint64_t`.
    U64,
    /// `float`.
    F32,
    /// `double`.
    F64,
}

impl Scalar {
    /// The DNA type `ty`'s scalar; `None` for a type a raw block does not write (a struct, a
    /// pointer).
    pub fn of(ty: &str) -> Option<Scalar> {
        Some(match ty {
            "char" | "uchar" | "uint8_t" | "bool" => Scalar::U8,
            "int8_t" => Scalar::I8,
            "short" | "int16_t" => Scalar::I16,
            "ushort" | "uint16_t" => Scalar::U16,
            "int" | "int32_t" => Scalar::I32,
            "uint" | "uint32_t" => Scalar::U32,
            "int64_t" => Scalar::I64,
            "uint64_t" => Scalar::U64,
            "float" => Scalar::F32,
            "double" => Scalar::F64,
            _ => return None,
        })
    }

    /// Its size in bytes.
    pub fn size(self) -> usize {
        match self {
            Scalar::U8 | Scalar::I8 => 1,
            Scalar::I16 | Scalar::U16 => 2,
            Scalar::I32 | Scalar::U32 | Scalar::F32 => 4,
            Scalar::I64 | Scalar::U64 | Scalar::F64 => 8,
        }
    }

    /// The range of an integer type.
    fn range(self) -> Option<(i128, i128)> {
        Some(match self {
            Scalar::U8 => (0, 255),
            Scalar::I8 => (-128, 127),
            Scalar::I16 => (i16::MIN.into(), i16::MAX.into()),
            Scalar::U16 => (0, u16::MAX.into()),
            Scalar::I32 => (i32::MIN.into(), i32::MAX.into()),
            Scalar::U32 => (0, u32::MAX.into()),
            Scalar::I64 => (i64::MIN.into(), i64::MAX.into()),
            Scalar::U64 => (0, u64::MAX.into()),
            Scalar::F32 | Scalar::F64 => return None,
        })
    }
}

impl RawField {
    /// Its scalar type, if a raw block writes it.
    pub fn scalar(&self) -> Option<Scalar> {
        Scalar::of(&self.ty)
    }

    /// Its size in bytes.
    pub fn size(&self) -> Option<usize> {
        Some(self.scalar()?.size() * self.dims.iter().product::<usize>())
    }

    /// `bytes` of this field (a single whole number) as an unsigned number, in the machine's
    /// order.
    fn bits(bytes: &[u8]) -> u64 {
        let mut wide = [0u8; 8];
        if cfg!(target_endian = "little") {
            wide[..bytes.len().min(8)].copy_from_slice(&bytes[..bytes.len().min(8)]);
        } else {
            let n = bytes.len().min(8);
            wide[8 - n..].copy_from_slice(&bytes[bytes.len() - n..]);
        }
        u64::from_ne_bytes(wide)
    }

    /// `bits` as this field's bytes, in the machine's order.
    fn to_bytes(&self, bits: u64, size: usize) -> Vec<u8> {
        let wide = bits.to_ne_bytes();
        if cfg!(target_endian = "little") {
            wide[..size].to_vec()
        } else {
            wide[8 - size..].to_vec()
        }
    }

    /// The bits of `bytes` a `raw` line carries: all of them, or, for a masked field, those of
    /// its mask (the others zero).
    pub fn carried(&self, bytes: &[u8]) -> Vec<u8> {
        match self.mask {
            Some(m) => self.to_bytes(Self::bits(bytes) & m, bytes.len()),
            None => bytes.to_vec(),
        }
    }

    /// The field's bytes `now`, with a line's `written` set: all of them, or, for a masked
    /// field, the mask's bits from `written` and the others from `now`.
    pub fn merged(&self, now: &[u8], written: &[u8]) -> Vec<u8> {
        match self.mask {
            Some(m) if now.len() == written.len() => self.to_bytes(
                (Self::bits(now) & !m) | (Self::bits(written) & m),
                now.len(),
            ),
            _ => written.to_vec(),
        }
    }

    /// How it is written, for a message: `a short`, `a list of 3 floats`.
    pub fn describe(&self) -> String {
        match self.dims.as_slice() {
            [] => format!("one {}", self.ty),
            dims => {
                let shape: Vec<String> = dims.iter().map(usize::to_string).collect();
                format!("a list of {} {}s", shape.join(" × "), self.ty)
            }
        }
    }
}

/// Why a raw value does not read: in words, and, where there is one, the word to write instead
/// (a float's bits that are not a NaN's: its decimal), with the span of the word written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawError {
    /// What is wrong, in words.
    pub message: String,
    /// The word to write in place of the one at its span.
    pub fix: Option<(null_syntax::Span, String)>,
}

impl From<String> for RawError {
    fn from(message: String) -> RawError {
        RawError { message, fix: None }
    }
}

impl std::fmt::Display for RawError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// The bytes of `value` written for `field`, or why it does not read.
pub fn read_value(value: &Value<'_>, field: &RawField) -> Result<Vec<u8>, RawError> {
    let scalar = field.scalar().ok_or_else(|| {
        format!(
            "`{}` is a {}, which a raw block does not write",
            field.path, field.ty
        )
    })?;
    let mut out = Vec::with_capacity(field.size().unwrap_or(0));
    read_into(value, &field.dims, scalar, field, &mut out)?;
    if let Some(m) = field.mask {
        let size = scalar.size();
        let set = RawField::bits(&out) & (u64::MAX >> (64 - 8 * size as u32));
        if !field.dims.is_empty() || scalar.range().is_none() {
            return Err(format!(
                "`{}` is masked, which only a single whole number is",
                field.path
            )
            .into());
        }
        if set & !m != 0 {
            return Err(format!(
                "`{}` carries only the bits {m} here, which RNA cannot name; `{}` sets others",
                field.path,
                value.text()
            )
            .into());
        }
    }
    Ok(out)
}

fn read_into(
    value: &Value<'_>,
    dims: &[usize],
    scalar: Scalar,
    field: &RawField,
    out: &mut Vec<u8>,
) -> Result<(), RawError> {
    let wrong = || {
        format!(
            "`{}` takes {}, not `{}`",
            field.path,
            field.describe(),
            value.text()
        )
    };
    let Some((&len, inner)) = dims.split_first() else {
        let word = value.as_word().ok_or_else(wrong)?;
        return read_scalar(word, scalar, field, out).map_err(|e| match e {
            WordError::Message(m) => RawError::from(m),
            WordError::Decimal(m, w) => RawError {
                message: m,
                fix: Some((value.span(), w)),
            },
        });
    };
    let Value::List(_) = value else {
        return Err(wrong().into());
    };
    let items = value.items();
    if items.len() != len {
        return Err(format!(
            "`{}` takes {}; this list has {}",
            field.path,
            field.describe(),
            items.len()
        )
        .into());
    }
    for item in &items {
        read_into(item, inner, scalar, field, out)?;
    }
    Ok(())
}

/// Why one word does not read: in words, or with the decimal to write instead.
enum WordError {
    Message(String),
    Decimal(String, String),
}

impl From<String> for WordError {
    fn from(m: String) -> WordError {
        WordError::Message(m)
    }
}

fn read_scalar(
    word: &str,
    scalar: Scalar,
    field: &RawField,
    out: &mut Vec<u8>,
) -> Result<(), WordError> {
    let bad = || format!("`{}` is a {}: `{word}` is not one", field.path, field.ty);
    if let Some((lo, hi)) = scalar.range() {
        let n: i128 = word.parse().map_err(|_| bad())?;
        if n < lo || n > hi {
            return Err(format!(
                "`{}` is a {}, from {lo} to {hi}: {word} is out of its range",
                field.path, field.ty
            )
            .into());
        }
        let bytes = n.to_ne_bytes();
        // The low bytes of the two's complement, in the machine's order.
        let size = scalar.size();
        if cfg!(target_endian = "little") {
            out.extend_from_slice(&bytes[..size]);
        } else {
            out.extend_from_slice(&bytes[16 - size..]);
        }
        return Ok(());
    }
    // Bits, `0x` and lowercase hexadecimal digits: one per half-byte, every one written.
    let hex = word.strip_prefix("0x");
    if let Some(h) = hex
        && !h
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(format!(
            "`{}` is a {}: its bits are written `0x` and lowercase hexadecimal digits",
            field.path, field.ty
        )
        .into());
    }
    // Bits are a NaN's alone, one the word `nan` does not mean: anything else has one written
    // form, its decimal (or `nan`), which is the fix.
    let only_for_nan = |is_nan: bool, quiet: bool, decimal: String| -> Result<(), WordError> {
        if is_nan && !quiet {
            return Ok(());
        }
        Err(WordError::Decimal(
            format!(
                "`{}`: a {}'s bits are written only for a NaN other than `nan`; write `{decimal}`",
                field.path, field.ty
            ),
            decimal,
        ))
    };
    match scalar {
        Scalar::F32 => {
            let x = match hex {
                Some(h) if h.len() == 8 => {
                    let bits = u32::from_str_radix(h, 16).map_err(|_| bad())?;
                    let x = f32::from_bits(bits);
                    only_for_nan(x.is_nan(), bits == QUIET_F32, print_bare(x))?;
                    x
                }
                Some(_) => {
                    return Err(format!(
                        "`{}` is a float: its bits are `0x` and 8 hexadecimal digits",
                        field.path
                    )
                    .into());
                }
                None => read::<f32>(word, Dimension::None).map_err(|_| bad())?,
            };
            out.extend_from_slice(&x.to_ne_bytes());
        }
        _ => {
            let x = match hex {
                Some(h) if h.len() == 16 => {
                    let bits = u64::from_str_radix(h, 16).map_err(|_| bad())?;
                    let x = f64::from_bits(bits);
                    only_for_nan(x.is_nan(), bits == QUIET_F64, print_bare(x))?;
                    x
                }
                Some(_) => {
                    return Err(format!(
                        "`{}` is a double: its bits are `0x` and 16 hexadecimal digits",
                        field.path
                    )
                    .into());
                }
                None => read::<f64>(word, Dimension::None).map_err(|_| bad())?,
            };
            out.extend_from_slice(&x.to_ne_bytes());
        }
    }
    Ok(())
}

/// The quiet NaNs `nan` reads as, which print as `nan`; any other NaN prints as its bits.
const QUIET_F32: u32 = 0x7fc0_0000;
const QUIET_F64: u64 = 0x7ff8_0000_0000_0000;

/// `bytes` as `field` is written in a raw block; `None` when they are not its size or a raw
/// block does not write its type.
pub fn print_value(bytes: &[u8], field: &RawField) -> Option<String> {
    let scalar = field.scalar()?;
    if bytes.len() != field.size()? {
        return None;
    }
    let words: Vec<String> = bytes
        .chunks(scalar.size())
        .map(|b| print_scalar(b, scalar))
        .collect();
    Some(nest(&words, &field.dims))
}

fn nest(words: &[String], dims: &[usize]) -> String {
    match dims.split_first() {
        None => words.first().cloned().unwrap_or_default(),
        Some((_, inner)) => {
            let step = inner.iter().product::<usize>().max(1);
            let items: Vec<String> = words.chunks(step).map(|c| nest(c, inner)).collect();
            format!("[{}]", items.join(" "))
        }
    }
}

fn print_scalar(b: &[u8], scalar: Scalar) -> String {
    let mut wide = [0u8; 8];
    wide[..b.len()].copy_from_slice(b);
    let raw = u64::from_ne_bytes(wide);
    let raw = if cfg!(target_endian = "little") {
        raw
    } else {
        raw >> (64 - 8 * b.len())
    };
    match scalar {
        Scalar::U8 => (raw as u8).to_string(),
        Scalar::I8 => (raw as u8 as i8).to_string(),
        Scalar::I16 => (raw as u16 as i16).to_string(),
        Scalar::U16 => (raw as u16).to_string(),
        Scalar::I32 => (raw as u32 as i32).to_string(),
        Scalar::U32 => (raw as u32).to_string(),
        Scalar::I64 => (raw as i64).to_string(),
        Scalar::U64 => raw.to_string(),
        Scalar::F32 => {
            let bits = raw as u32;
            let x = f32::from_bits(bits);
            if x.is_nan() && bits != QUIET_F32 {
                format!("0x{bits:08x}")
            } else {
                print_bare(x)
            }
        }
        Scalar::F64 => {
            let x = f64::from_bits(raw);
            if x.is_nan() && raw != QUIET_F64 {
                format!("0x{raw:016x}")
            } else {
                print_bare(x)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(ty: &str, dims: &[usize]) -> RawField {
        RawField {
            path: "f".into(),
            ty: ty.into(),
            dims: dims.to_vec(),
            mask: None,
        }
    }

    fn value(text: &str) -> (null_syntax::Parse, String) {
        (null_syntax::parse(&format!("v {text}\n")), text.to_string())
    }

    fn round(text: &str, f: &RawField) -> Result<(Vec<u8>, String), String> {
        let (p, _) = value(text);
        let node = p.document().nodes().next().unwrap();
        let v = node.args().next().unwrap();
        let bytes = read_value(&v, f).map_err(|e| e.message)?;
        let back = print_value(&bytes, f).unwrap();
        Ok((bytes, back))
    }

    #[test]
    fn every_scalar_reads_and_prints_back_bit_for_bit() {
        for (ty, text) in [
            ("char", "255"),
            ("char", "0"),
            ("int8_t", "-128"),
            ("short", "-32768"),
            ("ushort", "65535"),
            ("int", "-2147483648"),
            ("uint", "4294967295"),
            ("int64_t", "-9223372036854775808"),
            ("uint64_t", "18446744073709551615"),
            ("float", "0.1"),
            ("float", "-0"),
            ("float", "inf"),
            ("float", "nan"),
            ("float", "0x7fa00001"),
            ("float", "0xffc00000"),
            ("double", "0.1"),
            ("double", "0x7ff0000000000001"),
        ] {
            let f = field(ty, &[]);
            let (bytes, back) = round(text, &f).unwrap_or_else(|e| panic!("{ty} {text}: {e}"));
            assert_eq!(bytes.len(), Scalar::of(ty).unwrap().size());
            assert_eq!(back, text, "{ty}");
        }
        // A NaN's payload and sign are its bits.
        let (bytes, _) = round("0x7fa00001", &field("float", &[])).unwrap();
        assert_eq!(
            f32::from_ne_bytes(bytes.try_into().unwrap()).to_bits(),
            0x7fa0_0001
        );
    }

    #[test]
    fn arrays_are_lists_of_their_exact_shape() {
        let (bytes, back) = round("[[1 2 3] [4 5 6]]", &field("short", &[2, 3])).unwrap();
        assert_eq!(bytes.len(), 12);
        assert_eq!(back, "[[1 2 3] [4 5 6]]");
        let name = "[80 105 101 114 0 120 121 0]";
        let (bytes, back) = round(name, &field("char", &[8])).unwrap();
        assert_eq!(&bytes, b"Pier\0xy\0", "every byte, after the NUL too");
        assert_eq!(back, name);
    }

    #[test]
    fn a_value_of_the_wrong_type_or_shape_is_refused() {
        for (ty, dims, text, says) in [
            ("short", &[][..], "32768", "out of its range"),
            ("ushort", &[][..], "-1", "out of its range"),
            ("char", &[][..], "-1", "out of its range"),
            ("int", &[][..], "1.5", "is not one"),
            ("float", &[][..], "0x7fa0", "8 hexadecimal digits"),
            ("double", &[][..], "0x7fa00001", "16 hexadecimal digits"),
            ("float", &[][..], "0x7FA00001", "lowercase"),
            ("float", &[][..], "0x3f800000", "write `1`"),
            ("float", &[][..], "0x7fc00000", "write `nan`"),
            ("double", &[][..], "0x3ff0000000000000", "write `1`"),
            ("short", &[3][..], "[1 2]", "this list has 2"),
            ("short", &[3][..], "1", "a list of 3 shorts"),
            ("float", &[][..], "[1]", "one float"),
            ("ListBase", &[][..], "0", "does not write"),
        ] {
            let err = round(text, &field(ty, dims)).unwrap_err();
            assert!(err.contains(says), "{ty} {text}: {err}");
        }
    }

    #[test]
    fn a_floats_bits_that_are_not_a_nans_have_their_decimal_as_the_fix() {
        let text = "v [1 0x3f800000]\n";
        let p = null_syntax::parse(text);
        let v = p.document().nodes().next().unwrap().args().next().unwrap();
        let err = read_value(&v, &field("float", &[2])).unwrap_err();
        let (span, word) = err.fix.unwrap();
        assert_eq!(word, "1");
        assert_eq!(&text[span.start..span.end], "0x3f800000");
    }

    #[test]
    fn a_masked_field_carries_its_masks_bits_alone() {
        // Sau's task 1b-03n: `Scene.flag`, which RNA reads in part, carries `SCE_FRAME_DROP`
        // (8), which `sync-mode` cannot name under `audio-sync`.
        let t = RawTable::from_json(
            r#"{"format": 1, "structs": {"Scene": [{"path": "flag", "type": "short", "mask": 8}]}}"#,
        )
        .unwrap();
        let f = t.field("Scene", "flag").unwrap().clone();
        assert_eq!(f.mask, Some(8));
        let (bytes, back) = round("8", &f).unwrap();
        assert_eq!(back, "8");
        assert_eq!(bytes, 8i16.to_ne_bytes());
        assert!(round("0", &f).is_ok());
        for wrong in ["9", "-1", "16"] {
            let err = round(wrong, &f).unwrap_err();
            assert!(err.contains("carries only the bits 8"), "{wrong}: {err}");
        }
        // Read, the mask's bits alone; set, the field's other bits kept.
        let now = 0x0103i16.to_ne_bytes();
        assert_eq!(f.carried(&(0x010bi16).to_ne_bytes()), 8i16.to_ne_bytes());
        assert_eq!(f.merged(&now, &8i16.to_ne_bytes()), 0x010bi16.to_ne_bytes());
        assert_eq!(f.merged(&0x010bi16.to_ne_bytes(), &[0, 0]), now);
        // A field written whole is unchanged by either.
        let whole = field("short", &[]);
        assert_eq!(whole.carried(&now), now);
        assert_eq!(whole.merged(&now, &[1, 0]), [1, 0]);
    }

    #[test]
    fn the_table_reads_and_finds_fields() {
        let t = RawTable::from_json(
            r#"{"format": 1, "structs": {"Object": [{"path": "transflag", "type": "short"}, {"path": "r.mode", "type": "int", "dims": [2]}]}}"#,
        )
        .unwrap();
        assert_eq!(t.field("Object", "r.mode").unwrap().size(), Some(8));
        assert!(t.field("Object", "nope").is_none());
        assert!(t.fields("Mesh").is_empty());
        let empty = RawTable::from_json(r#"{"format": 1}"#).unwrap();
        assert!(empty.structs.is_empty());
    }
}
