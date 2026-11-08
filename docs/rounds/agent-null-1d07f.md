# agent/null-1d07f

The report of agent/null-a working as `agent/null-1d07f` in `work/wt/null-1d07f/`. Null is on branch `agent/null-1d07f`, cut from `agent/cos-1d08a` (`d75b3b7`, every v0 Null branch). Sau is on branch `agent/null-1d07f`, cut from `agent/sau-y` (`91718db3`, the printer and 1d-07d/e) for the apply side. November 8, 2026. Lints and tests only.

## 1d-07f, Null's side: drivers in Null's expression syntax

**The syntax** (section 11; S38 and S54). A scripted driver's expression, in quotes, is Blender's simple expressions, the Python subset Blender evaluates without Python (`BLI_expr_pylike_eval`, `expr_pylike_eval.cc`), written as Python writes them, with Null's units on numbers. That subset is:
- numbers, `pi`, `True`, `False`, the driver's variables and `frame`;
- `+ - * /`, comparisons chained as Python's, `and`, `or`, `not`, and `a if c else b`;
- its 26 functions, each with the numbers of arguments Blender's `builtin_ops` takes.

This was my decision, which the lead settled as S78. The earlier decisions say "Null's own expression syntax" (S54) but not its grammar. The subset's own syntax keeps every Blender expression valid Null as it stands, so a driver read from a `.blend` prints back as it was. Units are the one thing Null adds, as values have them everywhere else (section 6). Components' expression syntax (section 8) is a different thing: it is computed when a project loads, while a driver is evaluated by Blender every frame.

**`null_check::driver`** (new, public, for Sau's apply):
- **`compile(text, vars, scene)`** gives a `Compiled`: the expression Blender is given, and `python: Some(why)` where the text is Python.
  - **In the subset**, each number with a unit is written as its internal value, read as a value is through the scene's unit scale. `spin * 100W` gives `spin * 100`, and `spin + 90deg` gives `spin + 1.5707963267948966`. With a scale of 0.01, `1m` is `100.00000223517424`, since Blender stores the scale as a float. Every other character is kept as written, so an expression with no unit is Blender's exactly.
  - **Python** (`.`, `[`, `**`, `%`, `self`, `bpy`, a function outside the subset) is kept as written, with why.
  - **Errors, at their byte range:**
    - a variable or function near a known one (the fix is the known one);
    - a unit that is none;
    - a function given the wrong number of arguments;
    - a unit in a Python expression, since Python cannot read one.
- **`reads_as(text, blender, vars, scene)`**: write-back leaves the text while Blender's expression is what it compiles to.
- **`FUNCTIONS` and `CONSTANTS`**: the subset's names.

**The checker.** A `driver="…"` string is compiled with the driver's `var` names, in the checker's scene and unit scale. An error is reported at its exact place inside the string, or at the whole string where escapes move the offsets. Python gets a note under the new code **`null-python`**. That makes twelve codes, so `codes::ALL` and section 18's list gain it, and the diagnostics test has its case.

**Spec.** Section 11 defines the syntax, the compiling, write-back's rule and the Python note. Its driver example now reads `driver="spin * 100W"`, and its v0 snapshot follows.

**Tests:**
- **Unit tests in `driver.rs`:**
  - the subset as written (functions of every arity, chained comparisons, the ternary, `.5` and `1e3`);
  - units in the internal unit and through the unit scale;
  - Python kept with why;
  - each mistake at its byte range with its fix;
  - `reads_as`.
- **`diagnostics.rs`:** `null-python`'s case, and four `null-type` driver cases (a variable misspelt, with the fix's result shown; a unit that is none; a unit in Python; `atan2` with one argument).
- **`spec_examples.rs`:** a misspelt variable and a `self` expression, each edited into section 11's driver.

**Sau's side is row 1d-07j,** the Sau worker's. It covers apply, printing and write-back of drivers through `compile` and `reads_as`, and the corpus's drivers round-tripping. The Sau branch cut here (`work/wt/null-1d07f/Sau`, `agent/null-1d07f`, from `agent/sau-y` `91718db3`) is left for them, with no commits of mine.

**Checks:** `cargo test --workspace` and clippy `-D warnings` are clean, and Null's lint-only `just check` passes.

## Open threads, for the next Null session

This is agent/null-a's last report in this session, on November 8, 2026.
- **In review, by agent/cosmosis-a:**
  - 1d-02d (Null `7574796`, main merged in `8fef77b`, on `agent/null-1d02d`): the `format` fuzz target;
  - 1d-03j (Null `a1f3eb2` on `agent/null-1d03j`, which holds 1d-02d and 1d-02e and merges `agent/cos-1d03b`): form 8 in full, short forms inside components, `full_form`, form 4 by the owner's struct, and a node group's interface first;
  - this row, 1d-07f (Null `ce677bc`, and the S78 citation after it).

  `agent/cos-1d08a` already merges 1d-03j (`d75b3b7`).
- **1d-08a needs its re-review once agent/cosmosis-a sends it.** My round 1 (Cosmósis `67ff833`) found these missing from the corpus:
  - section 5's syntax (a keyword attribute such as `to @target`, a raw string, a nested block comment, `/-`, a `\` continuation), which needs a token-level kind of construct in `v0_corpus.rs` and a matching removal in its load-bearing test;
  - four of the five driver variable types (only `transforms` is there);
  - `units` in full form;
  - `inf`, `-inf` and `nan` as values.

  The corpus also needs `sau fmt` again once 1d-03j lands, since a group's interface now prints before its `nodes`.
- **For Scowr's owner:** Scowr's lexer disagrees with itself about whitespace. `continuation_ahead` (`scowr_syntax/src/lexer.rs`) treats only ASCII space, tab and CR before a line's end as making `\` a continuation, while `is_space` takes any Unicode whitespace. So `\` before a vertical tab is a word, which taking the tab out would turn into a continuation. Null works around it in `null_print::format`, which keeps the whitespace after a lone `\` word, and in `full_form`, which leaves such a file as written (1d-02d). The lexer itself was not touched.
- **When 0-26's single test binary reaches main:** `agent/null-1d02d`, `agent/null-1d03j` and `agent/null-1d07f` need its modules and `main__` snapshot names on their next merge of main, as the lead did in batch 3a. 1d-07f adds no new test file. It adds cases to `diagnostics.rs` and `spec_examples.rs`, and a new snapshot, `diagnostics__null-python.snap`.
- **Reviews done this session, all on the board:**
  - approved: 1d-03c, 1d-03h, 1d-03i, C-10, 1d-05b, 1d-07d and 1d-07e;
  - sent back and awaiting their next round: 1d-08a.

  The 1d-05b round 3 review text is still in my scratch folder (`null-a-reviews/review-1d05b-round3.md`), because agent/sau-y's report had uncommitted edits when I approved it. I asked agent/sau-y to paste it in.

## Review

### 1d-07f, `ce677bc` and `6e6758e`, by agent/cosmosis-a, November 8, 2026: sent back for one fix; four notes

I read `driver.rs`, the checker's `driver_expression`, the tests and section 11, against Blender's `source/blender/blenlib/intern/expr_pylike_eval.cc` and `fcurve_driver.cc` in Sau's tree.

**Against Blender.**
- **The functions and arities match `builtin_ops` exactly.** It has 24 names: `radians`, `degrees`, `abs`, `fabs`, `floor`, `ceil`, `trunc`, `round`, `int`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan` and `exp` take 1, `atan2`, `pow` and `fmod` 2, `lerp` and `smoothstep` 3, `log` 1 or 2, `clamp` 1 or 3. Blender then special-cases `min` and `max`, with any count over 0. That makes 26.
- **The constants** are `builtin_consts`' `pi`, `True` and `False`.
- **`frame` is `names[VAR_INDEX_FRAME]`**, and the variables follow it. Blender searches the parameters last first, so a variable named `frame` wins over the frame, and accepting both is right.
- **The grammar follows Blender's parser:**
  - `a if c else b` as `parse_or` then `parse_expr`;
  - `or`, `and` and `not` nested as Blender nests them;
  - comparisons chained (`parse_cmp_chain`);
  - `+ -` and `* /` with unary `+ -` and parentheses;
  - a call's arguments as `parse_function_args` reads them, so `f()` and `f(a,)` are not simple.
- **Numbers as Blender lexes them:** `.5`, an exponent that needs a digit, and no C-style octal. Python's `**`, `//` and `%`, a `.`, `[` and quotes fall to Python, as they must.

**Units through the scene: one fix.**
- **What is right.** `compile` reads each number in a unit with `read_in` at the unit's own dimension, so a length is divided by `scale-length` and an area by its square, as a value is. The scale is float32, as Blender stores it, which gives the report's `100.00000223517424` for `1m` at 0.01. `print_bare` writes the shortest decimal that reads back, which Blender's `strtod` reads exactly. Inside a scene the checker passes the scene's scale.
- **The fix.** A driver's datablock is nearly always outside any scene: a light, an object. There `driver_expression` takes `self.scale`, which is `Outside::scale()`, and for `Outside::Disagree` that is `1.0`. So in a project whose scenes have different unit scales and no project `units`, `driver="gap * 1m"` compiles silently at scale 1. A value written `1m` outside every scene is an error there, through `Out::disagreement` (section 6, "Scene scale"). A driver should get the same: when a number's unit's dimension takes the scale (`scale_power() > 0`) and the scale outside is in disagreement, report `self.disagreement`'s message at the number, as a value's is. A test with two scenes at different scales would pin it.

**Notes (none blocks):**
1. **Whitespace.** Blender skips any `isspace` between tokens (`parse_next_token`): space, tab, line feed, CR, vertical tab, form feed. `tokens` skips only space and tab, so an expression written over lines, which a `.blend` can hold, is taken for Python. It should skip the same six.
2. **All-zero numbers.** Blender's octal rule refuses `01` but takes `00` (every digit after the first a 0), as Python 3 does. `tokens` calls any whole number of two or more digits starting with 0 Python. `00` should pass.
3. **Time units.** `2s` reads as seconds, the dimension of its unit, but a driver on a frame property (`frame-start` of a strip) gives Blender frames. The value would be off by the frame rate. Either refuse a time unit in a driver until the checker knows the driven property's dimension, or read it through the driven property's dimension and the scene's `fps`, as section 6 does for values. At the least, the spec should say which.
4. **A name near a variable is an error, not Python.** That is right for misspellings, but a real Python global near a variable's name (`noise` beside a variable `nose`) is then refused rather than noted. Rare, and the message says what to do. Worth one sentence in section 11.

I also ran 1d-08a's corpus drivers through this branch by reading: `var to transforms` gives `to` as the variable's name (the keyword attribute's two arguments), so `driver=#"to * 100"#` compiles as written. The raw string is then reported at the whole string, as `driver_expression` says it should be.

## 1d-07f, round 2, by agent/cosmosis-a (taking over from agent/null-a, on the lead's order)

agent/null-a had stopped, so the lead gave me this row's fix. The Sau worker reviews it, since I cannot approve my own commit.

**The fix (my review, `0277cae`).** A driver's number is now read as a value is in its place. Outside every scene, where the scenes' unit scales differ and the project sets no `units`, a number in a unit the scale divides (`scale_power() > 0`) is the error a value there is. It uses `Out::disagreement`'s message, at the number, under `null-unit`, once, as a value's is. Before, `Outside::scale()` gave such a driver a scale of 1 silently.
- **`driver::Context`** carries the place to `compile_in`: the scene, whether the driver is inside one, whether its property counts frames, and the disagreement's message. `compile(text, vars, scene)` stays as it was, with the context of a driver inside that scene.
- **`ExprError::unit`** marks a unit's mistake, so the checker reports it as `null-unit`, as a value's is: a unit that is none, a unit in Python, seconds with no frame rate and the scale. `diagnostics.rs`'s two unit cases move from `null-type` to `null-unit`, and both snapshots follow.

**The notes, done as the lead asked:**
- **Seconds on a property that counts frames** (a float frame, `unit: time`: a curve's `eval-time`, a scene's `frame-float`) are read as frames, through the scene's frame rate, as a value's (1c-01h). Outside every scene there is no frame rate, so they are refused there, as a value's are.
  - **What I did not do:** the lead allowed "refused if there's no single scene". I went with the value's rule (inside a scene, else refused) rather than taking the project's one scene for a driver outside it. A value outside every scene takes no seconds even when the project has one scene, and a driver should follow the same rule.
- **Whitespace** between tokens is any of Blender's six `isspace` characters, so an expression over lines stays simple.
- **`00` is a number**, as in Blender and Python 3. `01` is still Python.
- **Section 11** says how a number follows its place, the whitespace, the zeros, and that a name near a variable, function or constant is taken for a misspelling. So a Python global near one, such as `noise` beside `nose`, is refused, not noted.

**Tests:**
- **`tests/scale.rs`:**
  - Two scenes at 1 and 0.01 with no project `units`: two lights' drivers in `m` give one `null-unit` error, at the first `1m`. In degrees they check clean, and with agreeing scenes too.
  - A curve's `eval-time` driven by `spin * 2s` outside every scene is refused, and checks clean in frames. A scene's `frame-float` driven by `spin * 2s` inside it checks clean.
- **`driver.rs`'s unit tests:**
  - `2s` is `50` at 25 fps on a frame property, and `2` on another.
  - The disagreement is at its number, and an angle is untouched by it.
  - An expression over line feed, CR, vertical tab and form feed stays simple.
  - `00` passes, and `01` is Python.

**Checks:** `just check` passes.


### 1d-07f round 2, `f7a5409`, by agent/sau-y, November 8, 2026: sent back for one fix; the round's own changes approved

I read `driver.rs`, the checker's `driver_expression`, `tests/scale.rs` and section 11's change. I also ran `compile_in` on Python drivers from a probe crate (in my scratchpad) built against this branch.

**Approved as they stand.**
- **The disagreement.** It is reported at the number, under `null-unit`, once, as a value's is. An angle is untouched by it.
- **Seconds on a property that counts frames.** They are read as frames through the scene's frame rate, and refused outside every scene. This is the value's rule (1c-01h). Following it rather than taking the project's one scene is right: a driver and a value in the same place read the same way.
- **Whitespace** is Blender's six `isspace` characters.
- **`00` is a number and `01` is Python**, as Blender's octal check reads `tokenbuf`. `01e5` and `00.5` pass, since Blender sets `is_float` for them.
- **`ExprError::unit`**, and the two diagnostics moving to `null-unit`.
- **The API for 1d-07j (Sau's apply, printer and write-back).**
  - `Context { scene, in_scene, frames, disagreement }` and `compile_in` / `reads_as_in` are what apply and write-back need.
  - `frames` comes from the driven property through the public `unit_dimension(p) == Some(Frames)`.
  - `disagreement` is `None` in Sau, since Sau applies only a project that checks.
  - `Compiled { expression, python }` is enough for the printer to tell a simple expression from Python.

**The fix: a Python driver's own numbers and strings are refused as units.** The tokenizer reads a letter right after a number as a unit everywhere, inside quotes too. It does so before it knows the expression is Python. So drivers Blender runs through Python are errors in Null:

| Driver (from a `.blend`) | `compile_in` gives |
|---|---|
| `a + 0x1F` | error: `x` is not a unit |
| `a + 0b11`, `a * 0o7` | error: `b`, `o` is not a unit |
| `a * 1_000` | error: `_` is not a unit |
| `a * 1j` | error: `j` is not a unit |
| `a & 0x1F` | error at `1F`: a unit in Python |
| `obj['2m']`, as in `bpy.data.objects['3D Text'].location.x` | error: a unit in Python, inside a string |

The last row is the one that matters most. A Python driver naming an object or a custom property whose name holds a digit before a letter (`'3D Text'`, `["2x"]`) cannot be written in Null at all. 1d-07j's printer and `sau convert` would then turn a working `.blend` driver into a project that does not check.

Two parts:
1. **Text in quotes is a string's.** A `'` or `"` makes the expression Python, as it does now, and nothing between it and its closing quote is tokenized. That covers Python's escapes and `'''`/`"""`, and `r''`, `b''`, `f''` prefixes run into the quote.
2. **Python's own number forms make the expression Python, not a unit error.** These are a `0x`, `0b` or `0o` prefix (either case), a `_` between digits, and a `j`/`J` suffix. A suffix that is none of these and no unit stays the error it is, so `2mtr` is still caught as a misspelt unit.

A test with each row of the table would pin it. A Python driver whose text holds `2m` outside quotes (`a & 2m`) should keep its error, as now.

**Notes (none blocks):**
1. **`in_scene` for Sau.** Sau must pass the same `in_scene` and `scene` the checker used for the driver's datablock, or `reads_as_in` can disagree with the check. The checker's place (a scene's datablock, else outside every scene with `Outside::scale()`) is private. A public way to ask it, such as the `Context` the checker would build for a datablock's driver, would keep the two from drifting. Until then, 1d-07j builds it the way write-back reads values: the datablock's own scene, or the agreed scale outside.
2. **`a & 0x1F`'s message** points at `1F`, not at `0x1F`. The fix above removes it, but the span shows the tokenizer splits a number from its suffix at the letter.

## 1d-07f, round 3, by agent/cosmosis-a: a Python driver's own numbers and strings

The fix from agent/sau-y's review (`c4207a6`), and its note:
- **Text in quotes is the string's.** Units are now looked for in a Python expression with every string literal skipped: `'…'`, `"…"`, a triple quote, and `\` escapes inside them. A prefix such as `r` or `f` runs into the quote. So `bpy.data.objects['3D Text'].location.x` and `obj["2m"]` are Python, kept as written with a `null-python` note, as a `.blend`'s driver is.
- **Python's number forms are Python's.** `python_number` reads a `0x`, `0b` or `0o` prefix (either case), a `_` between digits and a `j` suffix. In the tokenizer such a number makes the expression Python, where before it was "not a unit". In the scan of a Python expression it is skipped, so `a & 0x1F` is no unit's error either. A suffix that is no unit and none of these forms stays the error it was (`2mtr`), and so does a unit outside a Python expression's strings (`a & 2m`).
- **The note: `driver::place`.** It is public and gives the `Place` the checker reads a datablock's drivers in. Inside a scene node, that is the scene's `units` over the project's and its frame rate. Outside every scene, it is the scale the scenes agree on, or the disagreement's message. `Place::context(frames)` gives the driver's `Context`. It reuses the checker's own `frame_rate` and `disagreement` (now `pub(crate)`), so the two cannot drift. Sau's 1d-07j takes it for apply and write-back.
- **Section 11** says that a Python expression's strings and Python's number forms are its own.

**Tests:**
- **`driver.rs`:** each row of the review's table (`0x1F`, `0b11`, `0o7`, `0XFF`, `1_000`, `1j`, `a & 0x1F`, `'3D Text'`, `"2m"`), a triple-quoted string and an escaped quote. All are Python, kept as written. `spin & 2m` stays a unit error, and `2mtr` a "not a unit".
- **`tests/scale.rs`:** `place` gives, outside two disagreeing scenes, the checker's own message word for word, and the checker reports it on the same project. Inside a scene it gives that scene's scale and frame rate, so `2s` at 25 fps is `50` on a frame property.

**Checks:** `just check` passes.

