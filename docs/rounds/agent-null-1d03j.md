# agent/null-1d03j

The report of agent/null-a working as `agent/null-1d03j` in `work/wt/null-1d03j/`, on branch `agent/null-1d03j` in Null. November 8, 2026. Null's tests only.

**The base.** This branch is cut from `agent/null-1d02d` (after main's batch 2), with `agent/cos-1d03b` merged in (1d-03b, 1d-03c, 1d-03h and 1d-03i, all approved), since this row works inside components and renames one of the two expansions. `agent/null-1d02e` is merged in too (the diagnostic wording), so the stack is one tree. The only conflict was `check.rs`'s module list and the checker's fields, which keep both sides, plus `diagnostics__null-kind.snap`, which takes 1d-02e's wording with `component` among the kinds.

## 1d-03j: short forms inside components, form 8 written in full

These are the 1d-03d review's points, as the lead set them after S77:
1. **Form 8 is written in full.** In a datablock's `nodes` block, a link (`@noise.fac`) or a `parent` (`@box`) that names one of the tree's nodes is written from the datablock: `@clay/@noise.fac`, `parent @clay/@box`. List items are treated the same, as a multi-input socket's links. A word that names no node of the tree, such as a datablock handle or a copy (`@plank.7`), is left as it is.
   - **The checker** reads a tree's links and frames either way. `Tree` now knows its datablock's handle, and `Tree::local` takes `@owner/` off a word before the node is looked up, as section 7's paths and the index of paths read it. A path from another datablock (`@oak/@noise.fac` in `@clay`) names none of this tree's nodes and is an error. In an expanded component the owner is the copy's path (`@p.2/@grain`), so the copies check too.
   - **Write-back** keeps a path in the form it is written in, from its owner or not.
2. **Short forms inside components.** A component's block is read as a file's top: its datablock lines take forms 1 and 3, and their blocks every other form. Its `if` and `else` blocks are read as the lines they hold. A placement's and an override's blocks are left as written, since their lines are parameters or the root's, which the component expansion reads.
3. **The rename.** `null_print::expand` is now `null_print::full_form`. `null_check::expand` keeps its name, being section 8's expansion. Section 8 and section 17 now say that the short forms are written in full first, then components are expanded (S77).
4. **Form 4 by the owner's struct.** A bare line takes `on` only when its owner has a boolean of its name. The owner is:
   - a datablock's struct, a light's by its `type`;
   - a modifier's, constraint's or f-curve modifier's kind;
   - `PoseBone` for a bone, and `FCurve` for an f-curve;
   - the target of the struct block's pointer property (`cycles { … }`);
   - a node's struct and inputs.

   Anywhere else (a slot's block, a placement's, a parameter's) a bare line is left as it is, so a parameter's `{ countdown }` never becomes `countdown on`.
5. **Section 17** says that all eight forms are written in full, and where each applies. Its form 4 and form 8 rows say what changed.

6. **A node group's interface before its `nodes`** (from the 1d-08a review, folded in by the lead). `canonical` put a group's `nodes` at its struct's RNA place, since `NodeTree` has a `nodes` property, so the interface lines (`input`, `output`, `panel`) came after it. `Order::key` now looks up `last` (`nodes`, then `layout`) before the RNA names, so the interface comes first, as sections 10 and 15 have it. A zone already wrote its items, its input lines, `output`, then its nodes, as section 10 says, and still does. **For the Cosmósis worker:** once this lands, `sau fmt` moves any group's interface lines in the corpus above its `nodes`, so the corpus needs formatting again.

**Tests:**
- **In `null_print/tests/short.rs`:**
  - Form 8 now has the form test: the short text checks, its full form `CLAY_FULL` checks, a write through either keeps its own form, and the index reads both the same.
  - `short_forms_inside_a_component_then_its_copies` puts every form inside a component. Its full form checks clean, then `null_check::expand` gives copies that check clean, with a link written `@p.2/@grain/@noise.fac`.
  - `a_bare_boolean_is_its_owners`: `use-cyclic` in an object stays bare; `hide-render` and a Cycles struct block's `use-denoising` take `on`.
  - The spec's examples now include section 8's components. In full form they check clean and write in full to themselves, and their components' copies check clean.
- **In `null_print/tests/canonical.rs`:** `a_groups_interface_comes_before_its_nodes`, with a zone inside it, against the project schema, idempotent.
- **In `null_check/tests/paths.rs`:** a tree's links and frames written from their datablock check clean, and one from another datablock is a `null-handle` error.

**Checks:** `cargo test --workspace` and clippy `-D warnings` are clean, Null's lint-only `just check` passes, and the `format` fuzz target, now calling `full_form`, builds and ran 60 seconds clean.

## Review

### 1d-03j, `a1f3eb2`, by agent/cosmosis-a, November 8, 2026: approved; one gap for the lead, two notes

I read `short.rs`, the `Tree::local` and `Order::key` changes, the tests and sections 8 and 17. The workspace's tests pass. I also merged `a1f3eb2` into 1d-08a's Null and ran the v0 corpus through it (below).

**What holds.**
- **Form 8 in full.** `Tree::local` takes `@owner/` off only when a `/` and an `@` follow, so `@clay-2/@x` in `@clay` is no path of `@clay`. In an expanded copy the owner is the copy's path, and 1d-03c's expansion maps `@clay/@noise.fac` through the copy's handles to `@p.2/@clay/@noise.fac`. So the two compose, as `short_forms_inside_a_component_then_its_copies` shows.
- **A component's block read as a file's top.** Forms 1 and 3 apply to its datablocks, and its `if` and `else` blocks keep their place. A parameter's line is no datablock, so it takes neither form, and form 4 by the owner's struct leaves `{ countdown }` alone.
- **The order (S77).** Short forms in full first, then components, is now in sections 8 and 17, and `full_form` is a better name.
- **A group's interface before its `nodes`.** Looking up `last` before the RNA names is the minimal fix, and the corpus confirms it. Re-formatted with this Null, the node-trees project's groups now write `is-modifier`, `input`, `panel` and `output` above `nodes`, as section 10 writes them, and nothing else moved.

**A gap, for the lead.** A placement's block and an override's are left as written, on the ground that "their lines are parameters or the root's". But the root's lines can be short forms:
- `override @gate.2 { hide-render }`, a bare boolean;
- `use @post @x { modifier @ring array count=7 }`, settings on the line;
- `override @a.2 { object "B" }`, which cannot occur, so only the first two matter.

The component expansion lays these lines on the copy's root as written, so the flat project Sau applies still holds short forms, though section 17 says apply reads the full form. The checker reads both, so nothing fails today. But Sau's apply (1d-03e) would have to read short forms too, or they would be misread. **The fix:** write a placement's and an override's non-parameter lines in full with the component's root object as their owner (form 4 by `Object`, form 3 on its modifier and constraint lines). It needs the component's parameter names, which `full_form` can read from the project as it does an action's slots. Either here or in a row of its own.

**Notes (none blocks):**
1. **Form 8's reach narrowed.** It now covers a tree's links and frames; 1d-03d's row said any path in its owner's block. I found no other place where a line names its own datablock's sub-objects by a short path. A driver target and an f-curve's sub-object path name other datablocks. So the narrower row is right, but section 7's general statement of short paths should say it is the tree's case that has a full form.
2. **`Order::key`** now puts `last` first for every struct. Only node trees have both a `nodes` RNA property and a `layout` form today, so nothing else moves, but a comment there could say so.
