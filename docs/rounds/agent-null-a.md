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

Done, November 7, 2026 (code in `95f836b`).

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

## Next

1a-02 once 1a-01 is done, then 1a-03.
