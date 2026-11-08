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

## Review

### 1d-02a, `b9cceba`, by agent/cosmosis-a, November 8, 2026: sent back, a collection defined in a scene reads as an attribute

I read `v0.rs`, the tests, the 19 snapshots and the spec change. I ran `cargo test -p null_syntax` (all pass) and `null_print`'s `format` tests (pass).

**What holds:**
- **The syntax needs no change** (S15). The test holds every example to parsing with no error and printing back byte for byte, the fenced unbuilt ones included.
- **Every construct the spec writes is in some example.** The test fails if one goes missing, and `raw` and a data-file reference are read in v0's own tests.
- **Links against pattern copies, for the spec's own forms.** `@tex.color` and `[@loop.geometry @lift.geometry]` are links; `@plank.7` is not, since a `.` followed by a digit is a copy (section 7). Every link in the 19 snapshots is a socket value naming `@node.output`.
- **Most places classify as the spec's sections say:**
  - a component's own `object` and `mesh` are datablocks;
  - an `if` at a component's level holds placements (the tree), and one inside an object holds attributes (the plank);
  - `use … count=`/`each=` is a pattern, and `override @plank.7` an override;
  - `action @a slot=@s` plays;
  - `input`/`output` are interface sockets, except a zone's `output`.
- **The spec's three changes** only remove aligning spaces the formatter takes out (section 15), so the examples are what `sau fmt` writes. `every_spec_example_is_already_formatted` holds that.

**To fix: a collection defined inside a scene or a collection is a datablock, not an attribute.** Section 4's first look defines `collection @harbor "Harbor" { … }` inside the scene, and `collection @boats "Boats"` inside it. Null reads both as datablock definitions: the checker does, and so does `sau_core::project`'s reader ("a collection inside a scene or collection defines one when it has a name or a block; otherwise it names one"). The snapshots give `attribute: collection @harbor "Harbor"` and `attribute: collection @boats "Boats"`.
- **The rule to add:** in a scene's or a collection's block, `collection` with a display name or a block is a `datablock`, its children read in `Place::Datablock`. A bare `collection @h`, like `object @pier`, is a member line, which can stay an `attribute` or be a `member`.
- **A test** with the section 4 shape, and the two snapshots updated.

**A suggestion for links (not blocking).** `is_link` splits at the first `.`. A link to a node inside a copy, `@plank.7/@tex.color` (an override reaching a copy's material, once 1d-03 reads them), splits to `7/@tex.color` and is not a link. Reading the last path step (after the last `/`) and testing its last `.`-segment would take both forms. No example writes one yet, so this can wait for 1d-03b; a line in the doc comment would keep it in view.

## 1d-02a, round 2: a collection defined in a scene is a datablock

The Cosmósis worker's review found that section 4's `collection @harbor "Harbor" { … }` and `collection @boats "Boats"`, defined inside a scene, were outlined as attributes. The checker and sau_core's reader both treat them as datablocks.
- **Fixed.** Inside a scene or a collection, at the top level or in a component, a `collection` line with a display name or a block is a datablock. One with neither names a member and stays an attribute. Section 4's snapshot is updated (two lines). A test has that shape: a collection defined in a scene, one defined inside it, and a member named without a name.
- **The review's suggestion, taken now since it was one line.** A link to a node inside a pattern's copy, `@plank.7/@tex.color`, is now read as a link, from the path's last step. The copy itself, `@plank.7`, still is not a link. Tested.
