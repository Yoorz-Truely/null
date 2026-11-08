# agent/null-1d01c

agent/null-b's branch for 1d-01c, the v0 freeze, in Sau's `work/wt/null-1d01c/`, cut from Null `main` at `64ef6cf` (batch 3a) on November 8, 2026.

## 1d-01c: Null v0 frozen

**What is on the branch:**
- **`2aaf1de`, agent/cos-1d08a merged** (every v0 Null branch). Three conflicts with main:
  - `check.rs`: the `refs` lines that both sides moved, kept once;
  - `lib.rs`: `pub mod raw` beside `pub mod rename`;
  - `tests/project_schema.rs`: both sides' new tests.

  Under 0-26's single test binaries, the branch's new test files join their crates' `tests/main.rs` and its `MODULES` list: `raw.rs` in `null_check`, `byte_strings.rs` in `null_print`. No snapshot needed a `main__` rename.
- **`5e21c53`, Null `660fad1` merged** (1d-07b's Null side, S79): texture node trees in section 10, absolute layout positions, and `width=` and `height=`. It merged cleanly.
- **`ff77143`, the freeze:**
  - the CHANGELOG's v0 section, with v0's history beneath it and an empty Unreleased for what RFCs bring;
  - `AGENTS.md` says the spec is frozen and that any change to what a file means needs an RFC first (`docs/rfcs/README.md`);
  - the spec's opening says v0 was frozen on November 8, 2026.
- **The tag `v0`**, annotated and local, is on `ff77143`. It is never pushed. Integrate's `--no-ff` merge keeps it reachable from `main`.

**Checks:**
- `just check` passed on the merge before it was committed.
- `cargo test --workspace` passes after each later commit, with no `.snap.new` left.
- Scowr is unchanged.
