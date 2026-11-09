//! A mesh's level 0 arrays (spec, section 9, "Level 0: attribute arrays"; task 1d-06b): `edges`,
//! `attribute`, `uv-map` and `vertex-group` lines beside the slice's `vertices` and `polygons`,
//! each written inline or named in a data file of its own with a `count=` and `bounds=` summary,
//! which is held to the file (Sau's decision S74 (1)). A data file holds one `array` node, read
//! only through the line that names it.

use null_value::{Dimension, Scene, read_in};

use super::*;
use crate::level0::{DATA_TYPES, DOMAINS, element};
use crate::values::{check_bool, check_float, check_float_array, check_int};

/// What one of a mesh's arrays holds, for reading its elements wherever they are written.
#[derive(Clone, Debug)]
pub(super) enum Elements {
    /// Positions: three lengths each.
    Positions,
    /// Polygons: three or more vertex indices each, below the vertex count.
    Polygons(Option<usize>),
    /// Edges: two vertex indices each.
    Edges(Option<usize>),
    /// An attribute's or a UV map's values, one of a data type each.
    Values(Box<Property>),
    /// A vertex group's weights: `[index weight]`, the index below the vertex count.
    Weights(Option<usize>),
}

/// A line that names a data file: what it expects the file to hold.
#[derive(Clone, Debug)]
pub(super) struct ArrayRef {
    pub file: FileId,
    pub span: Span,
    pub owner: (String, String),
    pub array: String,
    pub elements: Elements,
    pub count: Option<usize>,
    /// The summary's bounds, as read: (smallest, largest) of each component.
    pub bounds: Option<(Vec<f64>, Vec<f64>)>,
    pub bounds_span: Option<Span>,
    /// The domain size the array must have, where the mesh says it.
    pub expected: Option<(usize, &'static str)>,
    /// The data file found for it, once one is: a second is an error.
    pub found: Option<(FileId, Span)>,
}

/// The sizes of a mesh's domains, where its lines say them.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Counts {
    pub point: Option<usize>,
    pub edge: Option<usize>,
    pub face: Option<usize>,
    pub corner: Option<usize>,
}

impl Counts {
    fn of(&self, domain: &str) -> Option<usize> {
        match domain {
            "point" => self.point,
            "edge" => self.edge,
            "face" => self.face,
            "corner" => self.corner,
            _ => None,
        }
    }
}

fn prop<'a>(a: &'a Attr<'a>, key: &str) -> Option<&'a Value<'a>> {
    a.props.iter().find(|(k, _)| k.text == key).map(|(_, v)| v)
}

impl Checker<'_> {
    /// Every level 0 line of a mesh (`all`, the mesh's attributes) after its `vertices` and
    /// `polygons`: their domain sizes first, then each line against them.
    pub(super) fn level0(&mut self, file: FileId, owner: (String, String), all: &[Attr<'_>]) {
        let mut counts = Counts::default();
        let find = |name: &str| all.iter().find(|a| a.name == name);
        if let Some(a) = find("vertices") {
            counts.point = self.array_line(file, a, &owner, "vertices", Elements::Positions, None);
        }
        if let Some(a) = find("polygons")
            && prop(a, "file").is_some()
        {
            counts.face = self.array_line(
                file,
                a,
                &owner,
                "polygons",
                Elements::Polygons(counts.point),
                None,
            );
        } else if let Some(a) = find("polygons")
            && let [list @ Value::List(_)] = a.args.as_slice()
        {
            let polys = list.items();
            counts.face = Some(polys.len());
            counts.corner = Some(polys.iter().map(|p| p.items().len()).sum());
        }
        if let Some(a) = find("edges") {
            counts.edge = self.array_line(
                file,
                a,
                &owner,
                "edges",
                Elements::Edges(counts.point),
                None,
            );
        }
        for a in all {
            match a.name.as_str() {
                "attribute" => self.attribute_line(file, a, &owner, &counts),
                "uv-map" => self.uv_map(file, a, &owner, &counts),
                "vertex-group" => self.vertex_group(file, a, &counts),
                "shape-key" => self.shape_key_positions(file, a, &owner, &counts),
                _ => {}
            }
        }
    }

    /// One array line, its list inline or named in a data file (`file=`, `count=`, `bounds=`).
    /// Returns its element count where known.
    pub(super) fn array_line(
        &mut self,
        file: FileId,
        a: &Attr<'_>,
        owner: &(String, String),
        array: &str,
        elements: Elements,
        expected: Option<(usize, &'static str)>,
    ) -> Option<usize> {
        if let Some(path) = prop(a, "file") {
            if path.as_string().is_none() {
                self.out(file).error(
                    codes::TYPE,
                    path.span(),
                    format!(
                        "`file=` names a data file as a string, such as \"{}.{array}.noll\"",
                        owner.1.trim_start_matches('@')
                    ),
                );
                return None;
            }
            let count = match prop(a, "count") {
                Some(v) => {
                    let p = Property {
                        ty: "int".into(),
                        range: Some(crate::schema::Range {
                            hard: [0.0, f64::from(i32::MAX)],
                        }),
                        ..Property::any()
                    };
                    let n = check_int(v, "count", &p, &mut self.out(file));
                    n.map(|n| n as usize)
                }
                None => {
                    self.out(file).error(
                        codes::TYPE,
                        a.name_span,
                        format!("a data file's line gives its `count=`, the number of {array} it holds (S74)"),
                    );
                    None
                }
            };
            let bounds = prop(a, "bounds").and_then(|v| self.bounds(file, v, &elements));
            self.refs.push(ArrayRef {
                file,
                span: a.name_span,
                owner: owner.clone(),
                array: array.to_string(),
                elements,
                count,
                bounds,
                bounds_span: prop(a, "bounds").map(|v| v.span()),
                expected,
                found: None,
            });
            return count;
        }
        let [list @ Value::List(_)] = a.args.as_slice() else {
            // `vertices`' and `polygons`' own messages are the slice's.
            return None;
        };
        let items = list.items();
        if let Some((n, domain)) = expected
            && items.len() != n
        {
            self.out(file).error(
                codes::TYPE,
                list.span(),
                format!(
                    "`{array}` has one value per {domain}: {n}, not {}",
                    items.len()
                ),
            );
        }
        for item in &items {
            self.element(file, item, array, &elements);
        }
        Some(items.len())
    }

    /// Checks one element; returns its components as numbers, for bounds.
    fn element(
        &mut self,
        file: FileId,
        v: &Value<'_>,
        name: &str,
        elements: &Elements,
    ) -> Option<Vec<f64>> {
        let index = |max: Option<usize>| Property {
            ty: "int".into(),
            range: Some(crate::schema::Range {
                hard: [
                    0.0,
                    max.map_or(f64::from(i32::MAX), |m| m.saturating_sub(1) as f64),
                ],
            }),
            ..Property::any()
        };
        let mut out = self.out(file);
        match elements {
            Elements::Positions => {
                let co = Property {
                    unit: Some("length".into()),
                    ..Property::any()
                };
                check_float_array(v, 3, name, Dimension::Length, &co, &mut out);
                numbers(v, Dimension::Length, out.scale)
            }
            Elements::Polygons(points) | Elements::Edges(points) => {
                let want = if matches!(elements, Elements::Edges(_)) {
                    2
                } else {
                    3
                };
                let items = v.items();
                let shaped = matches!(v, Value::List(_))
                    && (items.len() == want || (want == 3 && items.len() >= 3));
                if !shaped {
                    out.error(
                        codes::TYPE,
                        v.span(),
                        if want == 2 {
                            "an edge is two vertex indices, such as [0 1]".to_string()
                        } else {
                            "a polygon is three or more vertex indices, such as [0 1 2]".to_string()
                        },
                    );
                    return None;
                }
                let p = index(*points);
                for i in &items {
                    check_int(i, name, &p, &mut out);
                }
                numbers(v, Dimension::None, 1.0)
            }
            Elements::Weights(points) => {
                let items = v.items();
                let [i, w] = items.as_slice() else {
                    out.error(
                        codes::TYPE,
                        v.span(),
                        "a weight is a vertex index and its weight, such as [3 0.5]",
                    );
                    return None;
                };
                check_int(i, name, &index(*points), &mut out);
                let unit = Property {
                    range: Some(crate::schema::Range { hard: [0.0, 1.0] }),
                    ..Property::any()
                };
                check_float(w, name, Dimension::None, &unit, &mut out);
                numbers(v, Dimension::None, 1.0)
            }
            Elements::Values(p) => {
                let dims = p.dimensions();
                match (p.ty.as_str(), dims.as_slice()) {
                    ("bool", _) => {
                        check_bool(std::slice::from_ref(v), name, v.span(), &mut out);
                        Some(vec![f64::from(u8::from(v.as_word() != Some("off")))])
                    }
                    ("string", _) => {
                        if v.as_string().is_none() {
                            out.error(
                                codes::TYPE,
                                v.span(),
                                format!("`{name}` takes a string, such as \"text\""),
                            );
                        }
                        None
                    }
                    ("int", []) => check_int(v, name, p, &mut out).map(|n| vec![n as f64]),
                    ("int", [n]) => {
                        let items = v.items();
                        if !matches!(v, Value::List(_)) || items.len() != *n {
                            out.error(
                                codes::TYPE,
                                v.span(),
                                format!("`{name}` takes {n} whole numbers in a list"),
                            );
                            return None;
                        }
                        for i in &items {
                            check_int(i, name, p, &mut out);
                        }
                        numbers(v, Dimension::None, 1.0)
                    }
                    (_, []) => {
                        check_float(v, name, Dimension::None, p, &mut out);
                        numbers(v, Dimension::None, 1.0)
                    }
                    (_, [n]) => {
                        check_float_array(v, *n, name, Dimension::None, p, &mut out);
                        numbers(v, Dimension::None, 1.0)
                    }
                    (_, [rows, cols]) => {
                        let items = v.items();
                        if !matches!(v, Value::List(_)) || items.len() != *rows {
                            out.error(
                                codes::TYPE,
                                v.span(),
                                format!("`{name}` is a {rows}x{cols} matrix, row by row"),
                            );
                            return None;
                        }
                        for row in &items {
                            check_float_array(row, *cols, name, Dimension::None, p, &mut out);
                        }
                        numbers(v, Dimension::None, 1.0)
                    }
                    _ => None,
                }
            }
        }
    }

    /// A `bounds=` summary: `[min max]`, or `[[mins…] [maxs…]]` for a vector.
    fn bounds(
        &mut self,
        file: FileId,
        v: &Value<'_>,
        elements: &Elements,
    ) -> Option<(Vec<f64>, Vec<f64>)> {
        let dim = match elements {
            Elements::Positions => Dimension::Length,
            _ => Dimension::None,
        };
        let scale = self.scale;
        let items = v.items();
        let read = |x: &Value<'_>| numbers(x, dim, scale);
        let pair = match items.as_slice() {
            [lo, hi] => read(lo).zip(read(hi)),
            _ => None,
        };
        if pair.is_none() {
            self.out(file).error(
                codes::TYPE,
                v.span(),
                "`bounds=` is the smallest and the largest value, `[min max]`, or for a vector `[[min x min y …] [max x max y …]]`",
            );
        }
        pair
    }

    /// `attribute @h <domain> <type> "Name" [values]`.
    fn attribute_line(
        &mut self,
        file: FileId,
        a: &Attr<'_>,
        owner: &(String, String),
        counts: &Counts,
    ) {
        let words: Vec<&str> = a.args.iter().filter_map(|v| v.as_word()).collect();
        let (Some(handle), Some(domain), Some(ty)) = (words.first(), words.get(1), words.get(2))
        else {
            return self.out(file).error(
                codes::TYPE,
                a.span,
                "an attribute is `attribute @handle <domain> <type> \"Name\" [values]`, such as `attribute @crease edge float \"crease_edge\" [0 1 1]`",
            );
        };
        if !DOMAINS.contains(domain) {
            let at = a.args[1].span();
            return self.out(file).error_fix(
                codes::ENUM,
                at,
                format!(
                    "`{domain}` is not a domain: an attribute's is one of {}",
                    list_near(domain, &DOMAINS)
                ),
                closest(domain, DOMAINS.iter().copied())
                    .map(|d| (format!("write `{d}`"), at, d.to_string())),
            );
        }
        let Some(p) = element(ty) else {
            let at = a.args[2].span();
            return self.out(file).error_fix(
                codes::ENUM,
                at,
                format!(
                    "`{ty}` is not an attribute's data type: one of {}",
                    list_near(ty, &DATA_TYPES)
                ),
                closest(ty, DATA_TYPES.iter().copied())
                    .map(|d| (format!("write `{d}`"), at, d.to_string())),
            );
        };
        let domain: &'static str = DOMAINS
            .iter()
            .find(|d| **d == *domain)
            .copied()
            .unwrap_or("point");
        let expected = counts.of(domain).map(|n| (n, domain));
        let array = handle.to_string();
        let values: Vec<Value<'_>> = a
            .args
            .iter()
            .filter(|v| matches!(v, Value::List(_)))
            .cloned()
            .collect();
        let line = Attr {
            name: a.name.clone(),
            name_span: a.name_span,
            span: a.span,
            args: values,
            props: a.props.clone(),
            node: a.node,
        };
        self.array_line(
            file,
            &line,
            owner,
            &array,
            Elements::Values(Box::new(p)),
            expected,
        );
    }

    /// `uv-map @h "Name" [[u v] …]`: one per corner.
    fn uv_map(&mut self, file: FileId, a: &Attr<'_>, owner: &(String, String), counts: &Counts) {
        let Some(handle) = a
            .args
            .first()
            .and_then(|v| v.as_word())
            .filter(|w| w.starts_with('@'))
        else {
            return self.out(file).error(
                codes::TYPE,
                a.span,
                "a UV map is `uv-map @handle \"Name\" [[u v] …]`, one `[u v]` per corner",
            );
        };
        let values: Vec<Value<'_>> = a
            .args
            .iter()
            .filter(|v| matches!(v, Value::List(_)))
            .cloned()
            .collect();
        let line = Attr {
            name: a.name.clone(),
            name_span: a.name_span,
            span: a.span,
            args: values,
            props: a.props.clone(),
            node: a.node,
        };
        let expected = counts.corner.map(|n| (n, "corner"));
        let p = element("float2").expect("float2 is a data type");
        let array = handle.to_string();
        self.array_line(
            file,
            &line,
            owner,
            &array,
            Elements::Values(Box::new(p)),
            expected,
        );
    }

    /// `vertex-group @h "Name" { weights [[index weight] …] }`: by increasing index.
    /// A shape key's `positions` (RFC 0005, Sau's decision S88): one position per vertex, as
    /// `vertices` are written, inline or in a data file named by the shape key's handle. Left out,
    /// they are the basis's, the mesh's `vertices`.
    fn shape_key_positions(
        &mut self,
        file: FileId,
        a: &Attr<'_>,
        owner: &(String, String),
        counts: &Counts,
    ) {
        let Some(handle) = a
            .args
            .first()
            .and_then(|v| v.as_word())
            .filter(|w| w.starts_with('@'))
        else {
            return;
        };
        let Some(node) = a.node else { return };
        let Some(line) = attrs(node).into_iter().find(|b| b.name == "positions") else {
            return;
        };
        if !self.array_or_file(file, &line, "positions [[0m 0m 0m] …]") {
            return;
        }
        let expected = counts.point.map(|n| (n, "vertex"));
        self.array_line(file, &line, owner, handle, Elements::Positions, expected);
    }

    fn vertex_group(&mut self, file: FileId, a: &Attr<'_>, counts: &Counts) {
        let Some(node) = a.node else { return };
        for w in node.children().filter(|c| c.name() == "weights") {
            let args = args_of(w);
            let [list @ Value::List(_)] = args.as_slice() else {
                self.out(file).error(
                    codes::TYPE,
                    w.code_span(),
                    "`weights` is a list of `[index weight]`, such as [[0 1] [3 0.5]]",
                );
                continue;
            };
            let mut last: Option<f64> = None;
            for item in list.items() {
                let got = self.element(file, &item, "weights", &Elements::Weights(counts.point));
                if let Some(nums) = got
                    && let Some(&i) = nums.first()
                {
                    if last.is_some_and(|l| i <= l) {
                        self.out(file).error(
                            codes::TYPE,
                            item.span(),
                            "a vertex group's weights go by increasing vertex index, each once",
                        );
                    }
                    last = Some(i);
                }
            }
        }
    }

    /// A data file's `array <kind> @owner <name> [ … ]`, against the line that names it.
    pub(super) fn data_file(&mut self, file: FileId, node: Node<'_>) {
        let args = args_of(node);
        let words: Vec<&str> = args.iter().filter_map(|v| v.as_word()).collect();
        let list = args.iter().find(|v| matches!(v, Value::List(_)));
        let (Some(kind), Some(handle), Some(array), Some(list)) =
            (words.first(), words.get(1), words.get(2), list)
        else {
            let span = node.code_span();
            return self.out(file).error(
                codes::TYPE,
                span,
                "a data file holds `array <kind> @handle <array> [ … ]`, such as `array mesh @rock vertices [ … ]`",
            );
        };
        let key = ((kind.to_string(), handle.to_string()), array.to_string());
        let Some(at) = self
            .refs
            .iter()
            .position(|r| (r.owner.clone(), r.array.clone()) == key)
        else {
            let span = node.code_span();
            return self.out(file).error(
                codes::HANDLE,
                span,
                format!(
                    "no line names this data file: {kind} {handle} has no `{array} file=…` line"
                ),
            );
        };
        // A second data file for the same line says so (S75's review note).
        if let Some((first, first_span)) = self.refs[at].found {
            let span = node.code_span();
            let d = Diagnostic::error(
                codes::HANDLE,
                file,
                span,
                format!(
                    "a second data file for `{array}` of {kind} {handle}: one line names one file"
                ),
            )
            .with_label(first, first_span, "the first is here");
            self.out.push(d);
            return;
        }
        self.refs[at].found = Some((file, node.code_span()));
        let r = self.refs[at].clone();
        let items = list.items();
        if let Some(n) = r.count
            && n != items.len()
        {
            self.out(r.file).error(
                codes::TYPE,
                r.span,
                format!(
                    "`count={n}`, but the data file holds {} {array}",
                    items.len()
                ),
            );
        }
        if let Some((n, domain)) = r.expected
            && n != items.len()
        {
            self.out(r.file).error(
                codes::TYPE,
                r.span,
                format!(
                    "`{array}` has one value per {domain}: {n}, but its data file holds {}",
                    items.len()
                ),
            );
        }
        // Each element checked, and the bounds taken from them.
        // Each component's smallest and largest by `total_cmp`, so `-0` is below `0` and the
        // summary is the same bits in any element order; NaNs are left out, and a component of
        // NaNs only has no bounds (Sau's decision S75).
        let mut columns: Vec<Vec<f64>> = Vec::new();
        for item in &items {
            if let Some(nums) = self.element(file, item, array, &r.elements) {
                if columns.is_empty() {
                    columns = vec![Vec::new(); nums.len()];
                }
                for (k, x) in nums.into_iter().enumerate() {
                    if k < columns.len() && !x.is_nan() {
                        columns[k].push(x);
                    }
                }
            }
        }
        let extremes: Option<(Vec<f64>, Vec<f64>)> =
            (!columns.is_empty() && columns.iter().all(|c| !c.is_empty())).then(|| {
                let pick = |c: &Vec<f64>, max: bool| {
                    c.iter()
                        .copied()
                        .reduce(|a, b| {
                            let a_first = a.total_cmp(&b).is_le();
                            if a_first != max { a } else { b }
                        })
                        .unwrap_or_default()
                };
                (
                    columns.iter().map(|c| pick(c, false)).collect(),
                    columns.iter().map(|c| pick(c, true)).collect(),
                )
            });
        if r.bounds.is_some() && extremes.is_none() && !columns.is_empty() {
            let span = r.bounds_span.unwrap_or(r.span);
            self.out(r.file).error(
                codes::RANGE,
                span,
                "`bounds=` cannot be given: a component of this data file holds only NaNs, which have no bounds (S75)",
            );
        }
        let (lo, hi) = extremes.unwrap_or_default();
        if let Some((blo, bhi)) = &r.bounds {
            let same = |a: &[f64], b: &[f64]| {
                a.len() == b.len()
                    && a.iter()
                        .zip(b)
                        .all(|(x, y)| (*x as f32).to_bits() == (*y as f32).to_bits())
            };
            if !lo.is_empty() && !(same(blo, &lo) && same(bhi, &hi)) {
                let span = r.bounds_span.unwrap_or(r.span);
                self.out(r.file).error(
                    codes::RANGE,
                    span,
                    format!(
                        "`bounds=` disagrees with the data file, whose smallest and largest are {lo:?} and {hi:?}"
                    ),
                );
            }
        }
    }

    /// The lines that name a data file none of the project's files holds.
    pub(super) fn unmatched_refs(&mut self) {
        for r in std::mem::take(&mut self.refs)
            .into_iter()
            .filter(|r| r.found.is_none())
        {
            self.out(r.file).error(
                codes::HANDLE,
                r.span,
                format!(
                    "no data file holds `{}` of {} {}: write it as `array {} {} {} [ … ]` in a file of its own",
                    r.array, r.owner.0, r.owner.1, r.owner.0, r.owner.1, r.array
                ),
            );
        }
    }
}

/// The numbers a value holds, flattened: each word read in `dim` at `scale` (`on` and `off` as
/// 1 and 0); `None` if any word does not read.
fn numbers(v: &Value<'_>, dim: Dimension, scale: f32) -> Option<Vec<f64>> {
    let scene = Scene {
        scale_length: scale,
        ..Scene::default()
    };
    match v {
        Value::List(_) => {
            let mut out = Vec::new();
            for i in v.items() {
                out.extend(numbers(&i, dim, scale)?);
            }
            Some(out)
        }
        _ => {
            let w = v.as_word()?;
            if let Ok(n) = w.parse::<i64>() {
                // `-0` is a float's negative zero, below `0` in the bounds (S75).
                let x = if n == 0 && w.starts_with('-') {
                    -0.0
                } else {
                    n as f64
                };
                return Some(vec![x]);
            }
            read_in::<f32>(w, dim, &scene)
                .ok()
                .map(|x| vec![f64::from(x)])
        }
    }
}
