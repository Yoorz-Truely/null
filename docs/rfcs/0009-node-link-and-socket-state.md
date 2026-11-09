# 0009: Muted links, hidden sockets and the value under a link

- Status: accepted (S95)
- Date: November 9, 2026
- Decided by: the lead under the user's standing instruction (Sau's decision S59), recorded as Sau's decision S95
- Affects: forms over RNA, canonical form, diagnostics
- Null format: unchanged

## Problem

Section 10 writes a node's inputs as values or links and its place and size in the tree's `layout` block. Three pieces of node state that Blender saves have no form, and Sau's printer reports each as a problem rather than writing it (Sau's decision S79 (4)). So `sau convert` loses them, and a `.blend` that holds them does not come back the same (task 1d-08b):

- **A muted link** (`NodeLink.is_muted`, DNA's `NODE_LINK_MUTED`). A muted link stays in the tree but carries nothing: the input takes its own value, as if unlinked. Users mute links to compare a tree with and without a branch (Ctrl+Alt+click in the editor). It changes what the tree computes.
- **A hidden socket** (`NodeSocket.hide`, DNA's `SOCK_HIDDEN`). Ctrl+H on a node ("Toggle Hidden Node Sockets") hides every socket that is not linked, inputs and outputs alike, and the editor draws the node without them. It changes nothing a render or an evaluation sees: it is the editor's, as a node's place is.
- **The value under a link** (an input's `default_value` while it is linked). Blender keeps the socket's own value when a link comes in, and it is the value again when the link goes or is muted. Section 10 writes a linked input as its link alone, so the value under it is lost.

From a principled BSDF whose base colour comes from a ramp through a muted link, and a hue node with its unused sockets hidden:

```null
node @bsdf bsdf-principled {
  base-color @ramp.color        // the link is muted: today this reads as live
  roughness 0.6
}
node @tint hue-saturation { color @mix.color }   // its Fac input and nothing else is hidden: lost
```

## Design

### Muted links

A link's line, or each link of a list, takes `mute=`:

> **A muted link** carries nothing: the input takes its own value, as if unlinked. It is written `mute=on` after the link, `base-color @ramp.color mute=on`. For a socket that takes several links, `mute=` is a list of `on` and `off`, one for each link in the list's order: `geometry [@a.geometry @b.geometry] mute=[off on]`. A link written with no `mute=` is live, and a printer leaves `mute=off` out.

The checker refuses `mute=` on a line that names no link, and a list of another length than the line's links.

### The value under a link

A linked input takes `value=`, the socket's own value:

> **The value under a link.** A linked input keeps its own value, which it takes when the link goes or is muted. It is written `value=` after the link, as the socket's value is written without one: `roughness @noise.fac value=0.6`. Left out, it is the socket's default. A socket with no value of its own (geometry, shader, bundle, closure) and a socket that takes several links take no `value=`.

A printer writes `value=` only where the value is not the socket's default, so a link made in the editor on an untouched socket prints as today. A muted link usually comes with one: `base-color @ramp.color mute=on value=[0.8 0.1 0.1 1]`.

### Hidden sockets

A node's `layout` line takes `hide-inputs=` and `hide-outputs=`, lists of the node's socket names as its input lines name them, with a group node's or a group's own sockets by their interface handles less the `@`:

> **Hidden sockets** are the editor's, as a node's place is (S45), and are written on its layout line: `@tint [-100 0] hide-inputs=[fac] hide-outputs=[]`. A list left out is empty. A linked socket cannot be hidden: Blender's own hide refuses it (`rna_NodeSocket_hide_set`), so the checker refuses a hidden input that its node's block links. Each name is one of the node kind's inputs (or outputs), read as the node's input lines are.

Keeping them in `layout` means Ctrl+H in the editor changes only the layout block, as dragging a node does, and reading a tree's meaning never needs them. A zone's layout line, which places two nodes, takes neither: a zone node's own state has no form (S80, RFC 0002).

### What the printer does

It writes all three where Blender holds them and stops reporting them as having no v0 form. Sau applies `mute=` through `NodeLink.is_muted`, `value=` through the socket's `default_value` before its link is made, and `hide-*=` through `NodeSocket.hide`, after the links, since Blender refuses to hide a linked socket.

## What breaks

Nothing that Sau writes or any project holds:
- **Layout lines.** The checker refuses `hide-inputs=` and `hide-outputs=` on a layout line today ("a layout line takes `width=`, and `height=` for a frame"), so no v0 file holds them.
- **Input lines.** Today the checker takes any key on an input line that names a link and ignores it (probed: `base-color @ramp.color mute=on` and `roughness @ramp.alpha value=0.6` check clean); a line with a value already refuses keys ("`roughness` takes no properties"). That is a gap of its own, so a hand-written v0 file could hold `mute=on` with no effect, and under this proposal it would mute. Sau's printer and write-back never write a key there. No file of Cosmósis's corpus or Null's slice has one (each is checked by the same rule). With this proposal the checker warns at any other key on a link line, naming it (`null-unknown-property`, a warning): a v0 file that checks clean today still does, and Sau never applies such a key.

Write-back's smallest edit (section 17) gains three things it may rewrite in place: a link line's `mute=` and `value=`, and a layout line's lists.

## Migration

None.

## Alternatives

- **Hidden sockets on the input lines** (`fac 1 hide=on`). This mixes the editor's state into the lines that say what a tree computes, which S45 keeps apart. An output has no line, so it would need a second form anyway. And Ctrl+H on a node would rewrite every input line of it.
- **A block per socket** (`socket fac { hide; value 1 }`). Sockets are not sub-objects (section 7), and a block would make the common case, a link with nothing else, longer for every node.
- **Links as lines of their own** (`link @ramp.color -> @bsdf.base-color { mute }`). Section 10 writes a link as an input's value; a second way to write one would leave two forms for one thing. `mute=` reads as a link's own setting where the link is written.
- **`mute=` as a list of the muted links** (`mute=[@b.geometry]`). It names each link twice and breaks when one link appears twice in a list. A list of `on` and `off` follows the line's own order.
- **Leaving the value under a link out for good.** It is lost on every `.blend` round trip, and it decides what a muted link's input computes, so a muted link without it would change meaning when written and read back.

## Decision

Accepted by the lead as Sau's decision S95, November 9, 2026, with one amendment folded in above: an unknown key on an input line that names a link, which v0 takes and ignores, becomes a warning naming it, not an error, so no v0 file that checks clean today fails; Sau never applies such a key.
