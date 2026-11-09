//! Canonical order (spec, section 15; task 1c-01e): within each block, attributes in the
//! schema's definition order, and the same for a line's `key=value`s, then the formatter's layout.
//! Everything else stays as written: the order of datablocks in a file, of nodes in a tree, of
//! the items of a list, and of attributes the schema does not know. A comment on the lines just
//! above an attribute moves with it, as does one at its end.
//!
//! The order of a datablock's attributes is RNA's: the base struct's properties first, then each
//! struct's own, in definition order (`ID`'s `name`, then `Object`'s `data`, … `location` …).
//! Null's own forms (spec, section 8) stand where the RNA property they write stands: a scene's
//! `units` at `unit-settings`, its members at `collection`, `fps` and `fps-base` at `render`; an
//! object's `material` slots at `material-slots`; a mesh's `primitive` before `vertices`; a
//! material's `nodes` and `layout` at `node-tree`; a Grease Pencil's `drawing`s before its
//! layers, whose `layer` and `layer-group` lines keep their order, the layer tree's; a curves
//! datablock's `points` before its `curves` (RFC 0006). A node's settings come first and then its
//! inputs in the order its declaration lists them.

use null_check::{SHADER_TREE, Schema, TEXTURE_TREE, forms};
use null_syntax::ast::Node;
use null_syntax::{NodeKind, SyntaxElement, SyntaxNode, TokenKind, parse};

use crate::format::format;

/// `text` in canonical order and layout. Text that does not parse cleanly keeps its order (the
/// layout is still the formatter's), since moving lines past an error could change its meaning.
pub fn canonical(text: &str, schema: &Schema) -> String {
    let file = parse(text);
    if !file.errors().is_empty() {
        return format(text);
    }
    let mut out = String::with_capacity(text.len());
    for element in file.root().children() {
        match element {
            SyntaxElement::Node(n) if n.kind() == NodeKind::Node => {
                let order = top_order(schema, Node(n));
                rewrite_node(schema, n, &order, &mut out, true);
            }
            other => out.push_str(element_text(&other)),
        }
    }
    format(&out)
}

/// How a block's attributes are ordered.
#[derive(Clone, Debug, Default)]
struct Order {
    /// Names in canonical order; a name not listed goes after them all, as written.
    names: Vec<String>,
    /// Names that go last, after the names not listed, in this order: a group's `nodes` and its
    /// trailing `layout` (task 1d-02b).
    last: Vec<String>,
    /// The node tree a `nodes` block is, by RNA identifier, which orders each `node` line's
    /// settings and inputs by that tree's node of its kind (task 1d-02b).
    tree: Option<String>,
    /// Sub-object lines of a family (`modifier` of `Modifier`, `constraint` of `Constraint`,
    /// an f-curve's `modifier` of `FModifier`): each ordered by its kind's struct (task 1d-02b).
    families: Vec<(String, String)>,
    /// A component's block: each datablock it defines ordered as a top-level one (task 1d-02b).
    component: bool,
    /// What a child line's own block is ordered by, by the child's name.
    children: Vec<(String, Order)>,
    /// Whether this is a node tree's `nodes` block, whose `node` lines keep their order but are
    /// each ordered by their kind.
    nodes: bool,
    /// The pointers to a struct written as a block of its own (S70), by name, with the struct's
    /// RNA identifier: a block of one is ordered by that struct's order.
    pointers: Vec<(String, String)>,
    /// Names that stand where another stands, keeping their order with its lines as written: a
    /// Grease Pencil's `layer-group` with its `layer`s, the order of its layer tree (RFC 0006).
    same: Vec<(String, String)>,
}

impl Order {
    fn key(&self, name: &str) -> usize {
        if let Some((_, other)) = self.same.iter().find(|(n, _)| n == name) {
            return self.key(other);
        }
        // `last` first: a node group's `nodes` is also its struct's RNA property, which would
        // put it before the interface lines section 10 writes first (the 1d-08a review).
        if let Some(i) = self.last.iter().position(|n| n == name) {
            return usize::MAX - self.last.len() + i;
        }
        match self.names.iter().position(|n| n == name) {
            Some(i) => i,
            None => usize::MAX - self.last.len() - 1,
        }
    }

    fn child(&self, schema: &Schema, node: Node<'_>) -> Order {
        let name = node.name();
        let tree = self.tree.as_deref().unwrap_or(SHADER_TREE);
        if self.nodes && name == "node" {
            let kind = node.args().nth(1).and_then(|v| v.as_word()).unwrap_or("");
            return node_order(schema, tree, kind);
        }
        if self.nodes && name == "zone" {
            return zone_order(tree);
        }
        if let Some((_, base)) = self.families.iter().find(|(n, _)| *n == name) {
            let kind = node.args().nth(1).and_then(|v| v.as_word()).unwrap_or("");
            return match family_struct(schema, base, kind) {
                Some(rna) => struct_order(schema, &rna, kind),
                None => Order::default(),
            };
        }
        if self.component && schema.struct_by_kind(&name).is_some() {
            return top_order(schema, node);
        }
        if let Some((_, rna)) = self.pointers.iter().find(|(n, _)| *n == name) {
            let kind = schema
                .struct_by_rna(rna)
                .map_or(String::new(), |s| s.kind.clone());
            return struct_order(schema, rna, &kind);
        }
        self.children
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, o)| o.clone())
            .unwrap_or_default()
    }
}

fn element_text<'a>(e: &SyntaxElement<'a>) -> &'a str {
    match e {
        SyntaxElement::Node(n) => n.text(),
        SyntaxElement::Token(t) => t.text,
    }
}

/// The order of a top-level node: a datablock's by its kind, a project's (`units`, then its
/// scenes), anything else as written.
fn top_order(schema: &Schema, node: Node<'_>) -> Order {
    let kind = node.name();
    if kind == "component" {
        // Its parameters first, then its datablocks and placements as written (section 8).
        return Order {
            names: vec!["param".into()],
            component: true,
            ..Order::default()
        };
    }
    if kind == forms::PROJECT {
        return Order {
            names: vec!["units".into(), "scene".into()],
            children: vec![("units".into(), units_order(schema))],
            ..Order::default()
        };
    }
    let Some(s) = schema.struct_by_kind(&kind) else {
        return Order::default();
    };
    let mut rna = s.rna.clone();
    if kind == "light" {
        // A light's properties follow its type: `type sun` reads `SunLight`'s.
        let ty = node
            .children()
            .find(|c| c.name() == "type")
            .and_then(|c| c.args().next())
            .and_then(|v| v.as_word().map(str::to_string))
            .or_else(|| {
                node.entries().find_map(|e| match e {
                    null_syntax::ast::Entry::Prop { key, value } if key.text == "type" => {
                        value.as_word().map(str::to_string)
                    }
                    _ => None,
                })
            })
            .unwrap_or_else(|| "point".into());
        if let Some(sub) = schema.struct_by_kind(&format!("{ty}-light")) {
            rna = sub.rna.clone();
        }
    }
    struct_order(schema, &rna, &kind)
}

/// RNA's order for struct `rna`, base first, with Null's forms of `kind` where the property they
/// write stands, and the orders of the blocks its properties hold.
fn struct_order(schema: &Schema, rna: &str, kind: &str) -> Order {
    let mut chain = Vec::new();
    let mut at = schema.struct_by_rna(rna);
    while let Some(s) = at {
        chain.push(s);
        at = s.base.as_deref().and_then(|b| schema.struct_by_rna(b));
    }
    let mut names: Vec<String> = Vec::new();
    let mut children = Vec::new();
    let mut pointers = Vec::new();
    for s in chain.iter().rev() {
        // Each struct's own properties, then what an add-on attaches to it (`cycles`).
        let own = s.properties.iter();
        let attached = schema
            .properties_of(&s.rna)
            .into_iter()
            .filter(|p| !s.properties.iter().any(|q| std::ptr::eq(q, *p)))
            .filter(|p| {
                // properties_of also lists the bases' own; keep only those attached to `s`.
                !chain
                    .iter()
                    .any(|b| b.properties.iter().any(|q| std::ptr::eq(q, *p)))
            });
        for p in own.chain(attached) {
            if !names.contains(&p.name) {
                names.push(p.name.clone());
            }
            if p.ty == "pointer"
                && let Some(target) = p.target.as_deref()
                && !schema.is_datablock_struct(target)
            {
                pointers.push((p.name.clone(), target.to_string()));
            }
        }
    }
    // Null's forms, before the property each writes (spec, section 8).
    // A texture of any type (`noise`, `wood`) holds a texture node tree, as a material holds a
    // shader tree.
    let texture = rna == "Texture"
        || schema
            .struct_by_rna(rna)
            .and_then(|s| s.base.as_deref())
            .is_some_and(|b| b == "Texture");
    let forms: &[(&str, &str)] = match kind {
        "scene" => &[
            ("units", "unit-settings"),
            ("fps", "render"),
            ("fps-base", "render"),
            ("object", "collection"),
            ("collection", "collection"),
        ],
        "object" => &[("material", "material-slots")],
        "mesh" => &[("primitive", "vertices")],
        // A Grease Pencil's drawings before its layers (RFC 0006).
        "grease-pencil" => &[("drawing", "layers")],
        "material" => &[("nodes", "node-tree"), ("layout", "node-tree")],
        _ if texture => &[("nodes", "node-tree"), ("layout", "node-tree")],
        _ => &[],
    };
    // Each form just before its anchor, after the forms placed there already; a form that has
    // the anchor's own name (a scene's `collection` members) is already in place.
    for (form, anchor) in forms {
        if form == anchor {
            continue;
        }
        let at = names
            .iter()
            .position(|n| n == anchor)
            .unwrap_or(names.len());
        names.insert(at, (*form).to_string());
    }
    // A curves datablock's `points` before its `curves`, as section 9 writes them (RFC 0006),
    // where RNA's collections stand the other way.
    if kind == "curves"
        && let Some(from) = names.iter().position(|n| n == "points")
    {
        let points = names.remove(from);
        let at = names
            .iter()
            .position(|n| n == "curves")
            .unwrap_or(names.len());
        names.insert(at, points);
    }
    // A sub-object line (section 7's kinds) stands where RNA's collection of it stands: `modifier`
    // at `modifiers`, `constraint` at `constraints`, an action's `slot` at `slots` (task 1d-02b;
    // a named list, after 1d-02b's review, not a guess from plurals).
    for (line, collections) in SUB_OBJECT_COLLECTIONS {
        if names.iter().any(|n| n == line) {
            continue;
        }
        if let Some(at) = names.iter().position(|n| collections.contains(&n.as_str())) {
            names.insert(at, (*line).to_string());
        }
    }
    if kind == "scene" {
        children.push(("units".into(), units_order(schema)));
    }
    // A tree's `nodes`, for a datablock that has one: a material's, a world's and a light's is a
    // shader tree; a node group's is its own type's (section 10).
    let tree = match kind {
        "material" | "world" | "light" => Some(SHADER_TREE.to_string()),
        _ if texture => Some(TEXTURE_TREE.to_string()),
        _ if schema
            .struct_by_rna(rna)
            .and_then(|s| s.base.as_deref())
            .is_some_and(|b| b == "NodeTree") =>
        {
            Some(rna.to_string())
        }
        _ => None,
    };
    let mut last = Vec::new();
    if let Some(tree) = &tree {
        children.push((
            "nodes".into(),
            Order {
                nodes: true,
                tree: Some(tree.clone()),
                ..Order::default()
            },
        ));
        if kind != "material" && !texture {
            // A group's interface lines (`input`, `output`, `panel`) keep their order, then its
            // `nodes`, then its trailing `layout` (section 10).
            last = vec!["nodes".into(), "layout".into()];
        }
    }
    // Sub-objects of a family, each by its kind's struct; a pose bone's constraints too.
    let mut families = Vec::new();
    if kind == "object" {
        families = vec![
            ("modifier".to_string(), "Modifier".to_string()),
            ("constraint".to_string(), "Constraint".to_string()),
        ];
        let mut bone = struct_order(schema, "PoseBone", "pose-bone");
        bone.families = vec![("constraint".to_string(), "Constraint".to_string())];
        children.push(("bone".into(), bone));
    }
    // An action's slots, layers, strips and channels hold f-curves (section 11).
    if kind == "action" {
        let fcurves = fcurve_holder(schema);
        children.push(("slot".into(), fcurves.clone()));
        let strip = Order {
            children: vec![("channels".into(), fcurves)],
            ..Order::default()
        };
        let layer = Order {
            children: vec![("strip".into(), strip)],
            ..Order::default()
        };
        children.push(("layer".into(), layer));
    }
    // NLA tracks, on any datablock that can be animated: each strip by `NlaStrip`'s order.
    if schema.is_datablock_struct(rna) {
        children.push((
            "track".into(),
            Order {
                children: vec![(
                    "strip".into(),
                    struct_order(schema, "NlaStrip", "nla-strip"),
                )],
                ..Order::default()
            },
        ));
    }
    // A Grease Pencil's layer tree keeps its order: a group stands with the layers (RFC 0006).
    let same = if kind == "grease-pencil" {
        vec![("layer-group".to_string(), "layer".to_string())]
    } else {
        Vec::new()
    };
    Order {
        names,
        last,
        tree: None,
        families,
        component: false,
        children,
        nodes: false,
        pointers,
        same,
    }
}

/// What holds f-curves (a slot, a strip's `channels`): each `fcurve` by `FCurve`'s order, its
/// keys as written, its modifiers by their kind's struct.
fn fcurve_holder(schema: &Schema) -> Order {
    let mut fcurve = struct_order(schema, "FCurve", "fcurve");
    fcurve.families = vec![("modifier".to_string(), "FModifier".to_string())];
    // `key` lines keep their order (they are no property), after the f-curve's settings.
    Order {
        children: vec![("fcurve".into(), fcurve)],
        ..Order::default()
    }
}

/// The struct of `kind` in the family whose structs extend `base` (`bevel` in `Modifier`).
fn family_struct(schema: &Schema, base: &str, kind: &str) -> Option<String> {
    schema
        .structs
        .iter()
        .find(|s| s.base.as_deref() == Some(base) && s.kind == kind)
        .map(|s| s.rna.clone())
}

/// A zone's block (section 10): its items first, then the input node's lines as written, then
/// `output`, then the nodes inside it, each ordered by `tree`'s node of its kind.
fn zone_order(tree: &str) -> Order {
    Order {
        names: [
            "repeat-item",
            "state-item",
            "input-item",
            "main-item",
            "generation-item",
            "output-item",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        last: vec!["output".into(), "node".into(), "zone".into()],
        tree: Some(tree.to_string()),
        nodes: true,
        ..Order::default()
    }
}

/// Section 7's sub-object lines, each with the RNA collections that hold them, by Null name.
const SUB_OBJECT_COLLECTIONS: &[(&str, &[&str])] = &[
    ("modifier", &["modifiers"]),
    ("constraint", &["constraints"]),
    ("bone", &["bones"]),
    ("bone-collection", &["collections"]),
    ("shape-key", &["key-blocks"]),
    ("vertex-group", &["vertex-groups"]),
    ("uv-map", &["uv-layers"]),
    ("attribute", &["attributes"]),
    ("view-layer", &["view-layers"]),
    ("slot", &["slots"]),
    ("layer", &["layers"]),
    ("strip", &["strips"]),
    ("track", &["nla-tracks"]),
    ("marker", &["timeline-markers"]),
];

/// A `units` block's order: `UnitSettings`', with the short names standing for the full ones.
fn units_order(schema: &Schema) -> Order {
    // Null's own short form keeps the spec's order (S35: `units metric scale=1 length=m
    // rotation=deg`; the lead's call on the 1c-01e review), then the rest in RNA's.
    let mut names: Vec<String> = ["system", "scale", "scale-length", "length", "length-unit"]
        .iter()
        .chain(&["rotation", "system-rotation"])
        .map(|s| s.to_string())
        .collect();
    for p in schema.properties_of("UnitSettings") {
        if !names.contains(&p.name) {
            names.push(p.name.clone());
        }
    }
    Order {
        names,
        ..Order::default()
    }
}

/// A node's order: its settings first, then its inputs as its declaration lists them.
fn node_order(schema: &Schema, tree: &str, kind: &str) -> Order {
    let mut names = Vec::new();
    // A node is found in its tree: a kind may name a shader node and a compositor node both
    // (`invert`, 1b-03c's review), and a material's tree is a shader tree.
    if let Some(def) = schema.node(tree, kind) {
        // Its own settings (not `Node`'s, whose `color` is the editor's colour for the node, not
        // an input), then its inputs.
        if let Some(s) = def.rna.as_deref().and_then(|rna| schema.struct_by_rna(rna)) {
            names.extend(
                s.properties
                    .iter()
                    .filter(|p| !def.inputs.iter().any(|i| i.name == p.name))
                    .map(|p| p.name.clone()),
            );
        }
        names.extend(def.inputs.iter().map(|i| i.name.clone()));
    }
    Order {
        names,
        ..Order::default()
    }
}

/// Writes `node`'s text with its `key=value`s and its block's lines in `order`, each child
/// recursively in its own; without the node's end (line feed or `;`) unless `whole`.
fn rewrite_node(
    schema: &Schema,
    node: SyntaxNode<'_>,
    order: &Order,
    out: &mut String,
    whole: bool,
) {
    let end = if whole {
        node.span().end
    } else {
        Node(node).content_span().end
    };
    let elements: Vec<SyntaxElement<'_>> = node.children().collect();
    // The `key=value`s, sorted into their own slots.
    let props: Vec<&SyntaxElement<'_>> = elements
        .iter()
        .filter(|e| matches!(e, SyntaxElement::Node(n) if n.kind() == NodeKind::Prop))
        .collect();
    let mut sorted_props: Vec<&SyntaxElement<'_>> = props.clone();
    sorted_props.sort_by_key(|e| order.key(prop_key(e)));
    let mut next_prop = 0;
    for e in &elements {
        if e.span().start >= end {
            break;
        }
        match e {
            SyntaxElement::Node(n) if n.kind() == NodeKind::Prop => {
                out.push_str(element_text(sorted_props[next_prop]));
                next_prop += 1;
            }
            SyntaxElement::Node(n) if n.kind() == NodeKind::Block => {
                rewrite_block(schema, *n, order, out);
            }
            other => out.push_str(element_text(other)),
        }
    }
}

fn prop_key<'a>(e: &SyntaxElement<'a>) -> &'a str {
    match e {
        SyntaxElement::Node(n) => n
            .child_tokens()
            .find(|t| t.kind.is_scalar())
            .map_or("", |t| t.text),
        SyntaxElement::Token(_) => "",
    }
}

/// One attribute line in a block, with the comment lines just above it.
struct Unit<'a> {
    node: SyntaxNode<'a>,
    /// Where it starts: its first comment line's, or its own.
    start: usize,
    /// Where its content ends, before its line feed or `;`.
    end: usize,
}

fn rewrite_block(schema: &Schema, block: SyntaxNode<'_>, order: &Order, out: &mut String) {
    let elements: Vec<SyntaxElement<'_>> = block.children().collect();
    let mut units: Vec<Unit<'_>> = Vec::new();
    let mut comment_start: Option<usize> = None;
    let mut after_newline = false;
    for e in &elements {
        match e {
            // Only a comment that begins a line goes with the line below it; one after `{` or
            // after a line's `;` stays where it is (the 1c-01e review).
            SyntaxElement::Token(t)
                if matches!(t.kind, TokenKind::LineComment | TokenKind::BlockComment) =>
            {
                if after_newline || comment_start.is_some() {
                    comment_start.get_or_insert(t.span.start);
                }
                after_newline = false;
                continue;
            }
            SyntaxElement::Token(t) if t.kind == TokenKind::Newline => {
                // A blank line between a comment and the next line detaches the comment.
                if after_newline {
                    comment_start = None;
                }
                after_newline = true;
                continue;
            }
            SyntaxElement::Token(t) if t.kind == TokenKind::Whitespace => continue,
            SyntaxElement::Node(n) if n.kind() == NodeKind::Node => {
                let start = comment_start.take().unwrap_or(n.span().start);
                units.push(Unit {
                    node: *n,
                    start,
                    end: Node(*n).content_span().end,
                });
                // A line holds its own line feed, so the next starts a line.
                after_newline = n.text().ends_with('\n');
                continue;
            }
            _ => comment_start = None,
        }
        after_newline = false;
    }
    let mut sorted: Vec<usize> = (0..units.len()).collect();
    // A tree's `nodes` block keeps its lines' order; a zone's sorts its items, input lines,
    // `output` and nodes (stable, so the nodes keep theirs).
    if !order.nodes || !order.names.is_empty() || !order.last.is_empty() {
        sorted.sort_by_key(|&i| {
            let n = Node(units[i].node);
            if n.is_commented_out() {
                usize::MAX
            } else {
                order.key(&n.name())
            }
        });
    }
    let base = block.span().start;
    let text = block.text();
    let mut at = base;
    let mut needs_break = false;
    for (slot, unit) in units.iter().enumerate() {
        // No blank line between the lines of a block: their order is the schema's, not a grouping
        // of the author's (blank lines between datablocks stay, at most one, section 15).
        let between = &no_blank_lines(&text[at - base..unit.start - base]);
        // A line that ends in a `//` comment cannot be followed on its line by a `;` and another
        // line: the comment would take them in. Its `;` becomes a line feed.
        if needs_break && !between.contains('\n') {
            out.push('\n');
            out.push_str(between.trim_start().trim_start_matches(';'));
        } else {
            out.push_str(between);
        }
        let chosen = &units[sorted[slot]];
        // The comment lines above it, then the line itself, recursively ordered.
        out.push_str(&text[chosen.start - base..chosen.node.span().start - base]);
        let child = Node(chosen.node);
        let child_order = order.child(schema, child);
        rewrite_node(schema, chosen.node, &child_order, out, false);
        needs_break = ends_in_line_comment(chosen.node);
        at = unit.end;
    }
    if needs_break && !text[at - base..].contains('\n') {
        let rest = &text[at - base..];
        out.push('\n');
        out.push_str(rest.trim_start().trim_start_matches(';'));
        return;
    }
    out.push_str(&text[at - base..]);
}

/// Whether `node`'s content ends in a `//` comment.
fn ends_in_line_comment(node: SyntaxNode<'_>) -> bool {
    let end = Node(node).content_span().end;
    node.children().any(|e| {
        matches!(e, SyntaxElement::Token(t) if t.kind == TokenKind::LineComment && t.span.end == end)
    })
}

/// `text` with every run of line feeds, and the whitespace between them, made one line feed.
fn no_blank_lines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending = String::new();
    let mut newline = false;
    for c in text.chars() {
        match c {
            '\n' => {
                if !newline {
                    out.push('\n');
                }
                newline = true;
                pending.clear();
            }
            // Any whitespace the lexer reads as such, a vertical tab's included (task 1d-02d's
            // fuzzing found one keeping a blank line alive for one pass).
            c if newline && c.is_whitespace() => pending.push(c),
            _ => {
                out.push_str(&pending);
                pending.clear();
                newline = false;
                out.push(c);
            }
        }
    }
    out.push_str(&pending);
    out
}
