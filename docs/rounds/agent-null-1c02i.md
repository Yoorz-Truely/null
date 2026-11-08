# agent/null-1c02i

The report of agent/null-a working as `agent/null-1c02i` in `work/wt/null-1c02i/`, on branch `agent/null-1c02i` in Null, stacked on `agent/null-1c02g` (1c-02g, approved). November 8, 2026. Lints and cargo tests only (S69).

## 1c-02i: a byte-list display name is an error (S72)

**The rule.** Blender strips every byte of a datablock's name that is not UTF-8: `BKE_id_new_name_validate` calls `BLI_str_utf8_invalid_strip`. Sau keeps that rule (S72), so a name is always a string.
- A display name written as a byte list that is not UTF-8 is a `null-type` error. Its fix is the name Blender would hold, written with Null's escapes. `object @pur [80 252 114]`, which is `Pür` in Latin-1, gets the fix `"Pr"`.
- When no byte survives, the error has no fix and asks for the name as a string.
- A byte list that is UTF-8, holds a 0, or holds something that is not a byte gets 1c-02g's errors, as before.
- Byte lists stay valid for string properties, such as file paths, held to the field in bytes.

**Blender's check, not Rust's** (`null_check::names`). `blender_strip` ports `BLI_str_utf8_invalid_byte` and the strip around it, which drops one refused byte at a time and checks again from there. That check differs from Rust's UTF-8:
- it takes 5- and 6-byte forms, code points past U+10FFFF, and surrogates other than seven listed sequences;
- it refuses U+FFFE and U+FFFF.

`utf8_reading` then drops whatever Blender keeps that a Null string cannot hold, such as a lone surrogate. Its unit tests use cases from Blender's own table (`BLI_string_utf8_test.cc`): overlong forms, the 5.1 and 5.3 sequences, 5- and 6-byte forms, and lone continuation bytes.

**Where.** `check_display_name` in `values.rs` is called for every datablock line and every collection defined in a scene. It runs even when the schema has no `name` field for the kind.

**Tests** (`null_print/tests/byte_strings.rs`):
- 1c-02g's "a display name may be a byte list" is replaced by: a byte-list name is an error, and its fix checks clean;
- a collection in a scene, whose fix keeps a quote and a carriage return exact (`"\"\u{d}"`);
- no UTF-8 at all gives no fix;
- 1c-02g's errors are unchanged for a UTF-8 list and for a 0 byte;
- a string name is still held to the field.

The byte-list file path tests are unchanged and pass.

**The corpus.** No corpus file writes a byte list, in a name or anywhere else (a search of `corpus/` in Cosmósis finds none), so the corpus checks as before. A `cargo test` of Cosmósis from this worktree would build main's Null, since `Cosmosis` here is a link to the main checkout, so I did not count that run.

**Checks:** `cargo test -p null_check -p null_print`, clippy and `cargo fmt` clean; Null's lint-only `just check` passes. Spec section 6 (the string row) and the changelog say it.

**For Sau's 1c-02h.** Its apply already reports a byte-list name and does not set it. With this change the checker refuses one first, so that path is reached only through a project that skipped `sau check`.
