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
