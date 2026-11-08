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

## 1d-03c: expansion

Stacked on 1d-03b (`529e976`) on the same branch. `null_check::expand` turns every placement into the datablocks it makes, as Null text. `check_expanded` checks that text. Null's tests and lints only.

**What changed.**
- **`expand.rs` (new).** `expand(schema, files)` gives each file's expanded text and the expansion's own problems.
  - **Components are taken out.** A scene or collection that places copies is written with one member line per copy's root (`object @plank.7`), including copies made inside copies (`object @tree/@left`).
  - **The copies' datablocks follow that scene or collection,** in placement order, each copy's datablocks in its component's order: `object @plank.7 "Plank"`, `mesh @plank.7/@plank-mesh "Plank"`. All other text, comments included, is as written.
  - **Handles** are as in 1d-03b. A pattern's copies are `@h.1` to `@h.N`. A copy inside a copy is a path from it, and a copy's other datablocks are paths from its root. Inside the copy, the component's local handles are rewritten to these, so `data mesh @plank-mesh` becomes `data mesh @plank.7/@plank-mesh` and `parent @branch` becomes `parent @tree/@left`.
  - **Parameters** come from the defaults, then the placement, then an `override` naming the copy. The placement's lines and the override's are read in their own scope: the placing copy's names, plus `n` and the item for a pattern. The lines that are not parameters are laid over the root.
  - **Overrides of other targets.** An override of another datablock (`@gate.1/@post-mesh`) or of a sub-object (`@plank.7/@fluid`) is laid over that. A line laid over replaces the line of its name; for `modifier`, `constraint` and `bone`, it replaces the one with its name and handle, and otherwise it is added.
  - **Conditions** are decided per copy: an `if`'s lines when it holds, else its `else`'s.
  - **Exact values.** Values are computed as i128 fractions in the internal unit, using `Unit::exact_factor` (new in `null_value`: `cm` is 1/100, `in` 254/10000). They are printed exactly as decimals, or to 36 significant digits when no decimal holds the value, so the single rounding (S59) is left to the checker and Sau.
    - Degrees stay degrees (`2*35deg` is `70deg`). Degrees and radians in one expression are an error, since π/180 is no fraction.
    - A division by zero is an error, which takes up note 3 of the 1d-03b review.
    - A quoted string is computed only when it is an operator expression of the names in scope. A display name, or `lightgroup "a - b"`, stays a string (note 4).
  - **Guards** (S38):
    - **More than 1,000,000 datablocks** is an error at the `use` line. A pattern is checked against the limit before it is unrolled, as count times its component's own datablocks, so `count=500001` fails at once rather than after half a million copies.
    - **Recursion that does not count down** is an error at the placement: the countdown must be smaller than the last time this component was placed, and a component with no countdown that places itself fails. This runs whatever the checker said, so the expansion always ends.
    - **More than 256 copies nested inside each other** is an error (new, see below).
- **`check.rs`.** `check_flat` (crate-private, behind `check_expanded`) runs the checker with copy handles counted as handles: steps of kebab-case handles, each with or without `.n`, joined by `/`, defined and named like any other. The copy-path resolution through placements is skipped there, since none remain. `check` itself is unchanged.
- **Tests.** In `tests/expand.rs`, with the spec's examples moved to `tests/examples/mod.rs` and shared with `tests/components.rs`:
  - **The pier.** It expands to twenty members `@plank.1` to `@plank.20`, twenty objects and twenty meshes, and nothing else. Plank 1 is at `0.25m` with weathered oak. Plank 7 is liquid, with water and the fluid modifier, at `1.75m`. Plank 7's mesh is `scale=[2.4 0.2 0.04]`.
  - **The expanded pier checks clean.**
  - **Expanding twice gives the same tree.** Expanding the expanded text again gives it back unchanged.
  - **The tree at depth 3** gives 15 objects and 15 meshes. Copy `@tree/@left/@right` has parent `@tree/@left`, location `0.7m`, and a segment of radius `0.0245m` and depth `0.49m`. The tree checks clean.
  - **A recursion that does not count down** (`depth depth`), and a component without a countdown that places itself, are errors, and the expansion ends.
  - **`count=500001`** of the two-datablock plank hits the guard at `pier.noll:2`.
  - **A pattern over a list.** It is combined with a count taken from a parameter and a quoted expression in it, a pattern inside a copy (`@north/@row.2`), an override through that path, an override of a copy's mesh, and a word condition. All of them expand as written and check clean.
  - **A mistake in expanded text is found once per copy that makes it,** so the expanded check is not vacuous.
  - **Unit tests** cover decimals, exact literals and arithmetic.

**A decision, settled as S76.** I added a nesting limit of 256 copies inside copies, and the lead kept it as Sau's decision S76, which section 8 now cites. Every chain ends because of S38's countdown, but one starting at a million recurses a million frames deep and would overflow the stack before reaching the 1,000,000-datablock guard. Section 8 says so. 
**Not done here.**
- Sau applying the expanded copies, with instancing, is 1d-03e.
- The expansion's problems carry spans in the written files. The expanded check's spans are in the expanded text, which has no map back to the source yet; that is needed once Sau reports problems through the expansion.

**Checks.** `just check` passes. The Null tests pass, and clippy is clean.

## Review

### 1d-03b, `529e976`, by agent/null-a, November 8, 2026: approved; four notes

I read `check/expr.rs`, `check/components.rs`, the v0 change, the spec's section 8 and the tests. I also ran twelve expressions through the checker, in a copy of this tree in my scratch folder.

**The expression rules are S74 (3)'s:**
- `+ * /` inside a word (`n*25cm`, `length/1m`, `3/2*length`), and subtraction, grouping and comparison only in quotes (`"depth - 1"`, `"length + 1m"`);
- a `-` inside a word is a name, with the quoted form as the message's advice;
- `( )` outside quotes is refused, because Scowr reserves it;
- a space outside quotes is refused.

**Units as powers is the right model:**
- `length*length` where a length goes is "an area";
- `length + 1` is refused, since a bare number in an expression is a pure number;
- an angle-typed property refuses a length parameter, and a whole-number property refuses a length.

`9.81m/s2` stays a unit, and `25cm/2` is a division.

**Both choices are right.**
- A bare number is a parameter's value in its internal unit, as section 6 has everywhere a value goes. Inside an expression it is a pure number, so `length*0.7` is a length. The two rules do not collide, because one is a value and the other an operand.
- Expansion waits for 1d-03c, so apply sees no components yet, and the 1,000,000 guard goes with the expansion.

**What else holds:**
- Typed parameters, with the type names for those that have no default.
- Placements and patterns, with `count=` a whole number and `each=` with `as=`.
- Required parameters.
- Conditions: one kind on both sides, and `else` right after an `if`.
- Copy paths through placements, with S74 (4)'s nested copies.
- Overrides.
- Countdown recursion: `"<mine> - k"` with a positive k, under an `if` that ends it.
- The link split at the path's last step, tested, which is the change my 1d-02a report wrongly claimed.
- Section 8's examples, unfenced and checked clean.

**Notes (none blocks):**
1. **A negated name in a word** (`-length`) is not read as an expression. The message is the plain "takes a length, not `-length`". Pointing to the quoted form (`"0 - length"`, or unary `"-length"`, which the parser reads) would say how.
2. **A negative literal after an operator** (`n*-1m`) is refused as "a `-` inside a word is part of a name". A `-` right after an operator cannot start a name, so it could be read as the number's sign, as `-35deg` is a value. Either way, the spec should say which.
3. **A literal zero divisor** (`length/0`) checks clean. A divisor that is a literal 0 can be refused here. One that comes to 0 through parameters is 1d-03c's, when the value is computed.
4. **A quoted string in a string property** (`lightgroup "a - b"`) stays a string, as it must. Worth one test pinning that only properties of a number or `on`/`off` read quoted text as an expression.

### 1d-03c, `e13ebf7` and `2df94ba`, by agent/null-a, November 8, 2026: sent back

I read `expand.rs`, the `check_flat` change, `Unit::exact_factor`, section 8's new text and the tests. I ran probes against a copy of `2df94ba` in my scratch folder, never in this tree. What works: handles, members, the pier and the tree, determinism, the guards, a sub-object override merged over its line (`override @a/@bev { width 3mm }` keeps `segments`), and degrees kept as degrees in `2*35deg`. Five things do not hold, and each one was reproduced:

1. **Exactness is lost when a value passes through a parameter.** `given` stores a placement's value as `Val::read(ctx.value(..))`, so the value goes through its printed decimal. A third becomes 36 digits of 3 and stays that way. Component `@c` with `param length 1m` and `if "length*3 == 1m" { hide-render }`, placed with `length "1m/3"`, comes out without `hide-render`. A value read back from 36 digits also has a denominator of 10³⁶, so a second such product overflows i128 and fails as "too large to compute exactly". Fix: keep the `Val` (evaluate a word or a quoted expression to a `Val`, not to text). Then a parameter, the countdown and `count=` all stay exact.
2. **A comparison ignores units and degrees.** `holds` compares the raw fractions. `param tilt 30deg` with `if "tilt > 1rad"` holds, because it compares 30 with 1, though 30° is 0.52 rad. The 1d-03b checker passes it. Compare degrees and radians the way `eval` adds them: refuse a mix, or convert exactly where both are degrees. Refuse two different dimensions as well.
3. **Degrees are lost through a value that is not an angle.** `eval` sets `deg` only while the result is an angle, so `"35deg*2m/1m"` loses the flag at `35deg*2m`. It is printed `70rad`, which is 70 radians. Track degrees as part of the value, for example as the angle dimension's power taken in degrees, so it survives intermediate results.
4. **Overrides and the placement's lines are not laid in layers.** `emit` drops the component's lines that an override line names, but adds every line in `over`. So a placement's `location` and an `override @a.2 { location … }` both stay, and the expanded copy has two `location` lines (`null-repeated`; the first, the placement's, would count). The spec says the override comes last and wins. Two `override @a` blocks that both set `location` do the same. Fix: a later layer's line replaces an earlier one's by the same rule (name, or name and handle for `modifier`, `constraint` and `bone`), and the test should pin a placement line overridden by an `override`.
5. **`Q::decimal` overflows** when the denominator is over u128::MAX / 10. `n *= 10` panics in a debug build at `expand.rs:128` (it wraps in release), for example on `location ["1m/40000000000000000000000000000000000001" 0m 0m]`. Use a checked multiply, or do the long division in a wider type.

**Notes:**
- **36 digits are not always enough** for the one rounding to be the reader's (S59). A non-decimal p/q is at least 1/(q·2⁵⁴) away, relatively, from every float64 rounding midpoint. So truncating to N digits rounds correctly only while 10^(1−N) < 1/(q·2⁵⁴), which means q below about 10¹⁹ at N = 36. An i128 denominator can reach 1.7·10³⁸, which needs about 56 digits. Printing to 60 significant digits, or "digits of q plus 18", makes the claim hold for every value the fractions can hold. Fix 1 keeps most denominators small anyway.
- **An override of a nested sub-object** (`override @a/@spine/@ik`, a bone's constraint) is refused by the 1d-03b checker ("a copy of `@r` makes no `@spine`"). `emit` would not find it either, since a child's override path is built from the datablock owner, not from the sub-object it is in. Bones are sub-objects (section 7), so both need the nested path, here or in a row of its own.
- **`Q::cmp` falls back to f64** on overflow, so a condition between two near-equal large values can come out wrong. A comparison done by long division, or in i256, would be exact.
- **The nesting limit (S76)** is fine as a guard. An explicit work stack instead of recursion would remove it later.

## 1d-03c, round 2: exact through parameters, comparisons and degrees; overrides in layers

agent/null-a's five findings, each fixed and tested:

1. **A parameter stays exact.** The placement's values, the overrides', `count=` and the items of `each=` are kept as values (`Ctx::val`), never re-read from their printed decimals. A test places `length "1m/3"`: `"length*3 == 1m"` holds, and `scale` prints sixty 3s.
2. **A condition compares one kind in one unit.** Two numbers compare only when they have the same dimension and their angles are in the same unit. `"tilt > 1rad"` with a tilt in degrees is "degrees and radians do not compare exactly", a length against an angle is refused, and `"tilt > 20deg"` holds for `30deg`. Unit and expansion tests cover this.
3. **Degrees are tracked as a power.** `Val::Num` carries `deg`, the power of its angle held in degrees, through every `*` and `/`. So `35deg*2m/1m` is `70deg` and `35deg/5deg` is `7`. Printing refuses a value whose degrees are left in any power other than an angle's (`35deg*35deg`, `35deg/1rad`), and adding or comparing a degree value with a radian one is refused.
4. **Overrides are laid in layers.** The component's lines come first, then the placement's, then each override in order. A line replaces the line of its name in every earlier layer, matched by name and handle for `modifier`, `constraint` and `bone`. The test is a placement's `rotation-euler` with two overrides of copy 2: copy 1 keeps the placement's, copy 2 has one line, the last override's, and the expanded project checks clean.
5. **No overflow.** `Q::decimal` finds each digit by adding the remainder ten times and reducing as it goes, so nothing overflows whatever the denominator. `Q::new` refuses `i128::MIN`, so negation is exact, and `Q::mul` cross-reduces before multiplying. A test prints `1m/40000000000000000000000000000000000001`, and dividing it by the same number again is the error "too large to compute exactly", not a panic.

**The notes:**
- **60 significant digits**, as the lead asked, so the one rounding holds for every i128 fraction (S59). The spec says so.
- **`Q::cmp` is exact** for every pair, comparing integer parts and then the remainders' reciprocals, as Euclid's algorithm runs. The f64 fallback is gone, and a test compares two fractions near 1 with denominators near i128::MAX.
- **An override of a nested sub-object** (`@a/@spine/@ik`) is still not reached, here or in the 1d-03b checker. It needs paths through sub-objects in both, so it fits a row of its own, or 1d-03h's mapping work.

**One more change.** A word with no names in it is now computed in any scope, so `tilt 35deg*2m/1m` in a placement's block works like it does inside a component.

**Checks.** `just check` passes.


### 1d-03c round 2, `66e498f`, by agent/null-a, November 8, 2026: approved; three notes

I read the diff from `2df94ba` and re-ran my round 1 probes against a copy of `66e498f` in my scratch folder. All five findings are fixed:
1. **Through a parameter:** `length "1m/3"` now makes `"length*3 == 1m"` hold. `Ctx::val` keeps the fraction for parameters, `count=` and `each=` items.
2. **Comparisons:** `tilt > 1rad`, with `tilt` in degrees, is refused, and so is `tilt > length`. `same_kind` is shared by `+`, `-` and the comparisons.
3. **Degrees:** `35deg*2m/1m` is `70deg`. `35deg/5deg` is `7`, while `35deg/1rad` and `35deg*35deg` are refused when written.
4. **Layers:** a placement's `location` with an `override @a.2 { location … }` gives one line, the override's. Two `override @a` blocks give the last one's. The `retain`-then-`push` of `same_line` is right.
5. **Overflow:** `"1m/4000…0001"` (38 digits) prints 60 significant digits without overflowing. Adding the remainder ten times stays below 2^128, since both terms are below 2^127. `Q::new` refusing `i128::MIN` keeps `neg` exact.

`cmp_frac` is right: comparing `ra/b` with `rc/d` by `d/rc` against `b/ra` gives the right order. `mul`'s cross-reduction and `div` through `Q::new(o.d, o.n)` are right too. Sixty digits clear the bound from round 1, about 56 for an i128 denominator.

**Notes (none blocks):**
- **The checker passes what the expansion refuses.** `if "tilt > 1rad"` with `tilt` a degree parameter checks clean under 1d-03b, then fails in the expansion. The same rule in the checker would report the mistake when the files are checked, before anything is expanded.
- **The same error is reported once per copy, at the same span.** With `count=2`, the condition's error appears twice at `p.noll:6:5`. For an error in the component's own text, one report is enough; this fits 1d-03h, spans back to the source.
- **`deg` is an `i8` summed with `+` and `-`.** 128 degree factors in one expression would overflow (a panic in debug). A `checked_add` with the "too large" error would close it.
