# agent/null-1a17

The report of agent/null-a working as `agent/null-1a17` on Null's row 1a-17, in `work/wt/null-1a17/` with Null on branch `agent/null-1a17`, cut from `main` after batch 1 (`db14c8e`). Sau, Cosmósis and Scowr are links to their main checkouts. November 8, 2026. Spec text only; lints only.

## 1a-17: what the slice taught, in Null's spec

**What changed in `docs/spec/null-v0.md`.**

- **Section 22, new: "What the slice taught".** A table of every change phase 1a's slice forced on the draft. Each row gives the change, the section that holds its rule, why it was forced, and its task or decision. There are 20 rows:
  - exact conversion and NaN (S59), and a node ending only at a line feed, `;` or `}`;
  - Scowr's `at`, `to` and `len`, and `#rrggbb` from a fixed table (S62);
  - `polygons`, `energy` in W and `lens` in mm, and Null's own forms over read-only RNA;
  - a primitive's proportions from its operator's `scale`, the material output node, and defaults as a new datablock's;
  - the formatter's layout and write-back's per-value smallest edit;
  - apply by handle, live reload, and Sau's own writes known by their text;
  - write-back onto the text Sau read, undo and redo restoring bytes, and a project with errors opening as nothing.

  A last line records what the slice confirmed without a change: 46 of the corpus's 87 transform attributes reprint with no edit, and the 41 left out hold their defaults (1a-16).
- **Section 16, live reload, now says what the slice does** (1a-10, 1a-10b, 1a-12, 1a-13, 1a-13a):
  - the watcher's thread loads the project and the newest save wins (S39);
  - a broken save keeps the last good state and pushes no step;
  - apply finds by handle, writes only what differs, removes what the files dropped, and is one undo step (S41);
  - it waits as long as each apply took, and the latency was measured (12.6 ms median, 14.1 ms worst, against a 50 ms target);
  - Sau's own writes are known by their text, not "by their content hash" as the draft said;
  - opening a project with errors changes nothing.

  The slice applies on save; VS Code's unsaved buffer comes with the language server.
- **Section 17, write-back** (1a-14, 1a-14b, 1a-16):
  - the slice writes back `location`, `rotation-euler` and `scale`, one line per push;
  - a file is written atomically, and only while it holds the text Sau read;
  - a file changed outside is left alone and live reload applies it. The draft's three-way merge, where Sau asks, becomes S40's merge by value, later. Until then Sau's unwritten value is lost to the file's, as the 1a-13 report accepts;
  - undo and redo restore a file byte for byte.
- **Section 5** says how Scowr's keyword attributes reach Null: two arguments (1a-01).
- **Section 10's node kind** now follows section 8's rule (S67) rather than "the RNA identifier in kebab-case".
- **The draft's first line** no longer says "Nothing is built yet", and section 21 points to section 22.
- **`CHANGELOG.md`** has one entry that lists every change the slice forced.

**Where each fact comes from.** Sau's reports for 1a-10 to 1a-16 (`agent-sau-i`, `-j`, `-k`, `-l`, `-m` and `-n`), Sau's decision register (S16, S21, S35, S39 to S41, S59, S62), Null's `CHANGELOG.md`, Sau's board and the row's own note. Null's checker gave the keyword-attribute rule (`null_check::check::args_of`), and its test gave the keywords (`null_syntax` `scowrs_keyword_attributes_reach_null_as_keywords`).

**Left out on purpose.** Phase 1c's changes (units in full, integers past 32 bits, Sau's exact fields under S64) and 1b's (the project schema, S67's kinds, S70) are not the slice's. They are in their own sections and changelog entries already.

**Checked.** Null's `just check` passes (lint-only). There is no code change.

## Review

## Fixes after review (`33f6f4a`)

- **Section 17.** Edit mode and sculpt mode writing their data on leaving, and after a pause, is now marked as later: phase 3 of Sau's roadmap, two-way editing with VS Code. The slice writes back transforms only.
- **Section 22, row 1.** The 21.5% is now said to be of short degree values read into a float64. It never happens for the slice's float32 rotations, and S59 holds every type to the exact rule all the same.
