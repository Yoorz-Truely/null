# 0006: Level 0 arrays for curves, point clouds and Grease Pencil drawings

- Status: proposed
- Date: November 9, 2026
- Decided by: the lead under the user's standing instruction (Sau's decision S59)
- Affects: geometry as source (section 9), identity (section 7), canonical form
- Null format: unchanged

## Problem

Section 9 gives a mesh's level 0 data its forms, and ends: "Shape keys, curves, point clouds and Grease Pencil drawings take the same forms for their own arrays (1d-06f)." RFC 0005 gave shape keys theirs (S88). Curves, point clouds and Grease Pencil drawings still have none. A file therefore cannot write their geometry, Sau cannot apply or print it, and a `.blend` that holds them loses it in conversion (section 19, task 1d-08b).

All three keep their geometry as Blender 5.3 keeps a mesh's: attributes on domains, with a few arrays that are their structure.
- **A point cloud** (`PointCloud`) has one domain, `point`, its positions the `position` attribute.
- **Curves** (`Curves`, the hair datablock) has two, `point` and `curve`: each curve a run of consecutive points, its `curve_offsets` saying where each run starts. A curve's type, resolution, cyclic flag, handles, NURBS order and weights, radius and tilt are all attributes by Blender's names (`curve_type`, `resolution`, `cyclic`, `handle_left`, `nurbs_order`, `radius`, `tilt` and the rest).
- **A Grease Pencil datablock** (`GreasePencil`) holds drawings, each a curves geometry of its own (`point` and `curve` domains, as `Curves`), and a tree of layers and layer groups. Each layer holds frames, and each frame shows one drawing from a frame number on, in a keyframe type. One drawing may be shown by several frames. The datablock's own attributes are on its `layer` domain.

## Design

The same forms as a mesh's (section 9): inline lists up to 2,000 elements, a data file of its own past that with `count=` and `bounds=`, and `attribute @h <domain> <type> "Name" [values]` for any other attribute, with Blender's internal attributes (names starting `.`) never written. Each array is one line in its owner's block, in the order below.

### Point clouds (section 9, after shape keys)

> **A point cloud's level 0 data** is:
> - `points [[x y z] …]`: each point's position (the `position` attribute), lengths as `location` is written;
> - `attribute @h point <type> "Name" [values]`: any other attribute, `radius` included, on its one domain, `point`.

```null
point-cloud @dust "Dust" {
  points [[0m 0m 0m] [0.1m 0m 0m] [0m 0.1m 0.2m]]
  attribute @radius point float "radius" [0.01 0.02 0.01]
}
```

### Curves

> **A curves datablock's level 0 data** (`curves`, Blender's hair) is:
> - `points [[x y z] …]`: each point's position;
> - `curves [n …]`: each curve's number of points, in order. Its points are the next `n` of `points`. Blender keeps this as `curve_offsets`, each curve's first point, which the sizes give;
> - `attribute @h <domain> <type> "Name" [values]`: any other attribute, on `point` or `curve`. A curve's type, resolution, cyclic flag, handles and handle types, NURBS order, weights and knots mode, radius and tilt are attributes by Blender's names. A built-in name that is not kebab-case keeps its quotes (`attribute @curve-type curve int8 "curve_type" [0 0]`).

```null
curves @fur "Fur" {
  points [[0m 0m 0m] [0m 0m 0.1m] [0.1m 0m 0m] [0.1m 0m 0.1m] [0.1m 0m 0.2m]]
  curves [2 3]
  attribute @radius point float "radius" [0.002 0.001 0.002 0.0015 0.001]
}
```

The checker holds `curves`' sum to `points`' count, each `n` to at least one, and each attribute to its domain's count: points, or the number of curves.

### Grease Pencil drawings and layers

> **A Grease Pencil datablock's level 0 data** is its drawings, each `drawing @h { … }`, a sub-object, in Blender's order. A drawing holds the same lines as a curves datablock: `points`, `curves` and `attribute` on `point` or `curve`. A stroke's material is its `material_index` attribute, its colours, opacities and widths their attributes by Blender's names (`vertex_color`, `opacity`, `radius`, `fill_color` and the rest).
>
> Its layers are `layer @h "Name" { … }` and `layer-group @h "Name" { … }`, sub-objects in the order of Blender's layer tree, a group holding its own layers and groups in its block. A layer holds:
> - `frame <number> @drawing`: from that frame on, the layer shows that drawing, with `keyframe-type=` when it is not a keyframe (`breakdown`, `moving-hold`, `extreme`, `jitter`, `generated`). One drawing may be shown by several frames, on any layers;
> - `GreasePencilLayer`'s settings by their Null names: `opacity`, `blend-mode`, `use-lights`, `translation`, `rotation`, `scale`, `parent @object` and the rest.
>
> A layer group holds `GreasePencilLayerGroup`'s settings (`is-expanded`, `color-tag`) and its children. The datablock's own attributes are `attribute @h layer <type> "Name" [values]`, one value per layer in the tree's order.

```null
grease-pencil @sketch "Sketch" {
  drawing @first {
    points [[0m 0m 0m] [0.5m 0m 0m] [0.5m 0m 0.5m]]
    curves [3]
    attribute @radius point float "radius" [0.01 0.01 0.01]
  }
  drawing @second {
    points [[0m 0m 0m] [0.5m 0m 0.25m]]
    curves [2]
  }
  layer @lines "Lines" {
    frame 1 @first
    frame 12 @second keyframe-type=breakdown
    frame 24 @first
    opacity 0.8
  }
  layer-group @effects "Effects" {
    layer @glow "Glow" {
      frame 1 @first
      use-lights off
    }
  }
}
```

**Identity (section 7's table).** A drawing has no name in Blender: its handle is kept in `.sau/ids.noll` by its index in the datablock's drawings, as a shape key's is by its `uid`. Layers and layer groups are known by their names, as the table already says.

**Data files.** A drawing's array past 2,000 elements goes in a file of its own as any array, named after the datablock, the drawing and the array (`sketch.first.points.noll`), whose `array` node names the drawing's path: `array grease-pencil @sketch @first points [ … ]`, and an attribute's `array grease-pencil @sketch @first/@radius [ … ]`.

## Compatibility

No file changes meaning. The three kinds' blocks gain lines that no file writes today, since none could be applied. The Null format stays at 1.

## Not covered

- A curve datablock's splines (`curve`, `surface`, `text-curve`: Blender's legacy curves) are not attribute arrays and keep their forms over RNA.
- A Grease Pencil drawing reference (a drawing from another Grease Pencil datablock, `GreasePencilDrawingReference`) is left to a later RFC; a datablock that holds one says so in conversion.
