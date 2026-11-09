//! Level 0 arrays beyond meshes (spec, section 9; RFC 0006, Sau's decision S91, task 1d-06f): a
//! point cloud's `points`, a curves datablock's `points` and `curves`, their attributes, and a
//! Grease Pencil datablock's `drawing` sub-objects with the same lines, its `layer` and
//! `layer-group` tree with each layer's `frame` lines, and its `layer` attributes. Each array
//! is written inline or named in a data file of its own, as a mesh's (`level0`).
//!
//! A drawing may instead be a reference to another Grease Pencil datablock, `drawing @h {
//! reference @other }` (RFC 0008, S98, task 1d-06h), which holds nothing else; a reference that
//! reaches its own datablock is refused under `null-reference-cycle`.

use super::level0::{Counts, Elements};
use super::rig::handles;
use super::*;

/// The domains an attribute of each geometry takes.
const POINT: [&str; 1] = ["point"];
const CURVES: [&str; 2] = ["point", "curve"];
const LAYER: [&str; 1] = ["layer"];

/// The keyframe types a frame takes besides a keyframe, as Blender's `GreasePencilFrame` has them.
const KEYFRAME_TYPES: [&str; 6] = [
    "keyframe",
    "breakdown",
    "moving-hold",
    "extreme",
    "jitter",
    "generated",
];

/// A drawing that is a reference (RFC 0008): the datablock it is in, its handle, the datablock
/// it shows and where that is written.
pub(super) struct GpReference {
    from: String,
    drawing: String,
    to: String,
    file: FileId,
    span: Span,
}

impl Checker<'_> {
    /// A point cloud's or a curves datablock's level 0 lines (`kind`), after its other lines.
    pub(super) fn geometry(&mut self, file: FileId, kind: &str, handle: String, all: &[Attr<'_>]) {
        let owner = (kind.to_string(), handle);
        let domains: &[&'static str] = if kind == "curves" { &CURVES } else { &POINT };
        self.geometry_lines(file, &owner, all, "", "", domains, kind == "curves");
    }

    /// A geometry's `points`, `curves` (when `curves`) and `attribute` lines among `lines`; the
    /// data files' arrays are `dot` and the line's name (`@first.points`), and `slash` and an
    /// attribute's handle (`@first/@radius`).
    #[allow(clippy::too_many_arguments)]
    fn geometry_lines(
        &mut self,
        file: FileId,
        owner: &(String, String),
        lines: &[Attr<'_>],
        dot: &str,
        slash: &str,
        domains: &[&'static str],
        curves: bool,
    ) {
        let mut counts = Counts::default();
        let find = |name: &str| lines.iter().find(|a| a.name == name);
        if let Some(a) = find("points") {
            counts.point = self.array_line(
                file,
                a,
                owner,
                &format!("{dot}points"),
                Elements::Positions,
                None,
            );
        }
        if curves {
            match find("curves") {
                Some(a) => {
                    counts.curve = self.array_line(
                        file,
                        a,
                        owner,
                        &format!("{dot}curves"),
                        Elements::Sizes,
                        None,
                    );
                    // Inline, the sizes add up to the points.
                    if let [list @ Value::List(_)] = a.args.as_slice()
                        && let Some(points) = counts.point
                    {
                        let sum: Option<usize> = list
                            .items()
                            .iter()
                            .map(|v| v.as_word()?.parse::<usize>().ok())
                            .sum();
                        if let Some(sum) = sum
                            && sum != points
                        {
                            self.out(file).error(
                                codes::TYPE,
                                list.span(),
                                format!(
                                    "`curves` gives each curve's number of points, which add up to {sum}, but `points` has {points}"
                                ),
                            );
                        }
                    }
                }
                None => {
                    if let Some(a) = find("points")
                        && counts.point.is_some_and(|n| n > 0)
                    {
                        self.out(file).error(
                            codes::TYPE,
                            a.name_span,
                            "points are in curves: a `curves` line gives each curve's number of points, such as `curves [2 3]`",
                        );
                    }
                }
            }
        }
        for a in lines.iter().filter(|a| a.name == "attribute") {
            self.attribute_line(file, a, owner, &counts, domains, slash);
        }
    }

    /// A Grease Pencil datablock's drawings, layer tree and layer attributes (`all`, its lines;
    /// `handle`, its handle), after its other lines.
    pub(super) fn grease_pencil(&mut self, file: FileId, handle: String, all: &[Attr<'_>]) {
        let owner = ("grease-pencil".to_string(), handle);
        let drawings = handles(all, "drawing");
        for a in all.iter().filter(|a| a.name == "drawing") {
            if !self.sub_header(file, a, "drawing @first") {
                continue;
            }
            let own = a.args[0].text().to_string();
            let Some(block) = a.node else { continue };
            let lines = attrs(block);
            for b in &lines {
                if !matches!(
                    b.name.as_str(),
                    "points" | "curves" | "attribute" | "reference"
                ) {
                    self.unknown(
                        file,
                        b,
                        "a drawing",
                        &["points", "curves", "attribute", "reference"],
                        None,
                    );
                }
            }
            if self.drawing_reference(file, &owner.1, &own, &lines) {
                continue;
            }
            self.geometry_lines(
                file,
                &owner,
                &lines,
                &format!("{own}."),
                &format!("{own}/"),
                &CURVES,
                true,
            );
        }
        let mut layers = 0;
        for a in all
            .iter()
            .filter(|a| matches!(a.name.as_str(), "layer" | "layer-group"))
        {
            layers += self.layer_node(file, a, &drawings);
        }
        let counts = Counts {
            layer: Some(layers),
            ..Counts::default()
        };
        for a in all.iter().filter(|a| a.name == "attribute") {
            self.attribute_line(file, a, &owner, &counts, &LAYER, "");
        }
    }

    /// A drawing's `reference @other` line among `lines` (RFC 0008, S98): one handle of a
    /// `grease-pencil`, and no other array line beside it. True when the drawing is a reference,
    /// whose arrays are then not read. The datablock `gp`'s reference is kept for
    /// [`Checker::reference_cycles`].
    fn drawing_reference(
        &mut self,
        file: FileId,
        gp: &str,
        drawing: &str,
        lines: &[Attr<'_>],
    ) -> bool {
        let mut references = lines.iter().filter(|b| b.name == "reference");
        let Some(first) = references.next() else {
            return false;
        };
        for again in references {
            self.out(file).error(
                codes::REPEATED,
                again.name_span,
                format!("drawing `{drawing}` is a reference once: one `reference` line"),
            );
        }
        for b in lines
            .iter()
            .filter(|b| matches!(b.name.as_str(), "points" | "curves" | "attribute"))
        {
            self.out(file).error(
                codes::TYPE,
                b.name_span,
                format!(
                    "drawing `{drawing}` is a reference to another Grease Pencil, which holds its own drawings: a reference takes no `{}`",
                    b.name
                ),
            );
        }
        self.reference(file, first, "grease-pencil");
        if let [v] = first.args.as_slice()
            && first.props.is_empty()
            && let Some(to) = v.as_word().filter(|w| w.starts_with('@'))
        {
            self.gp_references.push(GpReference {
                from: gp.to_string(),
                drawing: drawing.to_string(),
                to: to.to_string(),
                file,
                span: v.span(),
            });
        }
        true
    }

    /// Each drawing reference that reaches its own datablock, directly or through other
    /// datablocks' references (RFC 0008, S98), as Blender's
    /// `BKE_grease_pencil_references_cyclic_check` refuses one: an error on each reference of
    /// the cycle, naming the datablocks in it.
    pub(super) fn reference_cycles(&mut self) {
        let references = std::mem::take(&mut self.gp_references);
        let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
        for r in &references {
            graph.entry(&r.from).or_default().push(&r.to);
        }
        // The shortest way from `from` back to `to`, the datablocks on it in order.
        let way = |from: &str, to: &str| -> Option<Vec<String>> {
            let mut came: HashMap<&str, &str> = HashMap::new();
            let mut todo = std::collections::VecDeque::from([from]);
            let mut seen = HashSet::from([from]);
            while let Some(at) = todo.pop_front() {
                if at == to {
                    let mut out = vec![at.to_string()];
                    let mut k = at;
                    while let Some(p) = came.get(k) {
                        out.push(p.to_string());
                        k = p;
                    }
                    out.reverse();
                    return Some(out);
                }
                for next in graph.get(at).into_iter().flatten() {
                    if seen.insert(next) {
                        came.insert(next, at);
                        todo.push_back(next);
                    }
                }
            }
            None
        };
        for r in &references {
            let Some(back) = way(&r.to, &r.from) else {
                continue;
            };
            let mut cycle = vec![r.from.clone()];
            cycle.extend(back);
            let shown = cycle.join(" → ");
            let message = if r.to == r.from {
                format!(
                    "drawing `{}` of `{}` is a reference to `{}` itself; Blender refuses a reference that reaches its own Grease Pencil",
                    r.drawing, r.from, r.from
                )
            } else {
                format!(
                    "drawing `{}` of `{}` is a reference to `{}`, which reaches `{}` again ({shown}); Blender refuses a reference that reaches its own Grease Pencil",
                    r.drawing, r.from, r.to, r.from
                )
            };
            self.out(r.file)
                .error(codes::REFERENCE_CYCLE, r.span, message);
        }
    }

    /// A `layer` or `layer-group` line and what it holds; returns how many layers it is or holds.
    fn layer_node(&mut self, file: FileId, a: &Attr<'_>, drawings: &[String]) -> usize {
        let group = a.name == "layer-group";
        let example = if group {
            "layer-group @effects \"Effects\""
        } else {
            "layer @lines \"Lines\""
        };
        if !self.sub_header(file, a, example) {
            return usize::from(!group);
        }
        let rna = if group {
            "GreasePencilLayerGroup"
        } else {
            "GreasePencilLayer"
        };
        let props: Vec<&Property> = self
            .schema
            .properties_of(rna)
            .into_iter()
            .filter(|p| !p.readonly && p.ty != "collection" && p.name != "name")
            .collect();
        let mut names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
        if group {
            names.extend(["layer", "layer-group"]);
        } else {
            names.push("frame");
        }
        let mut layers = usize::from(!group);
        let mut frames: HashSet<String> = HashSet::new();
        let Some(block) = a.node else { return layers };
        for b in attrs(block) {
            match (group, b.name.as_str()) {
                (true, "layer" | "layer-group") => layers += self.layer_node(file, &b, drawings),
                (false, "frame") => self.frame(file, &b, drawings, &mut frames),
                _ => match props.iter().find(|p| p.name == b.name) {
                    Some(p) => self.property(file, &b, p, false),
                    None => {
                        let what = if group { "a layer group" } else { "a layer" };
                        self.unknown(file, &b, what, &names, None)
                    }
                },
            }
        }
        layers
    }

    /// `frame <number> @drawing`, with `keyframe-type=` when it is not a keyframe: a drawing of
    /// the datablock, and each frame number once in its layer.
    fn frame(
        &mut self,
        file: FileId,
        b: &Attr<'_>,
        drawings: &[String],
        frames: &mut HashSet<String>,
    ) {
        let [number, drawing] = b.args.as_slice() else {
            return self.out(file).error(
                codes::TYPE,
                b.span,
                "a frame is `frame <number> @drawing`, such as `frame 1 @first`",
            );
        };
        let p = Property {
            ty: "int".into(),
            ..Property::any()
        };
        check_int(number, "frame", &p, &mut self.out(file));
        if !frames.insert(number.text().to_string()) {
            self.out(file).error(
                codes::REPEATED,
                number.span(),
                format!(
                    "frame {} is given twice; a layer shows one drawing from a frame on",
                    number.text()
                ),
            );
        }
        let h = drawing.text();
        if !drawings.iter().any(|d| d == h) {
            let near = closest(h, drawings.iter().map(String::as_str));
            self.out(file).error_fix(
                codes::HANDLE,
                drawing.span(),
                format!("`{h}` is none of this Grease Pencil's drawings"),
                near.map(|n| (format!("write `{n}`"), drawing.span(), n.to_string())),
            );
        }
        for (k, v) in &b.props {
            if k.text != "keyframe-type" {
                self.out(file).error(
                    codes::UNKNOWN_PROPERTY,
                    k.span,
                    format!("a frame takes `keyframe-type=`, not `{}=`", k.text),
                );
                continue;
            }
            let w = v.as_word().unwrap_or_default();
            if !KEYFRAME_TYPES.contains(&w) {
                self.out(file).error_fix(
                    codes::ENUM,
                    v.span(),
                    format!(
                        "`{w}` is no keyframe type: one of {}",
                        list_near(w, &KEYFRAME_TYPES)
                    ),
                    closest(w, KEYFRAME_TYPES.iter().copied())
                        .map(|t| (format!("write `{t}`"), v.span(), t.to_string())),
                );
            }
        }
    }
}
