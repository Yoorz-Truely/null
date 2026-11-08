# agent/null-1d02a

The report of agent/null-a working as `agent/null-1d02a` in `work/wt/null-1d02a/`, on branch `agent/null-1d02a` in Null, cut from `agent/null-1d03a`'s top (`dd2db62`). That top holds 1d-01a, with 1d-07a merged in, and 1d-06a. November 8, 2026. Lints and Null's tests only.

## 1d-02a: the parser for the whole v0 syntax

**The syntax needed no change.** Every construct of v0 is written in Scowr's node syntax as it is (S15), and the shared parser already reads all of it, with its lossless tree. So what this row adds is the reading of that tree as Null's constructs. Nothing here asks Scowr for a change, so the row's condition (S15, the user's agreement) did not arise.

**`null_syntax::v0`** (`compiler/null_syntax/src/v0.rs`):
- **`outline(parse)`** gives every node as a construct, by its name and where it stands, as the spec's sections define them:
  - **datablocks** and their **attributes**, and **sub-objects** (section 7's lines);
  - **node trees** (`nodes`), **nodes**, **links** (a socket value naming `@node.output`, or a list of them, but not a pattern's copy `@plank.7`), **zones** with their **items** and **`output`**, the **layout** and its **positions**, and a group's **interface sockets** (section 10);
  - **f-curves**, **channels**, **keys**, a line that **plays** an action (`action @a slot=@s`), and **drivers** (a `driver=` property) with their **variables** and **targets** (section 11);
  - **data files** (`array …`) and the lines that name one (`file=`) (section 9);
  - **components**, **parameters**, **placements** and **patterns** (`count=`, `each=`), **conditions** (`if`, `else`) and **overrides** (section 8);
  - **`raw` blocks** (section 19).
- **`render(items)`** prints the outline one construct a line, with its line number, for snapshots.
- **Nothing here judges** whether a construct is right: kinds, handles, values and units stay the checker's.

**Tests:**
- **One snapshot per spec example** (`every_spec_example_is_read_as_v0_constructs_and_prints_back`, 19 snapshots), the examples fenced `null (<task>)` included. Each example parses with no error and prints back byte for byte from its tree. The test fails if a construct the spec writes appears in no example.
- **The formatter** (`null_print`'s `every_spec_example_is_already_formatted`): every example comes back from `format` byte for byte, so the spec writes what `sau fmt` writes.
  - Three examples did not: section 4's pier comment and material link comment were aligned with extra spaces, and its action's keys had two spaces after `key 1`. The formatter keeps one space (section 15's layout), so I took the extra spaces out of the spec. That is the only spec change.
- **`raw` blocks** have no spec example yet (1d-05b specifies them), so v0's own tests read one. They also read a data-file reference, and links against a pattern's copy.

**Checks:** `cargo test --workspace` and clippy `-D warnings` are clean, and Null's lint-only `just check` passes.
