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
