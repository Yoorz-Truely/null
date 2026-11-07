# agent/null-a

The Null worker's report, kept current with each commit. Working in Null's main checkout on `main` (Sau's decision S58). Tasks from Sau's board: 1a-01, 1a-04, then 1a-02 and 1a-03.

## 1a-01: share Scowr's front end by path

Done, November 7, 2026.

**What works.** Null's workspace depends on Scowr's `scowr_syntax` and `scowr_errors` by path (`../Scowr/compiler/…`, root `Cargo.toml`, `[workspace.dependencies]`). A new crate, `compiler/null_syntax`, is the only Null crate that names them: it re-exports the lossless tree, the typed view and the diagnostics types, and maps syntax diagnostics to Null's code `null-syntax`. The boundary (what is shared, what is Null's, how a change to Scowr is made) is in `AGENTS.md`, "The shared front end". No change to Scowr was needed.

**Measured.**

- `cargo test -p null_syntax`: 5 tests and 1 doctest pass. They check that all five `null` examples of the spec (section 4, read from `docs/spec/null-v0.md` at build time, so they stay checked) parse with no diagnostics and print back byte for byte; the tree of the pier example; that unit, colour, link, path and package words stay whole; that a broken line gives `null-syntax` diagnostics and the rest of the file still parses; and that Scowr's keyword attributes reach Null as keywords.
- Null's `just check` exits 0.
- Scowr's own tests at Scowr `4de7474`, run unchanged with the build guard as `RUSTC_WRAPPER` and a target folder outside Scowr: 192 passed, 0 failed, 49 suites. Scowr's `git status` is clean before and after.

**Found.**

- `cargo fmt --all` formats local path dependencies too: run in Null it listed every file of Scowr's workspace. Null's `just check` and `just fmt` now run `cargo fmt` (workspace members only). Cosmósis's justfile still runs `cargo fmt --all`; once Cosmósis depends on Null by path, `just fmt` there would rewrite Null's and Scowr's files. For the lead: Cosmósis needs the same change.
- The spec's example `modifier @bevel bevel { width 4mm segments 3 }` is one child node, `width`, with three arguments, since a node ends only at a line feed, `;` or `}`. The intended form is `{ width 4mm; segments 3 }` or `width=4mm segments=3` on the modifier's line. Left for the spec's owner and the checker (1a-06); the test records the current reading.
- Scowr reads `at`, `to` and `len` before a value as keyword attributes; Null's checker must treat them (noted in `AGENTS.md`).

**Unverified.** Nothing in 1a-01's "Done when" is unverified. `just verify` runs no fuzz target yet (1a-03 brings the first).

## 1a-04: exact numbers and units

Done, November 7, 2026 (code in `95f836b`, approved). The two choices below went to the user.

**What works.** A new crate, `compiler/null_value`, reads a word as a value of a dimension (none, length, angle) stored as `f32` or `f64`, and prints a stored value back:

- The exact written value is rounded once, to nearest even, into the stored type. A bare number, and a length in `m`, `cm`, `mm`, `km`, `in` or `ft`, becomes one exact decimal (a decimal shift, or a whole multiple by 254 or 3048) read by the standard library's correctly rounded parser. Degrees are multiplied by π/180 exactly: π is bracketed by Machin's formula in big integers and the bracket narrowed until both ends round alike.
- `print_bare` writes the shortest decimal that reads back to the same bits, in one canonical layout (plain from 10^-6 to 10^20, otherwise `1.5e-7`). `print_in` writes the shortest decimal in a unit that reads back exactly, or nothing, so `print` falls back to the bare internal unit, as the spec's conversion rule says.
- Wrong and unknown units are errors that say which units fit, with the nearest unit as a suggestion.

**Measured.**

- 17 tests pass, and a doctest (`cargo test -p null_value`, 256 cases per property). With 4,096 cases in release (as in `just verify`), they take 0.42 s; with 200,000 cases each, 18.7 s, all passing.
- The properties: every non-NaN `f32` and `f64` bit pattern prints and reads back bit-identical, bare, in every length unit, and in degrees (or radians where no degree decimal fits). Every decimal of up to 6 significant digits (`f32`) or 15 (`f64`) reads back exactly as written in `m`, `cm`, `mm`, `deg` and bare. Rounding a ratio matches the standard library on `m/10^k` down to `10^-400`. Degrees match an independent oracle (π to 120 decimals from Python's `decimal` module, decimal arithmetic) for `f32` and `f64`, including inputs of 400 to 3,000 digits.
- Exhaustively: all 4,278,190,082 non-NaN `f32` bit patterns print bare and read back bit-identical (`all_f32_round_trip_exhaustively`, ignored by default; `cargo test --release -p null_value -- --ignored`), 1,248 s on one thread at `nice 10`, thermal pressure nominal.
- Null's `just verify` passed after `95f836b`.
- Examples: `90deg`, `2.7m` and `5mm` read back as written in `f32` and `f64`. `90deg`, `180deg`, `45deg`, `30deg`, `360deg` and `-60deg` give exactly Rust's `FRAC_PI_2`, `PI` and the rest. `2.7cm` is the `f32` nearest 0.027.

**Dependencies** (licences checked): `num-bigint` 0.4, `num-integer`, `num-traits` (MIT OR Apache-2.0); for tests, `proptest` 1 without default features, and what it pulls in (`rand`, `getrandom`, `libc`, `chacha20`, `bitflags`, `regex-syntax`, `unarray`, `core_detect`, `cfg-if`, `rand_xorshift`), all MIT OR Apache-2.0. `cargo deny` gathers all 23 crates, the test ones included.

**Choices to review.**

- The spec says a value with a unit is "converted in float64 and rounded once". Converting in `f64` rounds twice (the decimal, then the product), so the reader converts exactly and rounds once, which meets the "one rounding" the task and S21 ask for. The spec's wording could say "converted exactly".
- NaN reads and prints as `nan`, the quiet NaN; its sign and payload are not written, so NaN is the one bit pattern that does not round-trip. The spec does not mention NaN.
- Lengths are read at unit scale 1. The scene's unit scale (S21, "metres through the scene's unit scale") is for the checker, which knows the scene.
- Integers (DNA's int properties) are not part of this task and are not read yet.

## Review

### 1c-01d again, the fix `e0d1519`, by agent/cosmosis-a, November 7, 2026: approved, one nit

I re-ran my scratch test against `e0d1519`, with the camera whose `clip-start 0.00000002m` is in range only at scale 0.01, and added five cases.

- **Two scenes both at 0.01:** clean, so objects are read at 0.01.
- **Scenes that differ:** 0.01 and 2; 0.01 beside `units imperial`; 0.01 beside `units none`; 0.01 beside an empty `units {}`; and 0.01 beside a scene with no `units` at all. Each gives one `null-unit` error on the first scaled value outside a scene. It names each scene's scale and says to give the project a `units` line.
- **Consistent treatment of unset scales.** An empty `units {}` and a missing `units` both count as scale 1, as a new Blender scene is, and section 6 says so.
- **The project's `units` win.** Project `units` at 0.01 over disagreeing scenes is clean.
- **No error without a scaled value outside a scene.** Disagreeing scenes with only `lens 50mm` (never scaled) or a bare `clip-start 0.1` outside them are clean.
- **One `units` reader.** `null_check::units` is now the only reader of `units`, and `null_print` uses it. A test checks that the two agree.

**Nit, not blocking:** after the disagreement error, the same value is also checked at the fallback scale 1, so it can report a second error (here `clip-start` out of range) that only follows from the first. One mistake, one error: skip the value's own checks once the scale is in doubt.

### 1b-02c, third round, the fix `32d581a`, by agent/cosmosis-a, November 7, 2026: approved

I re-ran my scratch test of 21 paths against `32d581a`.

- **The two errors are fixed.** Inside `material @oak`, with `object @bsdf` in the project:
  - `@bsdf.rougness` says "a principled-bsdf node has no socket `rougness`", with the fix `@bsdf.roughness`;
  - `@bsdf/@x` says the node has no sub-objects.

  Neither mentions the object any more.
- **The earlier results are unchanged.** In-block lookups and absolute paths still resolve as in the second round, and paths crossing owners are still errors.

One nit from my first review remains, not blocking: a trailing dot (`@oak/@bsdf.`) still gets the message meant for `.x/`.

### 1c-01d, Null `1b45e94`, by agent/cosmosis-a, November 7, 2026: sent back, the outside scale silently falls back to 1

I read the row, S21, section 6's new sentence, `check.rs`'s `Units`, `units_of` and `outside_units`, and `null_print`'s `outside_scale`. Then I ran a scratch test, not committed, on a copy of `1b45e94`. It uses the test's own camera, whose `clip-start 0.00000002m` is in range only at scale 0.01.

**What holds:**
- **The scale inside a scene.** A scene's own values use its `units`, which override the project's field by field.
- **Outside any scene.** The project's `units` apply, or, when the project has none, those of a single scene that sets any. The system `none` applies no scale, as in Blender.
- **The tests.** `1m` stores 100 at scale 0.01 and 0.5 at scale 2, and write-back prints locations back as written.
- **Judged against S21.** "Metres through the scene's unit scale" assumes one scene. Every scene shares the objects, so outside a scene the only faithful reading is a scale the scenes agree on, or one the project states. Using the project's `units` there is right.

**To fix:**
1. **With two or more scenes and no project `units`, the scale outside scenes is silently 1, even when the scenes agree.** With two scenes both at `units metric scale=0.01`, the camera's `clip-start` is out of range, so objects are read at scale 1: a written `1m` stores 1 Blender unit, which both scenes show as 0.01 m. The same happens when the scenes disagree (0.01 and 2), and when one sets 0.01 and the other only `units imperial`.
   - When every scene that sets a scale sets the same one, use it.
   - When they differ, report it on the first value with a scaled unit outside a scene, saying to state the project's `units`. Don't fall back to 1 silently.

   Section 6's sentence should say both.
2. **One reading of `units`, not two.** `check.rs`'s `units_of` and `null_print`'s `outside_scale` each parse `units` and choose the scale, and they already differ:
   - The checker counts a scene as setting units only if its `units` say something. The printer counts any `units` line, so a scene with an empty `units {}` makes two scenes for the printer and one for the checker.
   - The checker reads every `units` line of a node; the printer only the first.

   Put one function in `null_check` (or `null_value`) and use it in both, with a test that they agree. Sau's apply will need the same function (1a-10's review noted it reads at scale 1).

### 1c-01c, Null `1cda890`, by agent/cosmosis-a, November 7, 2026: approved, with two nits

I read the row, section 6's "Frames" and section 8's new `fps`/`fps-base` form, the checker's `frame_rate`, the `Out` scene and `write_in`. I ran a scratch test on a copy of `1b45e94`, which contains `1cda890`.

- **The "Done when" holds.** `1.5s` is 36 frames at 24 fps. At `fps 30000`, `fps-base 1001` it is the exact 1.5 × 30000 / 1001, rounded once. Write-back keeps `2s` (`null_print/tests/write.rs`).
- **Integer frames are bare.** `frame-start 2s` says "`frame-start` cannot take `s`: it takes a bare number". That settles my nit on 1c-02a.
- **Seconds and minutes on a float frame read through the scene's own `fps` and `fps-base`.** `frame-float 1001s` at `fps 30`, `fps-base 1.001` and `frame-float 2min` are clean. `fps 1.5` and `fps-base 0` are errors. A frame-float next to a bad `fps-base` falls back to a new scene's rate, without a second error.
- **Seconds outside a scene's block are an error**, as the spec says.

**Nits:**
1. Section 6 says 29.97 fps "is written `fps 30000` and `fps-base 1001`". Blender's own 29.97 preset is `fps 30`, `fps-base 1.001`, and that is what a converted file will hold. Its float32 1.001 gives 29.97002997…, not 30000/1001. Say both are valid, which one Blender's preset is, and that the two differ in the last bits.
2. Actions' keys are outside any scene, so seconds on a key's frame are an error. That is right while an action has no scene, but say so in section 6 or on 1d's action row, so the action form doesn't have to rediscover it.

### 1b-02c again, the fix `f563f87`, by agent/cosmosis-a, November 7, 2026: sent back, one line

I re-ran my scratch test against `f563f87`. It has 21 paths now, the old fourteen plus seven inside blocks.

**What the fix gets right:**
- **Both cases are fixed.** Inside `material @oak`, `@bsdf` and `@bsdf.roughness` resolve to its node, though `object @bsdf` exists. Inside `mesh @pier`, `@bevel` is its vertex group, and inside `object @pier` it is the modifier.
- **Relative paths stay inside the block.** Inside `@pine`, `@out` (a node of `@oak`) is an error.
- **Absolute paths still work from inside a block.** `@pier/@array` written inside `@oak` resolves to the object's modifier, and `@bevel` inside `object @bsdf` is its modifier.
- **Section 7 and the test** say and show both.

**To fix: when the first step is one of the owner's own sub-objects, an error in the rest of the path still falls through to the datablock elsewhere.** Inside `@oak`, with `object @bsdf` in the project:
- `@bsdf.rougness` says "ends with a socket, which only a node has", with no fix. Without the object it would say "a principled-bsdf node has no socket `rougness`", with `@bsdf.roughness` as the fix.
- `@bsdf/@x` lists the object's sub-objects ("it has @bevel"), not the node's.

No wrong path resolves, but the corrected path the row promises is lost, and the error names another owner. Fall through only when the owner has no sub-object with that first handle, for example `Err(_) if self.owners.contains_key(first) && !owner.subs.iter().any(|s| s.handle == *first) => {}`. Add `@bsdf.rougness` inside `@oak`, with `object @bsdf` present, to the test.

### 1b-02c, Null `50bb332`, by agent/cosmosis-a, November 7, 2026: sent back, a path inside a block can cross owners

I read the row, section 7's "Paths" and "Unique within its owner" rules, S61, `paths.rs` and its tests. Then I ran a scratch test of my own, not committed, against a copy of Null's main. It resolves fourteen paths that look valid in a project where handles meet across owners.

**What holds:**
- **Absolute paths** resolve, and they refuse to cross owners. `@oak/@mix` (`@mix` is another material's node) and `@pier/@weld` (`@weld` is another object's modifier) are errors that list the owner's real sub-objects.
- **Parsing:** an empty step (`@oak//@bsdf`), two sockets, a socket on a modifier (`@pier/@bevel.width`), a step past a node (`@oak/@bsdf/@x`), and a pattern copy before a socket (`@bsdf.7.roughness`, which names task 1d-03) all give errors.
- **Duplicates:** the per-owner duplicate check and the fix suggestions behave as the snapshot shows.

**To fix: leaving the owner out, the feature section 7 adds, resolves against the wrong datablock when handles meet.**

1. **A sub-object is shadowed by any datablock with the same handle.** `resolve` treats the first step as relative only when no datablock in the project is called that.
   - Inside `material @oak`, whose tree has `node @bsdf`, a project that also has `object @bsdf` makes `@bsdf` resolve, with no error, to the object.
   - `@bsdf.roughness` there fails with "ends with a socket, which only a node has", though `@oak` has exactly that node and socket.

   Inside an owner's block, its own sub-objects should be looked up first. The spec's sentence ("inside the owner's own block the owner may be left out") says the path names the owner's sub-object. Say so explicitly, so a datablock handle never shadows it.
2. **`within` is a handle, but a block is one datablock.** Handles are unique per kind, so `object @pier` and `mesh @pier` can both exist. Inside the mesh's block, `@bevel` (its vertex group) fails with "could start at the object or the mesh called `@pier`", though the writer is inside exactly one of them.

   Pass the owner itself (kind and handle) as `within`, as the caller has it, and walk only that owner.

Add both cases to `tests/paths.rs`.

**Nits:**
- A trailing dot (`@oak/@bsdf.`) says "a socket ends a path: `.` cannot be followed by `/`". That message is for `.x/`; here it should say the socket's name is missing.
- An absolute `@pier/@bevel` where both an object and a mesh `@pier` have a `@bevel` is an error, rightly. Section 7 should say how to write it unambiguously, or that such a pair needs distinct sub-object handles.

### 1c-02a, Null `c3d2f8b` revised in `90d16cd` (S65), by agent/cosmosis-a, November 7, 2026: approved, with four nits

I read the row, S61 (5) and S65 in Sau's DECISIONS.md, section 6's new "Types" table and the CHANGELOG line, and checked the examples against Blender's source.

- **The "Done when" holds.** Every type in the row has its form and an example: booleans, integers, floats, enums, flag sets, strings with bytes outside UTF-8, pointers, collections, arrays, multi-dimensional arrays and matrices. The CHANGELOG records it, S65 included.
- **The byte list is right.** `[47 47 109 252 108 108 101 114 46 112 110 103]` is `//m`, byte 252 and `ller.png`. "Raw bytes, not an escape" says why, as S65 does: Scowr's lexer refuses `\x`.
- **Matrices are right.** Blender keeps `float[4][4]` column by column (`m[3]` is the translation), so RNA's flat element for row `i` and column `j` is `j × rows + i`. Row by row is how `mathutils.Matrix` prints. The example's 2 m sits in row 0, column 3, as a translation in x should.
- **Pointers and lengths:** `none`, and over-long strings as errors, follow S61 (5).

**Nits, none blocking:**
1. **Integer frames.** The Integer row says an integer never takes a unit. But `frame_start` and `frame_end` are `PROP_INT` with `PROP_TIME` (`rna_scene.cc`), and the units table's `TIME` line offers seconds for "a frame". Say on the `TIME` line that integer frame properties stay bare whole numbers and only float frame properties take seconds, so 1c-01c doesn't have to decide.
2. **Which floats are matrices.** Say that "matrix" means a float array of subtype `MATRIX`. Other two-dimensional arrays, such as `bound_box`'s `[8 3]`, follow the multi-dimensional row with the first dimension outermost.
3. **A string's length.** Say "longer than Blender's field for it" is counted in bytes with room for the terminating NUL. A byte list containing 0 should be an error, since a DNA string cannot hold one.
4. **Flag sets.** Say whether a list out of the schema's item order is an error or is read and reprinted in order. "In the schema's item order" reads as the printed form, not the accepted one.

### 1c-01b, Null `2c66b92`, by agent/cosmosis-a, November 7, 2026: approved

Read the row, the 1c-01b section below and the diff. `just verify` passed on a clean copy of `2c66b92` (`git archive`, Scowr beside it, in my scratch folder): Clippy, the licence gate, 168 tests in the check run and again in the 4,096-case release run, and the fuzz target for 60 seconds.

- **The "Done when" holds.** The three property tests run for every unit but degrees, which have their own. I checked the reader a second way too: a scratch test, not committed, built each value's exact rational from the spec's own factors, wrote it to 130 significant digits and let the standard library round it, an oracle independent of `round_ratio` and of the midpoint test. Every unit agreed, f32 and f64, in 4,096 cases with up to 17 digits and exponents from -30 to 30.
- **The words are the spec's.** Each dimension's table matches section 6's line, word for word and factor for factor, including `-273.15C` and `-459.67F` as +0 and `F`'s exact 5/9.
- **The limits are right.** Past a leading power of 10^±400, a value times any ratio or offset factor in the table (from thou³, about 1.6 × 10^-14, to mi³, about 4.2 × 10^9) leaves every float's range, or falls below half the least subnormal. So `ratio` and `affine` may short-cut there. Decimal shifts, down to pm³'s 10^-36, go through `shifted` and need no limit.
- **Frames.** RNA's `time` still maps to a bare number, as the comment in `values.rs` says. Reading seconds as frames is the arithmetic of 1c-01h (agent/cos-units-null). The checker's side is 1c-01c, and through the unit scale 1c-01d. Until then a project at another unit scale stores its lengths as at scale 1. Section 6 says how it should be, and the rows track it.

**Nit, not blocking.** `in_unit`'s doc comment still says it divides exactly "for inches (0.0254 m) and feet (0.3048 m)". It now serves every ratio and offset unit.

### 1c-01a again, the fix `461a5c8`, by agent/cosmosis-a, November 7, 2026: approved

- **The sentence.** Section 6 now says that Blender's fields convert with float32 factors (`0.01f`, 0.555555555555), so a value typed into them often stores other bits than Null's rule gives. It quotes the measured counts (526 `cm`, 1,251 `mm`, 329 `ft`, 286 `in`, 169 `mi` of 2,000) and `9cm` as the example, says Null's rule stands, and points to S64 (1) and task 1c-01g for Sau's own fields.
- **The temperature line** no longer suggests that only `F` differs.
- **The word list** is now said to be Null's choice, not Blender's suppress flags. It names what Blender suppresses but Null keeps (`yd`, `km/h`, `mph`, `kW`, the camera's `cm` and `um`) and notes the turn's 1/(2π) in `buNaturalRotDef`.
- **The changelog** has a line.

No nits.

### 1c-01a, Null `04aad62`, by agent/cosmosis-a, November 7, 2026: sent back, one sentence

I checked section 6's units table against Sau's tree (Blender 5.3) and measured Blender's own parsing in Sau, through `just sandbox` with a private sandbox.

**Holds.**

- **All 16 kinds are there.** They match `PROP_UNIT_*` in `makesrna/RNA_types.hh`, `NONE` to `FREQUENCY`.
- **Every factor matches `blenkernel/intern/unit.cc`'s tables** as an exact decimal: lengths `km` to `pm` and `mi` to `thou`, mass `t` to `mg` and `tn`, `cwt`, `st`, `lb`, `oz` (the `UN_SC_*` values), time `d` to `µs`, camera `m` to `µm` in millimetres, power `GW` to `nW`, wavelength `mm` to `pm` in nanometres, `Hz` and `kHz`, and `K`. Temperature's offsets are right (273.15, 459.67, `UN_SC_FAH`).
- **Scene scaling is `BKE_unit_value_scale` exactly.**
  - Length, velocity and acceleration go by s; area and power by s²; volume and mass by s³.
  - Camera and wavelength are never scaled.
  - Nothing is scaled when the system is `NONE`.
- **Time.** Blender's interface really does not convert `PROP_UNIT_TIME`: `ui_but_is_unit` returns false for it ("for now disable time unit conversion", `interface.cc`).
- **Where the kinds are used.** `AREA`, `VOLUME`, `POWER` and `TEMPERATURE` appear in no RNA definition, only in `makesrna.cc`'s own tables. `MASS`, `FREQUENCY`, `WAVELENGTH`, `COLOR_TEMPERATURE`, `VELOCITY`, `ACCELERATION` and `TIME_ABSOLUTE` are used where the section says.

**To fix: Blender's typed values differ for most words, not only `F`.** The section says "Null's `F` uses 5/9 itself where Blender's interface uses 0.555555555555, so a typed `F` value may differ". Read on its own, that says every other word gives Blender's bits.

It does not. Every factor in `unit.cc` is a float32 literal (`UN_SC_CM 0.01f` is 0.009999999776…), and Blender multiplies by that. I typed 1 to 2000 of a word into `bpy.utils.units.to_value` and compared the float32 with Null's exact rule:

| Word | Typed values that store other bits |
| --- | --- |
| `cm` | 526 of 2000 (`9cm`: Blender 0.089999996, exact 0.090000004) |
| `mm` | 1251 of 2000 |
| `ft` | 329 of 2000 |
| `in` | 286 of 2000 |
| `mi` | 169 of 2000 |

S59 chose the exact rule, so Null is right. The sentence should say that Blender's interface converts with float32 factors, so a value typed into Blender may store other bits than the same word in Null, for most words, `F` among them.

**Nits, not blocking.**

- **The words left out.** They are said to be those "Blender marks as rarely used", which is not quite Blender's marking (`B_UNIT_DEF_SUPPRESS`):
  - Blender also suppresses `yd`, `km/h`, `mph`, `kW`, and the camera's `cm` and `µm`, which the table keeps;
  - Blender does not suppress the turn (`t`), which the table leaves out.

  Null's own list is fine; the sentence should say it is Null's choice.
- **The turn's factor.** Blender's turn has the factor `1.0 / (M_PI * 2.0)`, which would make a turn 1/(2π) radian. That is likely the upstream bug the author noted, and one more reason to leave turns out.

### 1b-02a, Null `f0b3faa` and `2e7b1c8`, by agent/cosmosis-a, November 7, 2026: approved

I checked section 7's sub-object handle rules and its table of 15 kinds against Sau's tree (Blender 5.3).

- **The six identities Blender keeps all exist as described.**
  - `ModifierData::persistent_uid` is an `int`, made by `BKE_modifiers_persistent_uid_init` (`modifier.cc:1082`) from a hash of the name, and the library path when linked, as the section says.
  - `bNode::identifier` is an `int32_t`, assigned by `node_unique_id` from a random generator seeded per tree.
  - `bNodeSocket::identifier` is `char[64]`.
  - A group interface socket has `char *identifier`, and a panel `int identifier`.
  - `KeyBlock::uid` is an `int`, from `Key::uidgen`.
  - `ActionSlot::handle` is an `int32_t`.
- **The name-only kinds have no such field:** constraints, bones (`bPoseChannel::session_uid` is per session, as written), vertex groups and attributes (`CustomDataLayer::uid` exists but is not stable for attributes).
- **The rules agree with S61:** made from the name, unique within the owner across kinds, `/` for paths, `.` for sockets and pattern copies, and `.sau/ids.noll` where Blender has only the name.
- **The table covers every kind the row names, and more:** modifiers, nodes, sockets, constraints, bones, shape keys, vertex groups, UV maps, attributes, view layers and action slots, plus group sockets and panels, bone collections, NLA, markers and Grease Pencil layers.

No nits.

### 1a-01, by agent/cosmosis-a, November 7, 2026: approved

Reviewed Null `34cf222` (the task's only commit) against its row on Sau's board.

**How.** Read the row, this report and the whole diff (`AGENTS.md`, root `Cargo.toml`, `Cargo.lock`, `compiler/null_syntax`, `justfile`). Ran Null's `just verify` on an exact copy of `34cf222` (`git archive`, in a scratch folder with `../Scowr` linked to the live checkout), since Null's main checkout held 1a-04's uncommitted work at the time. Ran Scowr's own `cargo test --workspace --locked --no-fail-fast` in `~/Projects/Scowr`, unchanged, with the build guard as `RUSTC_WRAPPER` and a target folder outside Scowr.

**Measured.**

- Null's `just verify` on `34cf222` exits 0: `cargo fmt --check`, Clippy with warnings as errors, `null_syntax`'s 5 tests and 1 doctest in debug and release, `cargo deny` (licences, bans, sources), and the fuzz runner (no targets yet).
- Null builds against Scowr's current tip, `770a472`, not only `4de7474`: the copy above built through the link to the live checkout.
- Scowr's own tests at its tip `770a472`: 192 passed, 0 failed, 49 suites. One suite (`scowr_check --test corpus`) failed in a first run started about 12:00, while another agent's Git operation was rewriting Scowr's `examples/fx-kinds.scowr` (its mtime is 12:00:23, after Scowr's last commit at 11:02; `git status` clean); it passed in the full run and twice more alone. Scowr's `git status` stayed clean before, during and after.
- `cargo fmt` without `--all` still covers Null's members and none of Scowr's: in the copy, a misformatted line in `null_syntax` made `cargo fmt --check` exit 1; `cargo fmt --check -v` named no Scowr file, `--all` named 40.
- The boundary is in `AGENTS.md`, "The shared front end": what is shared, what is Null's, that only `null_syntax` names `scowr_*`, that Scowr changes only with the user, and the two known syntax edges.

**"Done when"** holds: Null builds against Scowr's front-end crates by path, Scowr's own tests pass unchanged, and the boundary is written in `AGENTS.md`.

**Findings, none blocking.**

- The Scowr count the report quotes (192 at `4de7474`) could not be repeated at `4de7474` from an archive: Scowr's tests find their examples through `CARGO_MANIFEST_DIR`, and that run read the live checkout's newer examples (5 failures, all `legacy-param`, added after `4de7474`). It was repeated at the tip instead, with the same count.
- Scowr is being changed by other agents while Null builds against it by path, so a Null build or Scowr test run can catch Scowr mid-change, as above. Worth knowing before treating a one-off failure there as Null's.
- Release builds print `stripping debug info with rust-objcopy failed` (the toolchain's `rust-objcopy` cannot load `libLLVM.dylib`). A machine problem, not this task's; harmless to the results, but noise in every `verify`.
- `every_spec_example_parses_clean_and_prints_back` asserts exactly five examples, so adding one to the spec fails it. That is a reasonable tripwire; the message says why.
- The spec's own example `modifier @bevel bevel { width 4mm segments 3 }` does not mean what it says under the shared syntax (already recorded on 1a-06). It should be corrected in the spec itself, not only in the checker; for the spec's owner or 1a-17.

### 1a-04, by agent/cosmosis-a, November 7, 2026: approved

Reviewed Null `95f836b` (code) and `65c6ea7` (report) against the row on Sau's board.

**How.** Read the row, this report and the whole of `compiler/null_value` (`exact.rs`, `number.rs`, `unit.rs`, `tests.rs`). Ran Null's `just verify` on an exact copy of `65c6ea7` (`git archive`, `../Scowr` linked beside it). Checked the reader against an oracle of my own in Python: π from the Chudnovsky series in integers (neither Machin's formula nor the `decimal` recipe the tests' oracle uses), every conversion done exactly in fractions, and a hand-written half-to-even rounding into `f32` and `f64`, itself checked on π, 0.1, the least subnormal and a tie to zero. A probe crate outside Null's workspace fed it the cases.

**Measured.**

- Null's `just verify` on `65c6ea7` exits 0: `null_value`'s 17 tests (1 ignored, the exhaustive one) at Clippy, debug and 4,096 cases in release; `cargo deny`. The property tests at 100,000 cases each, in release: all pass, in 10.7 s. The 21-minute exhaustive test was not rerun.
- 9,144 oracle cases, all matching bit for bit, and every result printed and read back bit-identical: 6,000 random words (1 to 30 significant digits, exponents -30 to 25, bare and in every unit, `f32` and `f64`), 3,000 degree values written to 25 or 45 digits so that their radians fall within a hair of a rounding midpoint (the hardest case for the π bracket), and the fixed examples. `90deg`, `2.7m` and `5mm` print back as written in `f32` and `f64`.
- Read by hand, with no fault found: the π bounds are strict (each floored term errs by less than 1, so the error bound of 2(n + 1) per arctangent is generous); the degree bracket is monotonic and ends because π is irrational, the digits a precision cannot use being cut with the bracket widened; `round_ratio` finds its exponent in one step and rounds half to even, through subnormals and to infinity; `from_parts` is exact; lengths are exact decimal shifts and multiples of 254 and 3048, read once by the standard library's correctly rounded parser.
- Unit errors: a unit of the wrong dimension names the units that fit (`5deg` as a length, `2rad` as a length, `5m` where no unit goes); an unknown one suggests the nearest unit of the right dimension, within two edits (`5mmm` gives `mm`, `5dg` and `5Deg` give `deg`, `5meter` gives none); `deg`, `-` and `-nan` are not numbers.

**On the spec's "converted in float64 and rounded once".** The exact reading is the right one for S21 and S22 ("one exactly defined rule", exact to the stored bits), but it is a change to what the spec says and the spec should say it: "converted exactly and rounded once". Measured with the oracle on 20,000 degree values of 1 to 6 digits: for `f32` storage, converting in `f64` first (`x × (π/180)` in `f64`, as Blender does) and rounding into `f32` gives the same bits as the exact conversion every time; for `f64` storage it differs in 4,304 of 20,000 (21.5 %). So the slice's `f32` angles are unaffected, and only `f64` properties see the difference.

**Findings, none blocking.**

- The shortest print in a unit by search (`deg`, `in`, `ft`) can miss a decimal that exists, and then falls back to bare metres or radians: a value the user wrote in that unit is reprinted without it, which S21 ("Sau writes a value back in the unit its line already uses") would rather keep. Measured over 3,000 random values per row: never for `f32` (5 to 9 digits); for `f64`, only at 16 and 17 significant digits: degrees 0 and 6, feet 1 and 110, inches 2 and 75 (for example `-18750715569787871e-5ft`, a 17-digit value that reads back exactly yet prints as `-57152181056.71343`). The cause is the candidate window, ±1 in the last digit around an `f64` estimate. A value printed this way always reads back exactly, so nothing is lost. For 1a-07 (the minimal-edit writer): divide exactly for `in` and `ft` (by 254 and 3048), and widen the window for `deg`.
- `5MM` suggests `m`, not `mm`; `5mn` suggests `m` where `mm` is as near. Ties in edit distance go to the first unit listed. Preferring a case-insensitive match first would help.
- NaN payloads and the scene's unit scale are out of scope, as the report says; the spec should mention NaN when it is next edited.

### 1a-02, by agent/cosmosis-a, November 7, 2026: approved

Reviewed Null `0edf8f4` against the row on Sau's board.

**How.** Read the row, this report and the whole diff (`names.rs`, `lib.rs`, `tests/slice.rs`, the seven slice files and the eight snapshots). Ran Null's `just verify` on an exact copy of `0edf8f4` (`git archive`, `../Scowr` linked beside it). A probe crate outside Null's workspace checked printing independently, and the slice files were mutated in the copy to see the tests catch it.

**Measured.**

- Null's `just verify` on `0edf8f4` exits 0: `null_syntax`'s 5 unit tests, 6 slice tests and doctest, `null_value`'s 17, in debug and release; Clippy; `cargo deny`.
- Lossless, independently: the 7 slice files and the 37 `.noll` files of Cosmósis's six corpus drafts (`corpus/slice/`, written without reference to these files) each print back byte for byte, the lexer's tokens and the tree's tokens each cover every byte, and none has a diagnostic. 20,000 random mutations of those 44 files (1 to 6 insertions, deletions or replacements from braces, brackets, quotes, `#"`, `\`, comment openers and closers, CR, CRLF, tab, NUL, BOM, U+2028, multi-byte characters), most of them broken: all lossless by the same three checks, no panic.
- The tests catch changes: each of five mutations of `object.noll` alone, from the pristine file, fails `every_slice_file_lexes_parses_and_prints_back`: one digit (`0.35m` to `0.36m`), a trailing space, a CR, a doubled space, and `location` capitalised (also a `null-name` error). Restored, the suite passes, and no `.snap.new` is left behind.
- Coverage: the slice files write every phase 1a construct as the spec writes them: `project`, `scene` (and `collection`), `object` with `location`, `rotation-euler`, `scale`, `parent` and `data`, `mesh` as a `primitive` and as inline `vertices` and `faces`, `material` with a `principled-bsdf`'s `base-color` and `roughness` (as children and as properties, with links and a `layout`), `camera` and `light`; `lexical.noll` adds every lexical form of section 5. `the_slice_covers_its_constructs` fails if one goes missing.
- `null-name` fits section 18: each message names the file, line and column, says what was expected ("a node name is lower-case words joined by `-`"), and offers the kebab-case spelling as a fix; the code is stable, and section 18 lists codes "such as", so a new one is in keeping. The fixes `frameStart` to `frame-start` and `use2D` to `use2d` follow S16's mapping, and applying them gives a clean file (tested).

**"Done when"** holds: every slice construct lexes and parses to a tree that prints back byte for byte, with snapshot tests, and kebab-case names are enforced.

**Findings, none blocking.**

- Handles are checked only where a handle names a node (`layout`'s `@Tex [0 0]`). In arguments they are not: `object @Pier`, `node @Tex principled-bsdf`, `data mesh @Pier_Deck` and `material @weathered_oak` give no diagnostic, and neither does an upper-case enum value (`rotation-mode=XYZ`, which S16 writes lower-case). The report leaves these to the checker, which is reasonable, but the same handle `@Tex` is then an error in one place and not in the other. For 1a-06: check handle definitions and references, and enum values, under `null-name`.
- A `null-name` fix is spelling only, not schema-aware: `use2D` becomes `use2d`, which is kebab-case but need not be an RNA name. Once 1a-06 has the schema, the nearest RNA name should be the fix, so that a tool applying fixes does not get two different ones in a row.
- Scowr's syntax messages reach Null with Scowr's examples: `frame-start=1` written as a node gives `null-syntax: expected a key before =, such as vel=96`, a sound example. For 1a-03 (the parser's error recovery): give Null's diagnostics Null's examples.

### 1a-07, by agent/cosmosis-a, November 7, 2026: approved

Reviewed Null `ae2ebe4` on an exact copy (`git archive`, `../Scowr` linked beside it).

**Measured.**

- Null's `just verify` on the copy exits 0: the workspace's tests with `null_print`'s property tests, `cargo deny`, the release tests and a 61 s fuzz run (58,423 inputs, no crash).
- 34 adversarial cases of my own through `write`, each checked for: no new diagnostics, every comment unchanged, the text's length changed by exactly the edits, and the attribute reading back to the new bits. All hold:
  - Edits beside comments: a line comment glued to the list (`[1m 2m 3m]// c`), a block comment before the attribute, comments inside a list over several lines, and a trailing block comment.
  - Other shapes: a `;`-separated one-line block, a `\` continuation, the value as `attr=` on the node's line, a raw-string display name, and an attribute after a nested block.
  - Scoping: an attribute of the same name only inside a nested node (it inserts on the object, not the modifier), an attribute present only commented out with `/-` (it inserts), a `/-` duplicate of the whole datablock, and a handle made from the display name.
  - Insertion forms: into `{}`, `{ }`, a block holding only a comment, a node with no block but a trailing comment (`attr=value` before the comment), a one-line block, a last child with a comment or a `\` continuation, a file with no final line feed, a last child without one, and tab indentation.
  - Values: a missing value after `=`, a word where a list was, a list of another length, a string inside a list, a unit kept per word (`[10cm 2 3000mm]` gives `[25cm 2 3000mm]`), an unchanged value left exactly as written, degrees kept, and CRLF files.
- `format` on raw strings and block comments over several lines, `\` continuations, strings with spaces, `;`, blank lines, CRLF, tabs and `/-`: idempotent, and every comment, string and non-whitespace token unchanged.
- The `null_value` fix: my 1a-04 measurement again (99,000 values of 5 to 17 significant digits in `deg`, `in` and `ft`, `f32` and `f64`): the unit is now kept in every one, where 194 lost it before, the 17-digit `ft` case among them.

**"Done when"** holds: changing one value rewrites only that value's characters, and comments and layout survive (the property tests, and the cases above).

**Findings, none blocking.**

- Inserting into a block whose `{` line holds only a comment (`object @a { // only` and then `}`) puts the new line straight after `{`, so the comment moves from the brace's line to the end of the new attribute's line, where it reads as that attribute's comment. Insert after the comment instead, at the start of the next line.
- An insertion into a CRLF file adds a bare LF (`…\r\n  location [1m 2m 3m]\n}\r\n`), so the file ends up with mixed line endings; use the file's own. `format` turns CRLF into LF, which suits a canonical form; section 15 could say so.
- `format` indents a `\` continuation line at its attribute's depth (`location \` then `[1m 2m 3m]` at the same indentation); one level deeper would show it continues.

### 1a-06, by agent/cosmosis-a, November 7, 2026: approved, with one follow-up that must happen

Reviewed Null `4d7b0d7` and the report `bf5f99c` (the row's `5c52dd2` is not in Null's history; `bf5f99c` is the commit after `4d7b0d7`) on an exact copy at `bf5f99c` (`git archive`, `../Scowr` linked beside it).

**Measured.**

- Null's `just verify` on the copy exits 0: the workspace's tests (`null_check` 7, `null_value` 20 with the exhaustive one ignored, `null_syntax` 6, 5 and the rest), Clippy, `cargo deny` over the workspace and `fuzz/`, the release tests, and a 61 s fuzz run (52,842 inputs, no crash).
- "Done when": the misspelt name (`locaton`) is `null-unknown-property` with the fix `location`; the wrong unit (`45dgr`) is `null-unit` with the fix `deg`; the wrong type (`scale 2`) is `null-type`, its message naming the form wanted, "takes 3 numbers, as a list such as [0 0 0]", with no fix, which is fair: only the author knows the three numbers. The 29-error snapshot covers all ten checker codes (unknown-property, type, unit, range, enum, handle, readonly, repeated, name, kind), and `null-syntax` comes from the parser: eleven, as section 18 lists.
- `50mm` as a lens: `lens` is RNA's camera unit, millimetres, so `50mm` is 50, `50` is 50, and `5cm` and `0.05m` are refused ("takes mm or a bare number"); a length's `mm` still means 0.001 m (`5mm` is 0.005), and 50 prints back as `50mm`. Right.
- The sRGB rule: over all 256 channel values, float64 `pow` then rounding to float32 gives exactly the bits of one exact rounding (checked against 60-digit decimal arithmetic), on this Mac.

**The settled names and rules, against the register.** Consistent: `polygons`, RNA's own name (S16); handles and enum values kebab-case wherever they appear, `XYZ` an error fixed to `xyz` (S16, S18); scene and collection members as lines, `data <kind> @h`, material slots as lines (forms over read-only RNA collections, S35's short forms with the full names kept); `units` short names with the full names allowed as child lines (S35); a light's `energy` in watts with a bare number meaning watts, and a sun's bare (S21: a bare number is the internal unit); defaults as a new datablock's value (S35's omission of defaults, the formatter's); the code list (section 18); the `layout` example with `;` (the same reading as S59's bevel). None contradicts S16, S17, S21, S22 or S24.

**Must be fixed, on a follow-up row (it needs Sau first).** The primitive rule says "`size` is one length: a box of other proportions is a cube scaled by its object", and section 4's pier now reads `primitive cube size=1m` on an object with `scale [1.2 3 0.08]`. That changes what the example means: its `bevel width=4mm` is now 4 mm across and 0.32 mm in height, and every child of the pier inherits the squash. Blender's add operator shapes the mesh itself: in the built Sau, `primitive_cube_add(size=1, scale=(1.2, 3, 0.08))` gives a 1.2 × 3 × 0.08 mesh with the object's scale left at 1. The schema leaves `scale` out of the primitives as "placement" (Sau's `sau/tools/null-schema.py`, `PLACEMENT`), which is what forced the rule. Fix: keep the operators' `scale` in the schema for primitives (Sau), accept `primitive cube size=1m scale=[1.2 3 0.08]` (Null), and give the pier back an object scale of 1. My corpus drafts' `size=[2m 2m 2m]` is 1a-15's either way.

**Minor.** The sRGB wording says "computed in float64 and rounded once", the same wording S59 rejected for units; it is right in effect on this Mac, but `pow` differs between platforms (S12), so a 256-entry table of exactly rounded values would make it true everywhere. The wrong-type message for `scale` suggests `[0 0 0]`; `[1 1 1]`, or the written number three times (`[2 2 2]`, as a fix), would serve better.

### 1a-03, second review, by agent/cosmosis-a, November 7, 2026: approved

Reviewed the fixes `1d79a59`, `c64bfd8` and `8192b21`, and the report `4416d79`, on an exact copy of Null at `4416d79` (`git archive`, `../Scowr` linked beside it).

- Null's `just verify` on the copy exits 0: format, Clippy, the tests (the two new ones included), `cargo deny` over the workspace and now over `fuzz/` too, the release tests, and the 60 s fuzz run at the new limits (`lim: 8192`, 45,935 inputs, no crash).
- My crash input from the first review (97 bytes) replays without a panic. A fresh fuzz run from the committed seeds, 301 s at `nice 10` under heavy thermal pressure, `-max_len=16384 -len_control=0`: 91,952 inputs, coverage 3,235, corpus 2,075 inputs (2.8 MB), peak 1,068 MB, no crash, no artifact.
- A new or changed Scowr message cannot leak unseen any more: removing each of the 22 `KNOWN` entries in turn fails Null's tests for all 22 (the two depth messages through `nesting_past_the_limit_is_an_error_not_a_crash`). And the other way round: disabling each of the three rewordings, so Scowr's example would reach the user, fails four or five tests each.
- The invariant now matches Scowr's whole example phrases (`SCOWR_EXAMPLES`); `level=` and `travel=` are tests, and the crash input is a seed.
- The licence gate covers the fuzz crate: with NCSA in `deny.toml` it passes; with NCSA removed, `libfuzzer-sys` (`(MIT OR Apache-2.0) AND NCSA`) is rejected.

"Done when" holds: broken input gives a tree and diagnostics, never a panic, and the fuzz target runs 10 minutes without a crash (the author's 604 s; mine 301 s at a larger input limit). One thing to watch, not blocking: memory reaches about 1.1 to 1.2 GB in 5 to 10 minutes, under libFuzzer's 2 GB default, so a run much longer than 10 minutes will need `-rss_limit_mb` or a trimmed corpus.

### 1a-03, by agent/cosmosis-a, November 7, 2026: sent back

Reviewed Null `5cef529`, `a5262c3` and `900a7e1` against the row on Sau's board, and `59061db`, the spec edit, against S59. Exact copies of Null at `59061db` (`git archive`, `../Scowr` linked beside it) in my scratchpad; Null itself untouched but this section.

**Measured.**

- The fuzz target, rerun from the committed seeds at `nice 10`, thermal pressure heavy: 301 s, 288,898 inputs, coverage 3,284, peak 915 MB, no crash. libFuzzer's input limit stayed at 583 bytes, the largest seed, as in the author's run.
- **A second run, from that corpus with `-max_len=16384`, crashed after 13,547 inputs** (`invariants.rs:46`, "Scowr's example in expected a value after level="). The input is 97 bytes, under the 583-byte cap, so the default run can find it too. It is not a parser fault but a false alarm in the invariant: it bans the bare substring `vel=` from every message, and the shared parser echoes the user's key (`expected a value after level=`). Minimal reproductions through `invariants::check`: `modifier @sub subsurf level=` and `object @a travel=` panic; `object @a speed=` passes. As it stands, `just verify`'s fuzz run and the nightly can fail at random on a key that ends in `vel` (`level`, `travel`, `bevel`'s neighbours).
- A wholly new Scowr message is caught only when some test input produces it. Removing each of the 22 `KNOWN` entries in turn (standing in for a message Null has not read) fails Null's tests for 20; for `blocks are nested too deeply` and `lists are nested too deeply` every test passes, since no test nests past Scowr's limit of 256. Deep nesting itself is safe: up to 1,000,000 levels of `[`, `{` and `x {`, `invariants::check` holds, edits included, with no stack overflow and only known messages (about 257 or 513 diagnostics, then no more).
- Null's `diagnostics` is built as Scowr's own `Parse::diagnostics` is (message, span, fix), with Null's wording: nothing of Scowr's is lost.
- `59061db` matches S59: section 6 says "converted exactly and rounded once" and how NaN is written, section 4's bevel is `{ width 4mm; segments 3 }`, the CHANGELOG says so, and the test now expects two attributes. S59's items 3 to 5 are Cosmósis's and match its spec, section 4.

**To fix.**

1. The invariant's check for Scowr's examples: match the whole example phrases (`such as vel=96`, `such as track {`, `\u{266A}`), not `vel=`, `track {` and `266A` anywhere; add the crash input, or `modifier @sub subsurf level=`, as a test and a seed.
2. A deep-nesting case in `tests/recovery.rs` (more than 256 levels of `{` and of `[`), so both depth messages are pinned like the other 20, and the depth guard against stack overflow is tested in Null.
3. `tools/fuzz.sh`: pass a `-max_len` larger than the seeds (8 KB or so), so the fuzzer tries inputs the size of real files.

The pending `cargo deny` commit for the fuzz crate (NCSA) was not yet on the row when this was written; I will check it with the fixes.

### 0-03, the agent environment (the lead's), re-reviewed by agent/null-a, November 7, 2026: sent back

Re-review after agent/cosmosis-a's send-back (Cosmósis `e73d82e`), of Sau `01086e7`, `45616a1` and `f32ebbd` and the hand-run nightly at `45616a1`. The board was tested only on a throwaway `git clone --shared` of Sau's `main` (`c78357d`, sparse: `docs/`, `sau/`, `justfile`), never on the real board.

**Measured.**

- **Notes append (`01086e7`): yes.** On task 0-04 in the clone: claim, note, review (`--commit aaa111`), approve by another agent, note: the row reads `Ask the user first; first evidence; in review at aaa111; approved by agent/y; a note after approval`. Nothing was erased, and the approval survived the later note.
- **`review` drops old approvals: yes.** A second `review --commit bbb222` gave `...; a note after approval; in review at bbb222`, with `approved by agent/y` gone and every other note kept. Self-approval is still refused.
- **The board lock (`f32ebbd`): yes.** With a second process holding `sau-tasks.lock` for 5 s, `dev.py task note` printed "task: waiting for the board…", waited 4 s, then committed alone, leaving nothing staged. The lock is taken once per process, so `integrate`'s own board commits do not deadlock on it (read in the code).
- **Worktrees link Scowr (`45616a1`): yes, through the nightly run.** `work/nightly/Scowr` links to `~/Projects/Scowr`, and the nightly's Null verify compiled `scowr_errors` and `scowr_syntax` through that link.
- **The nightly at `45616a1`: passed.** `work/nightly/latest.json` reads `{"result": "passed", "commit": "45616a1", "seconds": 1903, "finished": "2026-11-07 13:36", "skipped": "blender-tests: no developer build in .build/developer (task V-01)"}`. Its log ends `nightly: exit 0`, with `verify: passed` for Null (its 60 s fuzz run included), Cosmósis and Sau.

**To fix** (both small, both in `sau/tools/dev.py`, both about approvals the earlier fix set out to protect):

1. **`integrate` counts any mention of "approved by" as an approval.** Its test is `"approved by" not in r["notes"]`, a substring anywhere in the notes, while `review` drops only the notes that start with `approved by `. In the clone, after the second `review`, a note "earlier this was approved by agent/y, then fixed" made the row pass `integrate`'s test, with no fresh approval in this round. `integrate` should look for a note that starts with `approved by ` and names an agent other than the task's own, as `approve` does.
2. **`done` replaces the notes and checks no approval.** `done --commit ccc333` turned the row's seven notes, approval included, into `ccc333`. That erases the author's evidence, which `workflow.md` now says never happens ("Notes add to the row, never replace it"). And `done` is how every Null and Cosmósis task ends under S58. It should append `done at <commit>` and refuse a task that is not in `review` with an approval from someone other than its author, as `integrate` refuses one.

**Also found, not blocking.** A note containing `|` is stored as is: the row is still found and later notes still append (tested), but the Markdown table gains a cell, so the row renders wrongly. Escape it as `\|`, or refuse it.

### 0-03 again, the fix `ed31e44`, re-reviewed by agent/null-a, November 7, 2026: sent back for one line

Tested on a fresh throwaway `git clone --shared` of Sau's `main` (`55fc14f`, sparse), on task 0-04 there; never on the real board.

**The approval fixes hold.**

- `done` on a task in review with no approval is refused ("not approved by another agent in this round of review").
- A note cannot record an approval, whether the whole note is `approved by agent/y` or it is hidden after a `; `.
- A note that only mentions an old approval in mid-text ("earlier this was approved by agent/y, then fixed") no longer counts.
- Self-approval is refused.
- After an approval, a new `review` round drops it, and `done` is refused again until a fresh approval.
- With one, `done` appends `done at ccc333` and keeps every earlier note.
- A `|` in a note is stored as `\|`, so the row keeps its 8 cells.
- `integrate` uses the same `approved()` as `done` (read in the code).

**To fix.** A line feed in a note splits the row in two: in the clone, `task note 0-02 --note "line one<LF>line two"` left 0-02 matching no row, so `just tasks` stopped listing it and the next `task note 0-02` failed with "no task 0-02 on the board". The task silently drops off the shared board, and a note made with `--note "$(…)"` from a command's output can easily hold a line break. Fix: in `task()`, beside the `|` escape, write line breaks as spaces (`" ".join(note.split())`, or `note.replace("\r", " ").replace("\n", " ")`), or refuse the note.

### 0-03 again, the fix `d67968a`, re-reviewed by agent/null-a, November 7, 2026: sent back, an ordering slip

Tested on a fresh throwaway `git clone --shared` of Sau's `main` just after `d67968a` (sparse), never on the real board.

**The line-feed fix works.** Notes holding a line feed, a tab, doubled spaces, CRLF and a `|` are each stored on one line (`line one line two tab two spaces; crlf end; a \| b`). The task stays on the board (`just tasks --all` lists it), the board keeps its 75 rows, and the next note appends.

**To fix.** The folding now runs *after* the check that a note records no approval, so that check can be got round with a line break. In the clone, on 0-04: claim, `review --commit a1`, then `task note 0-04 --note "fine;<LF>approved by agent/q"`. The check splits on `"; "` and finds no segment starting `approved by `; then the folding turns the note into `fine; approved by agent/q`. That counts as an approval, and `done --commit z9` marked 0-04 done although nobody ran `approve`. A tab after the `;` does the same. Fix: fold the note first (`note = " ".join(note.split())`), then run the approval check on the folded text, then escape `|`.

### 0-03 again, the fix `d00308d`, re-reviewed by agent/null-a, November 7, 2026: approved

Tried to break it on a fresh throwaway `git clone --shared` of Sau's `main` at `cf59d9f` (sparse), never on the real board.

**Holds.**

- An approval smuggled into a note is refused in every form tried: after `;` and a line feed; after a tab; with doubled and leading spaces; after U+2028, U+001C, U+0085 and a no-break space. Each of these is a line break or whitespace to Python, so the fold catches them before the check.
- `done` was still refused afterwards, with no approval in this round.
- `--by` refuses `agent/q; approved by agent/z`, a name with a line feed in it, and `a|b`. `--commit` refuses `a1; approved by agent/q` and three words, and takes `Null a1b2c3d`.
- With a real approval from another name, `done` accepts the task and appends `done at <commit>`, keeping every note.

**Not blocking.** `--by` takes two words, as `--commit` does, so an author can approve their own task as `agent/x again`, which is not literally `agent/x`. In the clone that approval let `done` through. Names are not authenticated anyway (anyone can type another agent's name), so this adds nothing a deliberate cheat lacks, but `--by` could take one word only and close it.

**"Done when"**: the board, guides, recipes, hooks, guards, `check` and `verify` work in all three repositories. The board's approval rules now hold against everything I tried. The nightly at `45616a1` passed (reviewed above).

### 0-06 (agent/sau-b's), reviewed by agent/null-a, November 7, 2026: approved

I reviewed Sau `80df787`, the one 0-06 commit on `agent/sau-b` (`git log main..agent/sau-b`; the branch's V-01 and 1a-09 commits are not part of it). I read the `Sau:` hook in `build_files/cmake/platform/platform_apple.cmake`, `sau/tools/check-links.sh`, the line in `check.sh` and the `HANDOFF.md` entry.

**Measured.**

- **`just check` at `80df787`, in a clean detached worktree** (`work/review-0-06`, made with `wt-setup` and removed afterwards): exit 0 after 905 s at heavy thermal pressure. It printed `check-links: no Homebrew or /usr/local library linked in Sau.app` and `check: passed`. The build log has no `ld: warning` line.
- **Independently of the script:** I ran `otool -L` over every Mach-O file in the built app (206 of them, found by `file`, not by the script's own selection). None lists a library under `/opt/homebrew` or `/usr/local`. In the build's `CMakeCache.txt`, the only paths under `/opt/homebrew` are CMake's own tools (`cmake`, `ninja`). `OPENIMAGEDENOISE_COMMON_LIBRARY` is NOTFOUND, and OpenImageDenoise itself comes from `lib/macos_arm64`.
- **It fails if the library comes back:** `check-links.sh` from `80df787`, run on the main checkout's old build (`.build/release`, read only), exits 1 and names `Contents/MacOS/Sau -> /opt/homebrew/opt/fbthrift/lib/libcommon.dylib`.
- **The hook's logic, in a standalone CMake probe:**
  - Without it, `find_library(common)` finds `/opt/homebrew/lib/libcommon.dylib`.
  - With it, in the same build folder, the cached path is forgotten and the search gives NOTFOUND.

  So both the search and the stale-cache case are covered.

**"Done when"** holds: `otool -L` on the app and its libraries lists nothing under `/opt/homebrew` or `/usr/local`, the linker warning is gone, and the check fails when such a link returns.

**Not blocking.**

- `check-links.sh`'s header says `smoke.sh` runs it; it is `check.sh` that does.
- `CMAKE_IGNORE_PREFIX_PATH` exists from CMake 3.23, and Sau's minimum is 3.21. On an older CMake the hook would do nothing without saying so. CMake 4.4.2 is installed here.
- `otool -L` sees only dynamic links. A static archive found under Homebrew would not show. Checking `CMakeCache.txt` for library paths under the two prefixes would cover that too.

### 1a-14a again, the fix `42662ae`, by agent/cosmosis-a, November 7, 2026: approved

Reviewed Null `42662ae` on an exact copy (`git archive`, `../Scowr` linked beside it), with my probe crate from the first round pointed at the copy.

**Measured.**

- Null's `just verify` on the copy exits 0, with the fuzz run at 51,485 inputs in 61 s.
- The 33 cases of the first round, plus 8 new ones, all hold. No edit leaves a parse diagnostic, and none panics.
- **Both findings are fixed.**
  - `object @cube` written twice in one file is `DefinedTwice`, and so is `@big-rock` beside `object "Big Rock"` in one file.
  - A `/-` copy next to the real definition is still ignored.
  - NaN and infinity are `NotFinite`, for a changed value and for a new attribute alike.
- **The display unit is now read from every form.**
  - `length=centimeters` and a `units { length-unit centimeters }` block both give `location [150cm 200cm 300cm]`.
  - A scene's `units length=mm` gives `[1500mm 2000mm 3000mm]`.
  - An unknown unit (`length=furlong`), or one of the wrong kind (`length=deg`), falls back to bare values, which stay right.
- **`FileEdit`'s doc now says how to apply the edits:** several edits per file, ranges in the text before any edit, never overlapping, applied from the last.
- **The 1a-07 fixes still hold:** a comment on the opening line in LF and in CRLF.

**Nits, not blocking.**

- `DefinedTwice`'s message still says "more than one file defines that object" when one file defines it twice. "More than one definition" would fit both cases.
- With `units length=cm` in the project and `length=mm` in a scene, a new location is written in cm, so the project's unit wins. The spec says a project's `units` "applies to every scene", which reads the other way when a scene sets its own. Worth one sentence in section 8.

### 1a-14a, by agent/cosmosis-a, November 7, 2026: sent back, two small fixes

Reviewed Null `4ee87a9` (code; report `77d667d`) on an exact copy (`git archive`, `../Scowr` linked beside it), and through a probe crate of my own calling `null_print::write_transform` by path (outside Null, nothing written into it).

**Measured.**

- Null's `just verify` on the copy exits 0 (5 min 31 s, the fuzz run 44,937 inputs in 61 s); `null_print`'s tests pass.
- 40 cases through `write_transform`, each applying the edits and parsing the result. These hold:
  - **Objects in two files.** An object defined in two files is `DefinedTwice`, and so is `@big-rock` in one file with `object "Big Rock"` in another. A `/-` copy, in the same file or another, is ignored. A mesh with the same handle placed first is not taken for the object.
  - **Handles missing.** `@nope`, `cube` without `@` and the empty handle are each `NoSuchObject`. A handle made from the display name works. A non-ASCII display name such as "Café Table" has no handle, and the checker asks for one, so `NoSuchObject` is consistent.
  - **Units kept or set by the project.** With `units length=cm`, a changed word keeps its own unit (`[1m 2m 3m]` gives `[1.5m 2m 3m]`). A new attribute is written in the project's unit (`location [150cm 200cm 300cm]`, `rotation-euler [0deg 0deg 45deg]`).
  - **Multi-byte text.** With multi-byte text before the edit and on its line (`Café — the 小屋`, `"Cubé 小"`, a trailing `// naïve`), every range falls on a character boundary and the text around it is kept, for a change and for an insertion.
  - **The 1a-07 findings.**
    - A comment on the opening line stays there, in LF and in CRLF.
    - A CRLF insertion ends in CRLF.
    - `format` indents a `\` continuation one level deeper, also over two continued lines.
  - **Other shapes.** These all work: a one-line block (`; scale [2 1 1]`), no block (` scale=[2 1 1]`), the value as `location=[…]`, no final line feed, a continued value, `scale 2` replaced by a list, and an attribute present only as `/-` (inserted).

**Sent back for:**

1. **The same object defined twice in one file is not `DefinedTwice`.** `write_transform` edits the first and says nothing (`object @cube {…}` twice in `a.noll`: the first one's `location` changes). Across two files the same project is an error. The duplicate check counts files, not definitions; count the definitions, so one file with two is refused as two files are.
2. **NaN and infinity are written as words.** `[NaN 2 3]` writes `location [nan 2m 3m]`, and infinity writes `inf`. These are words, not numbers, so the file no longer checks. A degenerate matrix in Sau can give a NaN. Refuse values that are not finite: a `WriteBackError::NotFinite`, say, with a test.

**Findings, not blocking.**

- **The project's display unit is read only from `units … length=<unit>`.** The spec's other forms all fall back to bare internal units without a word (`location [1.5 2 3]`):
  - items (`length=centimeters`, `rotation=degrees`);
  - child lines (`units { length-unit centimeters }`);
  - a `units` line in a `scene`.

  The values stay right, since bare is metres and radians, but the project's unit is lost. Worth reading every form the checker accepts, or saying in the doc comment which ones count.
- **Without `units`, a new rotation is bare radians** (`rotation-euler [0 0 0.7853982]`). That is what the spec says ("bare when there is none"), but every corpus file writes degrees. Blender's default display is degrees; a default of `deg` may read better. This is for the spec's owner.
- **`FileEdit`'s doc should say how to apply the edits.**
  - Several edits can come for one file (three values changed: three edits).
  - Every range is in the original text, and none overlaps.
  - So the caller applies them from the last to the first, or sorted with an offset.

  Sau's side (1a-14) needs exactly that.

### 1d-04b (agent/cosmosis-a's), Cosmósis `681e148`, reviewed by agent/null-a, November 8, 2026: sent back, one silent case

I read the row, the report's section, the diff (`diagnostic.rs`'s related places, `layout.rs`'s `.sau/ids.noll` and `holds_datablocks`, `schema_check.rs`, the templates, spec section 2) and `tests/layout_rules.rs`.

**Run, headless:**
- `cargo test --locked` passes, every suite (built into a scratch target folder, nothing written in Cosmósis).
- I tried the built `sau check` on a scratch project.

**What holds:**
- **Duplicates name both files.** A handle defined twice gives both files, the first as `::: objects/a.noll:1:8: first defined here`. With three definitions, each later one points at the first.
- **`ids.noll` is read and checked.** A syntax error in it is reported at its line, and a folder named `ids.noll` is an error.
- **The cache is still skipped.** A broken `.noll` elsewhere in `.sau/` is skipped, and so is a nested `objects/.sau/ids.noll`.
- **Syntax only where the forms aren't specified.** `settings.noll` and `view.noll` with any content are clean, checked for syntax alone.
- **The templates** are tested as Git itself reads them, which is the right test.

**To fix:**
1. **A broken link at `.sau/ids.noll` is silent.**
   - `check` tests `ids.exists()`, which is false for a link to nothing, so `sau check` passes with exit 0.
   - Everywhere else, `walk` reports a broken link as `cosmosis-io` ("it is a broken link").
   - The ID map is the one file whose loss changes every datablock's identity on the next load (S61 (2)), so it should be the last to go unnoticed.
   - Check `symlink_metadata` as `walk` does, and add the case to `layout_rules.rs`.

**Nits:**
- **Folder links.** A `.sau` that is a link to a folder elsewhere is followed for `ids.noll`, though `walk` never follows a link to a folder ("a link to a folder is not followed"). Either is defensible, but say which in spec section 2. Reading the map through a link to another project's cache seems the riskier choice.
- **Grammar, in Null's message, not this row:** "defined twice as a object". It is Null's checker message; I'll fix it in Null.

### 1d-04a again, Cosmósis `883d07b` (report `979fb69`), re-reviewed by agent/null-a, November 7, 2026: approved

**The two spec lines.** Section 4 now says three things:

- `cosmosis.noll` is a Null file first. Its syntax errors and non-kebab-case node names are Null's own (`null-syntax`, `null-name`), reported before the fields are read, so `Trees "^1"` is `null-name`.
- `version`, `sau` and `null-format` have no leading zeros, as semantic versioning writes them.
- An empty `path=""` is an error.

The cases now follow the spec's words.

**The nits, checked on a clean copy of Cosmósis main with a scratch probe.**

- `trees path=../Trees` now suggests `path="../Trees"`, keeping its case.
- A 20-digit `null-format` is `newer-format`.
- `path=""` is the new `cosmosis-manifest-empty-path`.
- `null-format 01`, `+2` and `version 01.2.3` stay refused.
- Cosmósis's tests pass: 48 across its suites.

**C-05's follow-up.** macOS's save folder is now matched by its shape, a name wholly in parentheses, since its words are localised. No kebab-case name has parentheses, so no project file can match.

**Not blocking.** The help for a leading zero still says only "whole numbers"; it could add "without leading zeros", as the spec now does.

### 1d-04a (agent/cosmosis-a's), Cosmósis `bea7dab` (report `5bae962`), reviewed by agent/null-a, November 7, 2026: sent back, two lines of spec

I read `tests/manifest_fields.rs` against the spec's section 4, case by case, and ran twelve more manifests of my own through `Manifest::parse` on a clean copy of Cosmósis main. The probe was a scratch test, removed afterwards.

**Holds.**

- The 59 cases pass.
- `FIELDS` equals the example's fields.
- Every `cosmosis-manifest-*` code in the source is met by a case.
- Most cases are the spec's own words:
  - the name is required, quoted and kebab-case;
  - `kind` is one of seven words and defaults to `project`;
  - `version` has three parts and `sau` two or three;
  - `null-format` is required, a whole number from 1, newer refused;
  - `license` is quoted;
  - a dependency gives a requirement, a `path=`, or both;
  - fields may go on the package's line;
  - a field written twice, an unknown field and a wrong form are errors.
- My extra manifests also behave as the spec says:
  - `null-format` on the package's line and again in the block gives `repeated-field`;
  - `1.0.0-alpha`, `null-format 1.0` and a 20-digit version are errors, not panics;
  - an unquoted `path=` is a `type` error;
  - `kind Project` suggests `project`;
  - a bare `"1.2"` requirement reads.

**The two ordering findings.**

- **`dependencies trees` giving `cosmosis-manifest-type` is right by the spec.** `dependencies` takes a block, and a bare word is "a value of the wrong form".
- **`Trees "^1"` giving `null-name` is right by Null's rules, but the spec does not say so.**
  - A dependency's name is written as a node's name, and Null's parser requires every node name to be kebab-case (S16).
  - It fires before Cosmósis reads the manifest, with a good fix ("write `trees`").
  - A package's name is a quoted string, which only Cosmósis checks (`bad-name`).
  - But section 4 says every error carries a code starting `cosmosis-manifest-`.

**To fix (spec, a line each).**

1. **Null's errors in the manifest.** Section 4 should say that syntax and node-name errors in `cosmosis.noll` are Null's own (`null-name`, `null-syntax`), reported before the manifest is read. A dependency's name is one of them. Then the `Trees` case is the spec's rule, not the reader's habit.
2. **Leading zeros.** The spec says versions are "whole numbers written bare", but `version 01.2.3` and `sau 0.01` are refused, and a case pins it. `01` is a whole number. Either add "without leading zeros, as semantic versioning writes them" to the spec, or accept them; as it stands the case tests the reader, not the spec.

**Nits, not blocking.**

- **The `path=` help lower-cases the path.** `trees path=../Trees` suggests `path="../trees"`, which is another folder on a case-sensitive file system. It should keep the path as written.
- **A huge format.** `null-format 99999999999999999999` is called "not a format number". It is a whole number from 1, only newer than this Cosmósis reads, so `newer-format` would be more accurate.
- **An empty path.** `path=""` is refused with "takes a quoted folder path … not `""`", but it is quoted. "An empty path" would say what is wrong.

### C-05 (agent/cosmosis-a's), Cosmósis `63d05e3`, reviewed by agent/null-a, November 7, 2026: approved

I reviewed `63d05e3`, the ignored names in `layout::is_ignored` and the spec's section 2, on a clean copy of Cosmósis main (`git archive`, with Null and Scowr linked beside it).

**Measured.**

- **The stress, with temporary names ending in `.noll`.** I ran `sau check` 300 times on a copy of `one-cube.sau` while a script made 174,528 saves by rename in 25 s. Its temporary names were:
  - `cube.noll.saving.noll`, `.cube.noll`, `.#cube.noll` and `~cube.noll`, which end in `.noll`;
  - `cube.noll.sb-a1b2c3d4-AbCdEf`, `cube.noll___jb_tmp___`, `cube.noll.vsctmp`, `#cube.noll#`, `cube.noll~`, `cube.noll.tmp` and `cube.noll.BAK`;
  - a `cube.noll` inside "(A Document Being Saved By TextEdit)".

  No check failed.
- **The control.** The C-04 build, before this change, failed 22 of 150 checks under the same script, each with `@cube` and `@cube-mesh` "defined twice".
- **Sau's live reload.** It loads through the same walk (`sau_core::project::load`). Its 5 live tests pass 3 runs of 3 against Cosmósis main, built in my scratchpad.
- **`.vsctmp` is confirmed.** It is VS Code's atomic-write postfix (`{atomic:{postfix:".vsctmp"}}` in `fileService.writeFile`). I found it in Cursor's bundle, a fork of VS Code (`out/main.js` and the workbench), since VS Code itself is not installed here. VS Code uses it for files it writes atomically (its storage and settings files), not for every editor save. It is right to keep.

**Can a real project file be ignored by mistake?** Not one named as S16 asks: a kebab-case name has no leading `.` or `~`, no `#`, `~` or `_`, and no `.` before `.noll`, so none of the patterns can match it. Two cases are worth a line in the spec:

- **Nothing reports a skipped name.** Cosmósis does not enforce kebab-case file names, so `C#.noll`, `draft~.noll` or a folder `old.bak/` would be skipped without a word.
- **Content files.** `media/` and `data/` hold SHA-256 names with an extension, which never match.

Reporting an ignored `.noll` name as a note would keep it visible without making it an error.

**Not blocking.**

- `starts_with("(A Document Being Saved By")` is case-sensitive and English. macOS localises that folder's name, so on a Mac in another language it would be read.
- A file named `4913` (Vim's write test) is ignored already, as is every file that does not end in `.noll`.

### C-04 (agent/cosmosis-a's), Cosmósis `4c9d68b`, reviewed by agent/null-a, November 7, 2026: approved

I reviewed `4c9d68b` on a clean copy (`git archive`), with Null's and Scowr's mains linked beside it.

**Measured.**

- **The test catches the bug.** The layout tests pass 5 runs of 5. With `layout.rs` put back as it was before the fix (the new test kept), the 200-check test fails in each of 3 runs, with 12, 17 and 20 false reports. Most are "broken link" for `objects/cube.noll.saving.noll`; some are the second form the fix covers, "No such file or directory" from reading it.
- **A real broken link is still reported** (`a_real_broken_link_is_still_reported`).
- **The whole crate.** `cargo test` passes all 57 tests, and `cargo clippy --all-targets -D warnings` is clean.
- **`sau check` while files are saved.** I ran it 300 times on a copy of `one-cube.sau` while a script made 139,056 saves by rename in 20 s. It used three temporary names: `cube.noll.saving.noll`, `.cube.noll.tmp` and `cube.noll~`.
  - It gave no `cosmosis-io` error.
  - 39 checks failed, all with `null-handle`: "`@cube` is defined twice", and the same for `@cube-mesh`.
- **A folder moved away and back** 3,000 times during 200 checks: no failure.

**Read.**

- A folder's listing that fails with NotFound is skipped below the root. At the root it is still an error, since the project itself is gone.
- `walk` reports a broken link only when `symlink_metadata` still finds the entry.
- `source` and `content` skip a file that is NotFound, and `content_files` counts only files that were read.

**Not blocking, for Sau's live reload (1a-12, 1a-13).**

- A temporary file whose name ends in `.noll`, read whole before the rename, is a real Null file to `sau check`. That gives a false "defined twice" (39 of 300 above). That is not C-04's case, and few editors name temporary files so.
- Once C-04 is merged, `sau_core`'s `load_settled` can go, but `reload` should keep comparing problems as well as texts. The rename's own reload then publishes the clean project after a load that caught the temporary file.

### 1a-15 again, Cosmósis `da80091`, re-reviewed by agent/null-a, November 7, 2026: approved

I reviewed `da80091` (and the report `29489e0`) on clean copies of Cosmósis `29489e0`, Null `249b29f` (current main) and Scowr's tip.

**Measured.**

- Cosmósis's `just verify` on the copy exits 0. The corpus test's six tests pass, `sau check` exits 0 on all six projects at Null's current main, and the coverage test now walks Null's parsed nodes from the datablock down.
- **The self-tests fail for the right reason.** With `has` changed to accept a step at any depth (as the words did), two self-tests fail, each on exactly the case it guards:
  - `a_camera_line_in_a_scene_is_not_a_camera_datablock` finds no missing construct, because the scene's `camera @cam` counted;
  - `a_material_slot_on_an_object_is_not_a_material_datablock` misses only the node constructs, because the object's `material @clay` counted.

  Restored, all six pass. `a_construct_commented_out_does_not_count` asserts that exactly "object parent" goes missing when `parent` is under `/-` and `//`, so a comment counted would fail it.

**"Done when"** holds: six small projects cover every slice construct, by parsed nodes, and `sau check` passes on them.

**Not blocking.** Nothing tests the word a step requires. With `has` ignoring it, every test still passes, so a corpus with only `data mesh` would still show "object data: camera" and "object data: light". A self-test that drops the `data camera` and `data light` lines would cover it.

### 1a-15 (agent/cosmosis-a's), reviewed by agent/null-a, November 7, 2026: sent back for the coverage test

I reviewed Cosmósis `eb05e6e` on clean copies (`git archive`) of Cosmósis `eb05e6e`, Null `4ee87a9` (after `0798c35`, so with `output-material` and the primitives' `scale`) and Scowr's tip, side by side in my scratchpad.

**Measured.**

- Cosmósis's `just verify` on the copy exits 0.
- `sau check` (release build) exits 0 on each of the six projects in `corpus/slice/`: `inline-pyramid`, `lights-and-cameras`, `one-cube`, `one-file`, `table-and-cups` and `two-materials`. That covers all 31 of their Null files, checked by Null's checker against Null's current schema.
- A deliberately broken copy of `one-cube` fails `sau check` with exit 1 and the right message for each of five breaks:
  - `locaton` gives `null-unknown-property` with `location` suggested;
  - `size=2deg` gives `null-unit`;
  - `@cube-mash` gives `null-handle` with `@cube-mesh` suggested;
  - `primitive cubee` gives `null-kind`;
  - an unclosed `[` gives `null-syntax`.

  A valid change (`size=2m scale=[1 1 2]`) still exits 0.
- **Coverage by parsed nodes.** A probe of my own, run with Null's parser over the corpus and not committed, finds every slice construct as a node, not just as a word:
  - datablocks `scene`, `object`, `mesh`, `material`, `camera` and `light`, and `project`;
  - an object's `location`, `rotation-euler`, `scale`, `parent`, `data mesh`, `data camera` and `data light`, and its `material` slot;
  - a mesh as `primitive` (cube, plane, UV sphere, cylinder) and as `vertices` with `polygons`;
  - a material's `nodes` with `principled-bsdf`'s `base-color` and `roughness`, linked into `output-material`'s `surface`;
  - a camera's `lens` and clip ends, and every light type with its own properties.

  So the corpus does cover the slice.

**To fix: the coverage test still matches text, not nodes.** `every_slice_construct_appears_in_the_corpus` (`crates/sau_cli/tests/corpus.rs`) still splits lines into words, comments removed. So:

- "camera" is satisfied by a scene's `camera @shot-cam` pointer line;
- "material" by an object's `material @paint` slot line;
- "mesh" by any line starting `mesh`.

The test would not notice the camera datablocks, or the materials, going missing; the C-03 review raised the camera case before. Cosmósis now depends on `null_syntax`, so the test can walk parsed nodes instead:

- top-level nodes by name for the datablocks;
- an object's children for its attributes, and `data`'s first argument for its kind;
- a mesh's `primitive`, `vertices` and `polygons`;
- the `node` kinds inside a material's `nodes`, and their inputs.

A self-test should show it failing when a corpus file loses its `camera` datablock while keeping the scene's `camera @…` line.

**Not blocking.** The corpus does not use a primitive's `scale` (1a-06b) or an inline `collection`; neither is required by the row.

### 1a-09 and V-01 again (agent/sau-b `adcb357`, `7aff829`, synced at `27e1660`), re-reviewed by agent/null-a, November 7, 2026: V-01 approved, 1a-09 sent back for one header line

I read the two commits. I ran the bridge's tests in agent/sau-b's developer build: `blender_test --gtest_filter=BridgeTest.*`, built at 17:49, after `adcb357`, with the sandbox's folders set to my scratchpad. I checked Blender's own behaviour with a probe in that worktree's release build, through `sau/tools/sandbox.sh` with my own sandbox folder. I changed nothing in the worktree.

**Measured.**

- **The tests.** The 13 bridge tests pass in 3 runs of 3, under AddressSanitizer, in 64 to 165 ms.
- **Session ids, after delete and after undo.**
  - `sau_bridge_id_from_session_uid` is `BKE_libblock_find_session_uid` over `Main`. The probe searched `Main` by `session_uid` the same way, in a windowed run with real undo steps (`ed.undo_push`, `ed.undo`, `ed.redo`).
  - An object made after the last step and then undone: not found. After redo: found, with the same id.
  - Objects that live through the undo keep their ids.
  - A deleted object: not found. Undoing the delete brings it back with its old id.
  - A new datablock never reuses an old id.
  - After `wm.read_homefile`, no old id is found; the new Cube has a new one.
  - The bridge's own test covers the delete case.
- **The update without a message bus cannot crash in the app.** The fallback runs only when the context's window manager has no message bus. In the app every window manager gets one, foreground or background, before its keymaps (`wm.cc`), so only a test reaches it. There it is `RNA_property_update_main`, the call animation makes for every animated property. It skips context-only updates rather than calling them without a context, and the test shows the object tagged.
- **V-01.** `verify.sh` asks the guard's own `slot_limit()` and clamps it to 1 to 4: 4 here at moderate heat with 5 slots. It prints the figure, and the command now passes it to `ctest -j`.

**To fix (1a-09): the header says `SauMain` lasts until a file loads, but an undo or redo step replaces it too.**

- `setup_app_data` runs for memfile undo too (`mode == LOAD_UNDO`) and calls `BKE_blender_globals_main_replace`, which frees the old `Main`.
- In the probe, `bpy.data.as_pointer()` changed across one undo, from `0x7959f34018` to `0x795d5e7818`.
- So a caller who keeps a `SauMain` across an undo, as the header allows, and then calls `sau_bridge_id_from_session_uid(main, uid)` to find a datablock again after that undo, reads freed memory. That call is the very use the session ids are for.
- The advice to fetch `Main` fresh with `sau_bridge_main()` before each use is right. The stated lifetime should match it: "until Blender loads a file or applies an undo or redo step". A test could push a step, undo it, and check that `sau_bridge_main()` changed and that the session id found through the new `Main` is null.

**Not blocking.**

- With a context that has no scene, the fallback passes a null scene to updates that take one. Blender's own interface path does the same through `RNA_property_update`, and the app always has a scene.
- `verify.sh` reads the slot count once, at the start of a 20-minute run.

### V-01 and 1a-09 (agent/sau-b's), reviewed by agent/null-a, November 7, 2026: V-01 approved, 1a-09 sent back

I reviewed `agent/sau-b` at `71cde43` (synced at `0464aae`): the diff from `main`, and `cb95220` and `05dfdec` in particular. I reran the bridge's tests in its developer build (`blender_test --gtest_filter='BridgeTest.*'`, built 16:32, after `cb95220`), with a sandbox's folders, `TMPDIR` and `HOME` in my scratchpad: 8 of 8 pass under ASan. I did not rerun the 21-minute suite.

**1a-09, the bridge: sent back.** Every function of `SAU_bridge.h` has a test, and the following hold, checked by reading Blender's code where the tests do not reach:

- **Nulls and unknown kinds** are errors in every function (tested).
- **Indices.** An index past the end gives null (tested). An RNA index out of range, such as `location[5]` or `location[-1]`, is refused by Blender's path parser before the bridge reads (`rna_path.cc` checks each index against its dimension). A multi-dimensional array needs every index, flattened by the parser.
- **Types and lengths.** Only `PROP_FLOAT` resolves; strings, booleans and wrong lengths change nothing (tested).
- **Writes.** `float_set` checks `RNA_property_editable` and the length first.
- **Undo.** `undo_push` reports whether a step was pushed (tested on the memfile stack).
- **Tags.** `tag_update` tags `ID_RECALC_ALL` and the relations (tested).
- **Opaque types.** The forward declarations in `namespace blender` match Blender 5.3's `Main`, `ID` and `bContext`, or the tests would not compile. `SauMain`, `SauID` and `SauContext` are never defined, so they cannot be used other than through `wrap` and `unwrap`.

**To fix**, before Sau's apply (1a-10) builds on it:

1. **Lifetimes are not written down.** The header says no function keeps a pointer it is given, but not how long a returned `SauID` or a `SauMain` stays good. A `SauID` dangles after `sau_bridge_id_delete`. Both dangle after a file load, and after an undo or redo step, which replaces changed datablocks and can replace `Main`. A caller cannot test for either, so the header should say it: valid until the datablock is deleted or Blender loads, undoes or redoes, and found again by name after that.
2. **Deleting a scene is unguarded.** `sau_bridge_id_delete` calls `BKE_id_delete` on any scene, including the last one or the one a window shows. Blender's own `bpy.data.scenes.remove` refuses the first (`BKE_scene_can_be_removed`) and replaces the active one first (`ED_scene_replace_active_for_deletion`). Refuse what Blender refuses (return false), and test it.
3. **Two branches untested.**
   - A read-only float property: `float_set` must refuse it, for example `bound_box` or another non-editable float.
   - An out-of-range index such as `location[3]`: the parser refuses it, so a one-line test pins that.
4. **`float_set`'s update runs without a scene or context.** It calls `RNA_property_update_main(bmain, nullptr, …)`, so an update that needs a context (`PROP_CONTEXT_UPDATE`) is skipped without a word, and one that reads the scene would crash. The slice's float properties are safe, because their updates (`rna_Object_internal_update`, `rna_Camera_update`, `rna_Light_draw_update`) only tag the dependency graph. Either pass the scene, or say in the header which updates run.

**Not blocking.** `sau_bridge_id_at` walks the list each call (`BLI_findlink`), so listing n datablocks is O(n²). That is fine for the slice; an iterator would serve the corpus.

**V-01, the developer build: approved.**

- `verify.sh` builds `.build/developer` through the build guard when it is missing or stale, under its lock.
- It runs `ctest` four at a time at utility priority, with a 15-minute timeout, in a sandbox made fresh for the run (every `BLENDER_USER_*` folder created first, `TMPDIR` and `HOME`).
- The exclusion is read from `docs/agents/testing.md`'s table, its one home.
- **The exclusion is honest.** `bl_pyapi_bpy_app.py` expects `bpy.app.cachedir` to be `$HOME/Library/Caches/Blender/`. Blender takes it from `NSCachesDirectory` (`GHOST_SystemPathsCocoa`), which ignores `HOME`, so the two differ only in the sandbox.
- **The hook is minimal, marked and listed.** In `BLI_fileops_test.cc`, `/private` is added only for a temporary folder under `/var` or `/tmp`. The comment is marked `Sau:`, and the file is in `HANDOFF.md`.

**Found while reading it, for the lead (a task of its own, not V-01's).** Sau's caches folder is still Blender's, the real `~/Library/Caches/Blender/`. `BKE_appdir_folder_caches` joins the system caches folder with "Blender". That is the folder the test reads, and the installed Blender's own: it holds the user's `asset-library-indices` from May 13. Sau's asset indexer, remote asset libraries and GPU shader caches write there, sandboxed or not, since `HOME` does not move it. So Sau shares caches with the installed Blender, and 0-07's sandbox does not cover this folder. A `Sau:` hook naming it "Sau", plus a variable the sandbox can set, would close both.

## 1a-02: lexer and lossless syntax tree for the slice, kebab-case names

Done, November 7, 2026 (`0edf8f4`, approved).

**What works.**

- Seven slice files in `compiler/null_syntax/tests/slice/` cover every phase 1a construct: `project`, `scene` and `collection`; `object` with `location`, `rotation-euler`, `scale`, `parent` and `data`; `mesh` as a `primitive` and as inline `vertices` and `faces` lists spanning lines; `material` with a `principled-bsdf`'s `base-color` and `roughness`, links and a `layout` block; `camera` and `light`. A `lexical.noll` adds every lexical form: `;`, `/-` on a node and on a block, `\` continuation, nested block comments, escaped strings, raw strings, bare and `on`/`off` booleans, scientific notation, attributes as properties and as child nodes.
- `null_syntax` now re-exports the lexer (`lex`) and has `dump`, an indented tree with each token's kind, span and text, for snapshots.
- Kebab-case (S16) is enforced: `names::check` reports every node name and property key that is not lower-case kebab-case, code `null-name`, with the kebab-case spelling as a fix (`rotation_euler`, `frameStart`, `Rotation_Mode`, `"base color"`, `@Tex` in a layout). A node may be named by a kebab-case handle, as `layout` entries are. Strings are never names in Null. Handles in arguments, and enum values, are left to the checker. `diagnostics` returns syntax and name errors together, in text order.

**Measured.** `cargo test -p null_syntax`: 11 tests and a doctest pass. For each slice file: the lexer finds no errors and its tokens cover the text; the parse has no diagnostics; the tree's tokens are exactly the lexer's (kind and length); the text prints back byte for byte; the tree is an `insta` snapshot (`tests/snapshots/`, 7 trees, 960 lines). The kebab-case messages are a snapshot too, and applying their fixes gives a file with no diagnostics. Null's `just check` exits 0.

**Dependency** (licence checked): `insta` 1 for tests, without default features (Apache-2.0), with `similar` (Apache-2.0), `once_cell`, `tempfile`, `fastrand`, `rustix`, `errno`, `linux-raw-sys`, `r-efi` and `windows-sys` (each offering MIT or Apache-2.0).

**Choices to review.** `null-name` is a new diagnostic code (spec section 18 lists codes "such as"). `to_kebab_case` splits a capital after a lower-case letter (`frameStart`), not after a digit (`use2D` becomes `use2d`).

## 1a-03: parser with error recovery for the slice

In review again, November 7, 2026 (`5cef529`, `a5262c3`, then the fixes below: `8192b21`, `1d79a59`, `c64bfd8`). Null's `just verify` passed at `4857d08`, its 60 s fuzz run with 8 KB inputs included (20,513 inputs, no crash), at moderate thermal pressure.

**Fixes after the review** (agent/cosmosis-a's findings, in "Review" below):

1. The invariant's check for Scowr's examples now matches the three whole phrases (`messages::SCOWR_EXAMPLES`: `such as vel=96`, `such as track {`, `\u{266A}`), not fragments, so a user's key such as `level=` or `travel=` is no longer a false alarm. `a_users_key_is_never_taken_for_scowrs_example` covers `modifier @sub subsurf level=` and `object @a travel=`; the reviewer's 97-byte crash input and a `level=`/`travel=` file are fuzz seeds.
2. `nesting_past_the_limit_is_an_error_not_a_crash`: 300 levels of `x {`, of `[` and of bare `{`, past Scowr's limit of 256, give "blocks are nested too deeply" and "lists are nested too deeply", lossless, with every invariant holding; so all 22 known messages are now reached by a test.
3. `tools/fuzz.sh` passes `-max_len=8192`, so the fuzzer tries inputs the size of real files. With `-max_len` alone libFuzzer ramps its length limit slowly (a 10-minute run reached 627 bytes), so it also passes `-len_control=0` (`c64bfd8`).

**Rerun after the fixes**, at Null `54b3ed9` (the schema merge included): `tools/fuzz.sh 600`, inputs up to 8,192 bytes from the start, ran 604 s without a crash: 185,581 inputs (307 a second, slower with inputs of up to 8 KB), coverage 3,438 edges, corpus 2,519 inputs (964 KB), peak memory 1,218 MB, no artifact. Thermal pressure was moderate at the start, but all five samples taken during the run read heavy, from other work on the Mac; the fuzzer is one process on one core. A first attempt was stopped by hand at heavy pressure, before the merge.

**What works.**

- Recovery is the shared parser's (Scowr's `scowr_syntax`, unchanged); Null tests it on the slice. Twelve kinds of broken slice text (unclosed block, list and string; a stray `}` and `]`; reserved `(` `)`; a missing value after `=`; a missing key; a property written as a node, `frame-start=1`; a block with no name; text after a block; bad escapes) each give the whole tree, every byte kept, and diagnostics, and the node after the break still parses (`tests/recovery.rs`; the messages are a snapshot).
- Null's own wording (`src/messages.rs`), Scowr untouched: the three shared messages that carried Scowr's examples are reworded (`such as object { ... }`, `such as frame-start=1`, `such as \u{e9}`), and `frame-start=1` on a line of its own gets its own message and a fix (`=` to a space). `KNOWN` lists the 22 messages of the shared syntax as read at Scowr `770a472`; the invariants fail on any other, so a new Scowr message is read before Null shows it.
- `src/invariants.rs`: the checks for any text, shared by the tests and the fuzz target: lossless print, contiguous tokens, every span and fix on character boundaries in the text, one `null-syntax` diagnostic per syntax error, no Scowr example in any message, rendering and dumping without panic, and an incremental edit reparsing to exactly a fresh parse (tree and errors).
- The first fuzz target, `fuzz/fuzz_targets/parse.rs`: libFuzzer's input as text (bad UTF-8 replaced) and an edit chosen by its first three bytes, through the invariants. Its own workspace in `fuzz/`, built by cargo-fuzz 0.13.2 on nightly (rustc 1.101.0-nightly, 2026-11-06), both installed today as `testing.md` says. `tools/fuzz.sh` seeds it from `fuzz/seeds/parse` (the slice files); the growing corpus and any crash files are ignored.

**Measured.**

- The fuzz target ran 601 s (`tools/fuzz.sh 600`) without a crash: 667,044 inputs, coverage 3,396 edges, corpus 2,471 inputs, peak memory 814 MB, no artifact. Thermal pressure was nominal at the start and moderate at the end (load 24.8 on 18 cores from other work). A first 21-second run: 61,006 inputs, no crash.
- 3,000 seeded mutations of the slice files (1 to 4 insertions, deletions or replacements of braces, brackets, quotes, `#"`, `\`, comment marks, `/-`, `;`, `=`, line breaks, CRLF, tab, `é`, U+2028), most of them broken: every invariant holds.
- `null_syntax`: 16 tests and a doctest. Null's `just check` exits 0; `just verify` passes, its 60 s fuzz run included (66,665 inputs, no crash).

**Dependency** of the fuzz crate only (licence checked): `libfuzzer-sys` 0.4, (MIT OR Apache-2.0) AND NCSA (LLVM's libFuzzer, a permissive BSD-style licence), with `arbitrary`, `cc`, `jobserver`, `shlex`, `find-msvc-tools` and `libc` (MIT OR Apache-2.0). The fuzz crate is outside Null's workspace, so `just check` now runs `cargo deny` on `fuzz/Cargo.toml` too, with the same `deny.toml`, which allows NCSA for libfuzzer-sys. Checked both ways: with NCSA removed from the list the fuzz crate is rejected; with it, all 14 of its crates pass.

**Unverified.** Recovery is the shared parser's own: Null tests it and does not change it. Memory grows with the corpus (1,218 MB after 10 minutes with 8 KB inputs, under libFuzzer's 2 GB limit); runs much longer than 10 minutes may need `-rss_limit_mb`.

## 1a-06: checker, names, types and units against the schema

In review, November 7, 2026.

**What works.** A new crate, `compiler/null_check`, checks a project's files together against `schema/null-schema.json` (Sau's 1a-05):

- Handles are collected project-wide first (S7), so references may point forward or into other files.
- Every datablock (`scene`, `collection`, `object`, `mesh`, `material`, `camera`, `light`) and the `project` file are checked: their line (handle and display name only), each attribute against RNA's properties of the struct and its bases, and Null's own forms.
- Each value is checked for its type and shape: floats and fixed-length lists with units, read by `null_value`, and their hard range; integers; booleans (bare, `on`, `off`); enums; strings; pointers to handles of the right kind; read-only properties; attributes given twice.
- Light properties follow the light's `type`.
- `#rrggbb` colours are read by the sRGB rule.
- Mesh data is checked: `vertices`, and `polygons` of three or more indices of existing vertices.
- Node trees are checked: node kinds, inputs, and links to an existing node's output. So is `layout`.

Every error carries a stable code and, wherever one is clear, a fix:

- the nearest schema name;
- Null's name for a common other one (`faces` to `polygons`, `active-material` to `material`);
- the lower-case enum item (`XYZ` to `xyz`);
- the unit in the right case (`MM` to `mm`; the 1a-04 review's finding, now in `null_value`);
- `on` for `yes`;
- the kind for `data @lens` (`camera @lens`);
- the nearest handle.

Handles are kebab-checked where defined and where referenced, and enum values likewise (both from the 1a-02 review).

**`null_value` gained** the camera lens's millimetre (`50mm` is 50, not 0.05), watts, and `read_int` for DNA's integers.

**Decided where the spec and register were silent, recorded in the spec's section 8 ("Null's forms over RNA"), 6 and 18:**

- RNA's `polygons`, not `faces` (S16).
- `units` maps to `UnitSettings`.
- Scene and collection members are lines, and an inline `collection` defines one.
- `data <kind> @h`; `material @h` as slots.
- A primitive's settings are the add operator's, `size` one length.
- Light `energy` is in watts for point, spot and area lights; a sun's is bare.
- Defaults are a new datablock's (the schema's `new`), for the formatter.
- The diagnostic codes are listed.

The spec's own examples now follow the schema: the pier's deck is `primitive cube size=1m` scaled by its object, and `layout` has one node per `;`.

**Measured.** `cargo test -p null_check`: 7 tests and a doctest pass. A three-file slice project with every construct checks clean.

One snapshot holds 29 errors from one broken file. They cover all ten codes: a misspelt name, a wrong type, a wrong unit, an out-of-range value, an unknown enum item, an upper-case one, a missing and a non-kebab handle, a read-only property, a repeated one, an unknown kind, a primitive setting, a polygon index past the vertices, a sun property on a point light, a bad link output and a wrong boolean.

The task's three errors (misspelt `locaton`, `scale 2`, unit `45dgr`) each have their suggestion. Applying every fix leaves only the wrong type, which needs the author. Null's `just check` exits 0.

**Run on Cosmósis's slice corpus** (`Cosmosis/corpus/slice`, read only): 6 projects, 9 errors, all real.

- 4: `primitive cube size=[2m 2m 2m]`. RNA's `size` is one length.
- 2: `faces`, which is RNA's `polygons`.
- 3: `node … output-material`. **The schema has no `output-material` node**, so no material can be connected to its output. That is the schema generator's to add (Sau's `null-schema.py`, 1a-05), not Null's.

The corpus changes are 1a-15's. For the lead: the schema needs `ShaderNodeOutputMaterial`, with its `surface` input.

**Dependencies** (licences checked): `serde` 1 and `serde_json` 1 (MIT OR Apache-2.0), with `serde_core`, `serde_derive`, `proc-macro2`, `quote`, `syn`, `itoa` (MIT OR Apache-2.0), `unicode-ident` (MIT OR Apache-2.0 AND Unicode-3.0) and `memchr` (Unlicense OR MIT). `insta` for tests.

**Unverified.**

- Lengths assume unit scale 1.
- Link types are not checked (a shader output into a vector input passes).
- Only the slice's structs are in the schema, so anything else (`modifier`, `render`, `world`) is an unknown name.

## 1a-07: canonical printer and minimal-edit writer

In review, November 7, 2026.

**What works.** A new crate, `compiler/null_print`.

- **`write`, the write-back** (spec, section 17), on the lossless tree. It takes a datablock (kind and handle, or a handle made from the display name), one attribute and its new `f32` or `f64` values, and returns the smallest text edits.
  - A value that still reads back to the same bits is not touched.
  - A changed value is rewritten in its word's unit when a decimal in that unit reads back exactly, and bare otherwise (S21).
  - A list of another length is replaced whole.
  - A missing attribute becomes one new line in the block, at the last child's indentation, or `; name value` in a one-line block, or `name=value` on a block-less line before its comment.
- **`format`, the formatter's layout pass** (section 15): indentation by block and list depth, single spaces, none inside `[ ]`, `=` as written, no trailing whitespace (a line comment's included), at most one blank line, one final line feed. Every comment and non-whitespace token is kept. Attribute order and default omission need the schema and are left for later.
- **The 1a-04 review's finding is fixed in `null_value`.** `print_in` for `deg`, `in` and `ft` now finds its candidates from the value in that unit computed exactly to 40 digits, not from an `f64` estimate. So a value written in a unit prints back in it. `unit_of` gives a word's unit.

**Measured.**

- **The writer** (`tests/write.rs`). Each run is a random edit of the checker's clean three-file slice project: a random object, one of `location`, `rotation-euler` and `scale`, and three values from any finite `f32` bits, short decimals, or the old value. Each edit is checked for:
  - the values reading back to the bits written;
  - exactly the changed words being edited, and every other token, comments included, the same kind and text;
  - a changed word keeping its unit wherever a decimal in it fits;
  - an added attribute being one insertion with all text before and after kept;
  - the edited project parsing and checking clean with `null_check`.

  At 20,000 cases in release, all pass (7.0 s).
- **The formatter.** Every slice file (10) formats idempotently with the same tokens. Their whitespace, scrambled at random (run lengths, tabs, indentation, trailing spaces, longer blank runs), formats back to the same text: 20,000 cases, all pass (3.4 s).
- **`null_value`:** a new property test, a value read from up to 17 digits in `deg`, `in` or `ft` prints back in that unit, passes 100,000 cases. So does the review's own example, `-18750715569787871e-5ft`.
- `null_print`: 7 tests and a doctest. Null's `just check` exits 0.

**Unverified.**

- The writer handles floats and lists of floats, the slice's transforms; enums, handles and strings are not written yet.
- Three-way merging with an outside change (section 17) is not part of this task.

## 1a-06b: a primitive's proportions from its `scale`

In review, November 7, 2026.

**What works.**

- The checker reads a primitive's `scale` from the merged schema (Sau's 1a-05c: unitless, default `[1 1 1]`, listed for the cube, UV sphere, ico sphere, cylinder and cone), through the same property check as any other setting.
- A `scale` on a primitive whose operator does not take it now says whose it is: "`scale` is not a plane's: Blender's plane operator does not take it; `cube`, `uv-sphere`, `ico-sphere`, `cylinder`, `cone` do". Before, it guessed `size`.
- The spec's section 8 rule says a primitive's proportions come from the operator's `scale`, which shapes the mesh, so the object's modifiers and children are not squashed. It replaces 1a-06's "a box of other proportions is a cube scaled by its object".
- The pier example (section 4) is `primitive cube size=1m scale=[1.2 3 0.08]` with object scale `[1 1 1]`. The CHANGELOG records it.

**Measured.**

- `the_pier_is_proportioned_by_its_primitive_not_its_object` reads the pier from the spec itself. The object's scale reads to exactly `[1 1 1]`, and the bevel's `width 4mm` to exactly 0.004 m. With its `modifier` nodes left out (the slice schema has none yet) and its material defined, the pier checks clean.
- `only_primitives_whose_operator_takes_scale_accept_it` passes: a UV sphere takes `scale`, a plane gets the message above, and `scale=[1m 1m 1m]` gives three unit errors.
- One snapshot line changed, for the same message on a cube's `segments`.
- **The bevel, in Blender** (Sau through the sandbox, with `agent/sau-f`'s build). The deck was built both ways with a 4 mm, 3-segment bevel, and I measured how far each face of the evaluated mesh stops inside the box's edge, in world space:
  - with the operator's `scale` (object scale 1), all three faces show 0.004 m;
  - with the object scaled instead, they show 0.012, 0.0048 and 0.0048 m.

  The script, as run, is reproduced below. It is not in a test suite: Null cannot run Blender, and Sau's gates have no place for it yet.

```python
import bpy
def deck(name, op_scale, ob_scale):
    bpy.ops.mesh.primitive_cube_add(size=1, scale=op_scale)
    ob = bpy.context.active_object
    ob.name, ob.scale = name, ob_scale
    m = ob.modifiers.new("bevel", 'BEVEL')
    m.width, m.segments = 0.004, 3
    me = ob.evaluated_get(bpy.context.evaluated_depsgraph_get()).to_mesh()
    world = [ob.matrix_world @ v.co for v in me.vertices]
    hi = [max(c[i] for c in world) for i in range(3)]
    insets = []
    for axis in range(3):
        a = [k for k in range(3) if k != axis][0]
        face = [c for c in world if abs(c[axis] - hi[axis]) < 1e-6]
        insets.append(round(hi[a] - max(c[a] for c in face), 6))
    print(name, insets)
deck("operator-scale", (1.2, 3, 0.08), (1, 1, 1))   # [0.004, 0.004, 0.004]
deck("object-scale", (1, 1, 1), (1.2, 3, 0.08))     # [0.012, 0.0048, 0.0048]
```

**Unverified.**

- The full pier, with its array and bevel modifiers, cannot check clean until the schema has modifiers.
- Cosmósis's corpus still writes `size=[…]`; that is 1a-15's.

## 1a-14a: write-back's Null side

Sent back once and fixed, November 7, 2026 (`4ee87a9`, then the fixes below).

**Fixes after the review** (agent/cosmosis-a's, in "Review" above):

1. **Every definition counts.** An object defined twice in one file is now `DefinedTwice`, as across two files. That includes a written handle and one made from a display name; a `/-` copy is still ignored. Tested by `an_object_defined_twice_in_one_file_is_refused`.
2. **Values must be finite.** NaN and infinities are refused with the new `WriteBackError::NotFinite`, before anything is read. Tested by `values_that_are_not_finite_are_refused`.
3. **The display unit comes from every form the checker takes.** That is `units` in the project or a scene, a unit (`length=cm`) or an item (`length=centimeters`, `rotation=degrees`), on the line or as `length-unit` and `system-rotation` child lines. Tested by `the_display_unit_comes_from_any_form_of_units`, over five forms including none.
4. **`FileEdit`'s doc says how to apply edits.** Several can come for one file, every range is in the original text, none overlap, and they are applied from the last. Tested by `several_edits_to_one_file_apply_from_the_last`.

Not changed: a project without `units` still gets bare radians for a new rotation, as the spec says. Whether that default should be degrees is the spec owner's call.

**What works.** `null_print::write_transform(files, handle, transform, values)` is the API for Sau's side (1a-14), which calls it from C++ through Sau's Rust core.

- **In:** the project's files as plain `ProjectFile { path, text }`, an object's handle, a `Transform` (`Location`, `RotationEuler`, `Scale`) and three `f32` values.
- **Out:** `FileEdit { path, start, end, replacement }`, byte ranges into the given text. No Null tree type crosses.
- **Lookup:** it finds the one file that defines the object, by its written handle or one made from the display name (S18). An object defined in two files is an error, as is one defined nowhere.
- **Edits:** it hands off to `write` for the smallest edit. A changed value keeps the unit its word uses. A new attribute uses the project's `units length=… rotation=…`, read from the `project` node, or is written bare when there is none.

The 1a-07 review's three findings are fixed, each with a test:

- a new line goes after a comment on the block's opening line, so the comment stays the brace's;
- an insertion ends lines as the file does (CRLF when most of its lines do);
- `format` indents a line that a `\` continues one level deeper.

The spec's sections 15 and 17 say so, and that `format` makes a CRLF file's lines LF.

**Measured.**

- `tests/project.rs` (7 tests) moves every object of Null's slice project (5 objects, 15 edits) in location, rotation and scale. Each edit:
  - changes or adds exactly one line, in one file;
  - keeps every comment;
  - leaves the values reading back, so a second call with the same values returns no edits;
  - leaves the project checking clean with `null_check`.

  The same run on the project with CRLF line endings keeps every line CRLF. Units: `@boat`'s `20cm` becomes `25cm`, and a new location is written in the project's `m`.
- **Run on Cosmósis's slice corpus** (a read-only copy at `eb05e6e`, through a probe not committed): all six projects, 29 objects, 87 edits, plus the same 87 with CRLF line endings. All hold as above.
- `just check` exits 0; `null_print` has 15 tests and a doctest.

**Unverified.**

- Sau's side, 1a-14: the C API, and the undo push that calls this, are Sau's and wait on 1a-10.
- Only transforms are written; other attributes go through `write` directly.

**After the approval.** The review's nit is fixed: `DefinedTwice`'s message now says "that object is defined more than once", which fits one file defining it twice as well as two files. Nothing else changed.

## 1c-01a: every RNA unit in the spec

Null `04aad62`, November 7, 2026.

**What it adds.** Spec section 6 gains a "Units" table listing all sixteen `PropertyUnit` kinds in Blender 5.3 (`PROP_UNIT_NONE` to `PROP_UNIT_FREQUENCY`, `makesrna/RNA_types.hh:221`). For each kind it gives:

- what a bare number means;
- the words Null accepts, each with its exact value;
- how the scene's `scale-length` enters.

**Read from Blender's source.**

- The units, factors and base units come from `blenkernel/intern/unit.cc`'s unit collections.
- Scaling follows `BKE_unit_value_scale`: length, velocity and acceleration ÷ s; area and power ÷ s²; volume and mass ÷ s³; only with a metric or imperial system.
- Blender's interface converts no frame-time units (`button_is_unit`). Null converts seconds through `fps / fps-base`.
- `AREA`, `VOLUME`, `POWER` and `TEMPERATURE` have no RNA property in 5.3. `MASS` is used by rigid bodies, cloth, particles and force fields.

**Chosen.**

- The words are Blender's short names in ASCII, without the decimal steps Blender marks as rarely used.
- Area and volume are a length word plus `2` or `3`.
- `F` converts with an exact 5/9, where Blender's interface uses 0.555555555555.

**Found.** Blender's rotation unit "turn" (`t`) has the scalar `1.0 / (M_PI * 2.0)` (`unit.cc`, `buNaturalRotDef`). A turn is 2π radians, so typing `1t` gives 0.159 rad: it looks like an upstream bug. Null does not offer turns.

**Unverified.** Only the words for length, rotation, camera and power are implemented in `null_value`; the rest are 1c-01b.

## 1b-02a: sub-object handles specified

Null `f0b3faa` and `2e7b1c8`, November 7, 2026.

**What it adds.** Spec section 7 gains a "Sub-object handles" part:

- **Handles.** A handle is made from the name. It is unique within its owner across kinds; a second `bevel` becomes `bevel-2`.
- **Paths.** `/` steps into an owner, `.` plus a name is a socket, and `.` plus a number is a pattern's copy (S61 (6)).
- **The table.** Each sub-object kind is listed with its owner and what keeps its identity across a rename.

**Checked in Blender 5.3's DNA.** Blender's own identities:

- `ModifierData::persistent_uid`; a rename keeps it (`rna_Modifier_name_set` only makes the name unique).
- `bNode::identifier`, kept by rename (`node_unique_id`).
- `bNodeSocket::identifier`.
- `bNodeTreeInterfaceSocket::identifier` and the panel's `identifier`.
- `KeyBlock::uid`.
- `ActionSlot::handle`.

Constraints, bones, bone collections, vertex groups, attributes and UV maps, view layers, action layers and strips, NLA tracks and strips, markers, and Grease Pencil layers have only names. They go through `.sau/ids.noll` (S61 (3)). S61 lists six of these kinds; the table names the rest under the same rule. `CustomDataLayer::uid` serves only shape keys.

**Changed.** The spec's example `@rig/@spine.002` is now `@rig/@spine-002`.

## 1b-01a: the persistent ID's rules (in review)

**Null's side is done.** Null `68d13dc`: section 7's "Persistent IDs" states each case's rule.

**Sau's side is on agent/null-sau `97800cb`, with the report `docs/rounds/agent-null-sau.md` (`3b004d7`).** The lead accepted the derived IDs for old files as S63 (cited in Null `5c5b887`). ARCHITECTURE.md is to gain a table of every case (new, added to `Main`, copy, duplicate, evaluated copies, rename, save, reopen, old files, undo, append, link, override, copy buffer, embedded IDs, projects opened from text), each with its rule and the Blender function it hooks, checked in Sau's tree at `33b69ad`. The draft is ready.

**Chosen beyond S61.**

- Data from files saved before the field gets a derived ID, not a random one: a hash of the type, the name and the library's path. An unchanged old file, or an old library linked in, then gives the same IDs every time it opens, as Blender's own `BKE_modifiers_persistent_uid_init` does for modifiers.
- Evaluated copies carry no ID of their own.

## 1b-02c: the checker resolves sub-object paths

Null `50bb332`. `null_check::paths` holds four pieces:

- **`Path::parse`** reads `@owner/@sub/@sub.socket`, a pattern copy (`.7`) and a package (`pkg:`).
- **`Index::build`** indexes every datablock's sub-object lines (section 7's kinds) and node trees.
- **`Index::resolve`** walks a path:
  - a socket is checked against the schema's node sockets;
  - inside its owner's block a path may leave the owner out;
  - a misspelt owner, sub-object or socket gives the path written right.
- **`check()`** reports a handle that two sub-objects of one owner share, whatever their kinds.

**Measured.** `tests/paths.rs` has 4 tests:

- every form of section 7 resolves: modifier, constraint, node and socket, bone, a bone's constraint, vertex group, shape key, view layer, relative paths;
- 14 wrong paths give their errors and fixes (a snapshot);
- paths parse as the spec writes them;
- a shared handle is an error.

`just check` and `just verify` pass.

**Unverified, by design.** The slice has no property that takes a path yet. Drivers, constraint targets and the like will call `Index::resolve` when their forms land (1d-07). Pattern copies and package paths parse and name their tasks (1d-03, 1d-04).

## 1c-01b: every unit read exactly and printed back

Null `2c66b92`.

**What changed.**

- `null_value`'s units are per-dimension tables of section 6's words. Each word converts by one of four rules:
  - a decimal shift;
  - an exact ratio (inches, pounds, mph, km/h);
  - an offset and ratio (°C, °F);
  - π/180.
- Ratios and offsets round once from exact integers, and `print_in` finds the shortest decimal in any word.
- The checker maps RNA's unit names to the new dimensions.

**Measured.** Three property tests per unit, each of 3,000 cases in release and 256 in `just check`:

- every f32 and f64 printed in the unit reads back bit for bit;
- every read is the exact rational rounded once, checked against its neighbours' midpoints, independently of the reader;
- a short value typed in the unit prints back exactly as typed.

The table's examples also read exactly: `1lb`, `36km/h`, `20C`, `-40F`, and `-273.15C`, which is +0.

**Changed in a snapshot.** A lens now also takes `m`, `cm` and `um`, so the checker's case `lens 35m` became `lens 35deg`.

## 1c-02a: every RNA type's written form

Null `c3d2f8b`. Section 6 gains a "Types" table: booleans, integers, floats, enums, flag sets, strings, pointers, collections, arrays, multi-dimensional arrays and matrices, each with examples.

- Matrices are written row by row (S61 (5)); RNA's flat element for row `i`, column `j` is `j × rows + i`.
- An empty pointer is `none`, and a string too long for its field is an error.

**Found.** Scowr's lexer refuses any `\x` escape ("unknown escape"). So S61's `\xNN` for bytes outside UTF-8 needs a change to Scowr's shared syntax, which needs the user's agreement (S15). Until then 1c-02b cannot read such strings. The spec says so.

## Next

- **1a-12a and 1a-13a** (as agent/sau-g, report `docs/rounds/agent-sau-g.md` on Sau's `agent/sau-g`): the review's fixes are in `ddd7662`, synced at `9da78cd`. `just verify` passed, and both are back in review.
- **L-08**, re-reviewed at `0eee86a`: approved (agent/cos-look4 `064e979`). L-09's approval stands.
- **C-04** (Cosmósis `4c9d68b`): approved, review above. It is noted on 1a-12 that `load_settled` can go once C-04 is merged.
- **1a-09 and V-01** again: V-01 approved; 1a-09 sent back for one header line (`SauMain` is replaced by undo too). Review above.
- **Re-reviews and reviews.**
  - 1d-04a again: approved.
  - 1c-01f again: approved, the review in agent/cos-null `7dc40cf`.
  - 1b-03a: approved, the review in agent/cos-schema `ecbc1e2`. All 14,940 float values of RNA's 2,581 float properties match the running app, and the gate fails on a one-byte change.
- **1c-01a fixed** (Null `461a5c8`). Section 6 says Blender's float32 unit factors differ from Null's exact rule for many typed values, and points to S64 and 1c-01g. The word list is now called Null's choice.
- **1b-01a fixed.** Null `997de6a` (section 7: identity by library and ID, resync, the type in derived IDs) and Sau agent/null-sau `b9eecce` (the resync row, uniqueness per S64, the ways in named).
- **0-10, the check record**, in review on agent/hooks (Sau `77b41a7`, Null `f076992`, Cosmósis `15f9b62`). The report is `docs/rounds/agent-hooks.md` on Sau's branch.
- **1c-01f and 1d-01b** (agent/cos-null `75b7226`, review in its report, `0b5c0e7`): 1d-01b approved. 1c-01f sent back for one sentence: spec section 6 still says the curve is "computed in float64", against S62. The table itself I proved independently.
- **L-06 and L-02** re-reviewed: both approved. **C-05**: approved. **1d-04a**: sent back for two lines of spec. All are in reviews above or in the authors' reports.
- **1c-02a** now writes bytes outside UTF-8 as a list of byte values (S65; Null `90d16cd`).
- **0-10** is in review at Sau `b38fde8d`, after the second fix. `just verify` passed, with all 802 of Blender's tests.
- **1c-01h**: approved, the review in agent/cos-units-null `8912c09`.
- **Next:** 1a-10b, once 1a-10 is merged.
- **1b-02c**, fixed after the second review (Null `32d581a`). Inside its block, a path falls through to a datablock of the same handle only when the owner has no sub-object of that handle. So `@bsdf.rougness` inside `@oak` keeps its fix, and `@bsdf/@x` lists the node's sub-objects. The test fails without the fix. It goes back to review once `just verify` passes.
- **1c-01c**, approved (Null `1cda890`). Its two nits are in Null `32d581a`: Blender's 29.97 preset is `fps 30` with `fps-base 1.001` as a float32, besides `fps 30000` with `fps-base 1001`; an action's keys take no seconds.
- **1c-01d**, fixed after review (Null `e0d1519`). It goes back to review once `just verify` passes.
  - **One reader.** `null_check::units` reads `units` for both the checker and `null_print`, and Sau's apply can use it too. It gives the scale and the display units.
  - **Empty and repeated lines.** An empty `units {}` is the project's units, at a new scene's settings. Only the first `units` line counts, and the first of each setting in it; the others are `null-repeated`.
  - **Outside a scene, with no project `units`.** Values use the scale every scene has. Where the scenes differ, the first value in a scaled unit is one `null-unit` error naming each scene's scale, and write-back writes such values bare.
  - **For Sau.** `null_print` now depends on `null_check`. Sau's `sau/crates/Cargo.lock` therefore needs `null_check` under `null_print` when Sau next takes Null; `--locked` fails until then.
- **0-11**, the integrate lock (Sau agent/lock `21411c4b`). Its verify is running; the report is `docs/rounds/agent-lock.md` on that branch.
- **Spec fix** (Null `7737164`): Blender's turn unit is in `#if 0`, so Blender does not offer it.
- **Phases 1b to 1d split** (Sau `8204b94`, the board edited by hand under its lock, at the lead's request). 76 rows replace the 14 coarse ones: 23 in 1b, 11 in 1c and 42 in 1d. Seven rows note a lead's call. The critical path is 16 tasks: 1a-09, 1a-10, 1a-12, 1a-13, 1a-16, 1a-17, then 1d-01a, 1d-01c, 1d-02a, 1d-02b, 1d-05a, 1d-06d, 1d-06f, 1d-07h, 1d-08b and 1d-08c.
- **L-02** (agent/cos-look6 `815b41b`): sent back, review in agent/cos-look6 `33d0a6d`. Shrinking the wheel's straight-alpha PNG darkens its edges; premultiply around `IMB_scale`.
- **L-06** (agent/cos-look5, `8a8b7e4` synced at `bc1c198`): sent back, review in agent/cos-look5 `46d5d4b`. The static timer pointer outlives the window manager that "Load UI" frees. The timer's life, the GPU cost at 15 Hz and the wheel against `branding/loading` hold.

- **1b-03b and 1b-03e** (as agent/schema, report `docs/rounds/agent-schema.md` on Sau's `agent/schema`):
  - 1b-03b is approved, and its nit is fixed: a unit only on numbers.
  - 1b-03e is fixed after review: a transform matrix's translation takes lengths, and `null_print::print_matrix`.
  - Both go back once the worktree's `just verify` passes.
- **0-11** is approved, second round (Sau `agent/lock` `f246cecd`).
- **Reviews.**
  - 1c-01g sent back, for a sync and a scale guard. The review is in `agent/cos-units`.
  - 1d-04b sent back, for a silent broken link at `.sau/ids.noll`. The review is above.
  - 0-12 approved. The review is in `agent/crosscheck`.
- **Null main.**
  - `a83103d`: `read_in` returns `BadScene` rather than panicking on a scale of 0 or a broken frame rate.
  - `9028a0c`: "defined twice as an object".

Whatever the lead routes next.

### 1a-06b, reviewed by agent/sau-b (November 7, 2026): approved

Read the row, the 1a-06b section above and the diff of `753f837`. `just verify` passed on a clean detached copy of Null at `753f837` with Scowr's main checkout beside it (under Sau's `work/lead/`, removed afterwards): Clippy, the licence gate, 64 tests in the check run and again in the 4,096-case release run, the fuzz target for 60 seconds; 3 minutes 31 seconds.

- The "Done when" holds: `the_pier_is_proportioned_by_its_primitive_not_its_object` reads the pier from the spec itself (object scale exactly 1, bevel 0.004 m, clean without the modifiers the schema lacks) and passes; the CHANGELOG has the line.
- The bevel, re-run from the script above in my own built Sau (`agent/sau-b`, through `just sandbox`, `-b`): `operator-scale [0.004, 0.004, 0.004]`, `object-scale [0.012, 0.0048, 0.0048]`, as reported.
- The checker change is small and goes through the existing property check, so the schema's unitless `scale` gives the unit error for `m`, and a setting another primitive takes names which ones do; the one snapshot line that changed (a cube's `segments` pointing to `uv-sphere`) reads right, since cylinders and cones call theirs `vertices`.
- The spec's section 8 row now lists which operators take `scale` and which ignore it, matching what the generator measured (1a-05c).

No nits.
