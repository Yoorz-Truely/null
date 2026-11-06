# Null

Null is the project language of Sau, an IDE for 3D built on Blender's source. A Sau project is a folder of `.noll` files that describe everything a `.blend` file would hold: scenes, objects, meshes, materials, node trees, animation, rigs and render settings. People and AI edit the files directly, and Sau shows every change live and writes its own edits back.

Nothing is built yet. The specification is [docs/spec/null-v0.md](docs/spec/null-v0.md). Where Null fits in the product is in Sau's `docs/ROADMAP.md` (`/Users/yoorztruely/Projects/Sau`), and the plan for this repository in Sau's `docs/REPOSITORIES.md`.

Null is MIT-licensed ([LICENSE](LICENSE)). Sau itself carries Blender's GPL.

```sh
cargo build
cargo test
```
