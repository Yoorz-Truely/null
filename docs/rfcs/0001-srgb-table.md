# 0001: The sRGB rule by a table of exactly rounded values

- Status: accepted
- Date: November 7, 2026
- Decided by: the lead under the user's standing instruction (Sau's decision S59), recorded as Sau's decision S62
- Affects: values
- Null format: unchanged

## Problem

Section 6 says a colour written `#rrggbb` is "the IEC 61966-2-1 sRGB curve computed in float64 and rounded once into each channel's type". Null computed that with `f64::powf`.

The 1a-06 review found that this gives the exactly rounded value for all 256 channel values on the Mac, but `pow` is not correctly rounded and differs between platforms and libraries (S12). The same file could then read to other bits elsewhere: `#808080` is `0x3e5d0a89` on this Mac, and need not be on another.

There is a second reading. "Blender's own" conversion, the one its colour picker's hex field uses (its OCIO colour management, `mathutils.Color.from_srgb_to_scene_linear` in Python), differs from the exactly rounded curve for 222 of the 256 values, by −3 to +8 units in the last place. For example, `#808080` is 0.21586056 in Blender and 0.21586050 exactly. These were measured in Sau, task 1c-01f.

## Design

`#rrggbb` reads each channel through a fixed table of 256 float32 values, `null_value::srgb::TABLE`. Entry `i` is the float32 nearest to the curve at `i / 255`:

- `c / 12.92` for `c ≤ 0.04045`;
- otherwise `((c + 0.055) / 1.055)^2.4`.

No floating-point `pow` is involved.

A test proves every entry with integers alone. With `r = (40 i + 561) / 10761`, the curve is `r^(12/5)`. An entry is the nearest float32 when the curve lies strictly between the midpoints to its neighbours. For a midpoint `m`, `m^5 < r^12` is decided exactly with `num-bigint`; on the linear part the comparison is of rationals.

A colour prints back as `#rrggbb` only if every channel is exactly a table entry (and alpha, if any, exactly 1); otherwise it prints as its list of floats (`null_value::print_srgb`). The spec's words in section 6 stay, citing S62.

Before and after, a file reads the same on this Mac:

```null
material @clay { base-color #808080 }   // [0.2158605 0.2158605 0.2158605 1] in float32, on every platform
```

## What breaks

Nothing on the Mac: the table equals what `powf` gave there, entry for entry. On a platform whose `pow` differed, a file's colours now read to the exact values, which is what the spec always said. No file of Cosmósis's corpus or Null's slice project changes.

A colour set in Blender's picker does not read back from any `#rrggbb`. So Sau's write-back prints it as a list of floats, and the file says exactly what Blender stores.

## Migration

None: no file changes meaning.

## Alternatives

- **Blender's own (OCIO) bits, measured through Sau.** A hex colour typed into Blender would then print back as hex. But it would make a file's meaning depend on Blender's colour-management configuration and version, which can change between releases, so a colour written years ago could shift. Rejected: `#808080` must mean the same thing years from now (S62).
- **A correctly rounded `pow` from a library.** It would avoid the table but add a dependency and still need the proof. The table is 256 numbers, proven once.

## Decision

Accepted as S62, November 7, 2026, for the reason above: a file's `#808080` means the same on every platform and in every Blender release.
