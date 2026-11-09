# 0002: What a zone's name and block say about its two nodes

- Status: accepted
- Date: November 9, 2026
- Decided by: the lead under the user's standing instruction (Sau's decision S59), recorded as Sau's decision S82
- Affects: identity, forms over RNA
- Null format: unchanged

## Problem

Section 10 writes a zone as one line, `zone @h <kind> "Name" { … }` (S71). In Blender a zone is two nodes, an input node and the output node it is paired with, and each has a name of its own: Blender names the pair it makes "Repeat Input" and "Repeat Output". The spec says what a node's quoted name is ("the node's Blender name"). It does not say which Blender name a zone's quoted name is, or how it reaches the second node. Sau needed an answer to apply and print the corpus's zones (task 1d-07c), and the lead settled it for Sau as decision S80.

Three more questions about a zone are open in v0's text:
- **What a zone's nodes may carry.** A node's frame, size, label, mute and colour are written on its node line and its layout line. 1d-07b's checker (S79) refuses `width=` and `height=` on a zone's layout line. A zone's line has no word for the rest.
- **Which nodes are inside a zone.** The spec says the block holds "the nodes inside the zone". Blender does not store that: it works out which nodes a zone holds from the links between them.
- **The width a zone's two positions place its nodes at.** The spec does not say.

The corpus's zones show the first question (`node-trees.sau`, 1d-08a):

```null
zone @loop repeat "Repeat" { … }                     // which node is "Repeat"?
zone @each foreach-geometry-element "For Each" { … } // Blender's own would be "For Each Geometry Element Input"
```

## Design

The zone paragraph of section 10 gains these words, after "Its kind is the pair's (…)":

> Its name N names both nodes: the input node is `N Input` and the output node `N Output`, as Blender names a zone it makes, so `zone @repeat repeat "Repeat"` is Blender's own. A pair named otherwise, and a zone node's own width, frame, label, mute and colour, have no form in v0 (Sau's decision S80): a printer reports them rather than writing them. The nodes inside a zone's block are those Blender counts as inside it from the links; writing a node inside or outside the block does not move it, and a printer writes each node in the block of the innermost zone holding it.

And to "Inside the zone's block, `@loop.geometry` is …":

> … at any depth, a zone nested in it included.

The "Layout is kept apart" paragraph gains, after "A zone's line holds two positions, its input node's and then its output node's":

> each at its node kind's default width.

Nothing else changes. Before and after, the same file means the same thing:

```null
geometry @stack-up "Stack Up" {
  nodes {
    zone @loop repeat "Repeat" {          // Blender's "Repeat Input" and "Repeat Output"
      repeat-item @geometry geometry "Geometry"
      geometry @in.geometry
      output { geometry @join.geometry }
      node @join join-geometry { geometry [@loop.geometry @lift.geometry] }
    }
  }
  layout { @loop [-300 0] [400 0] }       // both nodes at their kinds' default widths
}
```

## What breaks

Nothing.
- No file of Cosmósis's corpus changes meaning. Its zones `"Repeat"`, `"Simulation"` and `"For Each"` already apply this way in Sau (1d-07c, S80).
- No file of Null's slice project changes meaning either: it has no zones.
- The checker needs nothing new for the name, which it reads as free text. The refusal of `width=` and `height=` on a zone's layout line comes with 1d-07b's checker (S79), now on batch 3b.

## Migration

None.

## Alternatives

- **The quoted name is the input node's name, and the output node's is free.** This needs a second name in the block (`output "Repeat Output" { … }`), so a new form, for a difference almost no one makes. And Blender's own zone would print as `"Repeat Input"`, so its line would read differently from its label, which shows "Repeat".
- **Two names on the line** (`zone @loop repeat "Repeat Input" "Repeat Output"`). The same gain, a longer line, and a new form.
- **The zone's block decides membership** (a node written inside it is moved into it). Blender has no such field: membership follows from the links, and a node linked from outside a zone cannot be inside it. Apply would have to rewire links to obey the text's nesting, which changes the tree's meaning behind the writer's back.
- **Forms for a zone node's frame, size and settings now.** These are rare in practice. They are better added together with the other forms v0 leaves out (1d-07l, S79 (4)) than one at a time.

## Decision

Accepted on November 9, 2026, by the lead under the user's standing instruction (Sau's decision S59), recorded as Sau's decision S82. It writes into the spec what Sau already does under S80 (task 1d-07c), so no file changes meaning. The wording above is in section 10 (task 1d-07m).
