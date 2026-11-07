# agent/null-a

The Null worker's report, kept current with each commit. Working in Null's main checkout on `main` (Sau's decision S58). Tasks from Sau's board: 1a-01, 1a-04, then 1a-02 and 1a-03.

## 1a-01: share Scowr's front end by path

Done, November 7, 2026.

**What works.** Null's workspace depends on Scowr's `scowr_syntax` and `scowr_errors` by path (`../Scowr/compiler/…`, root `Cargo.toml`, `[workspace.dependencies]`). A new crate, `compiler/null_syntax`, is the only Null crate that names them: it re-exports the lossless tree, the typed view and the diagnostics types, and maps syntax diagnostics to Null's code `null-syntax`. The boundary (what is shared, what is Null's, how a change to Scowr is made) is in `AGENTS.md`, "The shared front end". No change to Scowr was needed.

**Measured.**

- `cargo test -p null_syntax`: 5 tests and 1 doctest pass. They check that all five `null` examples of the spec (section 4, read from `docs/spec/null-v0.md` at build time, so they stay checked) parse with no diagnostics and print back byte for byte; the tree of the pier example; that unit, colour, link, path and package words stay whole; that a broken line gives `null-syntax` diagnostics and the rest of the file still parses; and that Scowr's keyword attributes reach Null as keywords.
- Null's `just check` exits 0.
- Scowr's own tests at Scowr `4de7474`, run unchanged with the build guard as `RUSTC_WRAPPER` and a target folder outside Scowr: 192 passed, 0 failed, 49 suites. Scowr's `git status` is clean before and after.

**Found.**

- `cargo fmt --all` formats local path dependencies too: run in Null it listed every file of Scowr's workspace. Null's `just check` and `just fmt` now run `cargo fmt` (workspace members only). Cosmósis's justfile still runs `cargo fmt --all`; once Cosmósis depends on Null by path, `just fmt` there would rewrite Null's and Scowr's files. For the lead: Cosmósis needs the same change.
- The spec's example `modifier @bevel bevel { width 4mm segments 3 }` is one child node, `width`, with three arguments, since a node ends only at a line feed, `;` or `}`. The intended form is `{ width 4mm; segments 3 }` or `width=4mm segments=3` on the modifier's line. Left for the spec's owner and the checker (1a-06); the test records the current reading.
- Scowr reads `at`, `to` and `len` before a value as keyword attributes; Null's checker must treat them (noted in `AGENTS.md`).

**Unverified.** Nothing in 1a-01's "Done when" is unverified. `just verify` runs no fuzz target yet (1a-03 brings the first).

## 1a-04: exact numbers and units

Done, November 7, 2026 (code in `95f836b`, approved). The two choices below went to the user.

**What works.** A new crate, `compiler/null_value`, reads a word as a value of a dimension (none, length, angle) stored as `f32` or `f64`, and prints a stored value back:

- The exact written value is rounded once, to nearest even, into the stored type. A bare number, and a length in `m`, `cm`, `mm`, `km`, `in` or `ft`, becomes one exact decimal (a decimal shift, or a whole multiple by 254 or 3048) read by the standard library's correctly rounded parser. Degrees are multiplied by π/180 exactly: π is bracketed by Machin's formula in big integers and the bracket narrowed until both ends round alike.
- `print_bare` writes the shortest decimal that reads back to the same bits, in one canonical layout (plain from 10^-6 to 10^20, otherwise `1.5e-7`). `print_in` writes the shortest decimal in a unit that reads back exactly, or nothing, so `print` falls back to the bare internal unit, as the spec's conversion rule says.
- Wrong and unknown units are errors that say which units fit, with the nearest unit as a suggestion.

**Measured.**

- 17 tests pass, and a doctest (`cargo test -p null_value`, 256 cases per property). With 4,096 cases in release (as in `just verify`), they take 0.42 s; with 200,000 cases each, 18.7 s, all passing.
- The properties: every non-NaN `f32` and `f64` bit pattern prints and reads back bit-identical, bare, in every length unit, and in degrees (or radians where no degree decimal fits). Every decimal of up to 6 significant digits (`f32`) or 15 (`f64`) reads back exactly as written in `m`, `cm`, `mm`, `deg` and bare. Rounding a ratio matches the standard library on `m/10^k` down to `10^-400`. Degrees match an independent oracle (π to 120 decimals from Python's `decimal` module, decimal arithmetic) for `f32` and `f64`, including inputs of 400 to 3,000 digits.
- Exhaustively: all 4,278,190,082 non-NaN `f32` bit patterns print bare and read back bit-identical (`all_f32_round_trip_exhaustively`, ignored by default; `cargo test --release -p null_value -- --ignored`), 1,248 s on one thread at `nice 10`, thermal pressure nominal.
- Null's `just verify` passed after `95f836b`.
- Examples: `90deg`, `2.7m` and `5mm` read back as written in `f32` and `f64`. `90deg`, `180deg`, `45deg`, `30deg`, `360deg` and `-60deg` give exactly Rust's `FRAC_PI_2`, `PI` and the rest. `2.7cm` is the `f32` nearest 0.027.

**Dependencies** (licences checked): `num-bigint` 0.4, `num-integer`, `num-traits` (MIT OR Apache-2.0); for tests, `proptest` 1 without default features, and what it pulls in (`rand`, `getrandom`, `libc`, `chacha20`, `bitflags`, `regex-syntax`, `unarray`, `core_detect`, `cfg-if`, `rand_xorshift`), all MIT OR Apache-2.0. `cargo deny` gathers all 23 crates, the test ones included.

**Choices to review.**

- The spec says a value with a unit is "converted in float64 and rounded once". Converting in `f64` rounds twice (the decimal, then the product), so the reader converts exactly and rounds once, which meets the "one rounding" the task and S21 ask for. The spec's wording could say "converted exactly".
- NaN reads and prints as `nan`, the quiet NaN; its sign and payload are not written, so NaN is the one bit pattern that does not round-trip. The spec does not mention NaN.
- Lengths are read at unit scale 1. The scene's unit scale (S21, "metres through the scene's unit scale") is for the checker, which knows the scene.
- Integers (DNA's int properties) are not part of this task and are not read yet.

## Review

### 1a-01, by agent/cosmosis-a, November 7, 2026: approved

Reviewed Null `34cf222` (the task's only commit) against its row on Sau's board.

**How.** Read the row, this report and the whole diff (`AGENTS.md`, root `Cargo.toml`, `Cargo.lock`, `compiler/null_syntax`, `justfile`). Ran Null's `just verify` on an exact copy of `34cf222` (`git archive`, in a scratch folder with `../Scowr` linked to the live checkout), since Null's main checkout held 1a-04's uncommitted work at the time. Ran Scowr's own `cargo test --workspace --locked --no-fail-fast` in `~/Projects/Scowr`, unchanged, with the build guard as `RUSTC_WRAPPER` and a target folder outside Scowr.

**Measured.**

- Null's `just verify` on `34cf222` exits 0: `cargo fmt --check`, Clippy with warnings as errors, `null_syntax`'s 5 tests and 1 doctest in debug and release, `cargo deny` (licences, bans, sources), and the fuzz runner (no targets yet).
- Null builds against Scowr's current tip, `770a472`, not only `4de7474`: the copy above built through the link to the live checkout.
- Scowr's own tests at its tip `770a472`: 192 passed, 0 failed, 49 suites. One suite (`scowr_check --test corpus`) failed in a first run started about 12:00, while another agent's Git operation was rewriting Scowr's `examples/fx-kinds.scowr` (its mtime is 12:00:23, after Scowr's last commit at 11:02; `git status` clean); it passed in the full run and twice more alone. Scowr's `git status` stayed clean before, during and after.
- `cargo fmt` without `--all` still covers Null's members and none of Scowr's: in the copy, a misformatted line in `null_syntax` made `cargo fmt --check` exit 1; `cargo fmt --check -v` named no Scowr file, `--all` named 40.
- The boundary is in `AGENTS.md`, "The shared front end": what is shared, what is Null's, that only `null_syntax` names `scowr_*`, that Scowr changes only with the user, and the two known syntax edges.

**"Done when"** holds: Null builds against Scowr's front-end crates by path, Scowr's own tests pass unchanged, and the boundary is written in `AGENTS.md`.

**Findings, none blocking.**

- The Scowr count the report quotes (192 at `4de7474`) could not be repeated at `4de7474` from an archive: Scowr's tests find their examples through `CARGO_MANIFEST_DIR`, and that run read the live checkout's newer examples (5 failures, all `legacy-param`, added after `4de7474`). It was repeated at the tip instead, with the same count.
- Scowr is being changed by other agents while Null builds against it by path, so a Null build or Scowr test run can catch Scowr mid-change, as above. Worth knowing before treating a one-off failure there as Null's.
- Release builds print `stripping debug info with rust-objcopy failed` (the toolchain's `rust-objcopy` cannot load `libLLVM.dylib`). A machine problem, not this task's; harmless to the results, but noise in every `verify`.
- `every_spec_example_parses_clean_and_prints_back` asserts exactly five examples, so adding one to the spec fails it. That is a reasonable tripwire; the message says why.
- The spec's own example `modifier @bevel bevel { width 4mm segments 3 }` does not mean what it says under the shared syntax (already recorded on 1a-06). It should be corrected in the spec itself, not only in the checker; for the spec's owner or 1a-17.

### 1a-04, by agent/cosmosis-a, November 7, 2026: approved

Reviewed Null `95f836b` (code) and `65c6ea7` (report) against the row on Sau's board.

**How.** Read the row, this report and the whole of `compiler/null_value` (`exact.rs`, `number.rs`, `unit.rs`, `tests.rs`). Ran Null's `just verify` on an exact copy of `65c6ea7` (`git archive`, `../Scowr` linked beside it). Checked the reader against an oracle of my own in Python: π from the Chudnovsky series in integers (neither Machin's formula nor the `decimal` recipe the tests' oracle uses), every conversion done exactly in fractions, and a hand-written half-to-even rounding into `f32` and `f64`, itself checked on π, 0.1, the least subnormal and a tie to zero. A probe crate outside Null's workspace fed it the cases.

**Measured.**

- Null's `just verify` on `65c6ea7` exits 0: `null_value`'s 17 tests (1 ignored, the exhaustive one) at Clippy, debug and 4,096 cases in release; `cargo deny`. The property tests at 100,000 cases each, in release: all pass, in 10.7 s. The 21-minute exhaustive test was not rerun.
- 9,144 oracle cases, all matching bit for bit, and every result printed and read back bit-identical: 6,000 random words (1 to 30 significant digits, exponents -30 to 25, bare and in every unit, `f32` and `f64`), 3,000 degree values written to 25 or 45 digits so that their radians fall within a hair of a rounding midpoint (the hardest case for the π bracket), and the fixed examples. `90deg`, `2.7m` and `5mm` print back as written in `f32` and `f64`.
- Read by hand, with no fault found: the π bounds are strict (each floored term errs by less than 1, so the error bound of 2(n + 1) per arctangent is generous); the degree bracket is monotonic and ends because π is irrational, the digits a precision cannot use being cut with the bracket widened; `round_ratio` finds its exponent in one step and rounds half to even, through subnormals and to infinity; `from_parts` is exact; lengths are exact decimal shifts and multiples of 254 and 3048, read once by the standard library's correctly rounded parser.
- Unit errors: a unit of the wrong dimension names the units that fit (`5deg` as a length, `2rad` as a length, `5m` where no unit goes); an unknown one suggests the nearest unit of the right dimension, within two edits (`5mmm` gives `mm`, `5dg` and `5Deg` give `deg`, `5meter` gives none); `deg`, `-` and `-nan` are not numbers.

**On the spec's "converted in float64 and rounded once".** The exact reading is the right one for S21 and S22 ("one exactly defined rule", exact to the stored bits), but it is a change to what the spec says and the spec should say it: "converted exactly and rounded once". Measured with the oracle on 20,000 degree values of 1 to 6 digits: for `f32` storage, converting in `f64` first (`x × (π/180)` in `f64`, as Blender does) and rounding into `f32` gives the same bits as the exact conversion every time; for `f64` storage it differs in 4,304 of 20,000 (21.5 %). So the slice's `f32` angles are unaffected, and only `f64` properties see the difference.

**Findings, none blocking.**

- The shortest print in a unit by search (`deg`, `in`, `ft`) can miss a decimal that exists, and then falls back to bare metres or radians: a value the user wrote in that unit is reprinted without it, which S21 ("Sau writes a value back in the unit its line already uses") would rather keep. Measured over 3,000 random values per row: never for `f32` (5 to 9 digits); for `f64`, only at 16 and 17 significant digits: degrees 0 and 6, feet 1 and 110, inches 2 and 75 (for example `-18750715569787871e-5ft`, a 17-digit value that reads back exactly yet prints as `-57152181056.71343`). The cause is the candidate window, ±1 in the last digit around an `f64` estimate. A value printed this way always reads back exactly, so nothing is lost. For 1a-07 (the minimal-edit writer): divide exactly for `in` and `ft` (by 254 and 3048), and widen the window for `deg`.
- `5MM` suggests `m`, not `mm`; `5mn` suggests `m` where `mm` is as near. Ties in edit distance go to the first unit listed. Preferring a case-insensitive match first would help.
- NaN payloads and the scene's unit scale are out of scope, as the report says; the spec should mention NaN when it is next edited.

### 1a-02, by agent/cosmosis-a, November 7, 2026: approved

Reviewed Null `0edf8f4` against the row on Sau's board.

**How.** Read the row, this report and the whole diff (`names.rs`, `lib.rs`, `tests/slice.rs`, the seven slice files and the eight snapshots). Ran Null's `just verify` on an exact copy of `0edf8f4` (`git archive`, `../Scowr` linked beside it). A probe crate outside Null's workspace checked printing independently, and the slice files were mutated in the copy to see the tests catch it.

**Measured.**

- Null's `just verify` on `0edf8f4` exits 0: `null_syntax`'s 5 unit tests, 6 slice tests and doctest, `null_value`'s 17, in debug and release; Clippy; `cargo deny`.
- Lossless, independently: the 7 slice files and the 37 `.noll` files of Cosmósis's six corpus drafts (`corpus/slice/`, written without reference to these files) each print back byte for byte, the lexer's tokens and the tree's tokens each cover every byte, and none has a diagnostic. 20,000 random mutations of those 44 files (1 to 6 insertions, deletions or replacements from braces, brackets, quotes, `#"`, `\`, comment openers and closers, CR, CRLF, tab, NUL, BOM, U+2028, multi-byte characters), most of them broken: all lossless by the same three checks, no panic.
- The tests catch changes: each of five mutations of `object.noll` alone, from the pristine file, fails `every_slice_file_lexes_parses_and_prints_back`: one digit (`0.35m` to `0.36m`), a trailing space, a CR, a doubled space, and `location` capitalised (also a `null-name` error). Restored, the suite passes, and no `.snap.new` is left behind.
- Coverage: the slice files write every phase 1a construct as the spec writes them: `project`, `scene` (and `collection`), `object` with `location`, `rotation-euler`, `scale`, `parent` and `data`, `mesh` as a `primitive` and as inline `vertices` and `faces`, `material` with a `principled-bsdf`'s `base-color` and `roughness` (as children and as properties, with links and a `layout`), `camera` and `light`; `lexical.noll` adds every lexical form of section 5. `the_slice_covers_its_constructs` fails if one goes missing.
- `null-name` fits section 18: each message names the file, line and column, says what was expected ("a node name is lower-case words joined by `-`"), and offers the kebab-case spelling as a fix; the code is stable, and section 18 lists codes "such as", so a new one is in keeping. The fixes `frameStart` to `frame-start` and `use2D` to `use2d` follow S16's mapping, and applying them gives a clean file (tested).

**"Done when"** holds: every slice construct lexes and parses to a tree that prints back byte for byte, with snapshot tests, and kebab-case names are enforced.

**Findings, none blocking.**

- Handles are checked only where a handle names a node (`layout`'s `@Tex [0 0]`). In arguments they are not: `object @Pier`, `node @Tex principled-bsdf`, `data mesh @Pier_Deck` and `material @weathered_oak` give no diagnostic, and neither does an upper-case enum value (`rotation-mode=XYZ`, which S16 writes lower-case). The report leaves these to the checker, which is reasonable, but the same handle `@Tex` is then an error in one place and not in the other. For 1a-06: check handle definitions and references, and enum values, under `null-name`.
- A `null-name` fix is spelling only, not schema-aware: `use2D` becomes `use2d`, which is kebab-case but need not be an RNA name. Once 1a-06 has the schema, the nearest RNA name should be the fix, so that a tool applying fixes does not get two different ones in a row.
- Scowr's syntax messages reach Null with Scowr's examples: `frame-start=1` written as a node gives `null-syntax: expected a key before =, such as vel=96`, a sound example. For 1a-03 (the parser's error recovery): give Null's diagnostics Null's examples.

### 1a-03, by agent/cosmosis-a, November 7, 2026: sent back

Reviewed Null `5cef529`, `a5262c3` and `900a7e1` against the row on Sau's board, and `59061db`, the spec edit, against S59. Exact copies of Null at `59061db` (`git archive`, `../Scowr` linked beside it) in my scratchpad; Null itself untouched but this section.

**Measured.**

- The fuzz target, rerun from the committed seeds at `nice 10`, thermal pressure heavy: 301 s, 288,898 inputs, coverage 3,284, peak 915 MB, no crash. libFuzzer's input limit stayed at 583 bytes, the largest seed, as in the author's run.
- **A second run, from that corpus with `-max_len=16384`, crashed after 13,547 inputs** (`invariants.rs:46`, "Scowr's example in expected a value after level="). The input is 97 bytes, under the 583-byte cap, so the default run can find it too. It is not a parser fault but a false alarm in the invariant: it bans the bare substring `vel=` from every message, and the shared parser echoes the user's key (`expected a value after level=`). Minimal reproductions through `invariants::check`: `modifier @sub subsurf level=` and `object @a travel=` panic; `object @a speed=` passes. As it stands, `just verify`'s fuzz run and the nightly can fail at random on a key that ends in `vel` (`level`, `travel`, `bevel`'s neighbours).
- A wholly new Scowr message is caught only when some test input produces it. Removing each of the 22 `KNOWN` entries in turn (standing in for a message Null has not read) fails Null's tests for 20; for `blocks are nested too deeply` and `lists are nested too deeply` every test passes, since no test nests past Scowr's limit of 256. Deep nesting itself is safe: up to 1,000,000 levels of `[`, `{` and `x {`, `invariants::check` holds, edits included, with no stack overflow and only known messages (about 257 or 513 diagnostics, then no more).
- Null's `diagnostics` is built as Scowr's own `Parse::diagnostics` is (message, span, fix), with Null's wording: nothing of Scowr's is lost.
- `59061db` matches S59: section 6 says "converted exactly and rounded once" and how NaN is written, section 4's bevel is `{ width 4mm; segments 3 }`, the CHANGELOG says so, and the test now expects two attributes. S59's items 3 to 5 are Cosmósis's and match its spec, section 4.

**To fix.**

1. The invariant's check for Scowr's examples: match the whole example phrases (`such as vel=96`, `such as track {`, `\u{266A}`), not `vel=`, `track {` and `266A` anywhere; add the crash input, or `modifier @sub subsurf level=`, as a test and a seed.
2. A deep-nesting case in `tests/recovery.rs` (more than 256 levels of `{` and of `[`), so both depth messages are pinned like the other 20, and the depth guard against stack overflow is tested in Null.
3. `tools/fuzz.sh`: pass a `-max_len` larger than the seeds (8 KB or so), so the fuzzer tries inputs the size of real files.

The pending `cargo deny` commit for the fuzz crate (NCSA) was not yet on the row when this was written; I will check it with the fixes.

### 0-03, the agent environment (the lead's), re-reviewed by agent/null-a, November 7, 2026: sent back

Re-review after agent/cosmosis-a's send-back (Cosmósis `e73d82e`), of Sau `01086e7`, `45616a1` and `f32ebbd` and the hand-run nightly at `45616a1`. The board was tested only on a throwaway `git clone --shared` of Sau's `main` (`c78357d`, sparse: `docs/`, `sau/`, `justfile`), never on the real board.

**Measured.**

- **Notes append (`01086e7`): yes.** On task 0-04 in the clone: claim, note, review (`--commit aaa111`), approve by another agent, note: the row reads `Ask the user first; first evidence; in review at aaa111; approved by agent/y; a note after approval`. Nothing was erased, and the approval survived the later note.
- **`review` drops old approvals: yes.** A second `review --commit bbb222` gave `...; a note after approval; in review at bbb222`, with `approved by agent/y` gone and every other note kept. Self-approval is still refused.
- **The board lock (`f32ebbd`): yes.** With a second process holding `sau-tasks.lock` for 5 s, `dev.py task note` printed "task: waiting for the board…", waited 4 s, then committed alone, leaving nothing staged. The lock is taken once per process, so `integrate`'s own board commits do not deadlock on it (read in the code).
- **Worktrees link Scowr (`45616a1`): yes, through the nightly run.** `work/nightly/Scowr` links to `~/Projects/Scowr`, and the nightly's Null verify compiled `scowr_errors` and `scowr_syntax` through that link.
- **The nightly at `45616a1`: passed.** `work/nightly/latest.json` reads `{"result": "passed", "commit": "45616a1", "seconds": 1903, "finished": "2026-11-07 13:36", "skipped": "blender-tests: no developer build in .build/developer (task V-01)"}`. Its log ends `nightly: exit 0`, with `verify: passed` for Null (its 60 s fuzz run included), Cosmósis and Sau.

**To fix** (both small, both in `sau/tools/dev.py`, both about approvals the earlier fix set out to protect):

1. **`integrate` counts any mention of "approved by" as an approval.** Its test is `"approved by" not in r["notes"]`, a substring anywhere in the notes, while `review` drops only the notes that start with `approved by `. In the clone, after the second `review`, a note "earlier this was approved by agent/y, then fixed" made the row pass `integrate`'s test, with no fresh approval in this round. `integrate` should look for a note that starts with `approved by ` and names an agent other than the task's own, as `approve` does.
2. **`done` replaces the notes and checks no approval.** `done --commit ccc333` turned the row's seven notes, approval included, into `ccc333`. That erases the author's evidence, which `workflow.md` now says never happens ("Notes add to the row, never replace it"). And `done` is how every Null and Cosmósis task ends under S58. It should append `done at <commit>` and refuse a task that is not in `review` with an approval from someone other than its author, as `integrate` refuses one.

**Also found, not blocking.** A note containing `|` is stored as is: the row is still found and later notes still append (tested), but the Markdown table gains a cell, so the row renders wrongly. Escape it as `\|`, or refuse it.

## 1a-02: lexer and lossless syntax tree for the slice, kebab-case names

Done, November 7, 2026 (`0edf8f4`, approved).

**What works.**

- Seven slice files in `compiler/null_syntax/tests/slice/` cover every phase 1a construct: `project`, `scene` and `collection`; `object` with `location`, `rotation-euler`, `scale`, `parent` and `data`; `mesh` as a `primitive` and as inline `vertices` and `faces` lists spanning lines; `material` with a `principled-bsdf`'s `base-color` and `roughness`, links and a `layout` block; `camera` and `light`. A `lexical.noll` adds every lexical form: `;`, `/-` on a node and on a block, `\` continuation, nested block comments, escaped strings, raw strings, bare and `on`/`off` booleans, scientific notation, attributes as properties and as child nodes.
- `null_syntax` now re-exports the lexer (`lex`) and has `dump`, an indented tree with each token's kind, span and text, for snapshots.
- Kebab-case (S16) is enforced: `names::check` reports every node name and property key that is not lower-case kebab-case, code `null-name`, with the kebab-case spelling as a fix (`rotation_euler`, `frameStart`, `Rotation_Mode`, `"base color"`, `@Tex` in a layout). A node may be named by a kebab-case handle, as `layout` entries are. Strings are never names in Null. Handles in arguments, and enum values, are left to the checker. `diagnostics` returns syntax and name errors together, in text order.

**Measured.** `cargo test -p null_syntax`: 11 tests and a doctest pass. For each slice file: the lexer finds no errors and its tokens cover the text; the parse has no diagnostics; the tree's tokens are exactly the lexer's (kind and length); the text prints back byte for byte; the tree is an `insta` snapshot (`tests/snapshots/`, 7 trees, 960 lines). The kebab-case messages are a snapshot too, and applying their fixes gives a file with no diagnostics. Null's `just check` exits 0.

**Dependency** (licence checked): `insta` 1 for tests, without default features (Apache-2.0), with `similar` (Apache-2.0), `once_cell`, `tempfile`, `fastrand`, `rustix`, `errno`, `linux-raw-sys`, `r-efi` and `windows-sys` (each offering MIT or Apache-2.0).

**Choices to review.** `null-name` is a new diagnostic code (spec section 18 lists codes "such as"). `to_kebab_case` splits a capital after a lower-case letter (`frameStart`), not after a digit (`use2D` becomes `use2d`).

## 1a-03: parser with error recovery for the slice

Sent back once and fixed, November 7, 2026 (`5cef529`, `a5262c3`, then the fixes below).

**Fixes after the review** (agent/cosmosis-a's findings, in "Review" below):

1. The invariant's check for Scowr's examples now matches the three whole phrases (`messages::SCOWR_EXAMPLES`: `such as vel=96`, `such as track {`, `\u{266A}`), not fragments, so a user's key such as `level=` or `travel=` is no longer a false alarm. `a_users_key_is_never_taken_for_scowrs_example` covers `modifier @sub subsurf level=` and `object @a travel=`; the reviewer's 97-byte crash input and a `level=`/`travel=` file are fuzz seeds.
2. `nesting_past_the_limit_is_an_error_not_a_crash`: 300 levels of `x {`, of `[` and of bare `{`, past Scowr's limit of 256, give "blocks are nested too deeply" and "lists are nested too deeply", lossless, with every invariant holding; so all 22 known messages are now reached by a test.
3. `tools/fuzz.sh` passes `-max_len=8192`, so the fuzzer tries inputs the size of real files.

**What works.**

- Recovery is the shared parser's (Scowr's `scowr_syntax`, unchanged); Null tests it on the slice. Twelve kinds of broken slice text (unclosed block, list and string; a stray `}` and `]`; reserved `(` `)`; a missing value after `=`; a missing key; a property written as a node, `frame-start=1`; a block with no name; text after a block; bad escapes) each give the whole tree, every byte kept, and diagnostics, and the node after the break still parses (`tests/recovery.rs`; the messages are a snapshot).
- Null's own wording (`src/messages.rs`), Scowr untouched: the three shared messages that carried Scowr's examples are reworded (`such as object { ... }`, `such as frame-start=1`, `such as \u{e9}`), and `frame-start=1` on a line of its own gets its own message and a fix (`=` to a space). `KNOWN` lists the 22 messages of the shared syntax as read at Scowr `770a472`; the invariants fail on any other, so a new Scowr message is read before Null shows it.
- `src/invariants.rs`: the checks for any text, shared by the tests and the fuzz target: lossless print, contiguous tokens, every span and fix on character boundaries in the text, one `null-syntax` diagnostic per syntax error, no Scowr example in any message, rendering and dumping without panic, and an incremental edit reparsing to exactly a fresh parse (tree and errors).
- The first fuzz target, `fuzz/fuzz_targets/parse.rs`: libFuzzer's input as text (bad UTF-8 replaced) and an edit chosen by its first three bytes, through the invariants. Its own workspace in `fuzz/`, built by cargo-fuzz 0.13.2 on nightly (rustc 1.101.0-nightly, 2026-11-06), both installed today as `testing.md` says. `tools/fuzz.sh` seeds it from `fuzz/seeds/parse` (the slice files); the growing corpus and any crash files are ignored.

**Measured.**

- The fuzz target ran 601 s (`tools/fuzz.sh 600`) without a crash: 667,044 inputs, coverage 3,396 edges, corpus 2,471 inputs, peak memory 814 MB, no artifact. Thermal pressure was nominal at the start and moderate at the end (load 24.8 on 18 cores from other work). A first 21-second run: 61,006 inputs, no crash.
- 3,000 seeded mutations of the slice files (1 to 4 insertions, deletions or replacements of braces, brackets, quotes, `#"`, `\`, comment marks, `/-`, `;`, `=`, line breaks, CRLF, tab, `é`, U+2028), most of them broken: every invariant holds.
- `null_syntax`: 16 tests and a doctest. Null's `just check` exits 0; `just verify` passes, its 60 s fuzz run included (66,665 inputs, no crash).

**Dependency** of the fuzz crate only (licence checked): `libfuzzer-sys` 0.4, (MIT OR Apache-2.0) AND NCSA (LLVM's libFuzzer, a permissive BSD-style licence), with `arbitrary`, `cc`, `jobserver`, `shlex`, `find-msvc-tools` and `libc` (MIT OR Apache-2.0). The fuzz crate is outside Null's workspace, so `just check` now runs `cargo deny` on `fuzz/Cargo.toml` too, with the same `deny.toml`, which allows NCSA for libfuzzer-sys. Checked both ways: with NCSA removed from the list the fuzz crate is rejected; with it, all 14 of its crates pass.

**Unverified.** Recovery is the shared parser's own: Null tests it and does not change it. Memory grows with the corpus (814 MB after 10 minutes, under libFuzzer's 2 GB limit); longer runs may need `-rss_limit_mb`.

## Next

Whatever the lead routes next. The S59 spec corrections are committed (see the commit after `900a7e1`).
