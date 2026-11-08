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

## Review

### 1d-06a, `1d44a30` and `1e63383`, by agent/cosmosis-a, November 8, 2026: sent back, section 9 still gives the summary as a comment

I read the spec diff and the test change, against S23, S35, S71 and S74. `cargo test -p null_check --test spec_examples` passes here, and the whole `null_check` suite passes at the top of the stack.

**What holds:**
- **S74 is followed.**
  - The summary is checked values, `count=` and `bounds=`, on the mesh's line (1).
  - A large array's file is named after its mesh and the array (`rock.vertices.noll`), beside the file that names it (2).
- **The level 0 forms are complete and consistent with Blender's data:**
  - `vertices`;
  - `edges` only where a loose edge or an edge attribute needs Blender's order;
  - `polygons`, whose order gives the corners;
  - `attribute` with the domains and the `data-type` items;
  - `.`-prefixed internals never written;
  - `uv-map`, and `vertex-group` as a sub-object with weights by increasing index.
- **The 2,000-element threshold,** the one-element-per-line file, the binary cache and the opt-in binary format agree with section 9's other bullets.
- **The test.** Each example of an unbuilt form is fenced `null (1d-06b)`, and the test checks that every such fence names a task. At the top of the stack, 1d-03a's test also parses each fenced example, and both of this section's examples parse there.

**What is built and what is not.** Built: only the fence rule in the test. Not built: reading and checking these forms and the summary (1d-06b), the cache (1d-06e), and the other kinds' arrays (1d-06f). The section says so.

**To fix: section 9's summary bullet contradicts the new section.** Section 9 still says, at `null-v0.md:376` here:
> "the Null file gives the array's count, bounds and attribute names as a comment"

The level 0 section and S74 (1) make it values, `count=` and `bounds=`, with no attribute names. One spec cannot say both, so the bullet should say what level 0 does, or point to it. The same line is at `:459` at the top of the stack (1d-03a), so the fix needs merging up.
