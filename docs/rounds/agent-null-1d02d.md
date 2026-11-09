# agent/null-1d02d

The report of agent/null-a working as `agent/null-1d02d` in `work/wt/null-1d02d/`, on branch `agent/null-1d02d` in Null, stacked on `agent/null-1d03d` (with its approval, `ced7e9d`). November 8, 2026. Null's tests and the fuzz target's own runs.

## 1d-02d: the parser and formatter fuzzed

**The target, `fuzz/fuzz_targets/format.rs`.** It takes any bytes, read as UTF-8 with bad bytes replaced, and checks the following. Any broken property panics, which libFuzzer reports as a crash.
- Parsing, the syntax's diagnostics (`null_syntax::diagnostics`, with its kebab-case names) and the v0 outline and its render never panic.
- **The layout** (`null_print::format`):
  - It is idempotent.
  - It keeps every token. Only whitespace and line feeds may change, plus the trailing whitespace of a comment's lines, of a string never closed, and of a comment that runs to the end of the file.
  - It does not change whether the text parses.
  - On text that parses, it keeps the v0 outline: every construct, its words as tokens, and its depth. Line numbers are left out, since blank lines move them.
- **On text that parses clean:**
  - `sau fmt` (`without_defaults(canonical(..))`) gives text that parses clean and formats to itself.
  - The short forms' `expand` does the same.

The schema is the project schema, read once.

**Seeds and corpus, under 0-24's rules.**
- The committed seeds, `fuzz/seeds/format/` (34 files, 136 KB), are:
  - the spec's 20 examples;
  - the `parse` target's nine seeds, without their three edit bytes;
  - the five inputs the runs found, each named `regression-…`.
- The corpus grows in `fuzz/corpus/format`, which is ignored. Sau's `just clean` (0-24) minimises it with `cargo fuzz cmin` from the built target at `fuzz/target/*/release/format`. That is where `cargo fuzz build` puts it, so `clean` needs no build of its own.
- `tools/fuzz.sh`, which `just verify` runs, already runs every target in `fuzz_targets/`, so `format` runs there for its 60 seconds.

**What the runs found, each fixed with a test:**
1. **`null_print::expand` wrote a quoted node name unquoted** (`"("` became `(`, which does not parse). This is 1d-03d's code. The name is now written as written. Test: `short.rs`'s `a_quoted_node_name_stays_quoted`.
2. **`sau fmt` took two passes to settle** on a line holding only a vertical tab. `canonical`'s `no_blank_lines` took only spaces, tabs and CRs as a blank line's whitespace, while the lexer and the layout take any Unicode whitespace. So the line survived the first pass's blank-line rule, became blank in the layout, and went in the second pass. It now takes any whitespace. Test: `canonical.rs`'s `a_line_of_other_whitespace_is_a_blank_line`.
3. **The layout made a `\` word a continuation.** The lexer makes `\` a continuation only when spaces, tabs or CRs then a line's end follow it, but it reads any Unicode whitespace as whitespace. So `\` followed by a vertical tab is a word, and taking the tab out, as trailing whitespace, or making it a space before a `//`, turned it into a continuation. The layout now keeps the whitespace after a lone `\` word as written. That is the one trailing whitespace it keeps, and its doc says so. The lexer's two rules disagree, which is Scowr's to settle: `continuation_ahead` skips only ASCII spaces, while `is_space` takes any whitespace. Test: `format.rs`'s `whitespace_after_a_lone_backslash_word_is_kept`.
4. **`expand` turned a lone `\` word into a continuation** the same way, by writing the line afresh. `expand` now gives such a file back as it is, as it does a file that does not parse. Test: in `a_quoted_node_name_stays_quoted`.
5. Two findings were about the target's own comparison, not Null:
   - a list's words in the outline keep their layout, so they are compared as tokens;
   - an unclosed comment or string runs to the end of its line or the file, so its trailing whitespace is layout.
   The target now says so.

**The run the row asks for.** After the fixes, `cargo +nightly fuzz run format` ran 603 seconds (103,786 runs) with no crash, from the seeds and the corpus of the earlier runs, with inputs up to 8 KB, as `tools/fuzz.sh` runs it. Thermal pressure was nominal throughout, though the load from the other agents' builds kept it to about 170 runs a second. `cargo fuzz cmin` then took the corpus from 4,450 inputs to 2,167, which is what 0-24's `just clean` would leave.

**Checks:** `cargo test --workspace` and clippy `-D warnings` are clean, Null's lint-only `just check` passes, and the fuzz workspace builds on nightly.
