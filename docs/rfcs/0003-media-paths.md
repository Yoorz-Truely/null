# 0003: How a media datablock's file is written

- Status: accepted
- Date: November 9, 2026
- Decided by: the lead under the user's standing instruction (Sau's decision S59), recorded as Sau's decision S83
- Affects: forms over RNA, diagnostics
- Null format: unchanged

## Problem

Section 12 says that media live in `media/`, "named by content hash, and are referenced from Null with their colour space, frame range and other settings", and that packed files are unpacked into `media/` on conversion. It does not say how a reference is written. Sau needs to know this to convert a `.blend` file's images, sounds, fonts and caches (task 1d-05f) and to apply them back.

RNA gives each media datablock a `filepath` string: `Image`, `Sound`, `VectorFont`, `MovieClip`, `CacheFile` and `Volume`. Blender writes these paths relative to the `.blend` file, with `//` (`//textures/oak.png`). A project has no `.blend` file, so `//` names nothing. As section 12 stands:

```null
image @oak "oak.png" {
  filepath "//textures/oak.png"   // relative to which file?
  source file
}
```

Three more questions are open:
- What happens to a packed file, whose bytes are in the `.blend`, with a `packed-file` RNA can read but not write?
- How is a path checked?
- What does Sau do when a sound's, font's or movie clip's file is missing? Blender makes those datablocks only from their file.

## Design

Section 12 gains these paragraphs:

> **A media datablock's file** (`image`, `sound`, `vector-font`, `movie-clip`, `cache-file`, `volume`) is its `filepath`. It is written in one of two ways:
> - **A path from the project's root**, such as `filepath "media/3f2a….png"`. It has `/` separators, no `.` or `..` parts, and no part that leaves the root through a link.
> - **An absolute path** to a source outside the project, such as `filepath "/Volumes/Library/hdri/dusk.exr"`.
>
> Blender's `//`, relative to a `.blend` file, is never written: a project has no `.blend` file. Sau gives Blender the path joined to the project's root, and writes a path under the root back from the root.
>
> **Packed files are never written.** `packed-file` has no line. A file packed in a `.blend` becomes a file in `media/` when the `.blend` is converted, named by the SHA-256 of its bytes and its extension (`media/<sha256>.<ext>`). An external file is copied there the same way, so the project holds every byte it needs. Referencing a file in place is still written by hand as an absolute path.
>
> **`sau check`** reports, at each `filepath` line:
> - `cosmosis-media-path`, an error: a path that is neither form (`//`, `\`, a `.` or `..` part, or one that resolves outside the root through a link);
> - `cosmosis-media-missing`, an error: a path from the root that names no file;
> - `cosmosis-media-outside`, a warning: any absolute path, missing or not, since it is not portable and a shared project should show it.
>
> **A missing file at apply.** Sau reports it and still makes the datablock, so references to it resolve. A sound, a font and a movie clip are made from their file when it exists. When it does not, they are made empty, with the path set, as Blender keeps a datablock whose file has gone: a sound that plays nothing, a font with no glyphs, a clip with no frames. They load once the file is there. An image, a cache file and a volume are made from their name, with the path set, as now.

The other settings are RNA's as they are: an image's `source`, its `colorspace-settings` and `alpha-mode`, a sequence's frame range on the image user. A file converted from a `.blend` reads as follows, each `<…>` the 64 hexadecimal digits of the hash:

```null
image @oak "oak.png" {
  filepath "media/<the SHA-256 of its bytes>.png"
  colorspace-settings { name "sRGB" }
  alpha-mode channel-packed
}
sound @surf "surf.wav" {
  filepath "media/<the SHA-256 of its bytes>.wav"
}
```

Image sequences, UDIM tiles and movie frames are not copied one file per frame by this proposal. Converting a `.blend` reports them, and a later proposal says how their files are named.

## What breaks

Nothing.
- No file of Cosmósis's corpus or Null's slice project writes a media datablock.
- A hand-written `//` path, which named nothing in a project before, is now the error `cosmosis-media-path`, with a fix.

## Migration

None.

## Alternatives

- **References by hash alone** (`media sha256:9b74…` or `file=sha256:9b74…`). Content addressing is what `media/` already does, so a hash reference names the bytes without a path. But the bytes still have to be found, and the only place they live is `media/<sha256>.<ext>`, so the reference is that path with its folder and extension left out. It would need a second form for files outside the project, which have no hash a person can write, and a reader would need to know the naming rule to open the file. A path is what Blender's own field holds, what a person or an editor follows, and what `sau check` can test with no lookup. The layout check already holds every file in `media/` to its hash, so the path loses no safety.
- **Paths relative to the file that names them**, as data files are (section 9). Media are shared across a project far more than a mesh's array is, and a path from the root stays the same when a datablock moves between files, as section 2's "Composition" asks.
- **Keeping Blender's `//`, read as the project's root.** The same text would then mean the project's root in Null and the `.blend` file's folder in Blender. That breaks the moment a `.blend` is saved anywhere but the root, and it hides which of the two a path means.
- **Packed files kept packed**, as hex in a `raw` block or a data file. That puts megabytes of image data in text, against S23, which keeps large binaries out of `.noll` files, and it gives Git and LFS nothing to share between projects that use the same image.

## Decision

Accepted on November 9, 2026, by the lead under the user's standing instruction (Sau's decision S59), recorded as Sau's decision S83. It says how section 12's references are written, which no file of the corpus or the slice used before, so no file changes meaning. The wording above is in section 12 (task 1d-05h), and Sau and Cosmósis follow it (task 1d-05f).
