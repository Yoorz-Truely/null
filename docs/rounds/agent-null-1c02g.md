# agent/null-1c02g

The report of agent/null-a working as `agent/null-1c02g` in `work/wt/null-1c02g/`, on Null's branch `agent/null-1c02g`, stacked on `agent/null-1b03h` (1b-03h's string lengths). Sau, Cosmósis and Scowr are links to their main checkouts. November 8, 2026. Lints and Null's tests only.

## 1c-02g: byte-list strings wherever a string goes

**What changes** (`null_check`).

- **`check_string`** now takes the value as written, a string or a byte list (S65), and checks it in bytes:
  - a list's items must be whole numbers 1 to 255;
  - a list whose bytes are valid UTF-8 is a `null-type` error whose fix is the string (one spelling, S65);
  - a 0 byte is an error, since Blender's strings end at one;
  - past the field's `max-length` with its closing NUL, it is a `null-range` error, as for a string.
- **Where strings go.** Every string property takes it, and so does a datablock's display name: `object @pur [80 252 114]` is `Pür` in Latin-1. The list is read as the name only right after a handle, and only when it is all whole numbers. A datablock whose name is a byte list needs a written handle, since none can be made from it.
- **Spec and changelog.** Section 6's String row says the list is taken wherever a string goes, display names included, and is held to the field's length in bytes. `CHANGELOG.md` has the entry.

**Tests** (`null_print/tests/byte_strings.rs`), on the project schema with `ID.name` at 256 and `Image.filepath` at 1024 (`FILE_MAX`). Until 1b-03h's batch build gives the schema these lengths, the test sets them; once it does, the test requires them.

- A Latin-1 path written as a byte list (`//müller.png`) checks clean. The formatter and canonical order give the file back unchanged. Write-back's reader takes it as those 12 bytes, and its printer gives the same list.
- 1,023 bytes fit `filepath`; 1,024 are one `null-range` error, "holds at most 1023 bytes, not 1024".
- A display name as a byte list checks clean, and 256 bytes are a `null-range` error against `ID.name`.
- UTF-8 bytes are a `null-type` error with the string as the fix. 256, -1 and a word are refused as bytes, and a 0 is refused.

Null's whole workspace tests, clippy and `just check` pass.

**Not here.** "Applies" from the row's Done-when is Sau's: apply writes a string through `sau_bridge_string_set`, which takes UTF-8 text today. Applying a byte-list string needs a bridge function taking bytes, which is Sau's side of this row or one after it.

## Review
