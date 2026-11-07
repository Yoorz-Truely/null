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
| Turing-complete? | No, on purpose: total (Sau's decision S38). Patterns applied any number of times in one line, loops over counts and lists, conditions, arithmetic, parameters, and recursion that counts down to an end. Unbounded computation belongs to geometry nodes, drivers and generators | Every file always finishes loading, means one thing, and can be diffed and written back. Abstraction keeps files short. |
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
  modifier @bevel bevel { width 4mm; segments 3 }
  material @weathered-oak
}

mesh @pier-deck "PierDeck" {
  primitive cube size=1m scale=[1.2 3 0.08]     // level 1: a procedure
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
  layout { @tex [-420 0]; @bsdf [-160 0]; @out [120 0] }
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

- **Numbers** are written in decimal and read exactly into the property's own type: float32, float64 or an integer, as DNA stores it. The formatter prints the shortest decimal that reads back to the same bits, so `0.1` stays `0.1` and never becomes `0.10000000149`. `inf` and `-inf` are written as such. NaN is written `nan`, the quiet NaN, without its sign or payload: the one value that does not read back to the same bits (S59).
- **Units** follow the RNA unit of each property, as the table under "Units" below lists them. A bare number is Blender's internal unit (Blender units, radians, frames, seconds, watts). Writing a unit a property cannot take is an error.
- **Conversion rule.** A value with a unit is converted exactly and rounded once, to nearest even, into the stored type (Sau's decision S59): every factor in the table is an exact rational number except π/180, which is computed as precisely as the rounding needs, and the scene's unit scale and frame rate enter as the exact values of the floats Blender stores. Arithmetic in float64 would round twice. The formatter prints a value in the project's display unit (`rotation=deg`) whenever the shortest such decimal converts back to exactly the stored bits, and in the internal unit otherwise. `90deg` therefore reads back as `90deg`, and a rotation set by dragging prints exactly in radians if no short degree value matches it.
- **Vectors, colours and matrices** are lists: `[0m 0m 1m]`, `[0.8 0.62 0.41 1]`. Colours are scene-linear floats as Blender stores them; `#rrggbb` is accepted as sRGB input and converted by the fixed colour-management rule, the IEC 61966-2-1 sRGB curve, exactly rounded once into each channel's type from a fixed table of the 256 channel values, with no floating-point `pow` (Sau's decision S62), alpha 1 for a four-channel colour; and printed back in that form only if it converts to the same bits. Blender's colour picker converts hex through its colour management, which differs from the exact curve for most values, so a colour set there prints back as a list of floats, not `#rrggbb`.
- **Enums** are RNA enum identifiers in kebab-case (`cubic`, `agx`, `bezier`); flag sets are lists of them. Raw numbers are never needed. Every RNA type's form is under "Types" below.
- **References** are handles (section 7). A pointer property takes a handle; a socket takes a value or a link `@node.socket`.
- **Strings** are names, paths and text. Text datablocks (Blender's text editor content, Python scripts) are files of their own in the project, referenced by path.

### Types

How each RNA property type is written (task 1c-02a, November 7, 2026; Sau's decisions S61 (5) and S65). The schema says which type a property has; the checker refuses any other form (S24).

| RNA type | Written | Examples |
| --- | --- | --- |
| Boolean | `on` or `off`; the property's name alone means `on` | `use-shadow`, `use-shadow off` |
| Integer | A whole number in decimal, which may have an exponent (`1e3`), within the property's hard range; never a unit | `samples 512`, `frame-start 1` |
| Float | A number as above ("Numbers"), with a unit where the property has one ("Units") | `roughness 0.62`, `lens 50mm` |
| Enum | One item's identifier in kebab-case | `rotation-mode xyz`, `type point` |
| Enum flag set | A list of item identifiers, in the schema's item order; `[]` for none | `delimit [normal material]` (a decimate modifier), `delimit []` |
| String | A quoted string, which is UTF-8. A string Blender holds that is not valid UTF-8 (an old file path, a name from another system) is written as the list of its byte values instead, so every byte is written exactly: the schema says the property is a string, so a list there means its raw bytes (Sau's decision S65). A string that is valid UTF-8 is always written as a string. A string longer than Blender's field for it is an error, never cut | `name "Pier"`, `filepath "//textures/oak.png"`, `filepath [47 47 109 252 108 108 101 114 46 112 110 103]` (`//m`, the Latin-1 byte 252, `ller.png`) |
| Pointer | A handle (section 7); `none` for an empty pointer where the property allows one | `camera @shot-cam`, `parent none` |
| Collection | Its items as child lines, in order, each written as its struct is; a collection of one-value structs as a list of the values (section 8, `vertices` and `polygons`) | `material @oak` lines, `vertices [[0m 0m 0m] …]` |
| Array | A list of the property's length, each item written as the single value would be; a dynamic array of any length from its minimum | `location [0m 0m 1m]`, `color [0.8 0.62 0.41 1]` |
| Multi-dimensional array | Nested lists, the first dimension outermost, as RNA's dimensions are listed | `[[1 0] [0 1] [0 0]]` for dimensions `[3 2]` |
| Matrix | Row by row, as a person reads it and as Python's `mathutils.Matrix` prints it: `[[row 1] [row 2] …]`. DNA and RNA's flat array keep a matrix column by column, so row `i`, column `j` is RNA's element `j × rows + i` | `[[1 0 0 2m] [0 1 0 0m] [0 0 1 0m] [0 0 0 1]]` (a translation of 2 m in x) |

- **Units in arrays and matrices** go on each item that has one, as in `location [0m 0m 1m]`; in a matrix only the translation column's items are lengths.
- **Raw bytes, not an escape.** A string keeps the escapes Null shares with Scowr (`\"`, `\\`, `\n`, `\t`, `\u{...}`). Bytes outside UTF-8 are a byte list rather than a new `\xNN` escape, because Scowr's lexer refuses `\x` and its syntax changes only with the user's agreement (S15, S65).

### Units

Every unit kind in Blender's RNA (`PropertyUnit`, `PROP_UNIT_*` in `makesrna/RNA_types.hh`, Blender 5.3), with what a bare number means, the words Null accepts and the exact factor of each. The words are Blender's own short names in ASCII (`um` for µm, `ft` and `in` for `'` and `"`). Which words Null takes is Null's choice, not Blender's `B_UNIT_DEF_SUPPRESS` marking: it leaves out hectometres, decametres, decimetres, furlongs, chains, quintals, hectograms, decagrams, arcminutes, arcseconds and turns, and keeps `yd`, `km/h`, `mph`, `kW` and the camera's `cm` and `um`, which Blender marks as rarely used. Blender's own turn is disabled (inside `#if 0` in `unit.cc`'s `buNaturalRotDef`, where its factor reads 1/(2π) rather than 2π), so Blender offers no turns either. Words are case-sensitive: `mW` is a milliwatt, `MW` a megawatt. Task 1c-01a, November 7, 2026. `null_value` reads every word in the table and prints a value back in the word its line uses (task 1c-01b), in a given scene's unit scale and frame rate by the two rules below (task 1c-01h); the checker reading frame properties through the project's frame rate is task 1c-01c, and through its unit scale task 1c-01d.

| RNA unit | A bare number is | Words, with their exact value in the bare unit | Scene scale |
| --- | --- | --- | --- |
| `NONE` | The number itself | None. Factors, percentages (0 to 100, as stored), pixels, colours and directions have no unit | – |
| `LENGTH` | A Blender unit: a metre at scale 1 | `km` 1000, `m` 1, `cm` 1/100, `mm` 1/1000, `um` 10⁻⁶, `nm` 10⁻⁹, `pm` 10⁻¹²; `mi` 1609.344, `yd` 0.9144, `ft` 0.3048, `in` 0.0254, `thou` 0.0000254 | ÷ s |
| `AREA` | A square Blender unit | Each length word followed by `2` (`m2`, `cm2`, `ft2`), its value squared | ÷ s² |
| `VOLUME` | A cubic Blender unit | Each length word followed by `3` (`m3`, `cm3`, `in3`), its value cubed | ÷ s³ |
| `MASS` | A kilogram | `t` 1000, `kg` 1, `g` 1/1000, `mg` 10⁻⁶; `tn` 907.18474 (the short ton, 2,000 lb, as Blender's value is), `cwt` 45.359237, `st` 6.35029318, `lb` 0.45359237, `oz` 0.028349523125 | ÷ s³ |
| `ROTATION` | A radian | `rad` 1, `deg` π/180 | – |
| `TIME` | A frame | `d` 86400, `h` 3600, `min` 60, `s` 1, `ms` 1/1000, `us` 10⁻⁶ seconds, each times the scene's frame rate (`fps` ÷ `fps-base`) | – |
| `TIME_ABSOLUTE` | A second, whatever the scene's frame rate | `d` 86400, `h` 3600, `min` 60, `s` 1, `ms` 1/1000, `us` 10⁻⁶ | – |
| `VELOCITY` | A Blender unit per second | `m/s` 1, `km/h` 5/18, `ft/s` 0.3048, `mph` 0.44704 | ÷ s |
| `ACCELERATION` | A Blender unit per second squared | `m/s2` 1, `ft/s2` 0.3048 | ÷ s |
| `CAMERA` | A millimetre (a focal length or sensor size) | `m` 1000, `cm` 10, `mm` 1, `um` 1/1000 | – |
| `POWER` | A watt | `GW` 10⁹, `MW` 10⁶, `kW` 1000, `W` 1, `mW` 1/1000, `uW` 10⁻⁶, `nW` 10⁻⁹ | ÷ s² |
| `TEMPERATURE` | A kelvin | `K` 1; `C`, kelvin = C + 273.15; `F`, kelvin = (F + 459.67) × 5/9 | – |
| `WAVELENGTH` | A nanometre | `mm` 10⁶, `um` 1000, `nm` 1, `pm` 1/1000 | – |
| `COLOR_TEMPERATURE` | A kelvin | `K` 1 | – |
| `FREQUENCY` | A hertz | `Hz` 1, `kHz` 1000 | – |

- **Scene scale.** `s` is the scene's `scale-length` (`units … scale=`), applied only when its unit system is `metric` or `imperial`, never with `none`. This is exactly what typing the same word into Blender's field does (`BKE_unit_value_scale`): with `scale=0.01`, `1m` stores 100 Blender units. Power is divided by s² and mass by s³ because Blender's own rule does so. Camera, wavelength, time, rotation and temperature values are never scaled. Which scale (task 1c-01d): inside a scene's block, the scene's `units`, field by field over the project's; outside any scene (objects, meshes, cameras, which every scene shares), the project's `units` whenever the project has a `units` line, even an empty `units {}`; when it has none, the scale every scene has, a scene with no `units` being at 1 (1 with no scenes). Where the scenes' scales differ and the project has no `units`, a value written with a scaled unit outside every scene has no one value: the first such value is a `null-unit` error naming each scene's scale, and write-back writes such values bare. A new scene's system is `metric` at scale 1, and what a `units` line leaves out is a new scene's. Only the first `units` line of a project or scene counts, and within it the first setting of each field (`scale` and `scale-length` are one field, as are the system written first on the line and a `system` line); the others are `null-repeated` errors. The checker, write-back and Sau read `units` through one reader, `null_check::units`.
- **Frames.** A frame property's seconds become frames through the scene's `fps` (an integer) divided by its `fps-base` (a float32 such as 1.001), both taken exactly: `1.5s` is 36 frames at 24 fps, and 30000/1001 fps (29.97) is written `fps 30000` and `fps-base 1001`, exactly; Blender's own 29.97 preset is `fps 30` and `fps-base 1.001`, whose float32 is not exactly 1.001, so the two give slightly different frames for the same seconds. Blender's interface does not convert time units at all; Null does, by this rule. Only a float frame takes seconds: an integer frame (`frame-start`, `frame-end`) is a whole number of frames, always bare, and seconds there are an error, never rounded. Seconds on a frame property outside a scene's block are an error too, since there is no frame rate to take (task 1c-01c). An action's keys are outside every scene, so their frames are always written in frames; seconds there are an error (the action form will say so). Write-back keeps a frame's seconds where a decimal in seconds reads back exactly.
- **Temperatures** convert with their offsets exactly; `F` uses 5/9 itself.
- **Blender's own fields round differently.** Blender's unit factors are float32 literals (`0.01f` for a centimetre, 0.555555555555 for °F's 5/9), so a value typed into a Blender number field with a unit is converted inexactly and often stores other bits than Null's rule gives. Of 2,000 typed values each, as measured in Sau for task 1c-01a's review, this happens for 526 in `cm`, 1,251 in `mm`, 329 in `ft`, 286 in `in` and 169 in `mi`: `9cm` stores 0.089999996 in Blender and 0.090000004 by Null's rule, which is the float32 nearest 0.09. Null's rule stands. Sau makes its own number fields convert as Null does (Sau's decision S64 (1), task 1c-01g), so a value typed in Sau prints back in its file as it was typed.
- **Where Blender 5.3 uses them.** `AREA`, `VOLUME`, `POWER` and `TEMPERATURE` have no property in Blender 5.3's RNA; they are listed so that a property using them has a written form when one appears. `MASS` is used by rigid bodies, cloth, particles and force fields; `FREQUENCY`, `WAVELENGTH` and `COLOR_TEMPERATURE` by node sockets, lights and white balance. A light's `energy` has no RNA unit and takes `W` by Null's own form (section 8), not scaled by the scene, as Blender shows it.
- **Writing back.** A value is printed in the unit its line already uses whenever a decimal in that unit reads back to the same bits (S21), and as a bare number otherwise.

## 7. Identity and references

- **Handles.** Every datablock written in Null has a handle, `@pier`, unique within the project for its kind. The handle is the identity Null uses; the quoted string after it is Blender's display name, which may change freely (`object @pier "Pier"`). A handle may be omitted when the name is a valid handle and the same (`object "Pier"` gives `@pier`).
- **Persistent IDs.** Behind every handle Sau keeps a persistent ID: 128 random bits on every datablock in Blender's data, saved in `.blend` files (Sau's decisions S17, S61, S63 and S64, task 1b-01a). A rename in Sau or in the file never changes it, and neither do saving, reopening, undo and redo. A copy, a duplicate and an appended datablock get a new one; a linked datablock keeps its library's; a library override has its own, and keeps it when Blender resyncs the override with its library. An ID is unique within its library, and a datablock is identified by its library and its ID together, so a datablock linked from a copy of the open file may share its ID with a local one. Data from a file saved before Sau had the field gets an ID derived from the datablock's type and name and, when linked, its library's path, so an unchanged old file gives the same IDs every time it opens. The ID is never written in a `.noll` file: the project's map `.sau/ids.noll` ties each handle to its ID (S61 (2)), and is rebuilt from the open scene when it is missing.
- **Sub-object handles.** Modifiers, constraints, nodes, sockets, bones, shape keys, vertex groups, UV maps, attributes, view layers, action slots and the rest are named inside their owner and addressed by path: `@pier/@bevel`, `@weathered-oak/@bsdf.roughness`, `@rig/@spine-002`. The rules are under "Sub-object handles" below.
- **Across files.** A handle is visible in the whole project, so `material @weathered-oak` in an object file finds the material's file through the project index. Packages are addressed with their name: `@cosmosis-materials:oak`.
- **Linked libraries** (another `.blend` or another Null project) are packages or `link` nodes naming the source and the handles taken from it. Library overrides are written as the overridden values on top of the linked handle.

### Sub-object handles

Task 1b-02a, November 7, 2026. A sub-object is a named part of a datablock that is not a datablock itself. Its handle is written where it is defined, like a datablock's (`modifier @bevel bevel "Bevel" { … }`), and is the identity Null uses for it; the quoted string is Blender's name, which may change freely.

- **Made from the name.** A new sub-object's handle is its Blender name in kebab-case, as for datablocks (S18): `Spine.002` gives `spine-002`, `Base Color` gives `base-color`.
- **Unique within its owner, across kinds.** No two sub-objects of one owner share a handle, whatever their kinds, so `@pier/@bevel` names one thing without saying whether it is a modifier or a constraint. Blender lets a modifier and a constraint of one object share a name; the second then gets `-2` (`@bevel-2`), as Blender itself numbers names. A duplicate handle within an owner is an error.
- **Paths.** `/` steps from an owner to a sub-object, and may step again where a sub-object owns others (`@rig/@spine-002/@ik`, a bone's constraint on the object's pose). `.` followed by a name names a socket of a node (`@bsdf.roughness`); `.` followed by a number is a copy made by a pattern (`@plank.7`, Sau's decision S61 (6)). Nothing else uses `.`. A path starts at a datablock handle; inside the owner's own block the owner may be left out (`@bevel`). There a path starts at that owner's own sub-objects first, so a datablock elsewhere with the same handle never shadows them, and the owner is that one datablock, its kind and handle (an object and a mesh may both be `@pier`); a path whose first step names none of the owner's sub-objects is read as written, from a datablock; once it names one, the path stays the owner's and its errors are the owner's.
- **Lines and the checker.** A sub-object is defined by a line `<kind> @handle …` in its owner's block: `modifier`, `constraint`, `bone`, `bone-collection`, `shape-key`, `vertex-group`, `uv-map`, `attribute`, `view-layer`, `slot`, `layer`, `layer-group`, `strip`, `track`, `marker`, `input`, `output` or `panel`, and `node @handle <kind>` in a `nodes` block. The checker indexes them, resolves every path against them, and reports a path that names nothing with the path written right (`@pier/@bevle`: write `@pier/@bevel`). A pattern's copy and a package's datablock parse, and resolve once patterns (task 1d-03) and packages (task 1d-04) are read. Each kind's own properties are checked once its form is specified (task 1d-07). Task 1b-02c.
- **What keeps identity across a rename.** Where Blender keeps an identity of its own that survives renaming, Sau maps the handle to it. Where Blender identifies the sub-object only by its name, Sau keeps the handle through the project's map, `.sau/ids.noll`, keyed by the owner's persistent ID (Sau's decision S61 (3), task 1b-02b). Either way a rename in Sau changes the quoted name and keeps the handle and every reference.

| Sub-object | Owner | Blender's own identity (checked in Blender 5.3's DNA) | Kept by |
| --- | --- | --- | --- |
| Modifier | Object | `ModifierData::persistent_uid` (`int`), unique in the object, saved in files; renaming keeps it | Blender's field |
| Node | Node tree (a material's, world's, light's or a node group) | `bNode::identifier` (`int32_t`), unique in the tree, saved; renaming keeps it | Blender's field |
| Node socket | Node | `bNodeSocket::identifier` (a string, unique in the node), fixed by the node's declaration; written by its name in kebab-case (`base-color`) | Blender's field |
| Group input or output, group panel | Node group | `bNodeTreeInterfaceSocket::identifier` (a string such as `Socket_3`) and the panel's `identifier` (`int`), saved; renaming keeps them | Blender's field |
| Shape key | Mesh, curve or lattice (through its `Key`) | `KeyBlock::uid` (`int`), unique in the key, saved | Blender's field |
| Action slot | Action | `ActionSlot::handle` (`int32_t`), unique in the action, saved | Blender's field |
| Constraint | Object, or a bone in the object's pose | The name only (`bConstraint::name`) | `.sau/ids.noll` |
| Bone | Armature (its pose bone in the object shares the name) | The name only (`Bone::name`); `bPoseChannel::session_uid` lasts one session | `.sau/ids.noll` |
| Bone collection | Armature | The name only | `.sau/ids.noll` |
| Vertex group | Mesh, lattice or Grease Pencil data | The name only (`bDeformGroup::name`) | `.sau/ids.noll` |
| UV map, colour attribute, any attribute | Mesh, curves, point cloud | The name only; `CustomDataLayer::uid` serves shape keys alone | `.sau/ids.noll` |
| View layer | Scene | The name only | `.sau/ids.noll` |
| Action layer and strip, NLA track and strip | Action; the animated datablock | The name only (a strip may have none, and gets its handle from its track and position) | `.sau/ids.noll` |
| Timeline marker | Scene or action | The name only, which may be empty or repeated; a marker without a usable name gets its handle from its frame (`@f120`) | `.sau/ids.noll` |
| Grease Pencil layer and layer group | Grease Pencil data | The name only | `.sau/ids.noll` |

Blender gives a modifier its `persistent_uid` from a hash of its name when it is created, and a node its `identifier` at random, so neither is a handle: the handle is still made from the name, and the map ties the two together. A node socket is the one sub-object never renamed by a person outside node groups, so its handle is its name.

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

### Null's forms over RNA

The schema Sau generates from RNA (`schema/null-schema.json`) gives every name, type, unit, range and enum item. A few things are written in a form of Null's own, either because RNA's shape is not how a file says it or because RNA leaves something out. These are the slice's (phase 1a), settled November 7, 2026 by S16, S17, S18, S24 and S35. The checker (`null_check`) holds them in `forms.rs`.

| Form | Means | Why |
| --- | --- | --- |
| `project "Name" { units …; scene @s }` | The project: its name, its units, its scenes. No RNA struct | A project is Cosmósis's, not Blender's |
| `units metric scale=1 length=m rotation=deg` | A scene's `UnitSettings`: the argument is `system`; `scale` is `scale-length`; `length` is `length-unit`, written as a unit (`m`, `cm`, `mm`, `km`, `in`, `ft`) or an item (`meters`); `rotation` is `system-rotation`, as `deg` or `rad` or an item. The full names may be child lines instead. In `project` it applies to every scene | Short forms first (S35); RNA's `unit-settings` is read-only |
| `object @h` and `collection @h` in a `scene` or `collection` | Members: objects and child collections of the scene's master collection, or of the collection. A `collection` line with a display name or a block defines the collection there | RNA's `objects` and `children` are read-only collections |
| `data mesh @h` in an `object` | The object's data, its kind written before the handle (`mesh`, `camera`, `light`) | RNA's `data` points to any ID; the kind makes the line readable and checkable |
| `material @h` in an `object` | One material slot, in order; the first is the active material | RNA keeps materials in slots |
| `primitive <kind> settings…` in a `mesh` | The mesh as made by Blender's add operator of that kind (`cube`, `uv-sphere`, …) with the operator's settings by their names. `size` is one length, and a primitive's proportions come from the operator's `scale` (unitless, default `[1 1 1]`), which shapes the mesh: `primitive cube size=1m scale=[1.2 3 0.08]` is a 1.2 m by 3 m by 0.08 m box with its object's scale left at 1, so the object's modifiers (a 4 mm bevel stays 4 mm) and children are not squashed. The cube, UV sphere, ico sphere, cylinder and cone take `scale`; the plane, circle, grid, monkey and torus do not (Blender 5.3's operators ignore it or have none) | Level 1 geometry (section 9); the operators are RNA's |
| `vertices [[x y z] …]` and `polygons [[0 1 2] …]` in a `mesh` | Level 0 data: each vertex's `co`, and each polygon's `vertices`, three or more indices of existing vertices | A collection of one-value structs is written as a list of the values. The name is RNA's `polygons`, not `faces` (S16) |
| `nodes { node @h <kind> { inputs } }` and `layout { @h [x y]; … }` in a `material` | The material's node tree: each node's handle, kind and input sockets by name, a socket taking a value or a link `@node.output`; `layout` holds positions, one node per line or `;` | Section 10 |
| `energy` of a point, spot or area light | A power, in `W` | RNA gives `energy` no unit; Blender shows watts. A sun's `energy` is an irradiance, written bare |
| A light's properties | `Light`'s, and those of its `type`'s struct (`point-light`, `sun-light`, …) | RNA splits lights by type |
| `fps 24` and `fps-base 1.001` on a `scene` | The scene's frame rate, its render settings' `fps` (a whole number, 1 to 32767) and `fps-base` (0.00001 to 1000000), a new scene's being 24 and 1 | Seconds on the scene's frame properties are read through it (section 6, "Frames"); it is the scene's, so it is written there, as `units` is |
| `lens` of a camera | A focal length, in `mm`: `50mm` is 50 | RNA's camera unit is millimetres, not a length |

A property's default, which the formatter omits (section 15), is its value in a new datablock: the schema's `new` where it differs from RNA's own default (a new scene's units are `metric` and `meters`).

Handles are kebab-case wherever they appear, defined or referenced (S16, S18). Enum values are the items' kebab-case names; `XYZ` is an error whose fix is `xyz`.

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

The layout is fixed (settled November 7, 2026, task 1a-07):

- two spaces of indentation per open block and per open list, with a closing `}` or `]` that starts a line one level less;
- one space between entries, none just inside `[ ]`, one on each side inside a `{ }` that stays on one line, none before `;` and one after it;
- whitespace around `=` as written, since it can change what parses;
- no trailing whitespace, a comment's included, and lines that end in LF (a CRLF file's become LF);
- a line a `\` continues indented one level deeper than its attribute;
- at most one blank line in a row, none at the start of the file, and one line feed at its end.

Formatting keeps every comment and every other token as written.

## 16. Live reload

Changes are live in both directions, all the time, the way every query in a Convex app updates the moment its data or code changes (Sau's roadmap, design decision 9). Each change, from any source, is one checked, atomic transaction with a revision number, and every view (viewport, editors, code views, the language server's clients and the gateway's subscribers) follows the same revision.

There is no run step and no separate save step (the user, November 6, 2026: changes "with hot reload open and autosave, rather than making scripts"). VS Code is the editor (Sau's decision S44): Sau's extension sends each change of the open buffer to Sau through the language server, and Sau applies it as soon as it is valid, without a save, at a pace the scene can take. Saving a file in any other editor, an AI's edit or a `git checkout` makes Sau, while it is open:

1. read and parse the changed files off the main thread;
2. check them against the schema; on any error, keep the last good state, show the diagnostics in the code view and the status bar, and stop;
3. compare the new tree with the last applied one by handle, producing the list of created, deleted and changed datablocks and properties;
4. apply the changes to `Main` on the main thread through RNA, so Blender's own update callbacks and dependency-graph tags run, as one undo step named after the file;
5. redraw the affected editors.

Edits Sau makes itself (section 17) are recognised by their content hash and not reloaded.

## 17. Write-back

Every edit in Sau's interface (a drag, a slider, an operator, a node link) ends in an undo push. At that moment Sau already knows, from Blender's undo system, which datablocks changed. For each one it prints the new Null form, compares it with the current text of its node, and applies the smallest edit to the lossless syntax tree, keeping comments, ordering and untouched lines. The smallest edit is per value (task 1a-07):

- a value that still reads back to the same bits is left exactly as written;
- a changed value is rewritten in the unit its word already uses, whenever a decimal in that unit reads back to the new bits, and bare in the internal unit otherwise (S21);
- a list of another length is replaced whole;
- an attribute the datablock does not have yet is added as one new line in its block, in the project's display unit, or as `name=value` on its line when it has no block; a new line goes after a comment on the block's opening line, and ends as the file's lines end (LF or CRLF). The file is written atomically. Edit mode and sculpt mode write their data when the mode is left, and while inside, after a short pause. A file changed outside while an edit is pending is merged three-way, like an editor's reload prompt; if both sides changed the same value, Sau asks.

## 18. Diagnostics

Every message names the file, line and column, says what was expected, and where possible offers a fix: an unknown property suggests the nearest RNA name, a wrong unit says which units fit, a handle that resolves to nothing lists the near matches. Codes are stable, so tools and AI can act on them: `null-syntax` (the text does not fit the syntax), `null-name` (a name, handle or enum value that is not kebab-case), `null-kind` (a datablock, node or primitive kind the schema does not know), `null-unknown-property`, `null-type` (a value of the wrong type or shape), `null-unit`, `null-range` (outside the hard range), `null-enum`, `null-handle` (resolves to nothing, to the wrong kind, or is defined twice), `null-readonly` and `null-repeated`.

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
