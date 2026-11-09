# 0011: A text datablock's content in a file of its own

- Status: accepted
- Date: November 9, 2026
- Decided by: the lead under the user's standing instruction (Sau's decision S59), recorded as Sau's decision S103
- Affects: forms over RNA, write-back, diagnostics
- Null format: unchanged

## Problem

A Blender file can hold text datablocks: a Python script, an OSL shader, notes. Two of the conversion gate's corpus files hold one each (2-nodelib and 12-main_scene, Sau task 1d-09e). Null has a `text` kind, but its schema gives it no way to say what the text holds:
- its content is `Text.lines`, a read-only collection of lines;
- the only way RNA writes it is through a function, `Text.from_string()`.

So `sau convert` writes a text's settings and drops what it says, and the round trip loses the whole script.

The content is source in its own language. Written inside a `.noll` file as a quoted string, it would be escaped line by line, and no Python or OSL tool could read or edit it. It belongs in a file of its own, as an editor and Git expect: `shader.py`, not a string in `shader-py.noll`.

## Design

A `text` datablock names its content's file with a `body` line:

```null
// scripts/shader-py.noll
text @shader-py "shader.py" {
  body "scripts/shader.py"
  use-module on
}
```

```python
# scripts/shader.py, the text's whole content, byte for byte
import bpy
```

- **The path** is written from the project's root, by section 12's rules for a media datablock's file (S83): `/` separators, no `.` or `..` parts, and no part leaving the root. Unlike media, it is not named by its bytes. A text is source someone edits, so its file keeps a readable name.
- **The file** is the text's whole content, byte for byte: Blender's lines, each but the last ended by a line feed.
- **No `body` line** is an empty text, as a new text in Blender is.
- **`body` is Null's own form over RNA** (section 8): a property no RNA struct has, standing for `Text.lines`. The text's stored properties stay ordinary lines (`use-module`, `indentation`, and `filepath` for a text Blender keeps linked to a file outside the project).

**Bytes Blender would change.** Blender keeps a text as UTF-8 lines with no control character but the tab (`cleanup_textline` in `text.cc`):
- a carriage return is taken out, so a file with CRLF line endings reads as LF;
- a byte that is not UTF-8 is read as Latin-1 and written as its UTF-8 character.

A body file holding such bytes is applied as Blender reads it, and compared that way, so it shows as no change on the next apply. Cosmósis warns of it, since the file and the text no longer hold the same bytes.

**Where the files go** is Cosmósis's (spec, section 2): in `scripts/`, which already lists text datablocks, one `.noll` file and one body file per text. The body is named from the text's name:
- kept where it is a safe single file name;
- made safe by the rules for a key file's name otherwise (RFC 0004);
- given `.txt` when it has no extension or ends in `.noll`, so it is never read as Null;
- given `-2` when the name is taken, case ignored, as folders on macOS and Windows ignore it.

**Write-back.** A text edited in Sau's text editor writes its body file again, whole. Its `.noll` file does not change unless a setting did.

**Diagnostics** (Cosmósis's, each a code of its own, S97):
- `cosmosis-body-path`, an error: the `body` path is not a path from the root;
- `cosmosis-body-missing`, an error: no file is there;
- `cosmosis-body-bytes`, a warning: the file holds bytes Blender would change (a carriage return, a control character, bytes that are not UTF-8).

The spec gains section 12's entry "A text's body", and the CHANGELOG a line naming this proposal.

## What breaks

Nothing. No file of the corpus or of Null's slice project writes a `text` datablock, and `body` is a new line no file uses yet.

## Migration

None.

## Alternatives

- **The content as a string in the `.noll` file** (`content "import bpy\n…"`): self-contained, but escaped, unreadable as source, and invisible to every tool of its language.
- **The body as media** (`media/<sha256>.py`, S83): content-addressed and deduplicated, but renamed on every edit and unreadable by name; S83's media are files Sau does not edit, and a script is.
- **The body beside its `.noll` file by convention, with no `body` line**: shorter, but the name would have to be derived from the handle, a text with no content would be indistinguishable from a missing file, and a move would break silently.

## Decision

Accepted by the lead, November 9, 2026, as Sau's decision S103, on agent/cosmosis-a's design for task 1d-09e. Linked texts are left to the task for libraries (1d-09f).
