# agent/cos-1d03b

agent/cosmosis-a's branch for 1d-03b, in `work/wt/cos-1d03b/`. It is on branch `agent/cos-1d03b` in Null, cut from `agent/null-1d02a` at its review (`a92c2cd`) and fast-forwarded to its round 2 (`b11d1a2`). Sau, Cosmósis and Scowr are links to their main checkouts. November 8, 2026. Null's tests and lints only.

## 1d-03b: components parsed and checked

Section 8's components, parameters, placements, patterns, conditions, overrides and recursion that counts down (S32 to S35, S38, S61 (6), S74), read and checked as written. Expanding them is 1d-03c.

**What changed.**
- **`check/expr.rs` (new): expressions.** Inside a word, `+ * /` join numbers with units and names (`n*25cm`, `length/1m`). A quoted expression may also subtract, group and compare (`"depth - 1"`, `"(n - 1) * 25cm"`, `"state == liquid"`), as S74 (3) has it.
  - **Units as powers.** An expression's kind is its units, as powers of length, angle, mass and time, plus the units Null keeps apart (a lens's millimetre, power, temperature, wavelength, colour temperature), and whether it is a whole number.
  - **What follows.** A length times a length is an area, a length over a length a number, and only values of one kind add. The longest unit wins where a unit holds `/` (`9.81m/s2`), and division otherwise (`25cm/2`).
  - **What is refused, each with a reason:** `-` in a word (it is a name), spaces and `( )` outside quotes, and an unknown unit.
  - **Not computed here.** Computing the value, exactly and rounded once (S59), is 1d-03c's.
- **`check/components.rs` (new): components.** Collected before checking, as other files name them:
  - **Components:** each component's parameters, its own datablocks (local handles, the first `object` its root), its root's sub-objects, and the placements in its block.
  - **Placements** in scenes and collections, including collections defined inside them.
  - **Which components place each other,** directly or through others.
- **Checked:**
  - **A component.** Its handle and its root. Its parameters, typed by their default (a length, an angle, a number, a whole number, `on`/`off`, a handle of a kind, a word of a `one-of` list), or by a type's name when they have none: `length`, `area`, `volume`, `mass`, `angle`, `time`, `number`, `whole-number`, `on-off`, a kind of datablock, or `word` with `one-of`. A handle's default must name a datablock, and a parameter is given once.
    - Its datablocks are checked as any datablock is, with its parameters in scope: a value may be an expression of them, checked for its units against the property's (values, lists, primitives' settings, `on`/`off` and handle parameters where those go). Its own handles are local, found inside it and not in the project.
    - A line its block cannot hold is said, with the nearest construct as the fix.
  - **A placement** (`use @component @handle`), in a scene, a collection or a component.
    - The component exists (nearest as the fix); a package's (`@pkg:name`, S54) waits for 1d-04.
    - The copy has a kebab-case handle.
    - A pattern is `count=` (a whole number) or `each=` with `as=`, never both.
    - The block gives parameters by name (with `n`, and the item, in scope), else the root object's own properties.
    - An unknown name gets the nearest parameter or property as its fix, and every parameter with no default must be given.
  - **Conditions.** `if` takes one quoted comparison of values of one kind: numbers of the same units, or words and `on`/`off` by `==` and `!=`, a word compared with a word parameter being one of its words. `else` follows an `if` directly, in a component's block or in a datablock's.
  - **Paths through copies.** `@plank.7`, `@plank.7/@plank-mesh`, `@plank.7/@fluid` (a sub-object of the root), and copies inside a copy (`@tree/@left/@left`, S74 (4)). A pattern's copy is named by its number within its count, and a single copy by none. An unknown step gets the nearest name. They resolve wherever a handle goes (`parent @plank.3`), for the kind they reach.
  - **Overrides** (S33). One names a copy, and its block is a placement's without required parameters; or it names a datablock a copy made, and its block is that kind's.
  - **Recursion** (S38, S74 (5)). Where two components place each other, each needs a `{ countdown }` parameter, a whole number. The placement passes the target's `"<mine> - k"` (k a positive whole number), under an `if` that holds only while `<mine>` is above 0 (`> c` or `>= c`).
- **`null_syntax::v0`: the link split, from my 1d-02a note.** A link is read from the path's last step, so `@plank.7/@tex.color` is a link and `@plank.7` is not. Tested.
- **The spec.**
  - Section 8's three examples are no longer fenced: they check clean as part of the whole spec, so the whole-spec test covers them. The pier's file gains `material @water "Water"`, the project's own material that the component names, with a comment saying why.
  - Section 8 lists the types a parameter with no default names, and section 7 says a copy resolves through its placement. The changelog says so. The v0 snapshot of that example gains its one line.

**Tests** (`tests/components.rs`, seven snapshots and two clean projects).
- **Clean projects:**
  - the spec's pier and tree with their materials;
  - every construct where it is right: each parameter kind used where its kind goes, a pattern over a list, a placement in a component that is not recursion, conditions in a component's block and in a datablock's, an override of a copy's own mesh, an override inside a copy of a copy, and a reference to a copy.
- **Snapshots** (every error, most with a fix):
  - **component:** no root; a parameter twice; a misspelt type; a default off its list; a countdown that is no whole number; an unknown default or type; a handle default naming nothing; a line no construct.
  - **placement:** an unknown component; a missing handle; a word off a list; an unknown parameter; a wrong kind; `count=2.5`; `each=` without `as=` and the reverse; both; an unknown `key=`.
  - **required:** a parameter with no default not given.
  - **expressions:** an area where a length goes; `n-1` (a name, with the quoted subtraction as its fix); a length plus a number; a misspelt name.
  - **conditions:** a word off the list; a word compared with a length; a misspelt name; `else` alone.
  - **overrides:** a copy past the count; a pattern with no number; a misspelt step; a number on a single copy; a misspelt step inside a copy of a copy; a copy where a mesh goes.
  - **recursion:** no countdown; a value that does not get smaller; a placement outside the `if` that ends it.
- **Unit tests in `expr.rs`:** units through arithmetic, the syntax's limits, and what is plain.
- **The whole-spec test** checks section 8's examples clean, and `check.rs`'s errors snapshot gains `component` in the list of kinds.

**Choices, for the reviewer.**
- **A bare number is in its dimension's internal unit,** as everywhere a value goes (section 6). So a length parameter takes `2` as two metres. Inside an expression a bare number is a pure number: `length*0.7`.
- **No expansion here.** Sau's apply reads a project with components only once 1d-03c expands them. The 1,000,000-datablock guard is the expansion's too.

**Checks.** `cargo test --workspace`, Clippy with `-D warnings`, `cargo fmt --check`, and Null's lint-only `just check`.
