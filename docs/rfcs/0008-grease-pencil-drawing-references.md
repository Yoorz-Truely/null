# 0008: A Grease Pencil drawing that shows another Grease Pencil datablock

- Status: proposed
- Date: November 9, 2026
- Decided by: (the lead under the user's standing instruction, Sau's decision S59), with Sau's decision number
- Affects: forms over RNA, identity, diagnostics
- Null format: unchanged

## Problem

RFC 0006 (S91) writes a Grease Pencil datablock's drawings as `drawing @h { points …; curves …; attribute … }` and leaves one kind out: a **drawing reference**, which its "Not covered" section passes to this RFC. A datablock that holds one cannot be written. Sau's printer says so (1d-06f), and `sau convert` loses it.

That is also a bug for drawings that *are* written (agent/cosmosis-a's review of 1d-06f). Blender compacts the drawings array whenever a layer is removed, and compaction frees every reference, as the facts below show. A first apply that removes a layer while a reference sits mid-array frees the reference and moves later drawings into its place. The indices that the handle map keeps for every later drawing then name the wrong drawings.

### What Blender 5.3 stores

I read this from Sau's tree, Blender 5.3. No editor makes a reference, so no file in the corpus or the test files holds one to probe at run time.

- **The struct:** `GreasePencilDrawingReference` (`DNA_grease_pencil_types.h`) is a drawing slot of type `GP_DRAWING_REFERENCE` (1). It holds the base (type and flags, which have no bits defined) and one pointer, `id_reference`, to another `GreasePencil` datablock: not to one of its drawings but to the whole datablock. The header says that if the datablock has several drawings, the reference "references all of them sequentially".
- **Kept:** it is written and read with the file (`write_drawing_array`, `read_drawing_array`), copied with its datablock, and counts as a user of the datablock it names (`foreach_id`, `IDWALK_CB_USER`). The undo system keeps it. `BKE_grease_pencil_references_cyclic_check` exists to refuse a datablock that would reach itself through references.
- **Never made:** no operator, RNA function or Python call makes one. RNA shows a drawing's `type` (`DRAWING` or `REFERENCE`), read-only, and not the pointer. Nothing in the draw or evaluation code reads `id_reference`, so in 5.3 a reference shows nothing.
- **Freed whenever the drawings are compacted.** `remove_drawings_with_no_users` counts a drawing as used only when it is a `GP_DRAWING` with users or a fake user ("GreasePencilDrawingReference does not have a user count currently"). So every reference is freed, and a later drawing is swapped into its place. A frame that showed it keeps an index that is out of range or now names another drawing. `remove_layer` calls this compaction after taking its frames' users ("TODO: Remove drawing reference").

## Design

### The form

> **A drawing reference** is `drawing @h { reference @other }`: in place of points, curves and attributes, the one line `reference`, naming another `grease-pencil` datablock of the project. It is the drawing Blender keeps as a `GreasePencilDrawingReference`: it shows that datablock's drawings, in their order, where Blender draws references. Blender 5.3 keeps and saves a reference but draws nothing for it, so it is written so that a file keeps it. Frames show it as they show any drawing, `frame 12 @h`.

```null
grease-pencil @crowd "Crowd" {
  drawing @first {
    points [[0m 0m 0m] [0.5m 0m 0m]]
    curves [2]
  }
  drawing @walker { reference @walk-cycle }
  layer @people "People" {
    frame 1 @first
    frame 12 @walker
  }
}
```

The checker's rules:
- `reference` names a `grease-pencil` datablock, by handle (section 7). It is an error beside `points`, `curves` or `attribute` in the same drawing, under `null-type` as a drawing's other misplaced lines are.
- A reference that reaches its own datablock, directly or through other references, is refused, as `BKE_grease_pencil_references_cyclic_check` refuses one. This is a new meaning, so it takes a code of its own (S97): **`null-reference-cycle`**. It goes into section 18's list, `codes::ALL`, a snapshot and CHANGELOG, and names the datablocks in the cycle.

Identity is S91's: a reference is a drawing, and its handle is kept in `.sau/ids.noll` by its index among the datablock's drawings.

### What Sau does

- **Apply** makes a reference where the text writes one: a new `DrawingReference` appended to the drawings array, its `id_reference` set and counted, and its user count kept as the undo system keeps it (`id_us_plus`, `id_us_min`). A reference whose datablock the text changes is pointed again. RFC 0006's ordering of drawings, freeing those the text leaves out, takes references like any drawing.
- **The printer** writes `drawing @h { reference @other }` and stops reporting a reference as having no form.
- **Indices never shift:** the bug above. Compaction must stop freeing a reference that a frame shows, or that Sau holds while it removes a layer, since Sau already holds drawings with a fake user (1d-06f). Two ways, for the lead:
  1. **A `Sau:` hook in `remove_drawings_with_no_users`** (recommended), the one place the bug lives. A reference counts as used when any frame shows it (`count_frame_users_for_drawings` gives that), or when it is held. It also fixes Blender's own loss of a shown reference, and a hook is the smallest change. Holding needs one bit: `GreasePencilDrawingBase.flag` defines none, so `GP_DRAWING_BASE_HELD`, set and cleared by the bridge around `remove_layer` as the fake user is for drawings. A runtime-only bit is cleared again before anything is saved. If adding a DNA bit is too much, the hook can treat every reference as used, so that Sau alone frees references (RFC 0006's ordering frees the ones the text leaves out). In Blender, where nothing makes one, that keeps an unshown reference until Sau or the file drops it.
  2. **The bridge removes a layer by its own steps**, not compacting. That repeats `remove_layer`'s steps, which S90 asks the bridge not to do.

  Either way, a test removes a layer on a first apply with a reference mid-array, and checks that every handle still names its drawing (the row's done-when).
- **Write-back:** none. Nothing in Sau or Blender makes a reference, so none appears in Sau that the text lacks.

## What breaks

Nothing: `reference` is a line that no v0 file holds, since RFC 0006's checker refuses any line in a drawing but `points`, `curves` and `attribute`. No file of Cosmósis's corpus or Null's slice has a reference. The Null format stays at 1.

## Migration

None.

## Alternatives

- **A reference to one drawing** (`reference @other.@first`). Blender's struct names a datablock, not a drawing, and shows all its drawings in turn. A form for one drawing would mean something Blender cannot hold.
- **A reference as a frame's target** (`frame 12 @walk-cycle`, a datablock in place of a drawing). That reads well, but Blender keeps a reference in the drawings array, shared by frames like any drawing. Writing it per frame would make one reference per frame, or need a rule for when two frames share one. A drawing line keeps Blender's own shape, and S91's identity with it.
- **Leaving references to conversion's report.** That loses data on every `.blend` that holds one, and leaves the index bug in place: the bug needs the hook whatever form references take.
- **No new code for a cycle** (`null-handle`). A cycle is no unresolved handle; S97 asks for a code where the meaning is new.

## Decision

Filled in when decided.
