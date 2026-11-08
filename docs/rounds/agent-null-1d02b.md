# agent/null-1d02b

The report of agent/null-a working as `agent/null-1d02b` in `work/wt/null-1d02b/`, on branch `agent/null-1d02b` in Null, stacked on `agent/null-1d02a` (with its round 2, `c7a8969`). November 8, 2026. Lints and Null's tests only.

## 1d-02b: the formatter for every construct

**Canonical order for every v0 construct** (`null_print::canonical`, section 15). Before this, the order covered the slice: datablocks in RNA's order, a material's shader nodes, units, and struct blocks. Now it also covers:
- **Families:** a `modifier` or a `constraint` line's block is in its kind's struct's order (`BevelModifier`'s), and so are a pose bone's constraints.
- **Sub-object lines where RNA's collection of them stands:** `modifier` at `modifiers`, `constraint` at `constraints`, `slot` at `slots`. Null's own forms keep their places, which they take first.
- **Trees of any kind:** a world's and a light's `nodes` are shader trees, and a node group's are its own type's (`GeometryNodeTree`). Each `node` is ordered by that tree's node of its kind.
- **Zones:** items first, then the input node's lines as written, then `output`, then the nodes inside, which keep their order.
- **Groups:** interface lines (`input`, `output`, `panel`) as written, then `nodes`, then the trailing `layout`.
- **Actions:** an f-curve's settings in `FCurve`'s order, its keys as written, its modifiers by their kind (`FModifier`), in slots and in layered strips' `channels`.
- **NLA:** a `track`'s strips in `NlaStrip`'s order, on any datablock.
- **Components:** parameters first, then each datablock in its own kind's order; placements and overrides as written.

What is meaning by its order stays as written: a tree's nodes, a group's interface, keys, and a component's datablocks.

**Defaults left out** (`null_print::without_defaults`, new). A line of its own whose value reads to exactly its property's default goes. The default is the schema's `new` where it has one, else RNA's `default`. It applies in a datablock, a struct block, a modifier or a constraint. These stay:
- a value with a comment on its line or just above it;
- one of a `{ a; b }` line's attributes;
- a line with a `key=value`;
- an attribute the schema does not know;
- a value whose unit is read through the scene's scale or frame rate (lengths, areas, volumes, masses, velocities, accelerations, powers, frames in seconds), which this pass does not read;
- text that does not parse.

Null's `Property` now carries the schema's `default` and `new`, which the schema files already hold. The batch's schema-sync brings the rest.

**A choice, for the lead:** `canonical` does not call `without_defaults`. They are two entry points, and `sau fmt` (Cosmósis) should call `without_defaults` before `canonical`. I kept them apart for two reasons:
- Cosmósis's corpus is kept canonical (`sau fmt --check`), and some of its files write defaults on purpose (the pier's `scale [1 1 1]` says the object is not scaled). Turning it on in `canonical` would fail Cosmósis's check at the next integration. It belongs with Cosmósis's own change and its corpus formatted again.
- Sau's printer (1d-05a) can call both: it prints from Blender, and defaults are what it should leave out.

Section 15 says this.

**Tests:**
- **Idempotence and nothing lost** over every spec example (the corpus of v0's constructs): `canonical` keeps every word and comment, and a second pass changes nothing.
- **Generated trees, as a property test:** every spec example with its blocks' lines shuffled. Canonical keeps every word and comment, and a second pass changes nothing. Lines whose order is meaning keep the shuffled order, so the result need not be the example's own.
- **Order:** a bevel's `width` before `segments`; a zone's items, input lines, `output`, then nodes; a group's `layout` after its `nodes`, with its interface as written; an f-curve's settings before its keys, and keys as written.
- **Defaults:**
  - default lines go and set ones stay (`rotation-mode xyz`, `pass-index 0`, `scale [1 1 1]`);
  - nothing that is not surely a default goes;
  - a bevel's `segments 1` goes, by its own struct;
  - over every spec example, leaving out defaults and then ordering is stable.
- The slice's existing canonical tests, and its shuffle property test, pass unchanged.

**Checks:** `cargo test --workspace` and clippy `-D warnings` are clean, and Null's lint-only `just check` passes. `null_print` gains `serde_json`, already in the workspace through `null_check`.

**One notion of a default, for 1d-05a.** I reviewed Sau's printer (1d-05a) right after this. It compares against the schema's `default` alone, while section 8 says the default is a new datablock's value, the schema's `new` where it has one. For 29 properties the two differ: a collection's `color-tag` (`none`, not `color-01`), a material's `blend-method` and `use-nodes`, a light's `use-nodes` and `temperature-color`, a camera's `angle`, and others. So that the printer and the formatter cannot disagree, Null owns the definition:
- **`null_print::default_of(p)`:** `new`, else `default`, with a matrix's flat column-by-column default given row by row (the printer's own rule, moved here);
- **`null_print::is_default(value, p)`:** bit for bit.

`without_defaults` uses them, and the printer is to call them. Tested: `color-tag`'s default is `none`, the identity matrix row by row, and bits.

## Review

### 1d-02b, `d8ce969` and `fbcbdb6`, by agent/cosmosis-a, November 8, 2026: approved, pending the batch verify

I read the diff from 1d-02a round 2 (`c7a8969`), and ran `cargo test -p null_print` (every suite passes, the shuffle property tests included) and Clippy (`-D warnings`, clean).

**What holds:**
- **Canonical order covers every v0 construct,** each by the struct that defines it:
  - families by their kind's struct;
  - trees of any type, each node by its own tree's definition (a world's or light's shader tree, a group's own type);
  - zones (items, input lines, `output`, then nodes);
  - a group's interface, then `nodes`, then `layout`;
  - f-curves (settings, then keys as written), and NLA strips;
  - components (parameters first, the rest as written).

  What carries meaning keeps its written order, and the sort is stable.
- **The default has one definition.** `default_of` is the schema's `new`, else RNA's `default` (section 8), with a matrix's column-major RNA default turned row by row. `is_default` compares bit for bit. 1d-05a's printer and the formatter therefore cannot disagree.
- **`without_defaults` takes out only what is surely a default:** a line of its own, with one value, no `key=value`, no block, no `;`, and no comment on it or just above it. The value must read through section 6 to exactly the default's bits, and its unit must not read through the scene (scale or frame rate). A file that does not parse is returned as it is.
- **Keeping `canonical` and `without_defaults` apart** until Cosmósis reformats its corpus (C-09) is the right order, and section 15 says so.

**Notes (none blocks):**
- **The plural rule is a guess.** `struct_order` inserts a singular before every property name ending in `s` that has none (`modifiers` gives `modifier`). It also makes names no line uses (from `objects`, `bones`, `use-nodes`…). That is harmless, since an unknown name only places a line that does not exist, but a list of the sub-object kinds (section 7's) would say what is meant.
- **The float default's rounding.** `typed` reads a float default as `as_f64() as f32`. That is exact as long as the schema prints each float32 default in its shortest form, which Sau's generator does. A test pinning one default whose decimal is not exact in f32 (`0.1`) would keep it so.
