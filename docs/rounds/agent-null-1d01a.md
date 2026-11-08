# agent/null-1d01a

The report of agent/null-a working as `agent/null-1d01a` in `work/wt/null-1d01a/`, on branch `agent/null-1d01a` in Null, cut from Null's `agent/batch-2` (which holds 1a-17) as the lead asked, with `agent/null-1d07a` merged in. November 8, 2026. Lints and Null's tests only (S69).

## 1d-01a: Null v0's spec complete

**What 1d-07a's merge brought.** 1d-07a rewrote sections 10 and 11 and touched sections 4, 7 and 8, the sections this row reads. So I merged its branch, which is approved, rather than edit beside it. The one conflict was section 10's first bullet, which batch-2 (1a-17) had reworded and 1d-07a replaced; I kept 1d-07a's. The merge commit passed `just check`.

**Every section read against the implementation.**
- **Sections 4 to 8, 10 and 11** describe what the checker reads. I checked every ```null example of the whole spec as one project, against the schema of all project data. Section 4's first look did not check:
  - its `color-management` line (no such form);
  - `render cycles { samples …; resolution […]; denoise on }` (one spelling per property, S70: `render { engine cycles; resolution-x …; resolution-y … }` and `cycles { samples 512; use-denoising }`);
  - the array modifier's `relative-offset` (RNA's is `relative-offset-displace`);
  - `image @oak-diffuse interpolation=cubic` (a property on a reference line);
  - an image `source` that is media's form, not yet built;
  - handles defined nowhere (`@shot-cam`, `@dusk`, `@lamp`, `@boats`).

  It is now a whole project of seven files: a camera, a world, a lamp whose light plays the action, and a collection defined in the scene. Its handles are distinct from the other sections' examples.
- **A new test, `every_example_in_the_spec_checks_clean_as_one_project`** (`spec_examples.rs`), takes all 14 examples as one project, so an example the checker refuses fails the build. Section 4 says so.
- **Found by it, fixed in the checker.** Blender's image-node `interpolation` items (`Linear`, `Closest`, `Cubic`, `Smart`) are free text, and the schema marks them `quoted`. Section 6 says such an item is a string. But the checker took `Cubic` bare, and for `cubic` it offered the fix `Cubic`.
  - Now a bare word naming a free-text item, in any case, is a `null-type` error whose fix is the quoted item (`"Cubic"`).
  - A misspelling near one gets the quoted fix too.
  - Test: `an_enums_free_text_item_is_a_string_and_its_fix_is_quoted`.
- **Section 5's claims hold:**
  - a property on a line or as a child line;
  - raw strings;
  - a bare boolean and `on`/`off`;
  - a `\` continuation;
  - a struct block beside a flat form (`render { … }` and `fps`).
- **Section 15:** the layout and the canonical order are built, but leaving out defaults is not: `null_print::canonical` keeps them. The section now says so and gives it to 1d-02b.
- **Sections 12, 13 and 19** now open with what builds them: 1d-05f (media), phase 5 (generators), and 1d-05a to 1d-05e on 1b-04's audit (conversion). Section 14 already gave its vocabulary to phase 8, and section 9's lifting is phase 4b.
- **Section 18's codes** are the eleven `null_check::codes` and `null_syntax`'s `null-syntax`, no more and no fewer.
- **Section 22's table** has each lesson in the section it names (1a-17's work). Sub-object handles (section 7) and the types and units (section 6) were already in.

**Section 9 follows S23.**
- Arrays up to 2,000 elements are inline, and larger ones go in a Null text file of their own, one element per line.
- The summary is a comment beside the reference, checked on load.
- A binary copy in `.sau/` is only a cache.
- A binary data file is only by opt-in, for very large scanned data, its format specified when a project needs it.
- The syntax is 1d-06a's. A lifted mesh's residual is level 0 data.

**Section 20 lists nothing open:**
- the threshold is 2,000 elements (S23);
- data used once lives in its user's file (S36), as `sau convert`'s default;
- the check vocabulary is phase 8's.

The header says the draft is complete, and that freezing it is 1d-01c. The changelog records it.

**Overlaps with branches not merged here,** for the integrator. Each is a separate paragraph or row, so each should merge cleanly:
- **1b-01h** (in review): a "Renaming a handle" bullet in section 7, after "Across files".
- **1c-02d** (approved): "The handle map" subsection in section 7.
- **1c-02i** (in review): the string row of section 6's types table. Its fixed wording says a display name is a string (S72).
- **Two notes for the lead:**
  - S73's sub-object block in the handle map is specified by 1b-02b. Once 1c-02d and 1b-02b land, section 7's "The handle map" should name the block form.
  - The schema gives `interpolation`'s default as `linear` while its items are `Linear` and the rest. This is a Sau schema-generator detail: a quoted item's default should keep its case, or the formatter's default omission (1d-02b) will not match.

**Checks:** `cargo test --workspace` and clippy are clean, and Null's lint-only `just check` passes.
