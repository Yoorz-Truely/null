# agent/crosscheck

The report of agent/cosmosis-a working as `agent/crosscheck` in Sau's `work/wt/crosscheck/`, on Null's and Cosmósis's branch `agent/crosscheck` (Sau is a link to its main checkout), November 7, 2026.

## 0-12: Null's and Cosmósis's verify build Sau's crates against themselves

**What works.**

- **`tools/sau-crosscheck.sh <Null|Cosmosis>`**, the same file in both repositories. `just verify` runs it last. It runs `cargo check --locked --workspace --all-targets` over Sau's `sau/crates` through this repository's build guard, with its own target folder, `target/sau-crosscheck/`.
- **Why a folder of its own.** Sau's crates take Null and Cosmósis by path (`../../../../Null`), and Cargo reads such paths as written, without resolving links; I checked that with `cargo metadata`. So the check builds in `target/sau-crosscheck/tree/`, which holds:
  - Sau's committed `sau/`, and `unit.cc` where Sau has it, extracted from Sau's HEAD again only when HEAD moves;
  - this checkout, linked under its own name;
  - the other repositories beside it, as links.

  Sau's HEAD, not its working tree: a main checkout mid-merge (an integrate) or holding someone's edits made the first version of this check fail, or pass, for reasons not this repository's.
- **What it catches.** A changed signature Sau calls, or a dependency added here that Sau's `Cargo.lock` does not list (`--locked`, as with `null_print` gaining `null_check`). It fails with the compiler's error and a line saying which of the two to look at, and that Sau is the lead's to change.
- **With no Sau beside the checkout**, it prints that it skipped, and verify passes.
- **`tools/sau-crosscheck-test.sh <Null|Cosmosis>`**, run by hand because it builds Sau's crates twice. In throwaway clones it shows:
  - the unchanged clone passes;
  - a public function Sau's `sau_core` calls, given one more parameter, fails with `error[E0061]` in `sau_core` on that function. The functions are `null_print::write_transform` and `cosmosis::layout::find_root`, and this repository's own libraries still build with the change;
  - with no Sau beside the clone, the check skips and passes.
- **`AGENTS.md`** in both repositories says what verify now covers.

**Measured.**

- The test passes in both repositories: Null `1e1c584`, about one minute; Cosmósis `f9a316e`, the same.
- The check alone, built once, takes about 4 s while Sau's HEAD stays the same, and about 20 s after HEAD moves.

**For the lead (Sau).** Sau's `check.sh` keeps `build-guard.py` and `check-record.py` equal across the three repositories. Add `tools/sau-crosscheck.sh` and `tools/sau-crosscheck-test.sh` to that loop, comparing Null's copy with Cosmósis's, since Sau has none. I can't change Sau.
