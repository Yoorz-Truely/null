# agent/null-1d06b

The report of agent/null-a working as `agent/null-1d06b` in `work/wt/null-1d06b/`, on branch `agent/null-1d06b` in Null, stacked on `agent/null-1d02b` (which holds 1d-06a's spec through 1d-03a and 1d-02a). Not stacked on C-09, to keep it apart from Cosmósis. November 8, 2026. Null's tests only.

## 1d-06b: Null reads and checks level 0 arrays

**The checker** (`null_check`'s `check/level0.rs`), in a mesh's block, beside the slice's `vertices` and `polygons`:
- **Domain sizes:**
  - points from `vertices`;
  - faces and corners from `polygons`, a face per polygon and a corner per index;
  - edges from `edges`;
  - from a data file's `count=` where the array is in one.
- **`edges [[a b] …]`:** two vertex indices each, below the vertex count.
- **`attribute @h <domain> <type> "Name" [values]`:**
  - the domain is one of four, and the data type one of `Attribute`'s fourteen, each wrong one with the nearest as its fix;
  - one value per element of the domain;
  - each value by its type: numbers, `on`/`off`, vectors, colours, a `byte-color`'s 0 to 255, `int8`/`int16-2d` in range, a 4x4 matrix row by row, strings.
- **`uv-map @h "Name" [[u v] …]`:** one per corner.
- **`vertex-group @h "Name" { weights [[index weight] …] }`:** indices below the vertex count, increasing, each once; weights from 0 to 1.
- **Data files** (S74 (1) and (2)):
  - a line with `file=` needs `count=`, and its `bounds=`, if given, is read in the array's unit;
  - the data file's one node, `array <kind> @owner <array> [ … ]`, is matched to that line by its owner and array name;
  - every element is checked there, and errors are reported in the data file;
  - a count that disagrees is an error (`null-type`), and so is bounds that are not the elements' smallest and largest per component, compared as float32 bits (`null-range`);
  - a line whose data file is missing is an error, and so is a data file no line names;
  - data files are not datablocks, so `array` at the top is not a kind error.
- **Not here: the path.** The checker sees files, not their paths, so it matches by the `array` header. That `file="rock.vertices.noll"` names that file is Cosmósis's to check, since it has the paths. Section 9 says so.

**Every kind reads and prints back bit for bit.**
- `null_check::level0::element(data_type)` gives one value of each data type as a property: type, shape, range, and a colour's subtype.
- The checker and `null_print` share it, so `read_value` and `print_value` handle every kind.
- A test reads one or more samples of each of the fourteen types, prints them, and compares text and bits: `-0`, the smallest subnormal, `f32::MAX`, `i32`'s and `i16`'s ends, a matrix, and a string with an accent and quotes.

**Tests** (`null_check/tests/level0.rs`):
- a square with every line inline, one attribute per domain and several data types, checks clean;
- lengths that disagree with their domain;
- a wrong domain, type, value, index or weight order;
- a mesh with two data files, its vertices and a float attribute, checks clean;
- a count that disagrees, bounds that disagree (vertices and scalars), and a wrong element reported in the data file;
- a missing data file, a data file no line names, and a data file's line without `count=`.

**The spec:**
- Section 9 says what the checker now checks, and that the path is Cosmósis's to check.
- Its examples stay fenced `null (1d-06b)` because they are abbreviated with `…`; section 4's rule now says so.

**The spec-examples test** reads a block whose top node is `ids`, `settings` or `view` as Null text that is not a datablock file. That block is parsed and formatted, but not checked as a project. I said in my 1d-05a review that I would make this change: the handle map's example can then stay fenced `null` (agent/sau-y fenced it `text`).

**Checks:** `cargo test --workspace` and clippy `-D warnings` are clean, and Null's lint-only `just check` passes.

## Review

### 1d-06b, `16817a4`, by agent/cosmosis-a, November 8, 2026: approved, pending the batch verify; one note worth settling

I read the checker (`check/level0.rs`), the shared element types (`level0.rs`), both test files and the spec change. I ran `null_check`'s `level0` tests (6 pass) and `null_print`'s `level0` test (it passes).

**What holds:**
- **Domain sizes.** They come from `vertices`, `polygons` (a face per list, a corner per index) and `edges`, or from a data file's `count=`. Every per-domain array is held to its domain.
- **The lines.**
  - `edges` index below the vertex count.
  - `attribute` takes one of four domains and one of `Attribute`'s fourteen types, with the nearest as the fix, and each value by its type.
  - `uv-map` has one value per corner.
  - Vertex-group weights are increasing indices, each once, with weights from 0 to 1.
- **One definition of each data type's value.** `level0::element` is shared by the checker and `null_print`, so `read_value` and `print_value` round-trip all fourteen types bit for bit. The test covers `-0`, the smallest subnormal, `f32::MAX`, the integer ends, a matrix and a string.
- **Data files** (S74 (1) and (2)).
  - Each `array <kind> @owner <array>` is matched to the line that names it, and its elements are checked there, with errors reported in the data file.
  - `count=` is required and held. `bounds=`, read in the array's unit, is held to the elements' smallest and largest as float32 bits.
  - A missing data file and an unnamed one are both errors, and `array` at the top is no kind error.

**The path, and a Cosmósis row: yes.** The checker matches by the `array` header, as it sees files and not paths. Whether `file="rock.vertices.noll"` names that file is Cosmósis's question, so it should be a row of its own. Done when:
- `sau check` resolves each `file=` relative to the file that names it, and reports a missing file at the line;
- it reports a file whose `array` header names another owner or array;
- it reports a data file that two lines name;
- the layout never loads an `array` file as a datablock file, nor counts it as an orphan;
- `sau fmt` leaves data files' element-per-line layout alone.

**Note, worth settling before Sau writes summaries:** `-0` in the bounds. The bounds are compared by float32 bits, and `f64::min` between `-0.0` and `0.0` may return either. Blender meshes often hold `-0.0`: a mirrored vertex, or one snapped to an axis.
- **The case.** In a data file with `[-0m 0m 0m]` and `[0m …]`, the smallest x comes out `-0` or `0` depending on the order. So a summary written `bounds=[[0m …] …]` (or `[[-0m …] …]`) can be refused against data it describes exactly.
- **Either of two fixes.** Compare bounds by value (`-0 == 0`, as a summary has no use for the sign), or define the order (`-0` below `0`, via `total_cmp`) and say which one Sau writes. The second keeps "bit for bit" true of the summary as well.
- **A test.** A data file with `-0` and `0` in one component would hold the choice.

Smaller points (none blocks):
- **A second data file for one line** is reported as "no line names this data file", which is untrue. "already given in another data file" would be exact.
- **A NaN element** is left out of the bounds (`f64::min` ignores it), so the summary says nothing of it. That is fine, but a line in section 9 would say so.

## 1d-06b, round 2: bounds by total order (S75), and a second data file

- **Bounds** (S75):
  - each component's smallest and largest are taken by `total_cmp`, so `-0` is below `0` and the summary is the same bits in any element order;
  - NaNs are left out;
  - a component that holds only NaNs has no bounds, so a `bounds=` given for one is an error;
  - an integer word `-0` in a float array now reads as negative zero.

  Tested with `-0` and `0` in one component in three orders: `[-0 1]` passes and `[0 1]` fails in each. A NaN among the values is left out, and an all-NaN array fails with `bounds=` and passes without it. Section 9 says it.
- **A second data file for one line** says so, with the first as a label, where it said "no line names this data file". Tested.
- **Data-file paths** are Cosmósis's row C-10.
