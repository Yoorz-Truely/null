# Null agent guide

Null is the language of Sau's projects ([README](README.md)). This file is the router for agents. Building starts with Sau's phase 1a; the shared workflow, task board and limits are Sau's.

## Always

- Read the [specification](docs/spec/null-v0.md) before changing anything the language means.
- Decisions the user settled are not reopened; propose a change through `docs/rfcs/`.
- Original code only, with permissive dependencies checked first. Never copy or translate GPL-family code, including Blender's: Null is MIT, and reads Blender's schema only through the file Sau generates.
- Work from Sau's task board (`../Sau/docs/TASKS.md`, `just tasks` in Sau): claim a task before starting it, hold at most three, and move each through `review` and `done` (`../Sau/docs/agents/workflow.md`). Read Sau's `HANDOFF.md` for the current state.
- Work in your own worktree on your own branch, never in the main checkout or on `main` unless you are the integrator: `just wt-new <name> --repos null` in Sau makes one (`../Sau/docs/agents/parallel.md`).
- Commit every working change as soon as it works and `just check` exits 0 on its own (format, Clippy with warnings as errors, tests, the licence gate); a commit of documents alone needs only the git hooks. Before a task goes to review: `just sync` in Sau's worktree, then `just verify` (property tests and fuzzing too). Never push, never bypass the hooks (`just hooks` installs them).
- Only permissive dependencies: `deny.toml` lists the licences allowed, and `just check` refuses others.
- Every compile goes through the build guard (`tools/build-guard.py`, an identical copy of Sau's), which keeps the shared Mac cool. Do not bypass it.
- Sau (`~/Projects/Sau`), Cosmósis (`~/Projects/Cosmosis`) and Scowr (`~/Projects/Scowr`) are separate repositories. Change them only when the task says so.

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
