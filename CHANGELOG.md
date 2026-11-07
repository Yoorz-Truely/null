# Changelog

## Unreleased

- November 6, 2026: the repository is created. The draft v0 specification moves here from Sau's `docs/NULL.md`. The name Null and the `.noll` extension were chosen by the user.
- November 7, 2026: section 6 says a value with a unit is converted exactly and rounded once, and that NaN is written `nan` without its sign or payload, the one value that does not round-trip; section 4's bevel example is written `{ width 4mm; segments 3 }`, two attributes (Sau's decision S59).
- November 7, 2026: section 8 lists Null's forms over RNA for the slice (project, units, members, `data`, material slots, primitives, `vertices` and `polygons`, node trees, light power, lens millimetres) and that defaults are a new datablock's; section 18 lists every diagnostic code; section 6 defines the sRGB rule. Section 4's examples follow the schema: a primitive cube's `size` is one length, scaled by its object, and `layout` has one node per `;`. Task 1a-06.
- November 7, 2026: section 15 fixes the formatter's layout and section 17 the per-value rules of the smallest write-back edit; a value printed in a unit keeps that unit whenever a decimal in it reads back (task 1a-07).
