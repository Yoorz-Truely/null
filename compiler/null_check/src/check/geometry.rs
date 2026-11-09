//! Level 0 arrays beyond meshes (spec, section 9; RFC 0006, Sau's decision S91, task 1d-06f): a
//! point cloud's `points`, a curves datablock's `points` and `curves`, their attributes, and a
//! Grease Pencil datablock's `drawing` sub-objects with the same lines, its `layer` and
//! `layer-group` tree with each layer's `frame` lines, and its `layer` attributes. Each array
//! is written inline or named in a data file of its own, as a mesh's (`level0`).

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
                if !matches!(b.name.as_str(), "points" | "curves" | "attribute") {
                    self.unknown(
                        file,
                        b,
                        "a drawing",
                        &["points", "curves", "attribute"],
                        None,
                    );
                }
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
