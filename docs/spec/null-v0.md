# Null: Sau's project language

Draft v0, November 6, 2026. Nothing is built yet. The name Null and the `.noll` extension were chosen by the user on November 6, 2026 (section 2). It lived in Sau as `docs/NULL.md` until it moved to this repository on November 6, 2026, as Scowr's spec moved out of Crescend. Where it fits in the product: Sau's `docs/ROADMAP.md`.

Null is the source code of a Sau project. A project is a folder of Null files, media and data files, laid out by Cosmósis (`Cosmosis/docs/spec/cosmosis-v0.md`). Sau reads the folder, shows it, renders it and edits it. People and AI change a project by editing its files directly, and every change appears live. Sau's own edits are written back into the same files.

## 1. Principles

1. **The files are the project.** Nothing that matters lives anywhere else. The `.blend` cache, thumbnails and evaluated data can always be rebuilt from the files.
2. **Direct.** To change anything, edit the file that holds it to the exact value wanted. A file says what the project is, not how to build it, which is what lets Sau write a dragged object back as one changed line. Python generators (section 13) are optional and secondary.
   **Files describe, commands act.** What the project is lives in files. Doing something to it now (render, bake, play, frame the view, take a screenshot) is a command, the same for a person in a terminal, an AI through its tools and Sau's own buttons.
3. **Exact.** Any value written reads back to the same bits, and any `.blend` converts to Null and back with no difference.
4. **Blender's vocabulary.** Every name is a Blender RNA identifier in kebab-case: `_` written as `-`, enum values in lower case (settled November 6, 2026, decision S16; the mapping is exact both ways for every RNA identifier). Someone who knows Blender's interface or its Python API can read Null without learning new words.
5. **Readable diffs.** Small records on one line, containers one attribute per line, bulky data apart from intent. A typical edit changes one line.
6. **Accept liberally, print canonically.** The parser accepts any spelling the spec allows; the formatter prints one canonical form. Diagnostics say exactly what was expected and offer a fix.
7. **Small and regular.** One node syntax for everything, meaning given by a schema generated from Blender's RNA. An AI that has read this document should write valid files on the first try.
8. **Everything.** Every piece of data a `.blend` file can hold has a Null form. Preferences, keymaps, themes and workspace layouts are Null files too.

## 2. Name and extension

The language is **Null**, and its files end in **`.noll`** (the user, November 6, 2026): letters only, so it cannot be misread as a number; it sounds like "Null"; and "noll" is Swedish for zero. No existing use of `.noll` was found. The candidates that were weighed:

| Extension | Assessment |
| --- | --- |
| `.noll` | **Chosen.** Letters only, sounds like "Null", means zero in Swedish, no known use by another format. |
| `.n0` | Reads as "Null" and has no known use, but the zero is easily mistaken for the letter O, and it looks like the first of a numbered series (as split archives use `.r00`, `.r01`). |
| `.nol` | Already used by several small formats: Nokia operator logos (obsolete), KnowIt note lists, NetObjects Fusion and Cadcorp SIS's Named Object Library. |
| `.nu` | Taken by Nushell scripts (and the older Nu language). VS Code, GitHub and syntax highlighters would treat Null files as shell scripts, and `nu file.nu` would try to run them. |
| `.0` | Legal, but a digit-only suffix reads as a numbered or rotated file (`render.0`, `log.0`, `libfoo.so.1.0`). Many tools treat it as having no extension, `*.0` globs catch unrelated files, and it is hard to search for. |
| `.null` | Free of conflicts, but long, and "null" as a word collides with every language's null value in searches and editor settings. |

More options raised November 6, 2026: `.void` (thematic, but used by Half-Life map files), `.nil` (rarely used: Norton icon libraries, QIP data), `.nada`, `.zero`, `.nul` (avoid: `NUL` is a reserved device name on Windows, so `nul.nul` cannot exist there), and `.∅` (striking, but hard to type and unsafe in some shells and tools).

**Every operating system and editor.** None of these needs the OS to know the extension for a file to open: a Null file is plain UTF-8 text, and any editor opens it. Registering the extension only adds the icon, the default app and highlighting:

- **macOS:** Sau declares a document type whose identifier conforms to `public.plain-text`, so Quick Look, TextEdit and "Open With" treat it as text.
- **Windows:** extensions may contain digits (`.mp4`, `.7z`, `.3ds`). An installer, or one registry file, sets the ProgID, the icon and `PerceivedType=text`, which puts Notepad and every editor in "Open with".
- **Linux:** a shared-mime-info entry (`text/x-null`, a subclass of `text/plain`, glob `*.noll`) and a `.desktop` file.
- **Editors:** VS Code and JetBrains map the extension through a language extension or one setting (`"files.associations": {"*.noll": "null"}`); Vim, Emacs, Sublime and Zed the same way; tree-sitter grammars list their extensions; GitHub highlights it through a `.gitattributes` line (`*.noll linguist-language=…`) until Linguist knows the language.


## 3. What kind of language it is

Settled November 6, 2026 (Sau's decision register, S8). **Null is to Blender what SVG is to vector graphics**: declarative text the engine renders directly, editable as text or in the app, with high-level elements over exact low-level data (as SVG's shapes over its path data), grouping and transforms, and node graphs as data (as SVG's filters). Every element of a Blender project has a written form, down to the smallest unit Blender stores: "all geometry and logic has to be inside our language without exception, such that we can alter anything and everything by changing a line in the project files". Every term in this spec is defined strictly: what it means, in which unit, and what Blender value it becomes.

| Question | Answer | Why |
| --- | --- | --- |
| Scripting language? | No. Declarative, like Scowr, HTML or a typed configuration language: no statements, loops, variables or functions | It describes what the project is, so editing the file edits the thing, and Sau can write edits back. Computation belongs to geometry nodes, drivers and the optional Python generators. |
| Compiled or interpreted? | Loaded: parsed, checked and applied to Blender's model incrementally on every save | Applying only what changed is what makes reload fast. Rendering is the build. |
| Typed? | Statically and strongly, by the schema generated from RNA, with units as part of each type | Every mistake is caught before it reaches the scene. Units stop a length being used as an angle. |
| High-level or low-level? | Both, by levels (section 9): intent (a cube with a bevel) down to exact vertex arrays | People and AI edit intent; exactness is never lost. |
| Turing-complete? | No, on purpose. Geometry nodes and drivers are data that Blender evaluates, not Null code | Every file always finishes loading, means one thing, and can be diffed and written back. |
| Garbage-collected? | No. The toolchain is Rust and the engine Blender's C++ | Nothing pauses the viewport. |

## 4. A first look

```null
// project.noll
project "Harbor at Dusk" {
  units metric scale=1 length=m rotation=deg
  color-management view=agx look=none
  scene @main
}
```

```null
// scenes/main.noll
scene @main "Main" {
  frame-start 1
  frame-end 240
  fps 24
  camera @shot-cam
  world @dusk

  render cycles {
    samples 512
    resolution [1920 1080]
    denoise on
  }

  collection @harbor "Harbor" {
    object @pier
    object @lamp
    collection @boats
  }
}
```

```null
// objects/pier.noll
object @pier "Pier" {
  location [0m -4m 0.35m]
  rotation-euler [0deg 0deg 12deg]
  scale [1 1 1]
  data mesh @pier-deck

  modifier @array array {
    count 12
    relative-offset [1 0 0]
  }
  modifier @bevel bevel { width 4mm segments 3 }
  material @weathered-oak
}

mesh @pier-deck "PierDeck" {
  primitive cube size=[1.2m 3m 0.08m]     // level 1: a procedure
}
```

```null
// materials/weathered-oak.noll
material @weathered-oak "Weathered Oak" {
  nodes {
    node @tex image-texture { image @oak-diffuse interpolation=cubic }
    node @bsdf principled-bsdf {
      base-color @tex.color             // a link: the socket takes another node's output
      roughness 0.62
    }
    node @out output-material { surface @bsdf.bsdf }
  }
  layout { @tex [-420 0]  @bsdf [-160 0]  @out [120 0] }
}

image @oak-diffuse "oak_diffuse.png" { source media/3f9a…c1.png colorspace=srgb }
```

```null
// actions/lamp-flicker.noll
action @lamp-flicker "LampFlicker" {
  slot light @lamp-light
  fcurve energy {
    key 1  800W interpolation=bezier
    key 6  620W
    key 11 800W
  }
}
```

## 5. Syntax

Null uses Scowr's node syntax (settled November 6, 2026, decision S15; `Scowr/docs/spec/scowr-v0.md`, section 4), so the user's two IDEs share one way of writing and one set of front-end crates:

- A file is a list of nodes. A node is a name, then entries (values and `key=value` properties), then an optional `{ }` block of child nodes.
- `//` and nestable `/* */` are comments; `/-` comments out a whole node.
- A node ends at a newline, a `;` or its parent's `}`. A trailing `\` continues a line.
- Strings are `"..."` with escapes, raw strings `#"..."#`. A word is any run of characters other than whitespace and `{ } [ ] ( ) ; = "`; the schema decides what a word means. Lists are `[ ... ]` and may span lines.
- Any attribute can be a property on its owner's line or a child node: `modifier @bevel bevel width=4mm` and `modifier @bevel bevel { width 4mm }` mean the same.
- A boolean is written bare for on (`denoise`) or with `on` or `off`. The formatter omits attributes equal to their default, except settings the project pins (as Scowr settled on November 5, 2026).
- Error recovery: one broken line never hides the rest of the file, and the syntax tree keeps every byte.

What Null adds to Scowr's syntax is only schema: the node names, the value forms of section 6 and Blender's kinds.

## 6. Values

- **Numbers** are written in decimal and read exactly into the property's own type: float32, float64 or an integer, as DNA stores it. The formatter prints the shortest decimal that reads back to the same bits, so `0.1` stays `0.1` and never becomes `0.10000000149`.
- **Units** follow the RNA unit of each property: length (`m`, `cm`, `mm`, `km`, `in`, `ft`), rotation (`deg`, `rad`), time (`s`, `ms`, frames as bare numbers), power (`W`), temperature (`K`, `C`), wavelength (`nm`), camera lens (`mm`), mass (`kg`), velocity and acceleration. A bare number is Blender's internal unit (metres, radians, frames, watts). Writing a unit a property cannot take is an error.
- **Conversion rule.** A value with a unit is converted in float64 and rounded once, to nearest even, into the stored type. The formatter prints a value in the project's display unit (`rotation=deg`) whenever the shortest such decimal converts back to exactly the stored bits, and in the internal unit otherwise. `90deg` therefore reads back as `90deg`, and a rotation set by dragging prints exactly in radians if no short degree value matches it.
- **Vectors, colours and matrices** are lists: `[0m 0m 1m]`, `[0.8 0.62 0.41 1]`. Colours are scene-linear floats as Blender stores them; `#rrggbb` is accepted as sRGB input and converted by the fixed colour-management rule, and printed back in that form only if it converts to the same bits.
- **Enums** are RNA enum identifiers in kebab-case (`cubic`, `agx`, `bezier`); flag sets are lists of them. Raw numbers are never needed.
- **References** are handles (section 7). A pointer property takes a handle; a socket takes a value or a link `@node.socket`.
- **Strings** are names, paths and text. Text datablocks (Blender's text editor content, Python scripts) are files of their own in the project, referenced by path.

## 7. Identity and references

- **Handles.** Every datablock written in Null has a handle, `@pier`, unique within the project for its kind. The handle is the identity Null uses; the quoted string after it is Blender's display name, which may change freely (`object @pier "Pier"`). A handle may be omitted when the name is a valid handle and the same (`object "Pier"` gives `@pier`).
- **Persistent IDs.** Behind every handle Sau keeps a persistent ID in DNA, so a rename in either Sau or the file never breaks links. The map from handles to IDs is a cache in `.sau/`, rebuilt from the files.
- **Sub-object handles.** Modifiers, constraints, nodes, sockets, bones, shape keys, vertex groups, UV maps, attributes, view layers and action slots are named inside their owner and addressed by path: `@pier/@bevel`, `@weathered-oak/@bsdf.roughness`, `@rig/@spine.002`.
- **Across files.** A handle is visible in the whole project, so `material @weathered-oak` in an object file finds the material's file through the project index. Packages are addressed with their name: `@cosmosis-materials:oak`.
- **Linked libraries** (another `.blend` or another Null project) are packages or `link` nodes naming the source and the handles taken from it. Library overrides are written as the overridden values on top of the linked handle.

## 8. The objects

Every one of Blender's datablock types (31 kinds of ID in this source tree) has a node kind, generated from RNA and refined by hand only where RNA is not enough:

| Area | Kinds |
| --- | --- |
| Structure | `project`, `scene`, `view-layer`, `collection`, `object`, `workspace`, `screen` |
| Object data | `mesh`, `curve`, `curves` (hair), `surface`, `metaball`, `text`, `point-cloud`, `volume`, `grease-pencil`, `armature`, `lattice`, `camera`, `light`, `light-probe`, `speaker` |
| Look | `material`, `world`, `node-group` (geometry, shader, compositor), `texture`, `image`, `movie-clip`, `palette`, `brush`, `line-style` |
| Motion | `action` (with slots and layers), `nla` tracks on their owners, `driver` on any property, `shape-key`, `cache-file`, `mask` |
| Media | `image`, `sound`, `font`, `movie-clip`, `volume` and `cache-file` sources, all pointing into `media/` |
| Settings | `render` (Cycles, EEVEE, Workbench), `color-management`, `units`, `preferences`, `keymap`, `theme` |

Each object kind's properties are the RNA properties of its struct, by the same names. The generated field reference, one table per struct, is an appendix produced from the schema at build time; it is never written by hand.

## 9. Geometry as source

The 3D counterpart of Scowr's "sound as source" (Scowr spec, section 20). Geometry is written at one of three levels, and each level renders to the one below exactly:

| Level | What it is | Written as | Edited by |
| --- | --- | --- | --- |
| 0. Data | Exact attribute arrays: positions, edges, faces, corners, UVs, normals, weights, creases, colours, any named attribute | Inline lists for small meshes; a content-addressed data file in `data/` for large ones, referenced from the text | The viewport's edit mode, sculpting, and tools; rarely by hand |
| 1. Procedure | Primitives with parameters, modifier stacks, geometry-node graphs, curves and their profiles | Ordinary Null nodes | Hand, AI and Blender's editors alike |
| 2. Lifted | An existing mesh described as a procedure plus an exact residual: "a bevelled box of these dimensions, arrayed 12 times, plus these vertex offsets" | A level 1 description and a residual data file | As level 1; the residual keeps the result exact |

- **The threshold** between inline and data files is a Cosmósis setting (proposed default: 2,000 elements). `sau convert --inline` writes everything as text whatever its size.
- **Data files** are a documented, versioned binary layout of named attribute arrays, named by the SHA-256 of their contents, so the same data is stored once and Git sees a changed hash rather than a binary diff of an unchanged mesh. Their text summary (counts, bounds, attribute names) is printed in the Null file as comments and checked on load.
- **Images** follow the same ladder: pixels in `media/`; procedural textures as node graphs; lifted, a procedural texture plus a residual.
- **Lifting quality** decides how much of an asset is editable as intent, never how accurate it is. `sau lift` writes a grading report: how much is explained by the procedure and how large the residual is.

## 10. Node trees

Shader, geometry, compositor and texture node trees are written as their nodes and links:

- A node is `node @handle <kind> { <socket> <value or link> … <property> <value> … }`, the kind being the node's RNA identifier in kebab-case.
- A link is a socket value that names another node's output: `base-color @tex.color`. A socket with no line takes its default.
- Group nodes reference a `node-group`; group interfaces list their inputs and outputs with types and defaults.
- Zones (repeat, simulation, for-each) are blocks containing their nodes.
- **Layout is kept apart.** Node positions, frames, colours and widths live in the tree's trailing `layout` block, so dragging a node in the editor changes only that block, and reading the tree's meaning never needs the layout.

## 11. Animation and drivers

- Actions follow Blender's slotted, layered model: slots name what they animate; f-curves name the property by its RNA path; keys are `key <frame> <value>` with interpolation, handles and easing as properties.
- Frames are exact as Blender stores them (float32, so sub-frames are allowed and print exactly).
- Drivers are written on the property they drive: `energy driver="var * 800" { var single-prop @lamp-light.energy }`. Driver expressions stay in Blender's simple-expression subset where possible, so they evaluate without Python.
- NLA strips, markers, shape-key animation and armature poses are ordinary nodes.

## 12. Media and caches

Images, videos, sounds, fonts, HDRIs, OpenVDB volumes, Alembic and USD caches and simulation bakes live in `media/`, named by content hash, and are referenced from Null with their colour space, frame range and other settings. Packed files in a `.blend` are unpacked into `media/` on conversion. Sources outside the project are referenced by path and reported by `sau check` if missing.

## 13. Generators (optional)

A Python script in `scripts/` may generate part of a project: a scattered forest, a staircase from a formula. It runs with Blender's Python API in a sandbox that may create data but not touch files or the network, gives the same output every time, and its output is plain Null text. A generated part can be frozen into the project as ordinary text and edited by hand. Geometry nodes and drivers are preferred over generators wherever they can do the job, because they stay live data.

## 14. Checks

Files in `checks/` state what must be true of the scene and of renders: a polygon budget, no object below the floor, no missing media, every material has a name, a render of camera `@shot-cam` at frame 1 matches `checks/golden/shot-1.png` within a tolerance. `sau test` runs them, and a failure points at the file and line that caused it. The check vocabulary is specified with phase 8.

## 15. Canonical form

The formatter prints one form: canonical order of attributes (the RNA definition order), one object per file section, small records on one line, defaults omitted except pinned settings, numbers printed by the rule of section 6, and handles written as kebab-case of the display name when new. `sau fmt` is idempotent, and Sau's own write-back always produces canonical text for the lines it touches, without reformatting lines it does not.

## 16. Live reload

Changes are live in both directions, all the time, the way every query in a Convex app updates the moment its data or code changes (Sau's roadmap, design decision 9). Each change, from any source, is one checked, atomic transaction with a revision number, and every view (viewport, editors, code views, the language server's clients and the gateway's subscribers) follows the same revision.

There is no run step and no separate save step (the user, November 6, 2026: changes "with hot reload open and autosave, rather than making scripts"). Sau's own code editor applies an edit as soon as the text parses, a fraction of a second after typing stops, and writes it to disk. Saving a file in any other editor, an AI's edit or a `git checkout` makes Sau, while it is open:

1. read and parse the changed files off the main thread;
2. check them against the schema; on any error, keep the last good state, show the diagnostics in the code view and the status bar, and stop;
3. compare the new tree with the last applied one by handle, producing the list of created, deleted and changed datablocks and properties;
4. apply the changes to `Main` on the main thread through RNA, so Blender's own update callbacks and dependency-graph tags run, as one undo step named after the file;
5. redraw the affected editors.

Edits Sau makes itself (section 17) are recognised by their content hash and not reloaded.

## 17. Write-back

Every edit in Sau's interface (a drag, a slider, an operator, a node link) ends in an undo push. At that moment Sau already knows, from Blender's undo system, which datablocks changed. For each one it prints the new Null form, compares it with the current text of its node, and applies the smallest edit to the lossless syntax tree, keeping comments, ordering and untouched lines. The file is written atomically. Edit mode and sculpt mode write their data when the mode is left, and while inside, after a short pause. A file changed outside while an edit is pending is merged three-way, like an editor's reload prompt; if both sides changed the same value, Sau asks.

## 18. Diagnostics

Every message names the file, line and column, says what was expected, and where possible offers a fix: an unknown property suggests the nearest RNA name, a wrong unit says which units fit, a handle that resolves to nothing lists the near matches. Codes are stable (`null-unit`, `null-unknown-property`, `null-handle`), so tools and AI can act on them.

## 19. Converting `.blend` files

`sau convert scene.blend Scene.sau/` writes a Cosmósis project from any `.blend`; `sau convert Scene.sau/ scene.blend` goes back. The conversion is lossless:

- everything RNA reaches is written as named Null properties;
- data RNA does not reach (found by the census of roadmap phase 0) is written in a `raw` block per datablock, using SDNA's own struct and field names, until a named form replaces it; the coverage audit counts these and they may only shrink;
- runtime-only data is skipped, and the list of what counts as runtime is part of the spec.

Correctness is checked by a field-by-field comparison of the original and the round-tripped `.blend` (by DNA structure, not by bytes, since a `.blend` holds memory addresses), and by rendering both. Through Blender's importers, any format Blender reads (glTF, FBX, OBJ, USD, Alembic, PLY, STL) converts into Null the same way.

## 20. Open questions

1. The inline threshold for level 0 data (section 9).
2. Whether object data used by one object is written inside that object's file (a component, as in React) or always in its own folder. Proposed: inside, when used once; in its folder when shared (Cosmósis, section 3).
3. The check vocabulary (section 14), settled in phase 8.

## 21. What building Null takes

- **Design (phase 0 and 1a):** this draft, tested against the slice and the census, then frozen with a changelog and RFCs for later changes, as Scowr does.
- **The front end (Rust, in Null's repository):** lexer, lossless syntax tree, parser with error recovery, formatter, diagnostics, the checker that reads the generated schema, and the language server. Shared with Scowr where the syntax is shared.
- **The binding (C++, inside Sau):** the schema generator in `makesrna`, applying a checked tree to `Main` through RNA, printing any datablock to Null, the raw DNA fallback, the live-reload watcher and the write-back at undo push.
- **Tooling (phase 4):** the tree-sitter grammar, the VS Code extension, the generated field reference, examples and a short guide written for AI.
- **Stability:** a `format` version in every project's manifest; when Blender renames or moves an RNA property between releases, Sau ships a migration that rewrites old files, the way Blender's versioning code updates old `.blend` files.
