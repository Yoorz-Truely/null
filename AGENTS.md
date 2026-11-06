# Null agent guide

Null is the language of Sau's projects ([README](README.md)). This file is the router for agents. Nothing is built yet.

## Always

- Read the [specification](docs/spec/null-v0.md) before changing anything the language means.
- Decisions the user settled are not reopened; propose a change through `docs/rfcs/`.
- Original code only, with permissive dependencies checked first. Never copy or translate GPL-family code, including Blender's: Null is MIT, and reads Blender's schema only through the file Sau generates.
- Doc-only commits need only a Markdown check; code commits need `cargo fmt`, `cargo clippy` and `cargo test` to pass.
- Commit focused changes; never push unless the user asks.
- Sau (`~/Projects/Sau`), Shoon (`~/Projects/Shoon`) and Scowr (`~/Projects/Scowr`) are separate repositories. Change them only when the task says so.

## Task routes

| Task | Read |
| --- | --- |
| What the language means | `docs/spec/null-v0.md` |
| Proposing a language change | `docs/rfcs/README.md` |
| Project layout, packages and commands | Shoon's `docs/spec/shoon-v0.md` |
| Where this fits in the product; the plan for this repository | Sau's `docs/ROADMAP.md` and `docs/REPOSITORIES.md` |
| The syntax Null may share | Scowr's `docs/spec/scowr-v0.md`, section 4 |
