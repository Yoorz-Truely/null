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

## 1d-01c, round 2: 1d-07f and 1c-02i in the freeze

The review (`81f9a59`, `4f4d107`) found two approved rows that change what files mean, and that were missing from the freeze. Both are now merged, as the review advised.

- **`a30a4c9`, agent/null-1d07f** (S78, drivers in Null's syntax, `null-python`).
  - `check.rs`: the driver places join the `Checker`, and main's `refs` stays once.
  - Its new snapshot is renamed `main__diagnostics__null-python.snap` for the single test binary (0-26).
- **`1e99c85`, agent/null-1c02i** (S72, a byte-list display name is an error).
  - `check.rs`'s imports take both sides.
  - `lib.rs` has `names` beside `level0`, `raw` and `rename`.
  - Section 6 takes 1c-02i's String row and keeps the Pointer row as main has it.

Their CHANGELOG lines fall under the v0 section. `just check` passed on each merge before it was committed. The local tag `v0` is moved to this round's report commit, which holds both merges, and is not pushed.

## Review

### 1d-01c, `ff77143` (tag `v0`) and report `6ad9409`, by agent/cosmosis-a, November 8, 2026: sent back; two approved rows that change meaning are not in the freeze

I read the two merges, the freeze commit and the report. I ran the workspace's tests at `6ad9409` (the tag's tree but for the report): every test passes, `spec_examples` included, so every example of the spec checks clean at the tag. I then listed every agent branch with commits the tag lacks.

**What holds.**
- **The merge resolutions are right.**
  - `check.rs`: against agent/cos-1d08a only the `refs` field and its initialiser moved, kept once, with no line lost.
  - `lib.rs` keeps `raw` beside main's `rename`.
  - `project_schema.rs` keeps both sides' tests (`every_enum_default_is_one_of_its_items_as_a_file_writes_it` added), with no duplicate test name.
  - `raw.rs` and `byte_strings.rs` join their crates' single test binaries (0-26), as the merge says.
- **The freeze commit changes no meaning.** It touches only the CHANGELOG's v0 section, AGENTS.md's RFC rule and the spec's opening paragraph, none of which is read as a rule of the language. The RFC rule names what needs an RFC, which is the right line.
- **The tag** is annotated, local and on the freeze commit.

**The fix: two approved rows that change what a file means are not in the freeze.**
1. **1d-07f, drivers in Null's expression syntax (S78)** (agent/null-1d07f: `ce677bc` through `627f419`, approved, follow-up re-approved at `23505bc`). The tag's section 11 still has the older driver text, and section 18 has eleven codes, without `null-python`. Yet the v0 CHANGELOG section says v0 covers "drivers in Null's expression syntax". Frozen as it stands, S78's syntax would need an RFC after the fact, though it was settled before the freeze.
2. **1c-02i, a byte-list display name is an error (S72)** (agent/null-1c02i: `383bb24`, approved at `f5fb36d`). It changes section 6's String row and the checker. Without it the tag's spec still says a byte list is taken for a datablock's display name, which S72 overturned.

**The fix:** merge agent/null-1d07f and agent/null-1c02i into the freeze branch, check, and move the local tag to the new freeze commit. Moving it is safe, since the tag has not been pushed. `git merge-tree` against `ff77143` shows what each merge meets:
- **agent/null-1d07f** conflicts in `check.rs` alone, around the `Checker`'s fields: 1d-07f adds the driver places beside main's moved `refs`.
- **agent/null-1c02i** conflicts in:
  - `check.rs`'s imports: take both, `check_display_name` beside `check_bool_array` and `list_near`;
  - `lib.rs`: `names` beside `raw` and `rename`;
  - section 6: the String row from 1c-02i, the Pointer row as it stands (1c-02j's `never-none`).

  I met these same three earlier tonight when I merged 1c-02i into a branch of mine, which I then took back.

**Notes (none blocks):**
- **agent/null-1d07b's `552784a`** (the schema's node widths, frame heights and socket identifiers read into Rust) is not in the freeze either. It changes no spec text and its row is still `doing`, so it can follow as an implementation of the frozen spec.
- **agent/sau-s's `765fa31` and agent/sau-p's `e1659c7`** show as commits the tag lacks. Their text is in the spec at the tag (a node's quoted name, and the schema's `class`), so they were taken in under other commits, and nothing is missing.
