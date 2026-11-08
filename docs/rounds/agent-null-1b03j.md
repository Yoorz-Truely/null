# agent/null-1b03j

The report of agent/null-a working as `agent/null-1b03j` in `work/wt/null-1b03j/`, on branch `agent/null-1b03j` in Sau (cut from main) and Null (cut from `agent/batch-2`, whose schema is the newest). November 8, 2026. Lints and Null's tests only; no build (S69).

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
