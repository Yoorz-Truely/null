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

In progress, November 7, 2026.

**What works.** A new crate, `compiler/null_value`, reads a word as a value of a dimension (none, length, angle) stored as `f32` or `f64`, and prints a stored value back:

- The exact written value is rounded once, to nearest even, into the stored type. A bare number, and a length in `m`, `cm`, `mm`, `km`, `in` or `ft`, becomes one exact decimal (a decimal shift, or a whole multiple by 254 or 3048) read by the standard library's correctly rounded parser. Degrees are multiplied by π/180 exactly: π is bracketed by Machin's formula in big integers and the bracket narrowed until both ends round alike.
- `print_bare` writes the shortest decimal that reads back to the same bits, in one canonical layout (plain from 10^-6 to 10^20, otherwise `1.5e-7`). `print_in` writes the shortest decimal in a unit that reads back exactly, or nothing, so `print` falls back to the bare internal unit, as the spec's conversion rule says.
- Wrong and unknown units are errors that say which units fit, with the nearest unit as a suggestion.

**Measured.**

- 17 tests pass, and a doctest (`cargo test -p null_value`, 256 cases per property). With 4,096 cases in release (as in `just verify`), they take 0.42 s; with 200,000 cases each, 18.7 s, all passing.
- The properties: every non-NaN `f32` and `f64` bit pattern prints and reads back bit-identical, bare, in every length unit, and in degrees (or radians where no degree decimal fits). Every decimal of up to 6 significant digits (`f32`) or 15 (`f64`) reads back exactly as written in `m`, `cm`, `mm`, `deg` and bare. Rounding a ratio matches the standard library on `m/10^k` down to `10^-400`. Degrees match an independent oracle (π to 120 decimals from Python's `decimal` module, decimal arithmetic) for `f32` and `f64`, including inputs of 400 to 3,000 digits.
- Examples: `90deg`, `2.7m` and `5mm` read back as written in `f32` and `f64`. `90deg`, `180deg`, `45deg`, `30deg`, `360deg` and `-60deg` give exactly Rust's `FRAC_PI_2`, `PI` and the rest. `2.7cm` is the `f32` nearest 0.027.

**Dependencies** (licences checked): `num-bigint` 0.4, `num-integer`, `num-traits` (MIT OR Apache-2.0); for tests, `proptest` 1 without default features, and what it pulls in (`rand`, `getrandom`, `libc`, `chacha20`, `bitflags`, `regex-syntax`, `unarray`, `core_detect`, `cfg-if`, `rand_xorshift`), all MIT OR Apache-2.0. `cargo deny` gathers all 23 crates, the test ones included.

**Choices to review.**

- The spec says a value with a unit is "converted in float64 and rounded once". Converting in `f64` rounds twice (the decimal, then the product), so the reader converts exactly and rounds once, which meets the "one rounding" the task and S21 ask for. The spec's wording could say "converted exactly".
- NaN reads and prints as `nan`, the quiet NaN; its sign and payload are not written, so NaN is the one bit pattern that does not round-trip. The spec does not mention NaN.
- Lengths are read at unit scale 1. The scene's unit scale (S21, "metres through the scene's unit scale") is for the checker, which knows the scene.
- Integers (DNA's int properties) are not part of this task and are not read yet.
