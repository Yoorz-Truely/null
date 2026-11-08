# agent/cos-1c02j

agent/cosmosis-a's branch for 1c-02j, taken from agent/null-a by the lead. It is in `work/wt/cos-1c02j/`, on branch `agent/cos-1c02j` in Null, cut from Null's `agent/sau-x` (`5255221`), because 1c-02e's write-back writes `parent none`. Sau, Cosmósis and Scowr are links to their main checkouts. November 8, 2026. Null's tests and lints only.

## 1c-02j: the checker takes `none` for a pointer that may be empty

**Before.** Section 6 says a pointer takes `none` for an empty pointer where the property allows one, and 1c-02e's write-back writes `parent none`. But the checker sent every pointer value to its handle check, so `parent none` was "`parent` takes a handle of a object, such as `@name`, not `none`".

**What changed.**
- **`schema.rs`.** `Property` reads the schema's `never-none` (`never_none`). Sau's generator already writes it for RNA's never-null pointers: 105 in the project schema, 11 in the slice's, all read-only there.
- **`check.rs`.** For a pointer property, the word `none` is now:
  - accepted where the schema does not mark the pointer `never-none`;
  - a `null-type` error where it does ("`<name>` is never empty in Blender, so it takes a handle of a <kind>, not `none`").

  Any other word goes to the handle check as before. A read-only pointer gets its read-only error first, as every read-only property does.
- **The spec's pointer row and the changelog** say so.

**Tests.**
- **Checker:** `none_empties_a_pointer_blender_lets_be_empty` (`project_schema.rs`).
  - `parent none` on an object, and `camera none` on a scene, check clean against both schemas.
  - With a copy of the slice's schema that marks `Object.parent` `never-none` (no settable pointer of the slice's is), `parent none` is that one error.
  - `parent nothing` is still a handle error.
- **Round trip:** `an_empty_pointer_round_trips` (`null_print`'s `canonical.rs`). `parent none` checks clean, and `format` and `canonical` give the text back unchanged.

**Not covered here.** `none` in Null's own forms, such as `data none` and a material slot, is not a property pointer. This row does not change those forms; they keep their own rules.

**Checks.** `cargo test --workspace`, Clippy with `-D warnings`, `cargo fmt --check`, and Null's lint-only `just check`.
