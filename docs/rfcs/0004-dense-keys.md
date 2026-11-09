# 0004: Dense keys in a Null text file of their own

- Status: proposed
- Date: November 9, 2026
- Decided by: (the lead under the user's standing instruction, Sau's decision S59), with Sau's decision number
- Affects: forms over RNA, write-back, diagnostics
- Null format: unchanged

## Problem

Sau's decision S54 asks for "dense motion capture and baked keys in their own Null text files one key per line". Section 11 gives an f-curve only inline keys:

```null
fcurve location index=2 {
  key 1 0m
  key 12 1.5m interpolation=sine easing=ease-out
  …
}
```

That form is right for a hand-keyed curve of a few keys. It is wrong for a baked or captured curve with a key on every frame:
- a ten-second capture at 120 fps is 1,200 keys per channel;
- a rig of 50 bones has 450 such channels.

Written inline, the action's file runs to hundreds of thousands of lines. The datablocks that the action file also holds (its slots, its settings, the hand-keyed curves beside the dense ones) are lost in it, and a diff of a small edit is buried in the reflow of a huge file.

Section 9 already solves the same problem for a mesh's arrays: up to 2,000 elements inline, and a larger array in a file of its own, one element per line, named by the line that uses it and summarised there (S23, S74). Section 11 has no such form for keys. Sau needs one to convert a `.blend` with baked animation (1d-05d) and to write back a key moved in the graph editor without rewriting a file.

## Design

Section 11's "Keys" paragraph gains the following, after the key's settings:

> **Dense keys in a file of their own** (Sau's decision S54). An f-curve of up to 2,000 keys writes them inline, as above. One with more writes them in a Null text file of its own, beside the file that holds the action, one key per line. The file is not named by its content's hash, so an edit is a readable diff of the keys it changed.
> - The f-curve's block names the file and gives its summary, which the checker holds the file to: `keys file="hop.ball.location-2.keys.noll" count=1200 frames=[1 1200]`.
>   - `count=` is the number of keys.
>   - `frames=` is the first and the last key's frame, as Blender sorts them.
>   - A summary that disagrees with the file is an error, at the line that names it, as section 9's is.
>   - An f-curve writes its keys inline or names a file, never both.
> - **Naming.** The file's path is from the folder of the file that holds the action, as section 9's data files are, and is usually beside it. It is named after the action's file, the slot, the property and its index: `hop.ball.location-2.keys.noll` for `@hop`'s `@ball` slot's `location` at index 2, and `flicker.lamp.energy.keys.noll` for a property that is no array. A name another f-curve's file already takes gets `-2`, `-3`.
> - **The file** holds one node: `keys`, the action's handle, the slot's, the property, `index=` for an array's item, and a block of `key` lines exactly as an inline f-curve's (frame, value in the property's unit, settings as `name=value`), in frame order:
>
>   ```null
>   // actions/hop.ball.location-2.keys.noll
>   keys @hop @ball location index=2 {
>     key 1 0m
>     key 2 0.0123m
>     …
>   }
>   ```
>
> - **Checking.** The checker reads the file through the line that names it, checks each key as an inline key is checked, and reports a key's mistake at its own line in the file. A key file is never read as a datablock file. Cosmósis's layout check counts it among the data files that `file=` lines name (C-10), and `sau check` reports a missing or misplaced one as it reports a mesh's.
> - **Write-back** follows section 17's smallest edit:
>   - A key moved or changed in Sau rewrites its own line in the key file, and nothing else, unless the move changes the first or last frame, which also rewrites `frames=`.
>   - A key added or removed adds or removes its line, and rewrites `count=` and, where it changes, `frames=`.
>   - A curve that grows past 2,000 keys moves its keys into a file, and one that shrinks to 2,000 or fewer moves them back inline. Either is a change of the f-curve's own lines, the only time both files change for one edit.
> - `sau convert --inline` writes every f-curve's keys inline, whatever their number, as it does for arrays.

Key reduction stays an explicit step that reports its deviation, never something a write does (S54). This proposal does not add one.

## What breaks

Nothing.
- No file of Cosmósis's corpus or Null's slice project has an f-curve of more than 2,000 keys, or a `keys` node.
- The form is new, and an f-curve that does not use it means what it meant.

## Migration

None.

## Alternatives

- **One key file per action**, holding a `keys` block for each of its dense f-curves. There would be fewer files for a rig, but each would be huge and would mix channels. A key moved is still one line, but the file's name says nothing of what moved, and two people editing two bones' curves always touch the same file. A file per f-curve keeps each channel's history its own, as section 9 keeps one array per file.
- **Keys as columns, a list per setting** (`frames [1 2 3 …]`, `values [0m 0.0123m …]`, as some exchange formats do). This is shorter, but one key's frame, value, interpolation and handles are spread over lines far apart, so a moved key is several edits in several places. One line per key is S54's own wording, and section 17's one-line edit.
- **A binary file** (S23's opt-in for scans). This loses the readable diff that is the point of keeping animation in text. The opt-in stays for data that no one reads, and does not need a key form.
- **Keys named by content hash in `data/`.** Content addressing suits data that never changes in place. Keys change one at a time, and a new name for every edit would make each moved key a renamed file.
- **Always inline, the formatter folding long curves.** This leaves the action file as large as before: folding is the editor's view, not the file's.

## Decision

Filled in when decided.
