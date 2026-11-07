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

## Review

### 0-12, Null `11e0861` and Cosmósis `aed1a0a`, by agent/null-a, November 8, 2026: approved

I read the row, this report, `tools/sau-crosscheck.sh` and `tools/sau-crosscheck-test.sh` (the same file in both, by `cmp`), and both `justfile`s and `AGENTS.md`s.

**Run, headless:**
- `tools/sau-crosscheck-test.sh Null`: passes. With one more parameter on `null_print::write_transform`, the check fails with `error[E0061]` in `sau_core`; with no Sau beside the clone, it skips and passes.
- `tools/sau-crosscheck-test.sh Cosmosis`: passes, the same way, on `cosmosis::layout::find_root`.

**What holds:**
- **Sau's committed HEAD, not its working tree.** That is right: an integrate mid-merge in Sau's main, or someone's edits there, can't make another repository's verify pass or fail.
- **From a worktree,** `../Sau` is that worktree's Sau, so a Null change paired with a Sau change on the same agent branch builds together.
- **A dependency Sau's lock does not list** fails through `--locked`. That is exactly how `null_print` gaining `null_check` would have been caught.
- **The extracted tree** is a real folder with `unit.cc` included, so `sau_core`'s build script and `include_str!` read real files. It is extracted again only when HEAD moves.
- **The build guard and its own target folder** keep the run cool and apart from this repository's build.

**Nits, none blocking:**
- **`--offline`.** Sau's own builds pass `--offline`; this check does not. A crate missing from the local registry would make it reach the network rather than fail at once.
- **The lead's step.** The report's request stands: Sau's `check.sh` should keep the two copies of each script equal, as it does `build-guard.py`. The lead has added that (Sau `1dd37a45`).
