# Null: Sau's project language

Draft v0, November 6, 2026. Phase 1a's slice (Sau's roadmap) was built on it by November 8, 2026; what the slice changed in this draft, and why, is section 22. On November 8, 2026 the draft was made complete for v0 (task 1d-01a): every section was read against the implementation and says what is built, or which task or phase builds it; every example checks clean; section 20 has nothing open. Freezing it, after which changes go through RFCs, is task 1d-01c. The name Null and the `.noll` extension were chosen by the user on November 6, 2026 (section 2). It lived in Sau as `docs/NULL.md` until it moved to this repository on November 6, 2026, as Scowr's spec moved out of Crescend. Where it fits in the product: Sau's `docs/ROADMAP.md`.

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
| Turing-complete? | No, on purpose: total (Sau's decision S38). Patterns applied any number of times in one line, loops over counts and lists, conditions, arithmetic, parameters, and recursion that counts down to an end (section 8, "Components, parameters and patterns"). Unbounded computation belongs to geometry nodes, drivers and generators | Every file always finishes loading, means one thing, and can be diffed and written back. Abstraction keeps files short. |
| Garbage-collected? | No. The toolchain is Rust and the engine Blender's C++ | Nothing pauses the viewport. |

## 4. A first look

A project of seven files. Every example in this spec checks clean against the schema of all project data (`null_check`'s `spec_examples` test), so what is written here is what the checker takes. An example of a form the checker does not read yet is fenced `null (<task>)`, naming the task that builds it, and joins the test when that task lands.

```null
// project.noll
project "Harbor at Dusk" {
  units metric scale=1 length=m rotation=deg
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
  render {
    engine cycles
    resolution-x 1920
    resolution-y 1080
  }
  cycles { samples 512; use-denoising }

  collection @harbor "Harbor" {
    object @pier
    object @harbor-lamp
    object @shot-cam
    collection @boats "Boats"
  }
}

world @dusk "Dusk" { color [0.02 0.03 0.06] }
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
    relative-offset-displace [1 0 0]
  }
  modifier @bevel bevel { width 4mm; segments 3 }
  material @weathered-oak
}

mesh @pier-deck "PierDeck" {
  primitive cube size=1m scale=[1.2 3 0.08] // level 1: a procedure
}
```

```null
// objects/shot-cam.noll
object @shot-cam "ShotCam" {
  location [9m -12m 3m]
  rotation-euler [80deg 0deg 37deg]
  data camera @shot-cam-lens
}

camera @shot-cam-lens "ShotCam" { lens 50mm }
```

```null
// objects/harbor-lamp.noll
object @harbor-lamp "Lamp" {
  location [0.5m -8m 2.5m]
  data light @harbor-lamp-light
}

light @harbor-lamp-light "Lamp" {
  type point
  energy 800W
  action @lamp-flicker slot=@lamp
}
```

```null
// materials/weathered-oak.noll
material @weathered-oak "Weathered Oak" {
  nodes {
    node @tex tex-image {
      image @oak-diffuse
      interpolation "Cubic"
    }
    node @bsdf bsdf-principled {
      base-color @tex.color // a link: the socket takes another node's output
      roughness 0.62
    }
    node @out output-material { surface @bsdf.bsdf }
  }
  layout { @tex [-420 0]; @bsdf [-160 0]; @out [120 0] }
}

image @oak-diffuse "oak_diffuse.png" {
  filepath "//media/oak_diffuse.png"
  colorspace-settings { name "sRGB" }
}
```

```null
// actions/lamp-flicker.noll
action @lamp-flicker "LampFlicker" {
  slot @lamp light "Lamp" {
    fcurve energy {
      key 1 800W interpolation=bezier
      key 6 620W
      key 11 800W
    }
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
- Scowr reads `at`, `to` and `len` before a value as keyword attributes. Null has no keywords of its own: the checker reads a keyword attribute as two arguments, the word and then its value, so `constraint @aim damped-track to @target` means what the same line with any other word would (task 1a-01).

What Null adds to Scowr's syntax is only schema: the node names, the value forms of section 6 and Blender's kinds.

## 6. Values

- **Numbers** are written in decimal and read exactly into the property's own type: float32, float64 or an integer, as DNA stores it. The formatter prints the shortest decimal that reads back to the same bits, so `0.1` stays `0.1` and never becomes `0.10000000149`. `inf` and `-inf` are written as such. NaN is written `nan`, the quiet NaN, without its sign or payload: the one value that does not read back to the same bits (S59), except in a `raw` block, where a float may be written as its bits (section 19). A value outside a property's hard range, an infinity included, is written and read as it is, since a `.blend` can hold one in DNA though RNA clamps what is set: the checker says so in a `null-range` note, not an error, and Blender clamps it when Sau sets it (the lead's call on task 1c-02b).
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
| Integer | A whole number in decimal, which may have an exponent (`1e3`); outside the property's hard range a note ("Numbers"), but past 32 bits an error, since Blender cannot store it; never a unit | `samples 512`, `frame-start 1` |
| Float | A number as above ("Numbers"), with a unit where the property has one ("Units") | `roughness 0.62`, `lens 50mm` |
| Enum | One item's identifier in kebab-case; an item whose name is free text, not an identifier (a colour space's), as a string (S16), and so is every item an enum's items at run time give that is not one the schema lists (an attribute's or a UV map's name), so it is never read as a misspelling of a listed one | `rotation-mode xyz`, `type point`, `name "ACES 1.3 sRGB"` |
| Enum flag set | A list of item identifiers, in the schema's item order; `[]` for none. Another order, or an item twice, is an error whose fix is the schema's order: the order is part of the written form, so the same set is always the same text | `delimit [normal material]` (a decimate modifier), `delimit []` |
| String | A quoted string, which is UTF-8. A string Blender holds that is not valid UTF-8 (an old file path, a name from another system) is written as the list of its byte values instead, so every byte is written exactly: the schema says the property is a string, so a list there means its raw bytes (Sau's decision S65). The list is taken wherever a string goes, a datablock's display name included (`object @pur [80 252 114]`, `Pür` in Latin-1), and is held to the field's length in bytes as a string is (task 1c-02g). A string that is valid UTF-8 is always written as a string, and a byte list that is valid UTF-8 is an error. A string longer than Blender's field for it is an error, never cut; the length counts bytes, the closing NUL included (a field of 64 holds 63). A 0 byte, in a string or a byte list, is an error, since Blender's strings end at one | `name "Pier"`, `filepath "//textures/oak.png"`, `filepath [47 47 109 252 108 108 101 114 46 112 110 103]` (`//m`, the Latin-1 byte 252, `ller.png`) |
| Pointer | A handle (section 7); `none` for an empty pointer where the property allows one. A struct the owner holds that is not a datablock, and that no form of Null's flattens (an add-on's settings, such as a scene's Cycles settings, or a struct such as `render`), is a block of that struct's properties. A property a flat form already sets is an error in such a block, with a fix that moves it there: a scene's `fps` is never `render { fps 30 }`, and its unit settings are `units`, never `unit-settings { … }` (S70: one spelling per property) | `camera @shot-cam`, `parent none`, `cycles { samples 64 }` |
| Collection | Its items as child lines, in order, each written as its struct is; a collection of one-value structs as a list of the values (section 8, `vertices` and `polygons`) | `material @oak` lines, `vertices [[0m 0m 0m] …]` |
| Array | A list of the property's length, each item written as the single value would be; a dynamic array of any length from its minimum | `location [0m 0m 1m]`, `color [0.8 0.62 0.41 1]` |
| Multi-dimensional array | Nested lists, the first dimension outermost, as RNA's dimensions are listed | `[[1 0] [0 1] [0 0]]` for dimensions `[3 2]` |
| Matrix | Row by row, as a person reads it and as Python's `mathutils.Matrix` prints it: `[[row 1] [row 2] …]`. DNA and RNA's flat array keep a matrix column by column, so row `i`, column `j` is RNA's element `j × rows + i` | `[[1 0 0 2m] [0 1 0 0m] [0 0 1 0m] [0 0 0 1]]` (a translation of 2 m in x) |

- **Units in arrays and matrices** go on each item that has one, as in `location [0m 0m 1m]`; in a matrix only the translation column's items are lengths. That is a 4×4 transform matrix (RNA subtype `MATRIX`, such as `matrix-world`): the first three rows' last items are read and printed as `location` is, in the display unit and through the scene's unit scale, and every other item is bare (task 1b-03e). Any other two-dimensional array (`bound-box`) takes its property's unit on every item.
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
- **Blender's own fields round differently.** Blender's unit factors are float32 literals (`0.01f` for a centimetre, 0.555555555555 for °F's 5/9), so a value typed into a Blender number field with a unit is converted inexactly and often stores other bits than Null's rule gives. Of 2,000 typed values each, as measured in Sau for task 1c-01a's review, this happens for 526 in `cm`, 1,251 in `mm`, 329 in `ft`, 286 in `in` and 169 in `mi`: `9cm` stores 0.089999996 in Blender and 0.090000004 by Null's rule, which is the float32 nearest 0.09. Null's rule stands. Sau makes its own number fields convert as Null does (Sau's decision S64 (1), task 1c-01g), so a value typed in Sau prints back in its file as it was typed: a lone number and unit typed into a field, or given to `bpy.utils.units.to_value`, is converted exactly by this rule, whether the unit is written in any of Blender's spellings for it (`9cm`, `9 centimeters`, `9CM`, `3'` for feet, `2 sq ft`), found as Blender finds it, or in a word of Null's that Blender has no spelling for (`90deg`, `2ft2`), which Blender's own fields would refuse. An expression (`2cm + 1in`, `1m50cm`), a unit Null leaves out (`9hm`) and anything else are still converted by Blender's float32 rule.
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
- **Lines and the checker.** A sub-object is defined by a line `<kind> @handle …` in its owner's block: `modifier`, `constraint`, `bone`, `bone-collection`, `shape-key`, `vertex-group`, `uv-map`, `attribute`, `view-layer`, `slot`, `layer`, `layer-group`, `strip`, `track`, `marker`, `input`, `output` or `panel`, and `node @handle <kind>` and `zone @handle <kind>` in a `nodes` block, the nodes inside a zone's block among the tree's own (section 10). The checker indexes them, resolves every path against them, and reports a path that names nothing with the path written right (`@pier/@bevle`: write `@pier/@bevel`). A pattern's copy and a package's datablock parse, and resolve once patterns (task 1d-03) and packages (task 1d-04) are read. A modifier's, a constraint's, a pose bone's, a node's, a zone's, an interface socket's, a slot's, a layer's, a strip's and an NLA track's own properties are checked (task 1d-07a, sections 8, 10 and 11); the other kinds' once their forms are specified (task 1d-07). Task 1b-02c.
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

### The handle map

The project's map of handles to persistent IDs is `.sau/ids.noll`: Null text that Sau writes, kept in Git with the project (Sau's decisions S61 (2) and S66; task 1b-01f). Opened from its text, a project gives every datablock the same ID every time, on every machine. It is not a project file the checker reads as datablocks (section 4's examples are), so its example is fenced as text.

```text
// Sau's map of handles to persistent IDs (decisions S61 (2) and S66). Sau writes this file;
// keep it in Git with the project.
ids {
  mesh @cube-mesh "1f0c39a2d64e5b7780c1aa0e95f3d2b4"
  object @cube "4b43ea435eaf647774ed7d63bbb80432"
}
```

- **One `ids` block** holds one entry per handle: the datablock's kind (section 8), its handle, and its ID as a string of 32 hexadecimal digits, the 16 bytes Blender keeps in order. Sau writes them in lower case; upper case reads the same. All zeros means no ID and is not an entry.
- **Sorted** by kind and then handle, so a change shows as the lines it adds or removes. Sau rewrites the whole file, atomically, after an apply that changed it, and never through a link. A comment written by hand is not kept.
- **What applying does with it.** A handle the map holds takes that ID: the datablock with it in the file, or a new one given it. A handle it does not hold gets a new random ID, which the next write records. With the map gone, a project still opens, with new IDs and a new map.
- **What is left out.** An entry of another form, a handle given twice and an ID given twice are reported and left out, the first of two kept; the handle then gets a new ID.
- **Sub-objects later.** Sub-objects Blender identifies only by name keep their handles here too, keyed by their owner's ID (S61 (3), the table above; task 1b-02b). Their form is reserved as a block on the owner's entry, `object @rig "…" { bone @spine "Spine.002" }`, which a reader of the map today ignores.

## 8. The objects

Every one of Blender's datablock types that holds project data has a kind, generated from RNA (Sau's tasks 1b-03b and 1b-03c), and so does every node, modifier, constraint, strip and other struct of a family. One rule names them all, with no exceptions beyond the slice's core (Sau's decision S67):

- **The rule.** A kind is the Blender type identifier with its family's affix removed, and the rest in kebab case, a capital starting each word and a digit staying with the word before it (`Tool3DCursor` is `tool3d-cursor`, `View3DShading` is `view3d-shading`, `CryptomatteV2` is `cryptomatte-v2`). The affix is the longest identifier of a struct it extends that it starts or ends with, `ID` never counting: `ShaderNodeBsdfPrincipled` is `bsdf-principled`, `BevelModifier` is `bevel`, `CopyLocationConstraint` is `copy-location`, `ColorStrip` is `color`, `BlendTexture` is `blend`. With no such affix, the whole identifier is the kind: `PointCloud` is `point-cloud`, `FreestyleLineStyle` is `freestyle-line-style`. The name can be computed both ways, so the schema maps each kind back to its identifier.
- **Where two meet.** Kinds read in one place (the datablock types; a node tree; a family such as modifiers) are unique there. Where two would share a name, the one of that place's own family, or the one with no affix, keeps it, and the other keeps its affix's word: in a compositor tree `CompositorNodeSwitch` is `switch` and `GeometryNodeSwitch` is `geometry-switch`; among datablocks `Image` is `image` and `ImageTexture` is `image-texture`. A kind left starting with a digit keeps its affix's word too, since a name starts with a letter (S16).
- **The core.** The slice's kinds keep their names: `project`, `scene`, `units`, `collection`, `object`, `mesh`, `mesh-vertex`, `mesh-polygon`, `material`, `camera`, `light`, `point-light`, `sun-light`, `spot-light`, `area-light`. The slice's `principled-bsdf` follows the rule and is `bsdf-principled`.
- **The datablock types** by this rule, as Blender 5.3 has them: `action`, `annotation`, `armature`, `cache-file`, `camera`, `collection`, `curve`, `curves` (hair), `surface`, `text-curve`, `grease-pencil`, `image`, `key` (shape keys), `lattice`, `library`, `light` and `point-light`, `sun-light`, `spot-light`, `area-light`, `light-probe` and `plane`, `sphere`, `light-probe-volume`, `mask`, `material`, `mesh`, `meta-ball`, `movie-clip`, `node-tree` and `shader`, `geometry`, `compositor`, `texture-node-tree`, `object`, `paint-curve`, `palette`, `particle-settings`, `point-cloud`, `scene`, `sound`, `speaker`, `text`, `texture` and `blend`, `clouds`, `distorted-noise`, `image-texture`, `magic`, `marble`, `musgrave`, `noise`, `stucci`, `voronoi`, `wood`, `vector-font`, `volume`, `world`. The interface's types (screens, workspaces, the window manager, brushes) are not project data; they belong in `view.noll`, settings and packages.
- **Add-ons' settings.** Cycles is bundled with Blender and on by default, and a project's render, object, light and world settings for it are its own Python-defined properties, so the project's schema carries them as a section of their own (Sau's task 1b-03g): the properties Cycles adds to Blender's structs (`Scene.cycles`, `Object.cycles`, `Light.cycles`, `World.cycles` and others) and every struct of its they reach, with Blender's version, since Cycles is bundled. They are read as Blender's own are, a scene's as `cycles { samples 64; device gpu }`. A schema made with Cycles off is the same but for that section. Other add-ons wait for packages.
- **Nodes** are read in their tree: the schema lists, for each node, the trees that take it and its kind in each. Its sockets come from the node's declaration in the running app, not RNA, each with its type, unit, range and default. A socket a node shows only for some of its settings is marked `enabled: false` (at the node's defaults); the empty socket that adds an item when linked, as a menu switch's or a repeat zone's lists grow, is `virtual`; a socket that takes several links is `multi-input`; one with no value of its own is `hide-value`; a menu socket's items are its node's at run time. Where a node's sockets differ between trees, the schema gives each tree's.

Each object kind's properties are the RNA properties of its struct, by the same names. The generated field reference, one table per struct, is an appendix produced from the schema at build time; it is never written by hand.

### Null's forms over RNA

The schema Sau generates from RNA gives every name, type, unit, range and enum item: `schema/null-schema-project.json` for all project data (every struct the datablock types own, with every node, modifier, constraint and strip kind; Sau's task 1b-03b), and `schema/null-schema.json` for phase 1a's slice. A string whose field Blender fixes gives its length in bytes, the closing NUL included, as `max-length` (Sau's task 1b-03h; `ID.name` is 256), and the checker refuses a longer one, a display name included; a string of any length has none. An enum whose items Blender computes when it is used is marked `items-at-run-time`: the checker takes a listed item, refuses a value near one as a misspelling, and leaves any other to Blender, so a value far from every listed item (`instance-type zzzzzz`) passes the checker unseen and is refused, if at all, only when Sau applies it. A struct's kind follows S67's rule above (`bsdf-principled`), the slice's own names kept. A few things are written in a form of Null's own, either because RNA's shape is not how a file says it or because RNA leaves something out. These are the slice's (phase 1a), settled November 7, 2026 by S16, S17, S18, S24 and S35. The checker (`null_check`) holds them in `forms.rs`.

| Form | Means | Why |
| --- | --- | --- |
| `project "Name" { units …; scene @s }` | The project: its name, its units, its scenes. No RNA struct | A project is Cosmósis's, not Blender's |
| `units metric scale=1 length=m rotation=deg` | A scene's `UnitSettings`: the argument is `system`; `scale` is `scale-length`; `length` is `length-unit`, written as a unit (`m`, `cm`, `mm`, `km`, `in`, `ft`) or an item (`meters`); `rotation` is `system-rotation`, as `deg` or `rad` or an item. The full names may be child lines instead. In `project` it applies to every scene | Short forms first (S35); RNA's `unit-settings` is read-only |
| `object @h` and `collection @h` in a `scene` or `collection` | Members: objects and child collections of the scene's master collection, or of the collection. A `collection` line with a display name or a block defines the collection there | RNA's `objects` and `children` are read-only collections |
| `data mesh @h` in an `object` | The object's data, its kind written before the handle (`mesh`, `camera`, `light`, `armature`) | RNA's `data` points to any ID; the kind makes the line readable and checkable |
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

### Modifiers, constraints and pose bones

Task 1d-07a. An object's modifiers, constraints and pose bones are sub-objects (section 7), written in its block:

- `modifier @h <kind> "Name" { … }` is one modifier, in stack order. Its kind is a modifier's (S67: `bevel`, `array`, `subsurf`, `nodes`), and its block holds that kind's properties and those every modifier has (`show-viewport`, `show-render`). A geometry nodes modifier (`nodes`) names its group with `node-group`, and its group's inputs are its lines by their handles (section 10).
- `constraint @h <kind> "Name" { … }` is one constraint, in stack order, of a constraint kind (`damped-track`, `copy-location`, `kinematic` for inverse kinematics).
- `bone @h "Name" { … }` is the pose of the armature's bone of that name: `PoseBone`'s properties (`rotation-mode`, `location`, the locks and the IK limits) and its own `constraint` lines. An armature's bones themselves are its data's (1d-07h).

The quoted name is Blender's and may be left out, the handle then standing for it.

```null
object @lid "Lid" {
  data mesh @lid-mesh
  modifier @bevel bevel "Bevel" { width 4mm; segments 3 }
  modifier @stacked nodes "Stacked" {
    node-group @stack-up
    count 3
  }
  constraint @aim damped-track "Aim" {
    target @dial
    track-axis track-negative-z
  }
}

mesh @lid-mesh "Lid" { primitive cube size=1m }

object @rig "Rig" {
  data armature @rig-bones
  bone @spine "Spine" {
    rotation-mode xyz
    constraint @reach kinematic "IK" {
      target @lid
      chain-count 2
    }
  }
}

armature @rig-bones "Rig"
```

### Components, parameters and patterns

Task 1d-03a, November 8, 2026 (Sau's decisions S19, S32 to S35, S38, S54, S61 (6) and S74). Parsing and checking these forms is 1d-03b, and expanding them 1d-03c, so the examples are fenced `null (1d-03b)`. A component is a part made once and placed many times, as a React component is (S19). Each placement may differ from the others in any regard, and each copy keeps a handle of its own.

- **A component** is a top-level `component @h "Name" { … }` in any file. Its block holds its parameters and the datablocks it makes. The first `object` in it is its **root**, the object a placement puts in the scene; the rest (its data, materials made for it, child objects) come with it. A datablock the component names but does not define (`material @oak`) is the project's, shared by every placement, never copied.
- **Parameters** are `param <name> <default>`, typed by the default: a length (`2.4m`), an angle, a number, a whole number, `on` or `off`, a handle of a kind (`param finish material @oak`), or a word from a list (`param state solid { one-of [solid liquid] }`). A parameter with no default (`param length length`, naming its type) must be given by every placement. Names are kebab-case and local to the component.
- **Arithmetic.** Wherever a value goes, a word may be an expression over numbers with units, parameter names and `+`, `*` and `/`, with the usual precedence: `length/1m`, `n*25cm`, `width*0.5`. Subtraction, grouping and comparisons are written as a quoted expression, with spaces around each operator: `"length - 10cm"`, `"(n - 1) * 25cm"`, `"state == liquid"`. A `-` inside a word is part of a name, since names are kebab-case, and Scowr reserves `( )` (S15, S74 (3)). Units follow section 6: an expression's value is computed exactly and rounded once into the property's type (S59), and a length times a length is an error where a length goes.
- **Placement.** `use @component @handle { … }`, in a scene or a collection, places one copy. Its root object becomes `@handle` and joins that scene or collection. The block gives parameters by name and sets the root object's own properties (`location`, `rotation-euler`, `parent`); a name is a parameter's first. A component from a package is placed the same way, `use @cosmosis-materials:oak-plank @deck` (S54).
- **Patterns.** `use @component @handle count=N { … }` places N copies in one line (S34). The copies' handles are `@handle.1` to `@handle.N`, counted as the pattern counts (S61 (6)), and in the block `n` is the copy's number. `use … each=[v1 v2 …] as=<name>` places one copy per item of a list, the item named `<name>` in the block, the copies numbered in the list's order.
- **The handles a copy makes.** The root is `@handle` or `@handle.n`. Every other datablock the component defines is found through it as a path (S74 (4)): `@plank.7/@plank-mesh` is copy 7's mesh. A copy's sub-objects keep the component's handles (`@plank.7/@fluid`).
  - **A component's own handles are local.** Inside its block, `@plank` names the component's root and `@plank-mesh` its mesh. They are not the project's handles, and the component's datablocks exist only through placements: no object `@plank` is made from the template itself. So the pier's component, its root and its placement may all be `@plank` (a component is a kind of its own, and handles are unique per kind).
  - **Copies inside a copy** are found through it as paths, as its other datablocks are. In `use @branch @tree`, the root is `@tree`, its own `use @branch @left` makes `@tree/@left`, and that copy's makes `@tree/@left/@left`. A pattern inside a copy gives `@tree/@rail.3`. An override or a write-back names a nested copy by that path (`override @tree/@left/@right { length 40cm }`).
- **Conditions.** `if "<condition>" { … }`, in a component's block or any block inside it, holds lines that apply only when the condition, a comparison of parameters, holds. `else { … }` may follow. A condition is known when the component is placed, so each copy is plain data.
- **Overrides** (S33). `override @plank.7 { … }`, beside the `use` line, gives one copy other parameters or properties. `override @plank.7/@plank-mesh { … }` reaches a datablock the copy made. An edit in Sau to a placed copy is written as an override on it by default, and to the component itself only on request (1d-03f), as Figma's instances and main components are edited.
- **Recursion that counts down** (S38, S74 (5)). A component may place itself, directly or through others, only when one whole-number parameter is marked `{ countdown }`, every such placement passes it a smaller value, and the placements sit under an `if` that holds only while it is above 0. The checker refuses any other recursion.

**Expansion** (S32). On load, after parsing and before checking, every placement is expanded into the datablocks it makes. Parameters are put in, conditions are decided, patterns are unrolled, and overrides are laid over each copy. The checker then checks that flat project, as if it had been written out, and Sau applies it as real Blender data. Copies whose data comes out identical share one datablock underneath, through Blender's instancing (1d-03e), and the handles stay one per copy. Expansion is deterministic: the same text gives the same datablocks, handles and order every time. Write-back keeps the abstract form (S35): a moved copy writes one `override` line, never the expanded data.

**Why every file finishes loading** (S38). Null is total, not Turing-complete:
- a pattern's `count=` is a whole number and its `each=` a list, both known before expansion;
- a condition only picks between lines;
- recursion is allowed only on a `countdown` parameter that every recursive placement makes smaller and that stops at 0, so every chain of placements ends.

Expansion therefore always ends. As a guard against a count that is merely huge, a project that expands to more than 1,000,000 datablocks is an error, reported at the line that asked for them, never a hang. Computation without a bound lives in geometry nodes (repeat, simulation and for-each zones) and in generators (section 13).

The pier of twenty planks, plank 7 liquid:

```null (1d-03b)
// components/plank.noll
component @plank "Plank" {
  param length 2.4m
  param state solid { one-of [solid liquid] }
  object @plank "Plank" {
    data mesh @plank-mesh
    if "state == solid" { material @weathered-oak }
    else {
      material @water
      modifier @fluid fluid { fluid-type flow }
    }
  }
  mesh @plank-mesh "Plank" {
    primitive cube size=1m scale=[length/1m 0.2 0.04]
  }
}
```

```null (1d-03b)
// objects/pier.noll
collection @pier "Pier" {
  use @plank @plank count=20 {
    location [0m n*25cm 0.35m]
  }
  override @plank.7 { state liquid }
}
```

A tree whose branches count down:

```null (1d-03b)
component @branch "Branch" {
  param depth 4 { countdown }
  param length 1m
  object @branch "Branch" {
    data mesh @segment
  }
  mesh @segment "Segment" { primitive cylinder radius=length*0.05 depth=length }
  if "depth > 0" {
    use @branch @left {
      depth "depth - 1"
      length length*0.7
      parent @branch
      location [0m 0m length]
      rotation-euler [0deg 35deg 0deg]
    }
    use @branch @right {
      depth "depth - 1"
      length length*0.7
      parent @branch
      location [0m 0m length]
      rotation-euler [0deg -35deg 0deg]
    }
  }
}
```

## 9. Geometry as source

The 3D counterpart of Scowr's "sound as source" (Scowr spec, section 20). Geometry is written at one of three levels, and each level renders to the one below exactly:

| Level | What it is | Written as | Edited by |
| --- | --- | --- | --- |
| 0. Data | Exact attribute arrays: positions, edges, faces, corners, UVs, normals, weights, creases, colours, any named attribute | Inline lists up to 2,000 elements; a larger array in a Null text file of its own, one element per line, referenced from the text (S23) | The viewport's edit mode, sculpting, and tools; rarely by hand |
| 1. Procedure | Primitives with parameters, modifier stacks, geometry-node graphs, curves and their profiles | Ordinary Null nodes | Hand, AI and Blender's editors alike |
| 2. Lifted | An existing mesh described as a procedure plus an exact residual: "a bevelled box of these dimensions, arrayed 12 times, plus these vertex offsets" | A level 1 description and its residual, written as level 0 data | As level 1; the residual keeps the result exact |

- **Text, at every size** (Sau's decision S23). An array of up to 2,000 elements is written inline. A larger one goes in a Null text file of its own, one element per line, so the files people read stay short, and a diff of an edited mesh shows the elements that changed. `sau convert --inline` writes everything inline whatever its size. The syntax of both forms, of the reference to the file and of its summary is task 1d-06a.
- **The summary.** The line that names the file gives the array's `count=` and `bounds=` as values (S74 (1)). They are checked on load, and a summary that disagrees with the data is an error (1d-06b).
- **Binary only as a cache, or by choice.** Sau keeps a binary copy of large arrays in `.sau/` as a cache. The cache is rebuilt when the text changes and is never the source (1d-06e). Very large scanned data may opt into a binary data file; its documented, versioned format becomes part of this spec when a project first needs it.
- **Images** follow the same ladder: pixels in `media/`; procedural textures as node graphs; lifted, a procedural texture plus a residual.
- **Lifting quality** decides how much of an asset is editable as intent, never how accurate it is. `sau lift` writes a grading report: how much is explained by the procedure and how large the residual is. Lifting is phase 4b of Sau's roadmap; v0 writes levels 0 and 1.

### Level 0: attribute arrays

Task 1d-06a, November 8, 2026 (Sau's decisions S23, S35 and S71). Reading and checking these forms is task 1d-06b, so the examples here are fenced as `null (1d-06b)` and are checked once it lands. A mesh's level 0 data is its attribute arrays, one line each in its block, in the order below.

- **`vertices [[x y z] …]`**: each vertex's position (the `position` attribute, point domain), lengths as `location` is written (section 6).
- **`edges [[a b] …]`**: each edge's two vertex indices, in Blender's order. It is written only when the mesh needs it: an edge no polygon has (a loose edge), or an edge attribute, whose values follow this order. Otherwise Blender makes the edges from the polygons, and the line is left out.
- **`polygons [[a b c …] …]`**: each polygon's vertex indices, three or more, in its corners' order. A mesh's corners are these lists read in order, so a corner attribute has one value for each index written here.
- **`attribute @h <domain> <type> "Name" [values]`**: any other attribute, a sub-object of the mesh (section 7).
  - Its domain is `point`, `edge`, `face` or `corner`, and its type one of `Attribute`'s `data-type` items (`float`, `int`, `boolean`, `float-vector`, `float2`, `float4`, `float-color`, `byte-color`, `quaternion`, `float4x4`, `int8`, `int16-2d`, `int32-2d`, `string`).
  - There is one value per element of the domain, each written as its type's single value is (section 6): a number; `on` or `off`; a list such as `[x y z]` or `[r g b a]`; a `byte-color`'s four integers 0 to 255 as Blender stores them; a `float4x4` row by row; a `quaternion` as `[w x y z]`.
  - The quoted name is Blender's and may be left out when the handle is its kebab-case form. A built-in name that is not kebab-case keeps its quotes (`attribute @crease-edge edge float "crease_edge" […]`).
  - Blender's internal attributes (`.edge_verts`, `.corner_vert` and the others whose names start with `.`) are never written: `edges` and `polygons` are they.
- **`uv-map @h "Name" [[u v] …]`**: a UV map, one `[u v]` per corner, with its settings in a block (`active-render`).
- **`vertex-group @h "Name" { weights [[index weight] …] }`**: a vertex group, a sub-object of the mesh as Blender 5.3 keeps it (section 7), with its weights: only the vertices in the group, by index, in increasing order.
- **Colours, creases, bevel weights, sharp edges and faces, seams and custom normals** are attributes like any other, by Blender's names: `crease_vert`, `crease_edge`, `bevel_weight_vert`, `bevel_weight_edge`, `sharp_edge`, `sharp_face`, `custom_normal`, and a colour attribute of type `float-color` or `byte-color` on `point` or `corner`.

**Inline up to 2,000 elements.** An array of up to 2,000 elements (vertices, edges, polygons, corners, weights, or an attribute's values) is written inline, as above. A list may span lines, and the formatter puts one element on each line once it does not fit on one.

**Larger arrays in a Null text file of their own.** A larger one is written in a file beside the one that holds the mesh, named after the mesh and the array (`rock.vertices.noll`), one element per line. It is not named by its content's hash, so an edit is a readable diff of the lines it changed (S23, S74 (2)):

- The mesh's line names the file, and gives its summary as values, which the checker holds the file to: `count=`, the number of elements, and `bounds=`, the smallest and largest value of each component. A summary that disagrees with the file is an error (1d-06b). It is written as values, not a comment, because a checked value belongs in the syntax tree, and a comment that changed what loads would be the only one in Null (Sau's decision S74 (1)).
- The file holds one node: `array`, the owner's kind and handle, the array's name or handle, and the list:

```null (1d-06b)
// meshes/rock.noll
mesh @rock "Rock" {
  vertices file="rock.vertices.noll" count=12480 bounds=[[-1.2m -0.9m 0m] [1.3m 1m 2.1m]]
  polygons file="rock.polygons.noll" count=12476
  attribute @crease-edge edge float "crease_edge" file="rock.crease-edge.noll" count=24954 bounds=[0 1]
  uv-map @uv-map "UVMap" file="rock.uv-map.noll" count=49904 bounds=[[0 0] [1 1]] { active-render }
  vertex-group @base "Base" { weights [[0 1] [1 1] [2 0.5]] }
}
```

```null (1d-06b)
// meshes/rock.vertices.noll
array mesh @rock vertices [
  [-1.2m 0.4m 0m]
  [-1.1m 0.5m 0.02m]
  …
]
```

- A data file is found by its path from the file that names it. It is not a datablock file: a `.noll` file whose one node is `array` is read only through the line that names it, by Null's checker and Cosmósis's layout alike (1d-06b).
- Its values are written as inline ones are. Sau writes lengths in the project's display unit when they read back exactly, as everywhere (S21), so a moved vertex changes one line.
- `sau convert --inline` writes every array inline whatever its size.

**Binary only as a cache, or by choice.** Sau keeps a binary copy of each large array in `.sau/` as a cache, rebuilt when its text changes, never the source (1d-06e). Very large scanned data may opt into a binary data file (`file="scan.vertices.bin"`); its documented, versioned format becomes part of this spec when a project first needs it.

Shape keys, curves, point clouds and Grease Pencil drawings take the same forms for their own arrays (1d-06f).

## 10. Node trees

Shader, geometry and compositor node trees are written as their nodes and links (task 1d-07a). Texture node trees wait for a row of their own (1d-07b).

- **Where a tree is.** A material's, a world's and a light's tree is its `nodes` block, a shader tree. A node group is a datablock of its tree's kind (S67): `shader`, `geometry` or `compositor`. It holds its interface, its `nodes` and its `layout`, as below. A scene's compositor is the `compositor` group its `compositing-node-group` names, and a geometry nodes modifier names its group with `node-group` (section 8).
- **A node** is `node @handle <kind> "Name" { … }`:
  - Its kind is named by section 8's rule (`tex-noise`, `bsdf-principled`, `transform`), and a node is read in its tree, since the schema gives each node the trees that take it.
  - Its handle is unique in the tree. The quoted name is the node's Blender name and may be left out; then the name is the handle without `@` (Sau's task 1b-02d).
  - Its block holds its **settings**: the node struct's own properties (`operation`, `interpolation`), and of the properties every node has, `label`, `mute`, `hide`, `parent`, `color`, `use-custom-color`, `color-tag`, `show-options`, `show-preview` and `show-texture`.
  - It also holds its **inputs**, by the names the schema gives its sockets, each a value of the socket's type or a link. Where a setting and an input share a name, the name is the input's. A socket with no line takes its default.
  - A node's position and width are the `layout` block's, never its own lines.
- **A link** is a socket value that names another node's output: `base-color @ramp.color`. A socket that takes several links (a join's `geometry`) takes a list of them, `[@a.geometry @b.geometry]`. A geometry, shader, bundle or closure socket has no value of its own, so it takes only links.
- **Frames.** A frame is a node, `node @box frame "Box"`, and a node drawn in it says `parent @box`.
- **A group's interface** is its `input`, `output` and `panel` lines, in the interface's order:
  - `input @h <socket-type> "Name" { … }` and `output @h <socket-type> "Name" { … }`. The socket type is an interface socket's kind (`float`, `float-distance`, `int`, `vector`, `geometry`, `object`, and the schema's others), and the block holds that type's settings (`default-value`, `min-value`, `max-value`, `description`, `hide-value` and the rest).
  - `panel @h "Name" { default-closed; <input and output lines> }`.
  - The handles are the sockets' names: they name the inputs and outputs of a group node using the group, the outputs of the group's own `group-input` node and the inputs of its `group-output` node. Like every sub-object's, they are unique within the group, across inputs, outputs and panels.
- **Group nodes.** `node @stack group { node-tree @stack-up; count 6 }`: a group node's `node-tree` names a group of its tree's kind, and its inputs and outputs are that group's interface.
- **Zones.** A zone is a pair of nodes Blender keeps together, and is written as one block (Sau's decision S71), `zone @h <kind> "Name" { … }`. Its kind is the pair's (`repeat`, `simulation`, `foreach-geometry-element`, `closure`, as the tree takes them). The block holds:
  - its items, each line named after the output node's item collection less its final `s`: `repeat-item @geometry geometry "Geometry"`, `state-item`, `input-item`, `main-item`, `generation-item`, `output-item`. Each item is a handle, a socket type and the item's name, with its settings in a block;
  - the input node's inputs and settings (`iterations 4`), and each item's value as it enters the zone;
  - `output { … }`, the output node's inputs: each item's value at the end of an iteration;
  - the nodes inside the zone.

  Inside the zone's block, `@loop.geometry` is the item as it comes into an iteration (the input node's output). Anywhere else it is the item as it leaves the zone (the output node's).
- **Layout is kept apart** (S45). Node positions live in the tree's trailing `layout` block, `@h [x y]`, so dragging a node in the editor changes only that block, and reading the tree's meaning never needs the layout. A zone's line holds two positions, its input node's and then its output node's. A node with no layout line is placed by Sau.

A material, with a frame:

```null
material @clay "Clay" {
  nodes {
    node @noise tex-noise { scale 4 }
    node @ramp val-to-rgb { fac @noise.fac }
    node @box frame "Surface" { label "Surface" }
    node @bsdf bsdf-principled "Skin" {
      parent @box
      base-color @ramp.color
      roughness 0.6
    }
    node @out output-material { surface @bsdf.bsdf }
  }
  layout { @noise [-600 0]; @ramp [-350 0]; @box [-120 60]; @bsdf [-100 0]; @out [200 0] }
}
```

A geometry node group with an interface, a panel and a repeat zone, used by a group node and by a modifier (section 8):

```null
geometry @stack-up "Stack Up" {
  is-modifier
  input @geometry geometry "Geometry"
  input @count int "Count" {
    default-value 4
    min-value 1
  }
  panel @spacing "Spacing" {
    input @gap float-distance "Gap" { default-value 10cm }
  }
  output @stacked geometry "Geometry"
  nodes {
    node @in group-input
    zone @loop repeat "Repeat" {
      repeat-item @geometry geometry "Geometry"
      iterations @in.count
      geometry @in.geometry
      node @lift transform {
        geometry @loop.geometry
        translation [0m 0m 1m]
      }
      node @join join-geometry { geometry [@loop.geometry @lift.geometry] }
      output { geometry @join.geometry }
    }
    node @out group-output { stacked @loop.geometry }
  }
  layout { @in [-500 0]; @loop [-300 0] [400 0]; @lift [-100 -120]; @join [150 0]; @out [600 0] }
}

geometry @tower "Tower" {
  is-modifier
  input @base geometry "Base"
  output @tower-out geometry "Tower"
  nodes {
    node @in group-input
    node @stack group "Stack Up" {
      node-tree @stack-up
      geometry @in.base
      count 6
      gap 20cm
    }
    node @out group-output { tower-out @stack.stacked }
  }
}
```

A compositor group, which a scene uses as its compositor:

```null
compositor @grade "Grade" {
  input @image color "Image"
  output @graded color "Image"
  nodes {
    node @in group-input
    node @contrast bright-contrast {
      image @in.image
      contrast 10
    }
    node @out group-output { graded @contrast.image }
  }
}

scene @shot "Shot" { compositing-node-group @grade }
```

## 11. Animation and drivers

Task 1d-07a. Actions follow Blender's slotted, layered model, and everything that plays them is written on the datablock that plays.

- **An action** holds its slots: `slot @h <kind> "Name"`, the handle first as every sub-object line's (S71), one for each datablock it animates, of the datablock kind it animates (`object`, `light`, `material`). Its handle is the slot's own; Blender keeps it in `ActionSlot::handle` (section 7).
- **F-curves.** `fcurve <path> index=N { … }` animates one property:
  - The path is a property of the slot's kind of datablock: `energy`, `location`. A light's slot reaches its light type's properties too.
  - `index=` picks an array's item, and is required for an array.
  - A sub-object's property is written as its path and the property (`fcurve @bevel width`), and a node socket as `@bsdf.roughness`. Their keys are bare, in the internal unit, until the checker resolves them through the datablocks that play the action (S71, 1d-07g).
  - The f-curve's block holds its settings (`extrapolation`, `color-mode`, `auto-smoothing`, `mute`, `lock`, `hide`), its keys and its modifiers.
- **Keys** are `key <frame> <value>`, then the key's settings as `name=value`: `interpolation`, `easing`, `type`, `handle-left-type`, `handle-right-type`, `back`, `amplitude`, `period`, and `handle-left` and `handle-right` as `[frame value]`.
  - A frame is a number of frames, exact as Blender stores it (float32, so sub-frames are allowed and print exactly). An action is outside every scene, so a frame is never written in seconds (section 6).
  - The value is written as the property's own: `800W` for a light's energy, `1.5m` for a location.
- **An f-curve's modifiers** are `modifier @h <kind> "Name" { … }` lines in its block, of the f-curve modifier kinds (`noise`, `cycles`, `generator`, `limits`, `stepped`, …).
- **Layers and strips.** An action with one layer and one keyframe strip, the usual case, writes its f-curves in its slots' blocks (S35: the short form first; S71). Its full form writes them under layers instead, and never both:
  - `layer @h "Name" { strip @h keyframe { channels @slot { fcurve … } } }`;
  - each `channels` block holds one slot's f-curves in that strip.
- **What plays.** On any datablock that can be animated:
  - `action @a slot=@s` plays action `@a` through its slot `@s`, which must animate that kind of datablock. `slot=` may be left out when the action has one slot.
  - NLA tracks are `track @h "Name" { strip @h "Name" { action @a slot=@s; frame-start 1; repeat 2; blend-type add; … } }`, in the stack's order, each strip with `NlaStrip`'s settings.
- **Drivers** are written on the property they drive, which then takes no value of its own:
  - `driver=` is the expression in quotes (a scripted driver), or one of `average`, `sum`, `min` and `max`. `index=` picks an array's item, and `use-self=on` lets the expression read its own datablock.
  - The block holds the variables, `var <name> <type> { target … }`. A variable's name is the name the expression uses (letters, digits and `_`), a plain word, not a handle (S71), and its type is one of `single-prop`, `transforms`, `rotation-diff`, `loc-diff` and `context-prop`.
  - `target @handle <property>` reads one property of a datablock. A transforms target reads its datablock's transform, with `transform-type=`, `transform-space=` and `rotation-mode=`. The two kinds of difference take two targets each.
  - Driver expressions stay in Blender's simple-expression subset where possible, so they evaluate without Python. Null's own expression syntax is task 1d-07f.

A light flickering by a short-form action with a noise modifier:

```null
action @flicker "Lamp Flicker" {
  slot @lamp light "Lamp" {
    fcurve energy {
      key 1 800W interpolation=bezier
      key 6 620W
      key 11 800W
      modifier @wobble noise "Wobble" {
        strength 40
        scale 2
      }
    }
  }
}

light @lamp-light "Lamp" {
  type point
  energy 800W
  action @flicker slot=@lamp
}
```

The full form, layered, played twice by an NLA strip:

```null
action @hop "Hop" {
  use-cyclic
  slot @ball object "Ball"
  layer @base "Base" {
    strip @keys keyframe {
      channels @ball {
        fcurve location index=2 {
          extrapolation linear
          key 1 0m
          key 12 1.5m interpolation=sine easing=ease-out
          key 24 0m handle-left=[20 0.4] handle-right=[28 0.4]
        }
      }
    }
  }
}

object @ball "Ball" {
  track @moves "Moves" {
    strip @hop-twice "Hop" {
      action @hop slot=@ball
      frame-start 1
      repeat 2
      blend-type add
    }
  }
}
```

A driver: a lamp whose power follows a dial's turn:

```null
object @dial "Dial" { rotation-euler [0deg 0deg 45deg] }

light @dial-lamp "Dial Lamp" {
  type point
  energy driver="spin * 100" {
    var spin transforms {
      target @dial transform-type=rot-z transform-space=world-space
    }
  }
}
```

## 12. Media and caches

Task 1d-05f builds this section; the slice references no media. Images, videos, sounds, fonts, HDRIs, OpenVDB volumes, Alembic and USD caches and simulation bakes live in `media/`, named by content hash, and are referenced from Null with their colour space, frame range and other settings. Packed files in a `.blend` are unpacked into `media/` on conversion. Sources outside the project are referenced by path and reported by `sau check` if missing.

## 13. Generators (optional)

Phase 5 of Sau's roadmap; not part of v0's implementation. A Python script in `scripts/` may generate part of a project: a scattered forest, a staircase from a formula. It runs with Blender's Python API in a sandbox that may create data but not touch files or the network, gives the same output every time, and its output is plain Null text. A generated part can be frozen into the project as ordinary text and edited by hand. Geometry nodes and drivers are preferred over generators wherever they can do the job, because they stay live data.

## 14. Checks

Files in `checks/` state what must be true of the scene and of renders: a polygon budget, no object below the floor, no missing media, every material has a name, a render of camera `@shot-cam` at frame 1 matches `checks/golden/shot-1.png` within a tolerance. `sau test` runs them, and a failure points at the file and line that caused it. The check vocabulary is specified with phase 8.

## 15. Canonical form

The formatter prints one form (built by tasks 1a-07, 1c-01e and 1d-02b): canonical order of attributes (the RNA definition order), one object per file section, small records on one line, defaults omitted except pinned settings, numbers printed by the rule of section 6, and handles written as kebab-case of the display name when new. `sau fmt` is idempotent, and Sau's own write-back always produces canonical text for the lines it touches, without reformatting lines it does not.

The layout is fixed (settled November 7, 2026, task 1a-07):

- two spaces of indentation per open block and per open list, with a closing `}` or `]` that starts a line one level less;
- one space between entries, none just inside `[ ]`, one on each side inside a `{ }` that stays on one line, none before `;` and one after it;
- whitespace around `=` as written, since it can change what parses;
- no trailing whitespace, a comment's included, and lines that end in LF (a CRLF file's become LF);
- a line a `\` continues indented one level deeper than its attribute;
- at most one blank line in a row, none at the start of the file, and one line feed at its end.

Formatting keeps every comment and every other token as written.

The canonical order (task 1c-01e, `null_print::canonical`):

- **Attributes in the schema's order.** Within a block, and among a line's `key=value`s, attributes follow RNA's definition order: the base struct's properties first, then each struct's own (`ID`'s, then `Object`'s: `data`, … `location`, `rotation-euler`, … `scale`). A light's follow its type's struct.
- **Null's forms stand where the RNA property they write stands** (and `units`' short forms keep section 8's order: system, `scale`, `length`, `rotation`, then the rest in RNA's, so S35's `units metric scale=1 length=m rotation=deg` is already canonical; the lead's call):
  - a scene's `units` at `unit-settings`, `fps` and `fps-base` at `render`, and its `object` and `collection` members at `collection`;
  - an object's `material` slots at `material-slots`;
  - a mesh's `primitive` before `vertices`;
  - a material's `nodes` and then `layout` at `node-tree`;
  - a project's `units` before its scenes;
  - a `units` block's settings in `UnitSettings`' order, short names standing for full ones.
- **Some orders stay as written:** lines of one name keep theirs among themselves (members, material slots), and a node tree's `node` lines keep theirs. A node's own settings come first (not those every node has, such as the editor's `color` for it), then its inputs as its declaration lists them; a node is found in its tree, since one kind may name nodes of two trees.
- **Attributes the schema does not know** come last, as written. So do lines commented out with `/-`.
- **Comments.** A comment on the lines just above an attribute moves with it, unless a blank line separates them. A comment at a line's end stays on that line, and so does one after a block's `{` or after a `;`.
- **Blank lines.** There are none between the lines of a block. Between datablocks, a file keeps at most one.
- **Files that do not parse** keep their order, and only get the layout.
- **Every construct has its order** (task 1d-02b): a modifier's or a constraint's settings in its kind's struct's order, and a pose bone's constraints the same; a sub-object line where RNA's collection of it stands (`modifier` at `modifiers`); a node in any tree by that tree's node of its kind; a zone's items, then its input lines, then `output`, then its nodes; a group's interface lines as written, then `nodes`, then `layout`; an f-curve's settings in `FCurve`'s order, its keys as written, its modifiers by their kind; an NLA strip in `NlaStrip`'s; a component's parameters first, then its datablocks, each in its own kind's order. What is meaning by its order stays as written: a tree's nodes, a group's interface, an action's keys, a component's datablocks and placements.
- **Defaults left out** (`null_print::without_defaults`, task 1d-02b). A line of its own whose value is its property's default (the schema's `new`, else RNA's `default`) goes, in a datablock, a struct block, a modifier or a constraint. A line keeps its value when it has a comment on it or just above it, shares a line with others, has a `key=value`, or is of a unit read through the scene (lengths, areas, volumes, masses, velocities, accelerations, powers, frames in seconds), which this pass does not read. Settings a project pins are kept once pinning exists. `sau fmt` leaves defaults out once Cosmósis calls it, with its corpus formatted again in the same change.


## 16. Live reload

Changes are live in both directions, all the time, the way every query in a Convex app updates the moment its data or code changes (Sau's roadmap, design decision 9). Each change, from any source, is one checked, atomic transaction with a revision number, and every view (viewport, editors, code views, the language server's clients and the gateway's subscribers) follows the same revision.

There is no run step and no separate save step (the user, November 6, 2026: changes "with hot reload open and autosave, rather than making scripts"). VS Code is the editor (Sau's decision S44): Sau's extension sends each change of the open buffer to Sau through the language server, and Sau applies it as soon as it is valid, without a save, at a pace the scene can take. Phase 1a's slice applies on save; the buffer's changes come with the language server. Saving a file in any other editor, an AI's edit or a `git checkout` makes Sau, while it has a project open (Sau's tasks 1a-10, 1a-10b, 1a-13 and 1a-13a):

1. load and check the project again on the watcher's own thread, never on Blender's. Saves made while one is loading replace each other, so the newest text is taken and nothing queues (S39);
2. on any error, keep the last good state: each problem is reported once (`file:line:column: error[code]: message`; the code view and the status bar will show it too), nothing is applied and no undo step is pushed. The next good save applies;
3. apply the newest checked project to `Main` on the main thread, between handling events and drawing, as one undo step (an outside save is one step, S41). Apply finds each datablock by its handle (on the first apply, by its name) or makes it, then reads each value the files write and writes it through RNA only where Blender's differs, so Blender's own update callbacks and dependency-graph tags run. Applying the same text twice changes nothing and pushes no step. Any datablock, scene member or material slot applied before that no file defines now is removed in the same step. Data something outside the project still uses stays, and objects made in Sau, which no file defines, are left alone;
4. redraw the affected editors.

After an apply, Sau waits as long as that apply took before it looks for a newer text (S39's pace by cost). S39's in-app control of the pace, and slowing down when the Mac runs hot, come later. Measured for the slice (Sau's task 1a-16, a gate in `just verify`): from the end of a save to the evaluated object, 12.6 ms median and 14.1 ms worst against a 50 ms target. Nearly all of that is the watcher's own delay; applying takes under 2 ms.

Edits Sau makes itself (section 17) are not applied back. After write-back writes a file, the watcher reads the folder at once, and a project whose texts are those Sau already holds applies nothing, so Blender's exact values are never rounded to their written spelling.

Opening a project (`Sau --project <folder>`, task 1a-12) loads, checks and applies it into an empty file in the same way. A project with errors reports each problem and changes nothing. Opening is not a step to undo.

## 17. Write-back

Every edit in Sau's interface (a drag, a slider, an operator, a node link) ends in an undo push. At that moment Sau compares each datablock of the open project with its file and, for each one that differs, prints the new Null form, compares it with the current text of its node, and applies the smallest edit to the lossless syntax tree, keeping comments, ordering and untouched lines. Phase 1a's slice writes back an object's `location`, `rotation-euler` (XYZ) and `scale` (Sau's task 1a-14). Every object of the slice's corpus, moved, turned or scaled and pushed, changes exactly one line of one file, or adds one (task 1a-16). The smallest edit is per value (task 1a-07):

- a value that still reads back to the same bits is left exactly as written;
- a changed value is rewritten in the unit its word already uses, whenever a decimal in that unit reads back to the new bits, and bare in the internal unit otherwise (S21);
- a list of another length is replaced whole;
- an attribute the datablock does not have yet is added as one new line in its block, in the project's display unit, or as `name=value` on its line when it has no block; a new line goes after a comment on the block's opening line, and ends as the file's lines end (LF or CRLF).

A file is written atomically, beside itself and then renamed over it, so an editor or the watcher sees the old text or the new, never half. It is written only while its text is the one Sau last read. A file changed outside Sau is left alone and read again, and live reload applies the outside save. If Sau held a change that it could not write, the file's value then wins. Merging the two by value, so that both writers' edits to different values stay and the user is asked only when both changed the same one (Sau's decision S40), comes later.

Undo and redo in Sau rewrite the files too (S41, task 1a-14b). Undo puts back each file's text from before the steps it crosses, byte for byte, where the file still holds exactly what Sau wrote; redo does the same forwards. A file edited outside Sau since then keeps the editor's text.

Later, in phase 3 of Sau's roadmap (two-way editing with VS Code), edit mode and sculpt mode will write their data when the mode is left and, while inside, after a short pause. The slice writes back transforms only.

## 18. Diagnostics

Every message names the file, line and column, says what was expected, and where possible offers a fix: an unknown property suggests the nearest RNA name, a wrong unit says which units fit, a handle that resolves to nothing lists the near matches. Codes are stable, so tools and AI can act on them: `null-syntax` (the text does not fit the syntax), `null-name` (a name, handle or enum value that is not kebab-case), `null-kind` (a datablock, node or primitive kind the schema does not know), `null-unknown-property`, `null-type` (a value of the wrong type or shape), `null-unit`, `null-range` (outside the hard range: a note where it is a number's, since Blender clamps it; an error for a string too long for Blender's field, which is never cut), `null-enum`, `null-handle` (resolves to nothing, to the wrong kind, or is defined twice), `null-readonly` and `null-repeated`.

## 19. Converting `.blend` files

Tasks 1d-05a to 1d-05e build this section on 1b-04's coverage audit, which lists the data RNA does not reach. `sau convert scene.blend Scene.sau/` writes a Cosmósis project from any `.blend`; `sau convert Scene.sau/ scene.blend` goes back. The conversion is lossless:

- everything RNA reaches is written as named Null properties;
- data RNA does not reach (found by the census of roadmap phase 0) is written in a `raw` block per datablock, using SDNA's own struct and field names, until a named form replaces it; the coverage audit counts these and they may only shrink (the form is below);
- runtime-only data is skipped, and the list of what counts as runtime is part of the spec;
- a value outside its property's hard range is clamped when Sau sets it through RNA, as Blender's interface clamps (section 6, "Numbers"); so is an infinity in a property whose range is the default, ±`FLT_MAX`, which becomes `FLT_MAX` with its sign. Such a value survives the conversion only where the datablock's `raw` block carries its field (Sau's task 1d-05b).

**The `raw` block** (Sau's task 1d-05b) holds a datablock's DNA fields that no RNA property reads and no rule over DNA's facts leaves out (Sau's tasks 1b-04a and 1b-04b). Which fields a struct has is the raw table, `schema/null-raw.json`, which Sau's build writes with the other schemas: per DNA struct, each such field of the struct or of a struct embedded in it, by its path, its DNA type and its array dimensions. A field behind a pointer or in a list is not in it yet.

```null
object @buoy "Buoy" {
  location [0m 0m 1m]
  raw {
    restore_mode 0
    col_mask 255
    collision_boundtype 4
    constinv [[1 0 0 0] [0 1 0 0] [0 0 1 0] [0 0 0 0x7fa00001]]
  }
}
```

(`restore_mode`, an `int`; `col_mask`, an `unsigned short`, written 0 to 65535; `collision_boundtype`, a `char`; and `constinv`, a `float[4][4]`, are `Object` fields of Blender 5.3's SDNA that no RNA property reads.)

- **One line per field,** its flat path in the code's names (`transflag`, `r.mode` for `mode` of the embedded `r`), which keep DNA's own spelling: inside a `raw` block, at any depth, names are not held to kebab-case. The `raw` line itself is.
- **Each value as its DNA type reads.** An integer in decimal, held to its type's range; `char`, `uchar` and `bool` as the byte's value, 0 to 255, the same on every platform. A float or double as the shortest decimal that reads back to its bits, `inf`, `-inf` or `nan`; a NaN other than the quiet one `nan` reads as (`0x7fc00000` for a float), its sign and payload included, as its bits instead: `0x` and exactly 8 lowercase hexadecimal digits for a float, 16 for a double. Bits are for such a NaN alone: any other value written as bits is an error whose fix is its decimal, or `nan`. Outside a `raw` block a float's bits are an error.
- **An array as a list of exactly its length,** nested for each dimension; a `char` array as the list of every byte, those after its closing NUL included.
- **Exact.** Bytes are written and set as DNA holds them, bit for bit, through no RNA setter: a `raw` field's value is never clamped (the line above). The checker refuses a field the table does not list for the datablock's struct, a value its type cannot hold and a list of another length. Sau prints every raw field of a datablock until the raw table carries DNA's defaults.

Correctness is checked by a field-by-field comparison of the original and the round-tripped `.blend` (by DNA structure, not by bytes, since a `.blend` holds memory addresses), and by rendering both. Through Blender's importers, any format Blender reads (glTF, FBX, OBJ, USD, Alembic, PLY, STL) converts into Null the same way.

## 20. Open questions

None is open for v0 (task 1d-01a, November 8, 2026). The draft's three were settled or given to a later phase:

1. **The inline threshold** for level 0 data is 2,000 elements (Sau's decision S23; section 9).
2. **Object data used by one object** is written in that object's file, and in a file of its own when it is shared (S36: "data used once lives in the file that uses it"; Cosmósis, section 2). The layout is free, so this is the default `sau convert` writes, not a rule the checker enforces.
3. **The check vocabulary** (section 14) is phase 8's.

Questions raised after v0 is frozen (task 1d-01c) go through RFCs, as Scowr's do.

## 21. What building Null takes

- **Design (phase 0 and 1a):** this draft, tested against the slice (section 22) and the census, then frozen with a changelog and RFCs for later changes, as Scowr does.
- **The front end (Rust, in Null's repository):** lexer, lossless syntax tree, parser with error recovery, formatter, diagnostics, the checker that reads the generated schema, and the language server. Shared with Scowr where the syntax is shared.
- **The binding (C++, inside Sau):** the schema generator in `makesrna`, applying a checked tree to `Main` through RNA, printing any datablock to Null, the raw DNA fallback, the live-reload watcher and the write-back at undo push.
- **Tooling (phase 4):** the tree-sitter grammar, the VS Code extension, the generated field reference, examples and a short guide written for AI.
- **Stability:** a `format` version in every project's manifest; when Blender renames or moves an RNA property between releases, Sau ships a migration that rewrites old files, the way Blender's versioning code updates old `.blend` files.

## 22. What the slice taught

Phase 1a built Sau's slice on this draft (Sau's roadmap; tasks 1a-01 to 1a-16, November 6 to 8, 2026): a project of cubes, primitives, materials, cameras and lights that Sau opens, reloads live and writes back. Every change the slice forced on the draft is below, with its reason; the sections hold the rules themselves. Sau's decision register holds the S decisions, and Null's `CHANGELOG.md` dates each change.

| Change | Section | Why | From |
| --- | --- | --- | --- |
| A value with a unit is converted exactly and rounded once, never in float64 arithmetic | 6 | float64 arithmetic rounds twice: for 21.5% of short degree values read into a float64, it gives other bits than the exact rule. It never does for the slice's float32 rotations, but S59 holds every type to the exact rule | 1a-04, S59 (1) |
| NaN is written `nan`, without sign or payload, the one value that does not read back to the same bits | 6 | A NaN's sign and payload have no decimal form | S59 (2) |
| A node ends only at a line feed, `;` or `}`, so a one-line block is `{ width 4mm; segments 3 }` | 4, 5 | Scowr's syntax: the first draft's `{ width 4mm segments 3 }` was one attribute with three values | S59 |
| `at`, `to` and `len` before a value are read as two arguments | 5 | Scowr's lexer makes them keyword attributes, and Scowr's syntax is shared unchanged | 1a-01 |
| `#rrggbb` converts through a fixed table of the 256 exactly rounded values of the sRGB curve, with no `pow` | 6 | "Computed in float64 and rounded once" held on one Mac, but `pow` differs between platforms | 1a-06's review, S62 |
| Faces are written `polygons` | 8 | RNA's name for them, and every Null name is RNA's (S16) | 1a-05, 1a-06 |
| A point, spot or area light's `energy` is in `W`; a sun's is bare | 8 | RNA gives `energy` no unit, though Blender shows watts; a sun's is an irradiance | 1a-05, 1a-06 |
| A camera's `lens` is in `mm` | 8 | RNA's camera unit is the millimetre, not a length | 1a-06 |
| Null's own forms: `project`, `units`, scene and collection members, `data <kind> @h`, `material` slot lines, the light's type struct, `vertices` and `polygons` as lists | 8 | RNA's `unit-settings`, `objects` and `children` are read-only, `data` points to any ID, materials live in slots, lights are split by type, and the short forms come first (S35) | 1a-06 |
| A primitive's proportions come from its add operator's `scale`, and the pier's object scale is back to 1 | 4, 8 | An object scale of `[1.2 3 0.08]` squashed the object's 4 mm bevel and its children; Blender's operator shapes the mesh itself | 1a-05c, 1a-06b |
| The schema has the material output node, with its `surface` input | 8 | The slice's materials end in `output-material`, which the first schema left out | 1a-05b |
| A property's default is its value in a new datablock | 8, 15 | RNA's own default is not what a new datablock gets (a new scene's units are `metric`) | 1a-06 |
| The formatter's layout is fixed | 15 | One printed form for every file | 1a-07 |
| Write-back's smallest edit is per value | 17 | A drag must change one line, and an unchanged value its spelling never | 1a-07, S21 |
| Apply finds datablocks by handle, writes only values that differ, removes what the files no longer define, and is one undo step | 16 | Applying twice must change nothing, and an outside save is one step to undo (S41) | 1a-10, 1a-10b |
| Live reload loads on the watcher's thread, takes the newest save, applies between events and drawing, and paces itself by each apply's cost | 16 | S39; a broken save keeps the last good state | 1a-13, 1a-13a |
| Sau's own writes are known by their text, not a content hash, and are not applied back | 16 | Applying them back would round Blender's exact values to the file's spelling | 1a-13 |
| Write-back runs on every undo push, writes a file atomically and only while it holds the text Sau read, and leaves a file changed outside alone | 17 | Never overwrite an editor's save; merging by value (S40) is later | 1a-14 |
| Undo and redo restore a file's text byte for byte | 17 | One history for the files and the scene (S41): a value's spelling must come back with it | 1a-14b |
| A project with errors opens as nothing, and opening is not an undo step | 16 | The last good state is the only state Sau shows | 1a-12 |

What the slice confirmed without a change: every attribute the corpus writes reprints from Blender's values with no edit, in every unit it uses (`m`, `cm`, `mm`, `deg`, `rad`, bare), and every attribute it leaves out holds its default (Sau's task 1a-16: 46 and 41 of the corpus's 87 transform attributes).
