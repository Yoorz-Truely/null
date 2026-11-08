# agent/null-1d03a

The report of agent/null-a working as `agent/null-1d03a` in `work/wt/null-1d03a/`, on branch `agent/null-1d03a` in Null, stacked on `agent/null-1d06a` and `agent/null-1d01a` (both in review), since all three edit the spec. November 8, 2026. Lints and Null's tests only.

## 1d-03a: components, parameters, patterns, bounded repetition and recursion specified

A new subsection of section 8, "Components, parameters and patterns". Section 3's "Turing-complete?" row points to it.

**The forms:**
- **`component @h "Name" { … }`** holds parameters and the datablocks it makes. Its first object is the root.
- **Parameters:** `param <name> <default>`, typed by the default. A list of words is `{ one-of […] }`. A required parameter names its type instead of a default.
- **`use @component @h { … }`** places one copy. The root takes `@h`, and the block gives parameters and the root's own properties.
- **Patterns:**
  - `count=N` places copies `@h.1` to `@h.N`, and `n` is the copy's number (S61 (6));
  - `each=[…] as=<name>` places one copy per item.
- **`if "…" { } else { }`**, **`override @h.7 { … }`**, and a copy's other datablocks reached by path (`@plank.7/@plank-mesh`).
- **Expansion on load, before checking** (S32): deterministic, with instancing underneath only where copies are identical. Write-back keeps the abstract form (S35): a moved copy writes one `override`.
- **Totality** (S38):
  - counts and lists are known before expansion, and conditions only pick lines;
  - recursion is allowed only on a `{ countdown }` parameter that each recursive placement makes smaller, under an `if` that holds while it is above 0;
  - a guard: a project that expands past 1,000,000 datablocks is an error, never a hang.
- **Examples:** the pier of twenty planks with plank 7 liquid (a fluid modifier and a water material), and a tree of branches counting down. They are fenced `null (1d-03b)`.

**Decisions I took, for the lead to overturn:**
1. **Arithmetic within Scowr's syntax, unchanged** (S15: its syntax changes only with the user's agreement). Scowr reserves `( )`, and a `-` inside a word is part of a kebab-case name. So an expression with only `+ * /` and no spaces is one word (`n*25cm`, `length/1m`). Subtraction, grouping and comparisons are a quoted expression with spaces (`"length - 10cm"`, `"(n - 1) * 25cm"`, `"state == liquid"`), as a driver's expression is already quoted.
2. **A copy's other datablocks are paths from its root** (`@plank.7/@plank-mesh`), not new top-level handles. A placement owns what it expands into, as an owner owns its sub-objects, and no handle can collide across copies.
3. **The root is the component's first object.** A component that makes no object (a material preset) is placed by its first datablock.
4. **The countdown is declared** (`{ countdown }`), so the checker proves termination by one rule rather than by analysis.

**Test:** `null (<task>)` fences must name a task, and must already parse with Null's syntax. All three new examples and 1d-06a's do. They join the full check when 1d-03b and 1d-06b land.

**Checks:** the spec-examples test passes, and Null's lint-only `just check` passes.

## Review

### 1d-03a, `93ae1bd` and `063c628`, by agent/cosmosis-a, November 8, 2026: approved for its own section, with two questions; it lands after 1d-06a's fix is merged in

I read the components section and the test change, against S19, S32 to S35, S38, S54, S61 (6) and S74. I ran the whole `null_check` suite and Clippy (`-D warnings`) here. All pass.

**What holds:**
- **S74 is followed:**
  - arithmetic inside Scowr's syntax: `+ * /` in a word (`n*25cm`, `length/1m`); subtraction, grouping and comparisons quoted (`"depth - 1"`, `"state == solid"`) (3);
  - a copy's other datablocks by path from its root (`@plank.7/@plank-mesh`) (4);
  - recursion only on a declared `{ countdown }` parameter that every recursive placement makes smaller, under an `if` that holds only above 0, with expansion past 1,000,000 datablocks an error at the line that asked for them (5).
- **Totality (S38)** is argued from the forms themselves: the counts and lists are known before expansion, a condition only picks lines, and recursion counts down. So every file finishes loading.
- **Expansion (S32), overrides (S33), patterns (S34) and the abstract write-back (S35)** are each stated once, with their tasks (1d-03b, c, e, f).
- **The test parses each unbuilt fenced example.** It parses the body of every `null (<task>)` fence, all 5 of them: this section's three and 1d-06a's two. So an unbuilt example cannot drift from Null's syntax before its task lands. The 14 plain examples still check clean as one project.

**What is built and what is not.** Built: the syntax check of fenced examples. Not built:
- parsing and checking components, parameters, patterns, conditions and overrides (1d-03b);
- expansion (1d-03c);
- instancing identical copies (1d-03e);
- writing edits as overrides (1d-03f).

The section says so, and its examples are fenced `null (1d-03b)`.

**Questions for the section (not blocking; 1d-03b will meet them):**
1. **One handle, three names.** The pier example names the component, its root object and the placement all `@plank`: `component @plank`, `object @plank` and `use @plank @plank count=20`. Handles are per kind, so this may be meant. But the section never says whether a component's handle may equal its root's or a placement's, nor whether the template root `@plank` is a handle in the expanded project, besides `@plank.1` to `@plank.20`. One sentence would settle it.
2. **Placements inside a component.** The tree places `@left` and `@right` inside `@branch`. The section gives paths for the datablocks a copy defines (`@plank.7/@plank-mesh`), but not for the copies a copy places. The rule that makes `use @branch @tree` give `@tree/@left/@left` should be stated, so overrides can reach them and write-back can name them.

**Stacked on 1d-06a.** This branch carries 1d-06a's section 9 bullet that gives the summary as a comment (`null-v0.md:459`). 1d-06a goes back for it, so this branch lands only once that fix is merged in.
