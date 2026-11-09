# Null agent guide

Null is the language of Sau's projects ([README](README.md)). This file is the router for agents. Building starts with Sau's phase 1a; the shared workflow, task board and limits are Sau's.

## Always

- Read the [specification](docs/spec/null-v0.md) before changing anything the language means.
- Decisions the user settled are not reopened; propose a change through `docs/rfcs/`.
- **Null v0 is frozen** (the local tag `v0`, November 8, 2026; Sau's task 1d-01c). A change to the spec now needs an RFC first ([`docs/rfcs/README.md`](docs/rfcs/README.md)): any new form, word, unit, default, diagnostic or rule, any change to what a file means, its canonical form or its write-back. Only a fix to wording that changes no meaning, and implementation that follows the spec as it is, go in without one. An accepted RFC names itself in the spec and in `CHANGELOG.md`.
- Original code only, with permissive dependencies checked first. Never copy or translate GPL-family code, including Blender's: Null is MIT, and reads Blender's schema only through the file Sau generates.
- Work from Sau's task board (`../Sau/docs/TASKS.md`, `just tasks` in Sau): claim a task before starting it, hold at most three, and move each through `review` and `done` (`../Sau/docs/agents/workflow.md`). Read Sau's `HANDOFF.md` for the current state.
- The only agent writing Null works in this main checkout on `main`; a second agent writing Null at the same time works in its own worktree and branch, `just wt-new <name> --repos null` in Sau (Sau's decision S58, `../Sau/docs/agents/parallel.md`).
- Commit every working change as soon as it works and `just check` exits 0 on its own (format, Clippy with warnings as errors, tests, the licence gate); a commit of documents alone needs only the git hooks. Before a task goes to review: `just sync` in Sau's worktree, then `just verify` (property tests and fuzzing too, and Sau's crates built against this checkout when `../Sau` is there, Sau task 0-12: a change to an API Sau uses fails here first). Never push, never bypass the hooks (`just hooks` installs them).
- Only permissive dependencies: `deny.toml` lists the licences allowed, and `just check` refuses others.
- Every compile goes through the build guard (`tools/build-guard.py`, an identical copy of Sau's), which keeps the shared Mac cool. Do not bypass it.
- Sau (`~/Projects/Sau`), Cosmósis (`~/Projects/Cosmosis`) and Scowr (`~/Projects/Scowr`) are separate repositories. Change them only when the task says so.

## The shared front end

Null writes Scowr's node syntax and uses Scowr's front-end crates for it, by path from `../Scowr` (Sau's decision S15; task 1a-01, November 7, 2026). The boundary:

| Shared, Scowr's (`../Scowr/compiler/`) | Null's own (`compiler/`) |
| --- | --- |
| `scowr_syntax`: lexer, lossless syntax tree, parser with error recovery, incremental reparse, the typed view (`ast`) | `null_syntax`: the only crate that names Scowr's; re-exports the tree, gives syntax diagnostics Null's code (`null-syntax`) and checks that names are kebab-case (`null-name`, S16) |
| `scowr_errors`: spans, files, diagnostics, fixes, their rendering, `closest` for suggestions | Everything a word means: numbers and units, handles, kinds, the schema from RNA, the checker, the printer and write-back, the language server |

- Every other Null crate depends on `null_syntax`, never on a `scowr_*` crate, so when the shared crates move to their own repository (S15, S55) only `null_syntax`'s manifest changes. The paths are in the root `Cargo.toml`, `[workspace.dependencies]`.
- Scowr's other crates (`scowr_value`, `scowr_schema`, `scowr_check`, `scowr_fmt` and the rest) are Scowr's language, not shared syntax; Null does not use them.
- Never change Scowr from here. A change Null needs in the shared syntax is agreed with the user first, made in Scowr, and passes Scowr's own tests (`cargo test --workspace` in `../Scowr`). Record such a need on Sau's board as `blocked` with the exact change.
- Format with `cargo fmt` (what `just fmt` and `just check` run), never `cargo fmt --all`: `--all` also formats local path dependencies, which would rewrite Scowr's files.
- Known edges of the shared syntax, for Null's checker to handle: `at`, `to` and `len` followed by a space and a value are read as Scowr's keyword attributes (`NodeKind::Keyword`), not arguments; and a node ends only at a line feed, `;` or `}`, so `{ width 4mm segments 3 }` is one child, `width`, with three arguments.

## Task routes

| Task | Read |
| --- | --- |
| What the language means | `docs/spec/null-v0.md` |
| Proposing a language change | `docs/rfcs/README.md` |
| Project layout, packages and commands | Cosmósis's `docs/spec/cosmosis-v0.md` |
| Where this fits in the product; the plan for this repository | Sau's `docs/ROADMAP.md` and `docs/REPOSITORIES.md` |
| The syntax Null may share | Scowr's `docs/spec/scowr-v0.md`, section 4 |
| Taking and finishing work; commits; reports | Sau's `docs/agents/workflow.md` and `docs/TASKS.md` |
| Worktrees, machine limits, merging | Sau's `docs/agents/parallel.md` |
| What each repository owns; licences | Sau's `docs/agents/boundaries.md` |
| What to test, and the gates | Sau's `docs/agents/testing.md` |
