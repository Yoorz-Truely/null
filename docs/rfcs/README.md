# Changing Null

Null changes after v0 through written proposals, as Scowr's do (`../Scowr/docs/rfcs/`) and as Rust's RFCs do. Once real projects exist, a change in what a file means can cost every project a migration, so each one is argued on paper first.

## When a proposal is needed

- Any new node, attribute, value form, unit word or syntax that a `.noll` file can hold.
- Any change of meaning: what a word converts to, a default, a form of Null's own over RNA (spec, section 8), how identity and references resolve (section 7).
- Any change to the canonical form (section 15), to the rules of the smallest write-back edit (section 17), or to which diagnostics are errors (section 18).

No proposal is needed for:

- a fix to the spec's wording that changes no meaning;
- implementation work that follows the spec;
- new properties that arrive with a new Blender release through the generated schema (`schema/null-schema.json`), which need no change to the language.

## How

1. Copy [0000-template.md](0000-template.md) to `NNNN-short-name.md`, with the next free number.
2. Fill it in: the problem with a real example, the design with files before and after, what breaks, the migration, and the alternatives.
3. Add a row to Sau's task board (`../Sau/docs/TASKS.md`) naming the proposal, so it is decided rather than forgotten.
4. **The decision.** The user decides, or the lead under the user's standing instruction (Sau's decision S59). The decision is recorded in Sau's `docs/DECISIONS.md` with its S-number, and in the proposal's status line.
5. **If accepted:**
   - the change is written into the spec and `CHANGELOG.md`, naming the proposal;
   - the code follows, with tests;
   - if it changes what an existing file means or whether it reads, the Null format (`null-format` in `cosmosis.noll`) goes up by one and a migration rewrites older files (below).
6. **If rejected:** it stays in this folder marked rejected, with the reason, so the question is not reopened without new arguments.

## Versions and migrations

Every project names the Null format it is written for (`null-format 1` in `cosmosis.noll`, Cosmósis spec, section 4). A change that breaks old files raises the format. Cosmósis migrates an older project when it reads it: the generated migrations of Sau's decision S45, one per step, each with a test that turns a file of the old format into the new one. A project in a format newer than the reader knows is an error, never guessed at. The steps are `null_check::migrate`'s: `FORMAT` is the format this Null reads, and each RFC that raises it adds a `Migration` from the old format to `MIGRATIONS`, most of them a list of `Rename`s generated from Blender's own (`rename_properties`). A step leaves a file it already rewrote as it is. Cosmósis's `sau migrate` runs them (Cosmósis spec, section 4, "Migrations"; Sau task 1d-04d).

## Proposals

| Number | Title | Status |
| --- | --- | --- |
| [0001](0001-srgb-table.md) | The sRGB rule by a table of exactly rounded values | accepted (S62) |
| [0002](0002-zone-names.md) | What a zone's name and block say about its two nodes | accepted (S82) |
| [0003](0003-media-paths.md) | How a media datablock's file is written | accepted (S83) |
| [0004](0004-dense-keys.md) | Dense keys in a Null text file of their own | accepted (S86) |
| [0005](0005-bones-shape-keys-markers.md) | An armature's bones, shape keys and timeline markers | accepted (S88) |
| [0006](0006-curves-point-clouds-grease-pencil.md) | Level 0 arrays for curves, point clouds and Grease Pencil drawings | accepted (S91) |
| [0008](0008-grease-pencil-drawing-references.md) | A Grease Pencil drawing that shows another Grease Pencil datablock | accepted (S98) |
| [0009](0009-node-link-and-socket-state.md) | Muted links, hidden sockets and the value under a link | accepted (S95) |
