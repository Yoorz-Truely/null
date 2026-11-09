# agent/null-1d03d

The report of agent/null-a working as `agent/null-1d03d` in `work/wt/null-1d03d/`, on branch `agent/null-1d03d` in Null, stacked on `agent/null-1d02c` (`ed969ae`). November 8, 2026. Null's tests only.

## 1d-03d: short forms

**The short forms.** Section 17's new subsection, "Short forms", lists every short form the spec has, each with its full form and where it applies. There are eight:
1. a datablock's handle left out (section 7);
2. a sub-object's quoted name left out (sections 8 to 10), which is then its handle without `@`;
3. settings on their owner's line (section 5), for a datablock, modifier, constraint or pose bone;
4. a bare boolean (section 5);
5. `units`' short form (section 8);
6. `slot=` left out when the action has one slot (section 11);
7. an action's one layer and strip (section 11, S71 (3));
8. a path's owner left out in its own block (section 7).

Primitives and components are S35's other abstractions, with rows of their own (level 1 geometry, 1d-03b/c), so they are not here.

**`null_print::expand`** (new, in `short.rs`) writes a project's files with every short form in full, laid out by the formatter. It works on the whole project, because form 6 needs to know the action's slots, which may be in another file. The output shows meaning, so comments and `/-` lines are dropped.

**`null_print::write_through`** (new) is write-back's edit of one value. The value is addressed as the full form spells it (`Address`: the datablock, `Line`s down to the value, then `Attr::Named` or `Attr::Arg`), and the new value is a `New`: numbers, a boolean, a word or a path. The writer finds the value however the file writes it, and changes only that value, using the existing minimal-edit machinery of `write.rs`. That machinery is now shared: `Word::all`, `insert_text`, and `replace`/`printed`/`handle_of` are crate-visible. Here is what keeps each form:
- the action's layer, strip and channels steps go to the slot's block when the action has no `layer`;
- `units` settings are found by their short or full name, and a unit word stays a unit word (`length=cm` for `centimeters`);
- a bare boolean stays bare while it is on, and becomes `name off` when it is not;
- `slot=` that names the action's only slot writes nothing;
- a path to one of the owner's own sub-objects is written from the owner.
Values are written in the project's unit scale outside a scene, as `write_transform` writes them.

**Tests** (`null_print/tests/short.rs`).
- **One per form.** The short text checks clean and expands to its exact full form. The full form checks clean and expands to itself. A write through the short form gives the expected short text, which also checks clean. Expanding that text equals the same write through the full form.
- **Extra cases.** `units` is tested for its system, a unit word and the scale. The bare boolean writing `on` again writes nothing. For paths, the index of paths resolves `@box` in `@clay` and `@clay/@box` the same, and write-back keeps or writes the short path.
- **The spec's examples.** Every example of sections 8 (modifiers), 10, 11 and 17's new one, expanded, checks clean as one project, and expanding it again changes nothing. Each short form of the new example is gone from the expansion.

**Decisions:**
- **The strip and layer handles.** The strip has no name, so its handle in the full form of 7 is its kind, `@strip`. The layer is `@layer "Layer"`, Blender's `layer_default_name`. Either gets `-2` (and so on) when one of the action's sub-objects already has that handle. The spec says only that a nameless strip "gets its handle from its track and position", for NLA strips, so this is the full form's own rule.
- **Form 3 covers settings, not every `key=value`.** It applies to datablock, modifier, constraint and pose-bone lines. A key's `interpolation=`, an f-curve's `index=`, a primitive's settings and `slot=` are part of their line, and the checker reads them only there (`slot=` must be a property of the line).
- **Form 8 is not expanded in text.** The checker reads a tree's links and `parent` within the tree, and the paths index is not yet wired into the checker, so the full path would not check. Expansion therefore leaves these paths as written. Both readings are tested through `Index::resolve`, and write-back writes the short one.

**For the integrator.** Section 17's new example is spec example 20, so its v0 snapshot is `spec_example_20`. agent/sau-y's 1d-05b also adds an example (section 19) as `spec_example_20`, so the two collide on merge. Section 19's example becomes 21, and its snapshot needs renaming.

**Checks:** `cargo test --workspace` and clippy `-D warnings` are clean, and Null's lint-only `just check` passes.

## Review

### 1d-03d, `11b87a3`, by agent/cosmosis-a, November 8, 2026: approved; six notes, two for the lead

I read `short.rs`, the `write.rs` changes, the tests and section 17's new subsection, and ran the tests. I probed `expand` on a scratch worktree of `11b87a3`, never in this tree: settings on a constraint inside a pose bone, a `raw` block, a bare boolean in a struct block, an action with two slots, and a component.

**What holds.**
- **The eight forms and their full forms match the spec.** One test per form covers the row's three conditions: the short text parses and checks, it expands to its full form, and a write through it keeps it short. Two more checks make the tests strong: writing through the short form and then expanding equals the same write through the full form, and the full form expands to itself.
- **Form 6 is decided on the whole project** (`only_slot`), as it must be, since the slots may be in another file.
- **The two-slot action probe** expands to one layer and one strip with two `channels`, one per slot.
- **`raw` lines are left alone,** and a bare boolean inside a struct block (`cycles { use-motion-blur }`) gets its `on`.

**The three choices, against S71 and S73.**
1. **The layer's and strip's handles (`@layer "Layer"`, `@strip`, with `-2` on a clash).** They fit S71 (2), with the handle first after the kind, and S71 (3), with a short form and a full form and never both. Against S73 there is a gap, though not in this row. S73 (1) gives each sub-object's identity in `ids.noll` as its name, or Blender's own field for the kinds that have one. A strip has neither: it is found by its position in its layer. So when Sau first writes the full form, the map has no rule for `@strip`'s identity. **For the lead:** S73 could say that a kind with neither a name nor a field of its own (an action strip) is identified by its index under its owner. Then S73 (2)'s rename matching never applies to it, and a reorder counts as removed and new.
2. **Form 3 covers settings, not every `key=value`.** This fits S71 (2)'s line shape (handle, kind, name, then settings) and how the checker reads `interpolation=`, `index=`, `slot=` and a primitive's settings: as part of their line. Agreed.
3. **Form 8 is left unexpanded.** This is right while the checker does not read paths through the index, and S73's identity is untouched, since the map nests sub-objects under their owner whichever way a path is written. But the section's first paragraph says `expand` writes "every short form in full", and the table then says this one stays. The paragraph should say seven of the eight. **For the lead:** a row to wire the paths index into the checker would let form 8 expand too.

**Notes.**
4. **A component's datablocks.** This branch predates 1d-03b, so `expand` does not know components. Its `top` is the file's top level, so inside a `component` block `object "Plank"` gets no handle (form 1) and its line's settings stay on the line (form 3). When the two merge, a component's own datablock lines should count as top. 1d-03c's component expansion had the matching gap, a component's datablock with only a display name, and my 1d-03i fixes that side.
5. **The two expansions need an order and distinct names.** `null_print::expand` (short forms) and `null_check::expand` (components, 1d-03c) are both "expansion" in the spec, sections 17 and 8. Sau's load runs both. Short forms should come first, since the component expansion reads handles and settings lines, and section 8 or 17 should say so. A distinct name for one of them, such as `null_print::full_form`, would keep Sau's code readable.
6. **Form 4's test is any struct with a boolean of that name** (`is_boolean`), not the owner's struct. A line with no value that is not a property (a component's `{ countdown }` once components merge) would get `on` if any struct had a boolean called `countdown`. None has one today, as the probe shows. Resolving by the owner's struct would make it exact.
