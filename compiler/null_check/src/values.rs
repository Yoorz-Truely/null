//! Checking one value against one property: numbers with units, lists, enums, colours.

use null_syntax::ast::Value;
use null_syntax::{Diagnostic, FileId, Span, closest};
use null_value::{Dimension, ReadError, Scene, read_in, read_int, unit_of};

use crate::codes;
use crate::schema::Property;

/// Where diagnostics go.
pub(crate) struct Out<'o> {
    pub file: FileId,
    pub list: &'o mut Vec<Diagnostic>,
    /// The scene whose block is being checked: its frame rate, for seconds written on a frame
    /// property (task 1c-01c). `None` outside a scene.
    pub scene: Option<Scene>,
    /// The unit scale a value written with a unit is divided by (task 1c-01d).
    pub scale: f32,
    /// Outside every scene, where the scenes' scales disagree: the message for the first value
    /// written with a scaled unit, taken when it is reported.
    pub disagreement: &'o mut Option<String>,
}

impl Out<'_> {
    pub fn note(&mut self, code: &'static str, span: Span, message: impl Into<String>) {
        let d = Diagnostic::new(null_syntax::Severity::Note, code, self.file, span, message);
        self.list.push(d);
    }

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
    // RNA's unit names, kebab-case (spec, section 6, "Units"). A float frame (`time`) also
    // takes seconds, through its scene's frame rate (task 1c-01c); an integer frame is read by
    // `check_int`, always bare.
    match p.unit.as_deref() {
        Some("length") => Dimension::Length,
        Some("area") => Dimension::Area,
        Some("volume") => Dimension::Volume,
        Some("mass") => Dimension::Mass,
        Some("rotation") => Dimension::Angle,
        Some("time") => Dimension::Frames,
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

/// The dimension a property's RNA unit reads in (spec, section 6, "Units"), or `None` where
/// it has no unit or one Null does not read.
pub fn unit_dimension(p: &Property) -> Option<Dimension> {
    let d = dimension(p, false);
    (d != Dimension::None).then_some(d)
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
        Dimension::Frames => "a time in frames, such as 36 or 1.5s",
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
        Dimension::Frames => "times in frames",
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
        Dimension::Frames => "0",
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
    // Seconds on a frame property become frames through the scene's frame rate (spec,
    // section 6, "Frames"); outside a scene there is none to take.
    if dimension == Dimension::Frames && out.scene.is_none() && unit_of(word, dimension).is_some() {
        let at = Span::new(
            value.span().start + word.trim_end_matches(char::is_alphabetic).len(),
            value.span().end,
        );
        out.error(
            codes::UNIT,
            at,
            format!("`{name}` is in frames here: seconds need a scene's frame rate, and this is not inside a scene"),
        );
        return;
    }
    // Outside every scene, a scaled unit needs the one scale the scenes agree on.
    if out.scene.is_none()
        && dimension.scale_power() > 0
        && unit_of(word, dimension).is_some()
        && let Some(message) = out.disagreement.take()
    {
        out.error(codes::UNIT, value.span(), message);
    }
    let scene = Scene {
        scale_length: out.scale,
        ..out.scene.unwrap_or_default()
    };
    match read_in::<f32>(word, dimension, &scene) {
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
        // Past 32 bits Blender cannot store it at all: an error, not the range's note (the
        // lead's call on the 1c-02b review).
        Ok(n) if i32::try_from(n).is_err() => {
            out.error(
                codes::RANGE,
                value.span(),
                format!("`{word}` does not fit `{name}`, a 32-bit whole number"),
            );
            None
        }
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
            // A note, not an error (the lead's call on task 1c-02b): a `.blend` can hold a value
            // outside RNA's range (an infinity in DNA), and Null writes what Blender stores. Blender
            // clamps it when the value is set (spec, section 6, "Types").
            out.note(
                codes::RANGE,
                span,
                format!(
                    "`{word}` is outside `{name}`'s range, {} to {}; Blender clamps it when it is set",
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
        // The checker passes only a positive unit scale and frame rate (`Units::scale`,
        // `frame_rate`), so this is for completeness.
        ReadError::BadScene => out.error(
            codes::UNIT,
            span,
            format!("`{name}` cannot be read in this scene's unit scale or frame rate"),
        ),
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

/// Checks a fixed two-dimensional array of floats (a matrix, a bounding box's corners), written
/// row by row as a list of `rows` lists of `columns` (spec, section 6).
pub(crate) fn check_float_matrix(
    value: &Value<'_>,
    rows: usize,
    columns: usize,
    name: &str,
    dimension: Dimension,
    p: &Property,
    out: &mut Out<'_>,
) {
    let shape = format!(
        "`{name}` takes {rows} rows of {columns} {}, as a list of lists",
        plural(dimension)
    );
    let Value::List(_) = value else {
        out.error(codes::TYPE, value.span(), shape);
        return;
    };
    let items = value.items();
    if items.len() != rows {
        out.error(
            codes::TYPE,
            value.span(),
            format!("{shape}; this list has {}", items.len()),
        );
        return;
    }
    if let Some(row) = items.iter().find(|r| !matches!(r, Value::List(_))) {
        out.error(
            codes::TYPE,
            row.span(),
            format!("{shape}; `{}` is not a row", row.text()),
        );
        return;
    }
    // A 4×4 transform (subtype `matrix`) holds lengths in its translation column, the first
    // three rows' last items, read as `location` is, through the scene's unit scale; its other
    // items are bare (spec, section 6, "Units in arrays and matrices"; the lead's call on the
    // 1b-03e review).
    let transform = p.subtype.as_deref() == Some("matrix") && rows == 4 && columns == 4;
    for (i, row) in items.iter().enumerate() {
        let cells = row.items();
        if cells.len() != columns {
            out.error(
                codes::TYPE,
                row.span(),
                format!("{shape}; this row has {}", cells.len()),
            );
            continue;
        }
        for (j, cell) in cells.iter().enumerate() {
            let dimension = match (transform, i < 3 && j == 3) {
                (true, true) => Dimension::Length,
                (true, false) => Dimension::None,
                (false, _) => dimension,
            };
            check_float(cell, name, dimension, p, out);
        }
    }
}

/// Checks a flag set: a list of the enum's items, each once, in the schema's item order, `[]`
/// for none (spec, section 6, "Types"; task 1c-02b). Another order is an error whose fix is the
/// schema's.
fn check_flags(value: &Value<'_>, name: &str, p: &Property, out: &mut Out<'_>) {
    let items = p.item_names();
    let Value::List(_) = value else {
        out.error(
            codes::TYPE,
            value.span(),
            format!("`{name}` is a set of its items: write a list, `[]` for none"),
        );
        return;
    };
    let mut written = Vec::new();
    for item in value.items() {
        let Some(word) = item.as_word() else {
            out.error(
                codes::TYPE,
                item.span(),
                format!("`{name}`'s items are words"),
            );
            return;
        };
        if !items.contains(&word) && !p.items_at_run_time {
            let fix = closest(word, items.iter().copied());
            out.error_fix(
                codes::ENUM,
                item.span(),
                format!(
                    "`{word}` is not a value of `{name}`: it takes {}",
                    list(&items)
                ),
                fix.map(|f| (format!("write `{f}`"), item.span(), f.to_string())),
            );
            return;
        }
        if written.contains(&word) {
            out.error(
                codes::TYPE,
                item.span(),
                format!("`{word}` is given twice in `{name}`"),
            );
            return;
        }
        written.push(word);
    }
    let ordered: Vec<&str> = items
        .iter()
        .copied()
        .filter(|i| written.contains(i))
        .collect();
    if !p.items_at_run_time && ordered != written {
        let fixed = format!("[{}]", ordered.join(" "));
        out.error_fix(
            codes::TYPE,
            value.span(),
            format!("`{name}`'s items are written in the schema's order: {fixed}"),
            Some((format!("write `{fixed}`"), value.span(), fixed.clone())),
        );
    }
}

/// `#rrggbb` as scene-linear floats: the IEC 61966-2-1 sRGB curve, exactly rounded once into
/// float32 from the fixed table in `null_value` (spec, section 6; Sau's decision S62).
pub fn srgb(word: &str) -> Option<[f32; 3]> {
    null_value::read_srgb(word)
}

/// A string Blender can hold (spec, section 6): written as a string, or, where it is not UTF-8,
/// as the list of its bytes (S65, task 1c-02g). No 0 byte, since Blender's strings end at one, and
/// within its field with the closing NUL where the field is fixed (`max-length`, task 1b-03h),
/// counted in bytes either way. Too long is an error, never cut.
pub(crate) fn check_string(value: &Value<'_>, name: &str, p: &Property, out: &mut Out<'_>) {
    let span = value.span();
    let bytes: Vec<u8> = match value {
        Value::List(_) => {
            let mut bytes = Vec::new();
            for item in value.items() {
                match item
                    .as_word()
                    .and_then(|w| read_int(w).ok())
                    .and_then(|n| u8::try_from(n).ok())
                {
                    Some(b) => bytes.push(b),
                    None => {
                        return out.error(
                            codes::TYPE,
                            item.span(),
                            format!(
                                "`{name}`'s bytes are whole numbers 1 to 255, not `{}`",
                                item.text()
                            ),
                        );
                    }
                }
            }
            if let Ok(text) = std::str::from_utf8(&bytes) {
                // One spelling (S65): a string that is UTF-8 is always written as a string.
                let fix = format!("{text:?}");
                return out.error_fix(
                    codes::TYPE,
                    span,
                    format!(
                        "`{name}`'s bytes are valid UTF-8, so it is written as a string: {fix}"
                    ),
                    (!bytes.contains(&0)).then(|| (format!("write {fix}"), span, fix.clone())),
                );
            }
            bytes
        }
        _ => match value.as_string() {
            Some(text) => text.into_owned().into_bytes(),
            None => {
                return out.error(
                    codes::TYPE,
                    span,
                    format!(
                        "`{name}` takes a string, such as \"Name\", or the list of its bytes, not `{}`",
                        value.text()
                    ),
                );
            }
        },
    };
    if bytes.contains(&0) {
        out.error(
            codes::TYPE,
            span,
            format!("`{name}` cannot hold a 0 byte: Blender's strings end at one"),
        );
    }
    if let Some(max) = p.max_length
        && bytes.len() + 1 > max
    {
        out.error(
            codes::RANGE,
            span,
            format!(
                "`{name}` holds at most {} bytes, not {} (Blender's field is {max} with its closing NUL)",
                max.saturating_sub(1),
                bytes.len()
            ),
        );
    }
}

/// Checks an enum `value` against `p`'s items, with the nearest item as a fix.
pub(crate) fn check_enum(value: &Value<'_>, name: &str, p: &Property, out: &mut Out<'_>) {
    if p.flags {
        return check_flags(value, name, p, out);
    }
    let items = p.item_names();
    if let Some(text) = value.as_string() {
        // A free-text item (a colour space's name) is written as a string (S16).
        let quoted = p.items.iter().flatten().any(|i| i.quoted && i.name == text);
        if quoted || p.items_at_run_time {
            return;
        }
    }
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
    // Items computed when the enum is used (an image's layers, the colour spaces) are not all in
    // the schema, so a value it does not list is refused only when it is near one it does, as a
    // misspelling (task 1b-03e).
    if p.items_at_run_time
        && closest(word, items.iter().copied()).is_none()
        && !items.contains(&word.to_ascii_lowercase().replace('_', "-").as_str())
    {
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
