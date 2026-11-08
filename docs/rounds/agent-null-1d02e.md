# agent/null-1d02e

The report of agent/null-a working as `agent/null-1d02e` in `work/wt/null-1d02e/`, on branch `agent/null-1d02e` in Null, stacked on `agent/null-1d03d` (`11b87a3`). November 8, 2026. Null's tests only.

## 1d-02e: diagnostic wording

These are agent/cosmosis-a's five notes on 1d-02c, each done:
1. **The nearest choices first.** `values::list_near(word, items)` lists all the choices, in the schema's order, where there are twelve or fewer. Otherwise it lists the twelve nearest the wrong word by edit distance, ties in the schema's order, then a count. Every message about a wrong word with a list of choices uses it:
   - an enum value;
   - a datablock, primitive, node, zone, modifier, constraint and other sub-object kind;
   - an action strip and a driver variable kind;
   - a node output;
   - a socket type;
   - an attribute's domain and data type.

   Messages listing choices without a wrong word keep `list`. `bsdf-principle` now lists `bsdf-principled`, `bsdf-hair-principled`, `bsdf-diffuse`, …, and `objeet` lists `object` first. The sub-object and node messages had their own "first twelve, …" and now share the rule.
2. **One error per mistake.** `Checker::unknown` gives no `null-unknown-property` when the name in kebab-case is one of the owner's properties. The syntax pass's `null-name` reports that case with the same fix. `null_check::check` is always run with the syntax diagnostics where it is used (Cosmósis's `source` gives both).
3. **`key = value`.** When Scowr's "expected a key before =" comes after spaces that follow a node's name, `null_syntax::messages` says "`=` takes no spaces: write `name=…` on its owner's line, or `name …` as a line of its own". The fix replaces ` = ` with a space, giving the line `name "x"`.
4. **Read-only.** The error now ends "so a file cannot set it; remove the line". The fix removes the line, with its indentation and line feed where it is a line of its own. A `key=value` on its owner's line gets no fix.
5. **Range notes** give the value RNA's clamp gives: the hard minimum or maximum. A NaN, which `CLAMP` leaves as it is, says so.

**Tests.** `tests/diagnostics.rs` adds two cases: a NaN out of range, and a read-only line in a block of several lines. Its snapshots now end with what each fix makes of the case's text, so every fix's edit is held, not only its words. `check__errors.snap` follows the new wording. Section 18 says how messages are worded.

**For the integrator.** The messages changed are Null's. Cosmósis's and Sau's tests that quote Null messages were searched, and none quotes a changed one: Cosmósis's `colr` → `color` is unchanged.

**Checks:** `cargo test --workspace` and clippy `-D warnings` are clean, and Null's lint-only `just check` passes.
