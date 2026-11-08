# agent/null-1b01h

The report of agent/null-a working as `agent/null-1b01h` in `work/wt/null-1b01h/`, on branch `agent/null-1b01h` in Null and Cosmósis, cut from `agent/batch-2`. November 8, 2026. Lints and cargo tests only (S69).

## 1b-01h: `sau rename @old @new`

**Null: `null_check::rename`** (`compiler/null_check/src/rename.rs`). `rename(files, old, new)` takes the project's parsed datablock files. It returns the edits, the datablock's kind and how many references were changed, or a `RenameError`.
- **The definition** is a top-level line, or a collection defined inside a scene or collection, by name or block. `project` is the project's settings, not a datablock. If the handle is written, it is replaced. If it was made from the display name (S18), the new handle is written before the name, so the name stays as it was.
- **References.** A word `@old`, a path from it (`@old/@bevel`, `@old/@bsdf.roughness`) or a pattern's copy (`@old.7`) is renamed, in arguments, keyword values, properties and lists, at any depth.
- **Left alone**, because there the same word names something else:
  - a sub-object's own line (`SUB_KINDS`), and a node's, zone's or item's line in a tree;
  - every word inside a datablock that has a sub-object `@old`, since section 7 says a path there starts at the owner's own sub-objects;
  - in a tree: a link to an output (`@old.color`), `parent` (a frame) and the `layout` block;
  - an `action` line's `slot=`;
  - `@package:old`, strings and comments.
- **Refused:**
  - an old or new handle that is not `@` and a kebab-case name;
  - no datablock with the old handle;
  - two kinds with the old handle (`Ambiguous`), since a word cannot tell which one it names;
  - the same kind already having the new one (`Taken`);
  - any file with syntax errors (`Unparsed`).

**Cosmósis:**
- **`sau rename @old @new [path]`** (`crates/sau_cli/src/main.rs`):
  1. finds the project root and reads the layout;
  2. parses every file that holds datablocks, which leaves out the manifest, `settings.noll`, `view.noll` and the map;
  3. reads `.sau/ids.noll`, if there is one, before writing anything;
  4. applies each file's edits from its end, writing through a link as write-back does;
  5. moves the datablock's map entry to the new handle and writes the map atomically, never through a link;
  6. prints the kind, the counts and the changed files.
- **The map is checked first.** A map line it cannot read is refused before any file changes, since writing the map again would drop that line.
- **`cosmosis::ids`** reads and writes the map in the form sau_core writes it (sau_core's `apply/ids.rs`): sorted by kind and handle, hex lower-cased, an all-zero ID refused. `rename` moves an entry.
- `sau --help` lists the 16 commands. `sau_cli` depends on `null_check` by path.

**Tests:**
- **Null** (`tests/rename.rs`, 12):
  - references across files;
  - `@pier` left alone in `@pier-deck`;
  - a node, its outputs and its layout entry shadowing an image of the same handle;
  - a tree setting naming a datablock;
  - a sub-object shadowing the handle;
  - paths and copies;
  - a handle made from a name;
  - a collection inside a scene;
  - strings and comments;
  - every refusal;
  - another kind already holding the new handle;
  - the `project` line.
- **Cosmósis** (`crates/sau_cli/tests/rename.rs`):
  - **Each definition of each corpus project**, in a copy with a handle map holding one ID per definition, is renamed to `@<handle>-renamed`. That is 72 renames across the six slice projects and `cycles-render`. For each:
    - every changed byte is the handle made the new one, or the new handle written before a name;
    - the count of new handles is the summary's references plus the definition;
    - `sau check` passes on the result;
    - the map holds the same ID under the new handle and nothing under the old, with no entry lost.

    No handle in the corpus is shared by two kinds. Such a rename is checked to change nothing.
  - a bad map line refuses the rename and changes nothing;
  - usage and help.

**Checks.** Clippy with `-D warnings` and `cargo fmt` are clean in both repos, the tests above pass, and both lint-only `just check`s pass.

**Not done, and why:**
- **A rename made in Sau** (1b-01g) keeps handles. This is the other direction: the text changes, and the next apply keeps identity through the map.
- **Sub-object handles** (`@pier/@bevel` → `@pier/@chamfer`) are not renamed by this command. They need the owner's index from `paths.rs`, and the map's sub-object part (S61 (3)).
- **Patterns' and packages' handles** resolve only once 1d-03 and 1d-04 exist. A copy `@old.7` is renamed with its base, and `@package:old` is left alone.
