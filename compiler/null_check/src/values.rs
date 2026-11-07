//! Checking one value against one property: numbers with units, lists, enums, colours.

use null_syntax::ast::Value;
use null_syntax::{Diagnostic, FileId, Span, closest};
use null_value::{Dimension, ReadError, read, read_int};

use crate::codes;
use crate::schema::Property;

/// Where diagnostics go.
pub(crate) struct Out<'o> {
    pub file: FileId,
    pub list: &'o mut Vec<Diagnostic>,
}

impl Out<'_> {
    pub fn error(&mut self, code: &'static str, span: Span, message: impl Into<String>) {
        let d = Diagnostic::error(code, self.file, span, message);
        self.list.push(d);
    }

    pub fn error_fix(
        &mut self,
        code: &'static str,
        span: Span,
        message: impl Into<String>,
        fix: Option<(String, Span, String)>,
    ) {
        let mut d = Diagnostic::error(code, self.file, span, message);
        if let Some((what, at, with)) = fix {
            d = d.with_fix_replace(what, at, with);
        }
        self.list.push(d);
    }
}

/// What a property measures, from its schema unit; `power` for a light's energy.
pub(crate) fn dimension(p: &Property, power: bool) -> Dimension {
    if power {
        return Dimension::Power;
    }
    // RNA's unit names, kebab-case (spec, section 6, "Units"). Frames (`time`) are bare
    // numbers until their seconds are read through the scene's frame rate (task 1c-01c).
    match p.unit.as_deref() {
        Some("length") => Dimension::Length,
        Some("area") => Dimension::Area,
        Some("volume") => Dimension::Volume,
        Some("mass") => Dimension::Mass,
        Some("rotation") => Dimension::Angle,
        Some("time-absolute") => Dimension::Seconds,
        Some("velocity") => Dimension::Velocity,
        Some("acceleration") => Dimension::Acceleration,
        Some("camera") => Dimension::Lens,
        Some("power") => Dimension::Power,
        Some("temperature") => Dimension::Temperature,
        Some("wavelength") => Dimension::Wavelength,
        Some("color-temperature") => Dimension::ColorTemperature,
        Some("frequency") => Dimension::Frequency,
        _ => Dimension::None,
    }
}

/// A value of `dimension`, in words, with an example.
fn describe(dimension: Dimension) -> &'static str {
    match dimension {
        Dimension::None => "a number, such as 0.5",
        Dimension::Length => "a length, such as 1m",
        Dimension::Area => "an area, such as 1m2",
        Dimension::Volume => "a volume, such as 1m3",
        Dimension::Mass => "a mass, such as 1kg",
        Dimension::Angle => "an angle, such as 90deg",
        Dimension::Seconds => "a time, such as 1.5s",
        Dimension::Velocity => "a velocity, such as 1m/s",
        Dimension::Acceleration => "an acceleration, such as 9.81m/s2",
        Dimension::Lens => "a focal length, such as 50mm",
        Dimension::Power => "a power, such as 800W",
        Dimension::Temperature => "a temperature, such as 293.15K",
        Dimension::Wavelength => "a wavelength, such as 550nm",
        Dimension::ColorTemperature => "a colour temperature, such as 6500K",
        Dimension::Frequency => "a frequency, such as 50Hz",
    }
}

fn plural(dimension: Dimension) -> &'static str {
    match dimension {
        Dimension::None => "numbers",
        Dimension::Length => "lengths",
        Dimension::Area => "areas",
        Dimension::Volume => "volumes",
        Dimension::Mass => "masses",
        Dimension::Angle => "angles",
        Dimension::Seconds => "times",
        Dimension::Velocity => "velocities",
        Dimension::Acceleration => "accelerations",
        Dimension::Lens => "focal lengths",
        Dimension::Power => "powers",
        Dimension::Temperature => "temperatures",
        Dimension::Wavelength => "wavelengths",
        Dimension::ColorTemperature => "colour temperatures",
        Dimension::Frequency => "frequencies",
    }
}

fn zero(dimension: Dimension) -> &'static str {
    match dimension {
        Dimension::None => "0",
        Dimension::Length => "0m",
        Dimension::Area => "0m2",
        Dimension::Volume => "0m3",
        Dimension::Mass => "0kg",
        Dimension::Angle => "0deg",
        Dimension::Seconds => "0s",
        Dimension::Velocity => "0m/s",
        Dimension::Acceleration => "0m/s2",
        Dimension::Lens => "50mm",
        Dimension::Power => "0W",
        Dimension::Temperature => "0K",
        Dimension::Wavelength => "550nm",
        Dimension::ColorTemperature => "6500K",
        Dimension::Frequency => "0Hz",
    }
}

/// Checks a float32 `value` of `dimension` for property `name`, and its hard range.
pub(crate) fn check_float(
    value: &Value<'_>,
    name: &str,
    dimension: Dimension,
    p: &Property,
    out: &mut Out<'_>,
) {
    let Some(word) = value.as_word() else {
        out.error(
            codes::TYPE,
            value.span(),
            format!(
                "`{name}` takes {}, not `{}`",
                describe(dimension),
                value.text()
            ),
        );
        return;
    };
    match read::<f32>(word, dimension) {
        Ok(x) => check_range(x as f64, word, value.span(), name, p, out),
        Err(e) => unit_error(e, word, value.span(), name, dimension, out),
    }
}

/// Checks an integer `value` for property `name`, and its hard range.
pub(crate) fn check_int(
    value: &Value<'_>,
    name: &str,
    p: &Property,
    out: &mut Out<'_>,
) -> Option<i64> {
    let Some(word) = value.as_word() else {
        out.error(
            codes::TYPE,
            value.span(),
            format!("`{name}` takes a whole number, not `{}`", value.text()),
        );
        return None;
    };
    match read_int(word) {
        Ok(n) => {
            check_range(n as f64, word, value.span(), name, p, out);
            Some(n)
        }
        Err(ReadError::NotANumber | ReadError::NotAnInteger) => {
            out.error(
                codes::TYPE,
                value.span(),
                format!("`{name}` takes a whole number, such as 24, not `{word}`"),
            );
            None
        }
        Err(e) => {
            unit_error(e, word, value.span(), name, Dimension::None, out);
            None
        }
    }
}

fn check_range(x: f64, word: &str, span: Span, name: &str, p: &Property, out: &mut Out<'_>) {
    if let Some(r) = &p.range {
        let [lo, hi] = r.hard;
        if x < lo || x > hi || x.is_nan() {
            let show = |v: f64| match p.ty.as_str() {
                "int" => format!("{v}"),
                _ => null_value::print_bare(v as f32),
            };
            out.error(
                codes::RANGE,
                span,
                format!(
                    "`{word}` is outside `{name}`'s range, {} to {}",
                    show(lo),
                    show(hi)
                ),
            );
        }
    }
}

fn unit_error(
    e: ReadError,
    word: &str,
    span: Span,
    name: &str,
    dimension: Dimension,
    out: &mut Out<'_>,
) {
    match e {
        ReadError::NotANumber | ReadError::NotAnInteger => out.error(
            codes::TYPE,
            span,
            format!("`{name}` takes {}, not `{word}`", describe(dimension)),
        ),
        ReadError::UnknownUnit { unit, suggestion } => {
            let at = Span::new(span.end - unit.len(), span.end);
            let fix = suggestion.map(|u| (format!("write `{u}`"), at, u.suffix().to_string()));
            out.error_fix(
                codes::UNIT,
                at,
                format!(
                    "`{unit}` is not a unit `{name}` takes; it takes {}",
                    units_text(dimension)
                ),
                fix,
            );
        }
        ReadError::WrongUnit { unit, .. } => {
            let at = Span::new(span.end - unit.suffix().len(), span.end);
            out.error(
                codes::UNIT,
                at,
                format!(
                    "`{name}` cannot take `{unit}`: it takes {}",
                    units_text(dimension)
                ),
            );
        }
    }
}

fn units_text(dimension: Dimension) -> String {
    let units: Vec<_> = dimension.units().iter().map(|u| u.suffix()).collect();
    if units.is_empty() {
        "a bare number, with no unit".into()
    } else {
        format!("{} or a bare number", units.join(", "))
    }
}

/// Checks a list of `len` floats of `dimension` (a vector, a colour). A colour may also
/// be `#rrggbb`, sRGB (spec, section 6).
pub(crate) fn check_float_array(
    value: &Value<'_>,
    len: usize,
    name: &str,
    dimension: Dimension,
    p: &Property,
    out: &mut Out<'_>,
) {
    let colour = p.subtype.as_deref() == Some("color") && (len == 3 || len == 4);
    if let Some(word) = value.as_word()
        && colour
        && word.starts_with('#')
    {
        if srgb(word).is_none() {
            out.error(
                codes::TYPE,
                value.span(),
                format!("`{word}` is not an sRGB colour: write `#rrggbb`, six hexadecimal digits"),
            );
        }
        return;
    }
    let example: Vec<_> = (0..len).map(|_| zero(dimension)).collect();
    let shape = format!(
        "`{name}` takes {len} {}, as a list such as [{}]{}",
        plural(dimension),
        example.join(" "),
        if colour { ", or an sRGB `#rrggbb`" } else { "" }
    );
    let Value::List(_) = value else {
        out.error(codes::TYPE, value.span(), shape);
        return;
    };
    let items = value.items();
    if items.len() != len {
        out.error(
            codes::TYPE,
            value.span(),
            format!("{shape}; this list has {}", items.len()),
        );
        return;
    }
    for item in &items {
        check_float(item, name, dimension, p, out);
    }
}

/// `#rrggbb` as scene-linear floats: the IEC 61966-2-1 sRGB curve, exactly rounded once into
/// float32 from the fixed table in `null_value` (spec, section 6; Sau's decision S62).
pub fn srgb(word: &str) -> Option<[f32; 3]> {
    null_value::read_srgb(word)
}

/// Checks an enum `value` against `p`'s items, with the nearest item as a fix.
pub(crate) fn check_enum(value: &Value<'_>, name: &str, p: &Property, out: &mut Out<'_>) {
    let items = p.item_names();
    let Some(word) = value.as_word() else {
        out.error(
            codes::TYPE,
            value.span(),
            format!(
                "`{name}` takes one of {}, not `{}`",
                list(&items),
                value.text()
            ),
        );
        return;
    };
    if items.contains(&word) {
        return;
    }
    let lowered = word.to_ascii_lowercase().replace('_', "-");
    let suggestion = items
        .iter()
        .copied()
        .find(|i| *i == lowered)
        .or_else(|| closest(word, items.iter().copied()));
    let message = match suggestion {
        Some(s) if s == lowered => format!(
            "`{word}` is not kebab-case: `{name}`'s values are written in lower case, `{s}`"
        ),
        _ => format!(
            "`{word}` is not a value of `{name}`: it takes {}",
            list(&items)
        ),
    };
    let code = if suggestion == Some(lowered.as_str()) {
        codes::NAME
    } else {
        codes::ENUM
    };
    out.error_fix(
        code,
        value.span(),
        message,
        suggestion.map(|s| (format!("write `{s}`"), value.span(), s.to_string())),
    );
}

/// Items for a message: all of them, or the first twelve and a count.
pub(crate) fn list(items: &[&str]) -> String {
    let shown: Vec<_> = items.iter().take(12).map(|i| format!("`{i}`")).collect();
    let more = items.len().saturating_sub(12);
    if more > 0 {
        format!("{} and {more} more", shown.join(", "))
    } else {
        shown.join(", ")
    }
}

/// Checks a boolean: written bare for on, or `on` or `off`.
pub(crate) fn check_bool(args: &[Value<'_>], name: &str, span: Span, out: &mut Out<'_>) {
    match args {
        [] => {}
        [v] if matches!(v.as_word(), Some("on" | "off")) => {}
        [v] => {
            let fix = match v.as_word() {
                Some("true" | "yes" | "1") => Some("on"),
                Some("false" | "no" | "0") => Some("off"),
                _ => None,
            };
            out.error_fix(
                codes::TYPE,
                v.span(),
                format!(
                    "`{name}` is on or off: write it bare for on, or `on` or `off`, not `{}`",
                    v.text()
                ),
                fix.map(|f| (format!("write `{f}`"), v.span(), f.to_string())),
            );
        }
        _ => out.error(
            codes::TYPE,
            span,
            format!("`{name}` takes one value, on or off"),
        ),
    }
}
