//! Null's forms over RNA: where the schema alone does not say how a thing is written.
//!
//! Spec, section 8 ("Null's forms over RNA"), lists these with their reasons; this
//! module holds the tables the checker reads. Everything else is RNA's own, by its
//! own names (S16).

/// The project file's kind, Null's own: the project's name, units and scenes.
pub const PROJECT: &str = "project";

/// Names the checker answers with another, because they are not Null's but are
/// often written: (written, Null's name, why).
pub const ALIASES: [(&str, &str, &str); 3] = [
    (
        "faces",
        "polygons",
        "RNA calls a mesh's faces `polygons` (S16: RNA's own names)",
    ),
    (
        "active-material",
        "material",
        "an object's materials are written as `material @handle` lines, one per slot",
    ),
    (
        "unit-settings",
        "units",
        "a scene's unit settings are written with `units`",
    ),
];

/// The datablock kinds that can be an object's `data`.
pub const OBJECT_DATA: [&str; 7] = [
    "mesh",
    "camera",
    "light",
    "armature",
    "point-cloud",
    "curves",
    "grease-pencil",
];

/// The light structs whose `energy` is a power in watts. RNA gives `energy` no unit;
/// Blender shows it in watts for these. A sun's is an irradiance, written bare.
pub const POWER_LIGHTS: [&str; 3] = ["PointLight", "SpotLight", "AreaLight"];

/// `units`' short settings: (Null's short name, the `UnitSettings` property).
pub const UNITS_SHORT: [(&str, &str); 3] = [
    ("scale", "scale-length"),
    ("length", "length-unit"),
    ("rotation", "system-rotation"),
];

/// Length units as `units length=…` takes them: (unit, `length-unit` item).
pub const LENGTH_UNITS: [(&str, &str); 6] = [
    ("m", "meters"),
    ("cm", "centimeters"),
    ("mm", "millimeters"),
    ("km", "kilometers"),
    ("in", "inches"),
    ("ft", "feet"),
];

/// Rotation units as `units rotation=…` takes them: (unit, `system-rotation` item).
pub const ROTATION_UNITS: [(&str, &str); 2] = [("deg", "degrees"), ("rad", "radians")];

/// A scene's `fps`, Null's form on the scene for its render settings' `fps`: RNA's range.
pub const FPS_RANGE: (f64, f64) = (1.0, 32767.0);

/// A scene's `fps-base`, for its render settings' `fps-base`: RNA's range.
pub const FPS_BASE_RANGE: (f64, f64) = (1e-5, 1e6);

/// Properties a flat form of Null's sets, which a block of their struct must not set again
/// (S70): (struct, property, the flat form's name). A scene's `fps` and `fps-base` are its
/// render settings'.
pub const FLAT_FORMS: [(&str, &str, &str); 2] = [
    ("RenderSettings", "fps", "fps"),
    ("RenderSettings", "fps-base", "fps-base"),
];
