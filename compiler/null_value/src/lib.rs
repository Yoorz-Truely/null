//! Null's exact numbers and units: every stored float printed and read back to the same bits.
//!
//! Implements section 6 of the specification (`docs/spec/null-v0.md`) for the
//! slice, after Sau's decisions S21 and S22:
//!
//! - A number is read exactly as written and rounded once, to nearest even, into
//!   the property's stored type, `f32` or `f64` ([`read`]).
//! - A value with a unit is converted exactly and then rounded once. Every word of the
//!   spec's units table (section 6) is a decimal shift of the internal unit, an exact
//!   ratio (inches, pounds, miles per hour), an offset and ratio (°C, °F), or π/180
//!   (degrees). Shifts reach the standard library's correctly rounded reading as one
//!   exact decimal; ratios and offsets are rounded from exact integers; degrees are
//!   multiplied by π/180 with π bracketed as tightly as the rounding needs (module
//!   `exact`). A bare number is the internal unit: metres, radians, kilograms, kelvin.
//! - The printer writes the shortest decimal that reads back to exactly the
//!   stored bits ([`print_bare`]), in the display unit whenever such a decimal
//!   exists in it ([`print_in`], [`print`]), so `0.1`, `90deg`, `2.7m` and `5mm`
//!   read back as written.
//!
//! Lengths are read at unit scale 1; the scene's unit scale (S21) is applied
//! where the checker knows it. NaN reads and prints as `nan`, the quiet NaN:
//! its sign and payload are not written.
//!
//! # Use
//!
//! ```
//! use null_value::{Dimension, Unit, print, read};
//!
//! let angle: f32 = read("90deg", Dimension::Angle).unwrap();
//! assert_eq!(angle, std::f32::consts::FRAC_PI_2);
//! assert_eq!(print(angle, Some(Unit::Deg)), "90deg");
//!
//! let width: f32 = read("5mm", Dimension::Length).unwrap();
//! assert_eq!(print(width, Some(Unit::Mm)), "5mm");
//! assert_eq!(print(width, None), "0.005");
//!
//! assert!(read::<f32>("5deg", Dimension::Length).is_err());
//! ```

mod exact;
mod number;
mod unit;

pub use number::Float;
pub use unit::{Dimension, ReadError, Unit, print, print_bare, print_in, read, read_int, unit_of};

#[cfg(test)]
mod tests;
