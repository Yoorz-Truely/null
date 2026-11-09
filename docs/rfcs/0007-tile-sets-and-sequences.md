# 0007: Tiled images and image sequences in `media/`

- Status: accepted
- Date: November 9, 2026
- Decided by: the lead under the user's standing instruction (Sau's decision S59), recorded as Sau's decision S96
- Affects: forms over RNA, diagnostics
- Null format: unchanged

## Problem

RFC 0003 (S83) names one media file by its contents, `media/<sha256>.<ext>`, and the datablock's `filepath` names it. That works for a datablock whose data is one file. Two kinds of image are many files under one `filepath`, and section 12 leaves them out ("not yet copied one file per frame"):

- **A tiled image** (`source tiled`, UDIM). Its `filepath` holds a token, `<UDIM>` or `<UVTILE>`, and Blender reads each tile from the path with the token replaced: `wood.<UDIM>.png` is `wood.1001.png`, `wood.1002.png`, and so on (`BKE_image_get_tile_strformat`). A packed tiled image packs each tile apart.
- **An image sequence** (`source sequence`), and a movie clip made of image files. Its `filepath` names one frame's file, and Blender finds the others by the last run of digits in the file name, with the same number of digits (`BLI_path_sequence_decode`, `BLI_path_sequence_encode`): `render/shot_0001.png` reaches `render/shot_0002.png`. Which frames play is the image user's (`frame-start`, `frame-duration`, `frame-offset` on a texture node or the image editor), not the image's.

Today `sau convert` keeps their paths as they are and reports them (batch 4's media gate, Sau's `media_gate.py`):

```null
image @wood "wood" {
  filepath "/Users/ana/textures/wood.<UDIM>.png"   // kept where it was: not portable
  source tiled
}
image @shot "shot" {
  filepath "/Users/ana/render/shot_0001.png"       // the same
  source sequence
}
```

The project then depends on files outside it, which is what section 12 is there to prevent, and a packed tiled image is lost.

Neither file can simply be named by its own hash. Blender derives every member's path from the one `filepath`, by its number, so the members must share one name pattern. Hashes, which differ per file, cannot follow a pattern.

## Design

A tile set or a frame sequence is a **folder** in `media/`. The folder is named by the hash of what it holds, and each file in it is named by its number. Section 12 gains:

> **A tiled image or an image sequence** is a folder of files in `media/`, one per tile or frame:
> - a tile set is `media/<sha256>.tiles/`, each tile `<number>.<ext>` (`1001.png`, `1002.png`), and the image's `filepath` is `media/<sha256>.tiles/<UDIM>.<ext>`;
> - a sequence is `media/<sha256>.frames/`, each frame `<number>.<ext>` with the number's digits padded to one width for the whole folder (`0001.png` … `0240.png`, the width the source had), and the `filepath` names one frame, as Blender's does: `media/<sha256>.frames/0001.png`. A movie clip made of image files is written the same way.
>
> Every file in the folder has the same extension. The folder's `<sha256>` is the SHA-256 of a listing of its files: one line for each, in increasing number, holding its name, a space, the SHA-256 of its bytes in lowercase hexadecimal and a line feed (`1001.png 3f2a…\n1002.png 9b74…\n`). The folder is therefore named by its contents as a single file is, so two projects holding the same tiles hold the same folder, and the layout check can test it.
>
> **Converting** a `.blend` copies every tile Blender reads, or holds packed, into a tile set. A tile set written with `<UVTILE>` (`u1_v1`) is renumbered as UDIM numbers (`1001 + (u - 1) + 10 × (v - 1)`), which is how Blender numbers the tiles inside: the tiles are the same, only the token changes. A sequence copies every file that Blender's rule reaches from the `filepath`: the same folder, head and tail, the same number of digits. A missing number is left missing, as Blender shows that frame missing. The image users keep their frame ranges, which are RNA's.
>
> **A tile's own settings** (`UDIMTile`: its `label`, and the generated colour of a tile with no file) are written `tile <number> { … }` in the image's block, only for a tile that has any. The tiles themselves are the folder's files. Sau gives Blender the tiles the folder holds, as reloading the image finds them.
>
> **`sau check`** reports, at the `filepath` line, as errors:
> - `cosmosis-media-set`: a folder whose name is not the hash of its listing; a file in it not named `<number>.<ext>`; two extensions, or two widths in a sequence; a tile number outside 1001 to 2000; a tiled image whose path under `media/` has no `<UDIM>`, or has `<UVTILE>`; a sequence whose `filepath` names no file of its folder;
> - `cosmosis-media-missing`, as before, for a path from the root that names no file or folder.
>
> An absolute path to a tile set or a sequence outside the project keeps Blender's own form, tokens and digits as Blender reads them, with the `cosmosis-media-outside` warning, as RFC 0003 gives any absolute path.

Cosmósis's section 2 says the same of the layout: `media/` holds files named `<sha256>.<ext>` and folders named `<sha256>.tiles` or `<sha256>.frames`, and nothing else.

The two images above, converted:

```null
image @wood "wood" {
  filepath "media/<the hash of its listing>.tiles/<UDIM>.png"
  source tiled
  tile 1002 { label "Back" }
}
image @shot "shot" {
  filepath "media/<the hash of its listing>.frames/0001.png"
  source sequence
}
```

```text
media/
  3f2a….tiles/            the hash of "1001.png 9b74…\n1002.png c0de…\n"
    1001.png
    1002.png
  77e1….frames/
    0001.png
    …
    0240.png
```

## What breaks

Nothing.
- No file of Cosmósis's corpus or Null's slice project holds a tiled image or a sequence.
- A path into such a folder named nothing before (`cosmosis-media-missing`), so no valid file changes meaning.
- `tile` is a new line.
- The layout check now accepts the two folder forms in `media/`, which it refused before.

## Migration

None. A project converted before this, with its tiles or frames left outside, still reads. Converting its `.blend` again brings them in.

## Alternatives

- **Each member named by its own hash, with the folder made only for Blender** (Sau writing links or a cache folder at apply). Each file would be shared even when one tile changes. But the folder Blender reads would not be the project's: links do not survive every Git host, every Windows checkout or every LFS setup, and a cache folder makes the project's `media/` something Blender never sees. That breaks section 12's rule that the project holds every byte it needs where it says.
- **A manifest file** (`media/<sha256>.tiles.noll`, listing tile → file hash) beside flat `media/<sha256>.<ext>` files. It is the same idea with the folder's listing written down. But Blender still needs the files under one pattern, so Sau would build that folder at apply. This is the first alternative with a text file added.
- **The folder named by a hash of the first file, or of the datablock's name.** That is shorter to compute, but it is not the contents: two projects with the same tiles would get different folders, and a changed tile would leave the name unchanged, against what `media/` promises.
- **Keeping the original file names inside the folder** (`wood.1001.png`). Blender would read them just as well. But the name adds nothing the datablock does not already say, and number-only names make the check simple: one pattern per folder.
- **A frame token for sequences** (`####`, as render output paths use). Blender does not read a sequence through a token, only through a frame's own file, so `filepath` would no longer be Blender's value. The digits of the frame's name are the token.

A cost of this design: changing one tile renames the folder. Git sees every other file as moved unchanged and LFS stores nothing again, but the diff lists the whole set. A per-file name would avoid this only by giving up the pattern Blender reads.

## Decision

Accepted as drafted on November 9, 2026, by the lead under the user's standing instruction (Sau's decision S59), recorded as Sau's decision S96. No file of the corpus or the slice holds a tiled image or a sequence, so no file changes meaning and the Null format is unchanged. The wording goes into section 12 and Cosmósis's section 2 with task 1d-05m, which also builds convert, apply, print and the `cosmosis-media-set` checks.
