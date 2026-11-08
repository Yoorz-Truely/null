# agent/null-1d02c

The report of agent/null-a working as `agent/null-1d02c` in `work/wt/null-1d02c/`, on branch `agent/null-1d02c` in Null, stacked on `agent/null-1d06b` (with its round 2 and approval, `19287e7`). November 8, 2026. Null's tests only.

## 1d-02c: every diagnostic

**The codes are listed once.** `null_check::codes::ALL` names the eleven codes in section 18's order: `null-syntax` (now also `null_check::codes::SYNTAX`, the same string as `null_syntax`'s), `null-name`, `null-kind`, `null-unknown-property`, `null-type`, `null-unit`, `null-range`, `null-enum`, `null-handle`, `null-readonly` and `null-repeated`.

**One snapshot per code** (`null_check/tests/diagnostics.rs`). Each code has one or more cases, each a small file. The snapshot shows each case's whole output as `sau check` prints it (file, line and column, severity, code, message), and its fix where it has one. The syntax's diagnostics and the checker's come together.
- **`null-syntax`:** a list never closed (fix: add `]`); a `=` with no key.
- **`null-name`:** a handle not in kebab-case (fix `@big-cube`); an enum value in capitals (fix `xyz`); a node name with `_` (fix `pass-index`, beside the unknown property's own).
- **`null-kind`:** a datablock kind, a primitive and a node kind misspelt, each with the nearest as its fix.
- **`null-unknown-property`:** an object's property and a bevel modifier's, misspelt, with fixes.
- **`null-type`:** a string for a whole number, a list too short, a boolean that is neither.
- **`null-unit`:** an angle where a length goes, seconds where a length goes, an unknown unit.
- **`null-range`:** a float and an integer outside their hard ranges, as notes, since Blender clamps them.
- **`null-enum`:** two misspelt enum values, with fixes.
- **`null-handle`:** a reference to nothing with the near match as its fix; a data reference to nothing; a handle defined twice, with the first as a label.
- **`null-readonly`:** a read-only property written.
- **`null-repeated`:** a property given twice.

**The test fails** when a code in `ALL` has no case, when a case does not raise its code, when a case names a code `ALL` lacks, and when section 18's list of codes, read from its backticks, is not `ALL` in order. Section 18 says so.

**Checks:** `cargo test --workspace` and clippy `-D warnings` are clean, and Null's lint-only `just check` passes.
