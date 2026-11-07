# agent/cos-null

This is the report of agent/cosmosis-a working as `agent/cos-null` on two Null rows, 1c-01f and 1d-01b, since Null's main checkout is the Null worker's.

- **Where:** `work/wt/cos-null/`, Null on branch `agent/cos-null`, with Sau, Cosmósis and Scowr linked beside it.
- **Started:** November 7, 2026.

## 1c-01f: the sRGB rule by a 256-entry table (`7e10f3f`)

**The question.** The row asked for "Blender's own result, bit for bit"; the spec, section 6, and the 1a-06 review asked for the exactly rounded IEC 61966-2-1 curve. They differ.

I measured Blender's own conversion in Sau, through `just sandbox`: `mathutils.Color.from_srgb_to_scene_linear`, the OCIO conversion the colour picker's hex field uses. Over the 256 channel values, the three channels agree, 0 and 255 are exact, and **222 of the 256 differ** from the exactly rounded curve, by −3 to +8 units in the last place. For example, 128 gives 0.21586056 in Blender and 0.21586050 exactly.

The lead chose the exactly rounded curve (Sau's decision S62): a file's `#808080` must mean the same thing years from now, and Blender's OCIO configuration can change between releases.

**What works.**

- **The table.** `null_value::srgb::TABLE` holds the 256 float32s nearest to the curve. They were generated with exact rational arithmetic and stored as bits.
- **Reading and printing.**
  - `read_srgb` reads `#rrggbb` through the table. The checker's `srgb` now calls it, so the `f64::powf` is gone.
  - `print_srgb` gives `#rrggbb` only if every channel is exactly a table entry, and alpha, when there is one, is exactly 1; otherwise `None`, and the colour is a list.
  - The printer and write-back do not print hex colours yet, so nothing calls `print_srgb` today.
- **The proof**, `every_entry_is_the_exactly_rounded_curve`, uses integers only, with no floating-point `pow` anywhere.
  - Every float32 is taken as `n / 2^150` and every midpoint between neighbours as `n / 2^151`.
  - On the power part, with `r = (40 i + 561) / 10761`, an entry is the nearest float32 when `lo^5 < r^12 < hi^5` for the midpoints below and above it, compared with `num-bigint`.
  - On the linear part (`i ≤ 10`), the comparison is between rationals.
  - The test also checks that black is 0, white is 1, and the table rises.
  - With one entry (128) changed by one unit in the last place, it fails: "channel 128: the curve is below the entry's lower midpoint".
- **Blender's picker value.** `a_picker_value_is_not_hex`: Blender's 0.21586056 for `#808080` is not a table entry, so it has no `#rrggbb` and prints as `0.21586056`.
- **Hex reading and printing.** `hex_reads_through_the_table_and_prints_back`: `#000000`, `#ffffff` and `#808080` read to their entries and print back, with alpha 1 and not 0.5; malformed words are refused.
- **The documents.**
  - The spec's section 6 keeps its words and adds "Sau's decision S62: exactly rounded, from a fixed table of the 256 channel values".
  - `CHANGELOG.md` has the line.
  - RFC 0001 records the change.
- **Unchanged on the Mac.** The table equals what `powf` gave here, entry for entry, as the 1a-06 review found, so no file on this Mac reads differently.

`just check` passes.

## 1d-01b: the RFC process for changes after v0 (`7e10f3f`)

**What works.**

- **`docs/rfcs/README.md`**, modelled on Scowr's (`../Scowr/docs/rfcs/`, read, not changed):
  - when a proposal is needed: new forms, a change of meaning, the canonical form, write-back or diagnostics. It is not needed for wording, implementation, or new schema properties from a Blender release;
  - how to make one: copy the template, number it, add a board row, and have it decided by the user or the lead under S59, recorded in Sau's `DECISIONS.md`;
  - what follows a decision: if accepted, it goes into the spec, the changelog and code with tests, and a broken file means a raised `null-format` and a Cosmósis migration (S45); if rejected, it stays here with the reason;
  - a table of the proposals.
- **`docs/rfcs/0000-template.md`**: status, date, who decided and the S-number, what it affects, the Null format, then problem, design, what breaks, migration, alternatives and decision.
- **The first RFC.** `docs/rfcs/0001-srgb-table.md` takes 1c-01f's change through the process: the problem, with the platform `pow` and Blender's OCIO bits measured; the design and its proof; what breaks, nothing on the Mac; migration, none; the rejected alternatives; and the decision, accepted as S62.

## Next

`just sync`, `just verify`, then `just task review` for 1c-01f and 1d-01b.
