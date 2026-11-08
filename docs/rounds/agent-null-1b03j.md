# agent/null-1b03j

The report of agent/null-a working as `agent/null-1b03j` in `work/wt/null-1b03j/`, on branch `agent/null-1b03j` in Sau and Null, both cut from `agent/batch-2` (whose schema is the newest, and whose tools match each other; Cosmósis beside them is a detached `agent/batch-2` worktree, for `merge_test`). November 8, 2026. Lints and Null's tests only; no build (S69).

## 1b-03j: an enum default in its item's own case

**Found in 1d-01a.** `interpolation`'s default was `linear`, while its items are the free-text `Linear`, `Closest`, `Cubic` and `Smart`. So the formatter's default omission (1d-02b) could never match it.

**The generator** (Sau's `sau/tools/null-schema.py`) now writes each enum default as its item's own Null name: an identifier's kebab-case name, or a free-text item as it is. A flag set's default is in the items' order, as section 6 has a file write it. This applies to both paths:
- makesrna's file (`prop_entry_built`, the project scope);
- the running app (`prop_entry`, through `default_flags`, the slice and the add-ons' sections).

A default the items do not list is kept, kebab-cased as before. That happens only for an enum whose items are made when it is used.

**Null's schema files** are updated by the same rule, since a worker does not build (S69): a default that is an item's kebab-cased identifier becomes that item's name, and a flag default is put in item order. Ten defaults changed in `null-schema-project.json`, and none in the slice's:
- `interpolation`, three times (image, environment and geometry image texture): `Linear`;
- `motion-model` and `default-motion-model`: `Loc`;
- a hair BSDF's `component`: `Reflection`;
- Cryptomatte's `layer-name`: `CryptoObject`;
- `snap-playhead-element`: `[key Strip]`;
- two flag defaults reordered.

The batch build's `just schema-sync` writes the files from a build. The `null-schema` gate then checks the copy against the build, so any difference between this rule and the generator would show there.

**Test** (`every_enum_default_is_one_of_its_items_as_a_file_writes_it`, in Null's `project_schema.rs`): over every enum property in both schema files, at any depth (structs, add-ons, node sockets), the default is one of the items exactly, and a flag set's default is in the items' order. The only exception is an enum whose items Blender makes when it is used: one marked `items-at-run-time`, or a slice item depending on another setting (`when`). That exception covers 9 run-time enums such as the colour spaces, and the slice's unit enums, whose default `default` is no listed item.

**Checks:** Null's `cargo test -p null_check` passes, and Null's and Sau's lint-only `just check` pass. `ruff check` is clean on the generator. Its `ruff format` differences were there before this change and are left alone.

## Review

### 1b-03j, Sau `5da91af0` and Null `6a1ee25`, by agent/cosmosis-a, November 8, 2026: approved, pending the batch's schema-sync

I read the generator change, the schema diff and the new test. I ran `cargo test -p null_check --test project_schema` (11 pass) and `ruff check` on `null-schema.py` (clean). Separately, I walked both schema files and found no enum whose default is outside its items, other than the run-time ones.

**What holds:**
- **One rule on both of the generator's paths.**
  - makesrna's (`prop_entry_built`): a default is its item's `enum_item_rna` name, and a flag set's default lists its items' names in item order.
  - The running app's (`prop_entry`): `names.get(default, kebab(default))` and `default_flags`.
  - In both, a default the items do not list stays kebab-cased, as before. That is only a run-time enum's.
- **The schema copies follow the same rule.** The ten changes in `null-schema-project.json` are each an item's own name or an item-order reordering: `Linear` three times, `Loc` twice, `Reflection`, `CryptoObject`, `[key Strip]`, and two flag sets. The slice's file needed none, as the report says.
- **The test** holds both files: every default is exactly an item's name, and a flag default is in item order, with run-time enums (`items-at-run-time`, or a `when` item) exempt. The order check (`ds.starts_with(&listed)`) also lets unlisted run-time flags follow the listed ones, as `default_flags` writes them.
- **Nothing reads an enum default yet** in `null_check` or `sau_core`, so nothing else changes behaviour. The formatter's default omission (1d-02b) is the first user.

**Left to the batch.** The ten hand-patched defaults should equal what the generator writes from a build. The batch's `just schema-sync` and the `null-schema` gate confirm that, and a difference there would mean the patch and the rule disagree.
