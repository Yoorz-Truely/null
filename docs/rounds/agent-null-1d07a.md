# agent/null-1d07a

The report of agent/null-a working as `agent/null-1d07a` in `work/wt/null-1d07a/`, on branch `agent/null-1d07a` in Null and in Cosmósis, cut from `main` after batch 1. Sau and Scowr are links to their main checkouts. November 8, 2026. Lints and Null's and Cosmósis's own tests only; nothing of Sau's was built.

## 1d-07a: node trees, modifiers, constraints, actions and drivers, checked against the schema

**The spec** (`docs/spec/null-v0.md`).

- **Section 8, new: "Modifiers, constraints and pose bones".**
  - `modifier @h <kind> "Name" { … }` and `constraint @h <kind> "Name" { … }` are written in stack order. Each kind is one of its family's (S67), and the block holds that kind's properties and the family's own.
  - A geometry nodes modifier's `node-group` names its group, and the group's inputs are its lines, by their handles.
  - `bone @h "Name" { … }` is the object's pose of that bone: `PoseBone`'s properties and its own constraints.
  - `data armature @h` joins the data kinds.
- **Section 10, now complete:**
  - **Where trees are:** in materials, worlds and lights, and in `shader`, `geometry` and `compositor` groups. A scene's compositor is its `compositing-node-group`.
  - **Nodes and links:**
    - a node has its settings (its own properties, plus ten every node has) beside its inputs; an input wins a shared name;
    - links, lists of links into a multi-input socket, and links only into a geometry or shader socket;
    - frames, with `parent`;
    - a node's position and width are the `layout` block's alone.
  - **A group's interface:** `input`, `output` and `panel`, the socket type an interface socket's kind (`float-distance`), the handles being the socket names. Group nodes take their group's interface.
  - **Zones** are written as one block, `zone @loop repeat "Repeat" { … }`. It holds:
    - item lines named after the output node's item collections (`repeat-item`, `state-item`, `input-item`, `main-item`, `generation-item`, `output-item`);
    - the input node's inputs and settings;
    - an `output { … }` block;
    - the nodes inside.

    `@loop.item` is the item coming in, inside the block, and the item leaving, outside it. A zone's layout line holds two positions.
- **Section 11, now complete:**
  - an action's slots, `slot @h <kind> "Name"`;
  - `fcurve <path> index=N`, with keys `key <frame> <value> setting=…` and f-curve modifiers;
  - the short form, with f-curves in their slots, and the full form, with `layer`, `strip … keyframe` and `channels @slot`, never both;
  - `action @a slot=@s` on the datablock that plays it;
  - NLA `track` and `strip` lines;
  - drivers written on the property they drive: `driver="…"` or `driver=sum`, `index=`, `var <name> <type> { target … }`.

  Each construct has an example.
- **Sections 4 and 7.** Section 4's examples now follow these forms: `tex-image` (S67), and the action's f-curve in its slot (`slot @lamp light "Lamp"`, not `slot light @lamp-light`). Section 7 counts `zone` lines, and the nodes inside a zone, among a tree's sub-objects.
- **`CHANGELOG.md`** has the entry.

**The checker** (`null_check`).

- The new modules are `check/trees.rs` (nodes, links, frames, interfaces, group nodes, zones, layout), `check/subs.rs` (modifiers, constraints, f-curve modifiers, pose bones) and `check/anim.rs` (actions, slots, layers, strips, channels, f-curves, keys, what plays, NLA, drivers).
- Before any tree or datablock is checked, every group's interface and every action's slots are read, so a group node, a modifier or an `action` line may name one defined in another file.
- `handle_ref` takes a datablock whose struct extends the one asked for. A `geometry` group is a `node-tree`, and anything is an `id`.
- The index of sub-object paths counts zones and the nodes inside them.
- The schema's `multi-input` is read.

**Measured.** `null_check/tests/spec_examples.rs` checks the spec's own examples:

- **Every example of sections 8, 10 and 11** (seven, one of section 8 and three each of sections 10 and 11) checks clean against `schema/null-schema-project.json`. The examples are checked together as one project, since they name each other.
- **Every construct the sections name appears in an example** (37 phrases); a missing one fails the test.
- **42 copies of the examples, each with one mistake,** each give the expected error. Among them:
  - a misspelt modifier, constraint, pose bone, node, zone, interface, key, f-curve modifier, NLA strip or driver setting, each with its fix;
  - an unknown modifier, node, zone, socket or slot kind;
  - a link to an output the node, group or zone lacks (`@loop.geomtry` inside the zone), and a value given to a shader socket;
  - `location` on a node, a `parent` that is not a frame, and a zone's layout line with one position;
  - an array f-curve without `index=` or past its end, a key in the wrong unit (`620m` on `energy`), f-curves in both forms, a `channels` naming no slot;
  - a slot of the wrong kind, and a missing `slot=`;
  - a variable's bad type, name or target count;
  - a driven property with a value of its own.

  I checked that a changed expectation fails the test.
- **The rest.** Null's whole workspace tests pass. The one snapshot change is two messages' wording ("a node kind a shader tree takes"). Clippy and `just check` pass. `tools/sau-crosscheck.sh Null` passes: Sau's crates build against this Null. `sau check` passes on every Cosmósis corpus project with this checker.

**Cosmósis** (branch `agent/null-1d07a`). Its test `a_modifier_line_waits_for_its_form`, written in C-06 to flip once this landed, is flipped. It is now `a_misspelt_modifier_property_is_caught_with_its_fix`: `wdth` in a bevel modifier is "a bevel modifier has no `wdth`; did you mean `width`?". `cargo test -p sau_cli` passes against this Null. Cosmósis and Null land together, or Cosmósis's old test fails on the new Null.

**Choices the lead may overrule.**

- **Zones are one block,** not two nodes, with item lines named after Blender's collections. Inside the block, `@zone.item` is the item coming in; outside, the item leaving. This is the one place a link's meaning depends on where it is written.
- **A slot's handle is its own.** `slot @lamp light "Lamp"` replaces section 4's `slot light @lamp-light`, so that it follows section 7's `<kind> @handle` rule. A datablock names the slot it plays with `slot=`.
- **The short form puts f-curves in a slot,** for one layer and one keyframe strip (S35). The full form never mixes with it.
- **A driver variable's name is a plain word, not a handle,** since the expression uses it (letters, digits, `_`).
- **A sub-object's property in an f-curve (`fcurve @bevel width`) and a node socket's (`@bsdf.roughness`) are written bare** until the checker resolves them through the datablocks that play the action. That is left for 1d-07g.

**Not here.** These remain unchecked:
- group sockets' value types, beyond their names and links;
- a zone's for-each sockets beyond the schema's static ones;
- texture node trees (1d-07b);
- an armature's own bones (1d-07h);
- drivers' expressions (1d-07f).

Applying any of this in Sau is 1d-07b to 1d-07i.

**Also.** `just wt-new … --repos null` made a Null-only worktree without the `Scowr` link, so Cargo could not load Null's workspace until I added it by hand. This is the same gap as before 0-15.

## Review
