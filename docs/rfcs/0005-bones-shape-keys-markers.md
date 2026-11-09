# 0005: An armature's bones, shape keys and timeline markers

- Status: proposed
- Date: November 9, 2026
- Decided by: (the lead under the user's standing instruction, Sau's decision S59)
- Affects: forms over RNA, identity, canonical form, write-back
- Null format: unchanged

## Problem

Section 7 names three kinds of sub-object, each with its identity, but no section gives any of them a form. A file therefore cannot write them, Sau cannot apply or print them, and a `.blend` that holds them loses them in conversion (section 19). Task 1d-07q needs all three.

- **An armature's bones.** Section 8 writes an object's pose bones (`bone @spine "Spine" { rotation-mode xyz … }`) and says "an armature's bones themselves are its data's (1d-07h)". Their rest pose (where each bone's head and tail are, its roll, its parent) has no form. So the spec's own rig example, `armature @rig-bones "Rig"`, is an armature with no bones, and the pose bone `@spine` it poses does not exist.
- **Shape keys.** Section 7's table identifies a shape key by `KeyBlock::uid`, through its owner's `Key`. A shape key's value is what an action animates (`fcurve @smile value`, section 11), but the shape key itself, its positions and its settings cannot be written.
- **Timeline markers.** Section 7's table gives a scene's or an action's markers handles by name, or by frame (`@f120`) when the name is empty or repeated, but no line defines a marker.

## Design

Each is a sub-object line in its owner's block, as section 7 says sub-objects are: `<kind> @handle "Name" … { … }`, its settings those of its RNA struct, by their Null names, with a new datablock's defaults left out (section 15).

### An armature's bones (section 8, after "Modifiers, constraints and pose bones")

> **An armature's bones.** `bone @h "Name" { … }` in an `armature`'s block is one bone of its rest pose, in the armature's order (Blender's: each bone after its parent). It holds:
> - `head [x y z]`, `tail [x y z]` and `roll`: the rest pose as Blender's edit bones hold it, in the armature's space, lengths as `location` is written and the roll an angle (section 6). These are the values a user types into an edit bone's fields;
> - `parent @h`, another bone of the armature, and `use-connect` when its head is its parent's tail;
> - `Bone`'s other settings by their Null names: `use-deform`, `inherit-scale`, `use-inherit-rotation`, `use-local-location`, `use-relative-parent`, the envelope's (`envelope-distance`, `envelope-weight`, `head-radius`, `tail-radius`), the B-Bone's (`bbone-segments`, `bbone-x`, `bbone-z`, …), `hide` and the rest;
> - `collections [@h …]`, the bone collections it belongs to.
>
> `bone-collection @h "Name" { … }` is one of the armature's bone collections, in Blender's order, with `BoneCollection`'s settings (`is-visible`, `is-solo`) and `parent @h` for a nested one. A bone names its collections, rather than a collection its bones, so a bone's line holds everything about it.
>
> The object's `bone @h` lines (above) pose the bones of its armature by the same name; an armature's bone and a posed bone are different lines in different blocks, as Blender keeps `Bone` and `bPoseChannel` apart.

```null
armature @rig-bones "Rig" {
  bone-collection @body "Body"
  bone @spine "Spine" {
    head [0m 0m 0m]
    tail [0m 0m 1m]
    collections [@body]
  }
  bone @neck "Neck" {
    parent @spine
    use-connect
    head [0m 0m 1m]
    tail [0m 0.1m 1.3m]
    roll 5deg
    collections [@body]
  }
}
```

> **How Sau writes them** (an implementation note; it adds no meaning). Blender's `Bone.head`, `tail` and roll are read-only outside edit mode: a bone's stored values (`head`, `tail` relative to its parent, `arm_head`, `arm_tail`, `arm_roll`, its matrices) are derived from the edit bones when edit mode is left. Sau writes the text's values without entering edit mode: it makes the armature's edit-bone list from its bones (`ED_armature_to_edit`), sets each edit bone's head, tail, roll, parent and connection from its line, and writes the list back (`ED_armature_from_edit`, then `ED_armature_edit_free`), which derives each bone's stored values as leaving edit mode does. Nothing else of the object or its mode changes. The printer reads the same values back through the edit bones made the same way, so what it prints is what the text gave.

### Shape keys (section 9, after vertex groups)

> **`shape-key @h "Name" { … }`** in a mesh's block (a curve's or a lattice's the same) is one shape key of its `Key`, in Blender's order. It holds:
> - `positions [[x y z] …]`: one position per vertex, lengths as `vertices` are written; more than 2,000 in a data file of its own, as any array (`positions file="face.smile.noll" count=… bounds=…`);
> - `relative-key @h`, the shape key it is relative to (the basis when left out), and `vertex-group @h`, the vertex group that weighs it: both handles, of the mesh's shape keys and vertex groups;
> - `KeyBlock`'s other settings: `value`, `slider-min`, `slider-max`, `interpolation`, `mute`, `lock-shape`.
>
> The first shape key is the basis (Blender's reference key). It gets **no `positions` of its own**: they are the mesh's `vertices`, which Blender keeps equal to it. A `positions` line on the basis is an error.
>
> `shape-keys { … }` in the same block holds the `Key`'s own settings (`use-relative`, `eval-time`) and what plays on the `Key`: `action @a slot=@s` and NLA `track` lines (section 11), whose slot is of kind `key`. A shape key's animated value is an f-curve of that slot written by the shape key's handle: `fcurve @smile value`.

```null
mesh @face "Face" {
  vertices [[0m 0m 0m] [1m 0m 0m] [0m 1m 0m]]
  polygons [[0 1 2]]
  vertex-group @mouth "Mouth" { weights [[1 1] [2 0.5]] }
  shape-key @basis "Basis"
  shape-key @smile "Smile" {
    positions [[0m 0m 0m] [1m 0m 0.2m] [0m 1m 0.1m]]
    vertex-group @mouth
    value 0.3
  }
  shape-keys {
    action @blink slot=@face-keys
  }
}
```

### Timeline markers (section 8, the scene's forms; section 11, an action's)

> **`marker @h "Name" frame=N { … }`** in a scene's or an action's block is one timeline marker, its frame bare (an integer, as Blender stores it), with `TimelineMarker`'s settings in its block: `camera @h`, the camera a scene's marker switches to, and `select`. A marker's name may be empty or shared; such a marker's handle is made from its frame (`@f120`), and its quoted name is left out when it is empty. Markers are written in frame order.

```null
scene @main "Main" {
  marker @intro "Intro" frame=1
  marker @cut "Cut" frame=48 { camera @close-up }
  marker @f96 frame=96
}
```

### The checker, write-back and identity

- The checker reads each line as its section says: `head`, `tail` and `positions` as lengths, `roll` as an angle, `frame=` as an integer frame, and each handle against the owner's own sub-objects of that kind (`parent` against the armature's bones, `relative-key` against the mesh's shape keys, `collections` against its bone collections, `vertex-group` against its vertex groups). A bone's `parent` that leads back to the bone is an error, as is a shape key's `relative-key` naming itself.
- Identity is section 7's: a bone and a bone collection by name, through `.sau/ids.noll`; a shape key by `KeyBlock::uid`; a marker by name, or by frame where the name does not serve.
- Write-back is section 17's smallest edit: a bone moved in edit mode rewrites its `head`, `tail` or `roll` line; a shape key sculpted rewrites its `positions` (inline, or its data file's lines); a marker moved rewrites its `frame=`.

## What breaks

Nothing. Every form is new: a v0 file never held one, and section 8's rig example, an armature with no bones, still reads. No file of Cosmósis's corpus or Null's slice project changes meaning.

## Migration

None: no file changes meaning.

## Alternatives

- **Bones by `head-local` and `tail-local` (the bone's stored values) instead of the edit bones'.** Those are derived and read-only in RNA, and `head` and `tail` relative to the parent read badly. The edit bones' values are what a user sees and types, and what every exporter writes.
- **A bone's world or object-space matrix.** Exact but unreadable, and roll is lost in it to a reader.
- **Bone collections listing their bones.** That splits one bone's facts between lines; a bone naming its collections keeps write-back of one bone to one block.
- **The basis's positions written too.** They would duplicate `vertices` and could disagree with them; Blender keeps the two equal, so one is written.
- **Markers as a list on the scene (`markers [[1 "Intro"] …]`).** A list cannot hold a marker's camera, and markers are sub-objects with handles (section 7), so each is a line.

## Decision

Pending: the lead's, under S59.
