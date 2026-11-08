# agent/null-1d06a

The report of agent/null-a working as `agent/null-1d06a` in `work/wt/null-1d06a/`, on branch `agent/null-1d06a` in Null, stacked on `agent/null-1d01a` (in review), since both edit section 9. November 8, 2026. Lints and Null's tests only.

## 1d-06a: level 0 geometry specified (S23)

A new subsection of section 9, "Level 0: attribute arrays".

**The forms,** in a mesh's block:
- **`vertices`** (positions) and **`polygons`** (corner order), as the slice already writes them.
- **`edges`**, only when needed: a loose edge, or an edge attribute whose values follow Blender's edge order. Otherwise Blender makes the edges.
- **`attribute @h <domain> <type> "Name" [values]`**, a sub-object of the mesh. It takes the domain and data-type items the schema's `Attribute` has, and one value per element, written as the type's single value is. Internal `.`-named attributes are never written, because `edges` and `polygons` are they.
- **`uv-map @h "Name" [[u v] …]`**, one value per corner.
- **`vertex-group @h "Name" { weights [[index weight] …] }`**, sparse and in index order.
- Colours, creases, bevel weights, sharp edges and faces, seams and custom normals are plain attributes, by Blender's names.

**Sizes** (S23):
- Up to 2,000 elements is inline.
- A larger array goes in a Null text file beside the mesh's file, named `<mesh>.<array>.noll`, one element per line. It holds one `array <kind> @owner <array> [ … ]` node.
- The mesh's line names it with `file=`, and gives `count=` and `bounds=`.
- A binary copy in `.sau/` is a cache only, and a binary data file is by opt-in, its format specified when needed.

**Decisions I took, for the lead to overturn:**
1. **The summary is values (`count=`, `bounds=`), not a comment.** The row and S23's draft said "a summary comment checked on load". A checked value belongs in the syntax tree, the formatter and write-back already handle values, and a comment that changes what loads would be Null's only one.
2. **Data files are not content-addressed.** Cosmósis's `data/` names files by SHA-256. A text file named by its hash would be renamed by every edit, so a diff would show a whole file deleted and added, against S23's "Git shows the elements that changed". So the file sits beside its mesh, named after it.
3. **A data file is a `.noll` file with one `array` node,** read only through the line that names it. Cosmósis's layout must then not treat it as a datablock file. That is 1d-06b's, with a Cosmósis change.

**Examples not built yet** are fenced `null (1d-06b)`. The spec-examples test leaves them out until that task lands, and fails on a `null (…)` fence that names no task. Section 4 says so.

**Checks:** `cargo test -p null_check --test spec_examples` passes, and Null's lint-only `just check` passes.
