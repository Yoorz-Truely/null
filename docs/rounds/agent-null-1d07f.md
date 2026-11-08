# agent/null-1d07f

The report of agent/null-a working as `agent/null-1d07f` in `work/wt/null-1d07f/`. Null is on branch `agent/null-1d07f`, cut from `agent/cos-1d08a` (`d75b3b7`, every v0 Null branch). Sau is on branch `agent/null-1d07f`, cut from `agent/sau-y` (`91718db3`, the printer and 1d-07d/e) for the apply side. November 8, 2026. Lints and tests only.

## 1d-07f, Null's side: drivers in Null's expression syntax

**The syntax** (section 11; S38 and S54). A scripted driver's expression, in quotes, is Blender's simple expressions, the Python subset Blender evaluates without Python (`BLI_expr_pylike_eval`, `expr_pylike_eval.cc`), written as Python writes them, with Null's units on numbers. That subset is:
- numbers, `pi`, `True`, `False`, the driver's variables and `frame`;
- `+ - * /`, comparisons chained as Python's, `and`, `or`, `not`, and `a if c else b`;
- its 26 functions, each with the numbers of arguments Blender's `builtin_ops` takes.

This is a decision of mine. The decisions say "Null's own expression syntax" (S54) but not its grammar. The subset's own syntax keeps every Blender expression valid Null as it stands, so a driver read from a `.blend` prints back as it was. Units are the one thing Null adds, as values have them everywhere else (section 6). Components' expression syntax (section 8) is a different thing: it is computed when a project loads, while a driver is evaluated by Blender every frame.

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

**Not done yet: Sau's side.** Apply, printing and write-back of drivers through `compile` and `reads_as`, and the corpus's drivers round-tripping, are the row's other half. They go on the Sau branch cut here.

**Checks:** `cargo test --workspace` and clippy `-D warnings` are clean, and Null's lint-only `just check` passes.
