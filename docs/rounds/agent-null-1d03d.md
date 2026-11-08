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
