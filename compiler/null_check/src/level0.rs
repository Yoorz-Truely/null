//! Level 0 geometry's element types (spec, section 9, "Level 0: attribute arrays"; task 1d-06b):
//! how one value of each of `Attribute`'s data types is written, as a property the checker and
//! `null_print` already read and print, so every kind reads and prints back bit for bit.

use serde_json::json;

use crate::schema::{Property, Range};

/// The domains an `attribute` line takes, in Blender's order.
pub const DOMAINS: [&str; 4] = ["point", "edge", "face", "corner"];

/// The data types an `attribute` line takes, as `Attribute.data-type`'s items.
pub const DATA_TYPES: [&str; 14] = [
    "float",
    "int",
    "boolean",
    "float-vector",
    "float2",
    "float4",
    "float-color",
    "byte-color",
    "quaternion",
    "float4x4",
    "int8",
    "int16-2d",
    "int32-2d",
    "string",
];

/// One value of an attribute of `data_type`, as a property: its type, its array shape and its
/// range (a `byte-color`'s channels 0 to 255, an `int8`'s -128 to 127).
pub fn element(data_type: &str) -> Option<Property> {
    let ranged = |ty: &str, lo: f64, hi: f64| Property {
        ty: ty.into(),
        range: Some(Range { hard: [lo, hi] }),
        ..Property::any()
    };
    let i32_range = (f64::from(i32::MIN), f64::from(i32::MAX));
    let mut p = match data_type {
        "float" | "float-vector" | "float2" | "float4" | "float-color" | "quaternion"
        | "float4x4" => Property::any(),
        "int" | "int32-2d" => ranged("int", i32_range.0, i32_range.1),
        "int8" => ranged("int", -128.0, 127.0),
        "int16-2d" => ranged("int", f64::from(i16::MIN), f64::from(i16::MAX)),
        "byte-color" => ranged("int", 0.0, 255.0),
        "boolean" => Property {
            ty: "bool".into(),
            ..Property::any()
        },
        "string" => Property {
            ty: "string".into(),
            ..Property::any()
        },
        _ => return None,
    };
    p.name = data_type.to_string();
    p.array = match data_type {
        "float-vector" => Some(json!([3])),
        "float2" | "int16-2d" | "int32-2d" => Some(json!([2])),
        "float4" | "float-color" | "quaternion" | "byte-color" => Some(json!([4])),
        "float4x4" => Some(json!([4, 4])),
        _ => None,
    };
    if data_type == "float-color" {
        p.subtype = Some("color".into());
    }
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_data_type_has_an_element() {
        for t in DATA_TYPES {
            let p = element(t).unwrap_or_else(|| panic!("{t}"));
            assert_eq!(p.name, t);
        }
        assert_eq!(element("float4x4").unwrap().dimensions(), [4, 4]);
        assert_eq!(
            element("byte-color").unwrap().range.unwrap().hard,
            [0.0, 255.0]
        );
        assert!(element("float3").is_none());
    }
}
