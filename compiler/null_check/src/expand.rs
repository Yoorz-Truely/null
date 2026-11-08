//! Expansion (spec, section 8; Sau's decisions S32, S34, S38 and S74; task 1d-03c): every
//! placement of a component made into the datablocks it makes, as Null text the checker reads
//! and Sau applies. Parameters are put in, conditions decided, patterns unrolled and overrides
//! laid over each copy. The same text gives the same datablocks, handles and order every time.
//!
//! - A copy's root is its placement's handle, `@plank` or, in a pattern, `@plank.1` to
//!   `@plank.20`; a copy inside a copy is a path from it (`@tree/@left/@left`). The other
//!   datablocks a copy makes are paths from its root (`@plank.7/@plank-mesh`), as the checker
//!   resolves them (1d-03b). The root joins the scene or collection the placement is in, and so
//!   does every copy made inside it.
//! - Arithmetic is exact: numbers are kept as fractions in their dimension's internal unit, an
//!   angle in degrees as degrees through every step, and a parameter keeps its fraction. They are
//!   written as decimals, exactly where a decimal holds the value, else to 60 significant digits,
//!   so the one rounding into the property's type is the reader's (S59).
//! - A recursion that does not count down is an error, and so is an expansion past 1,000,000
//!   datablocks, at the line that asked for them, never a hang.
//!
//! [`expand`] gives each file's expanded text; [`check_expanded`] checks it, its copy handles
//! taken as the handles they are.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Write as _;

use null_syntax::ast::{Entry, Node, Value};
use null_syntax::{Diagnostic, FileId, Parse, Span, parse};
use null_value::{Dimension, Unit};

use crate::check::expr::{self, Dim, Expr};
use crate::codes;
use crate::schema::Schema;

/// The most datablocks a project may expand to (S38's guard against a count that is merely huge).
pub const LIMIT: usize = 1_000_000;

/// The deepest a chain of copies inside copies may go. A countdown ends every chain, but one
/// that starts at a million would take the expansion's stack before the datablock limit.
pub const DEPTH: usize = 256;

/// One file, expanded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expanded {
    /// The file, as given.
    pub file: FileId,
    /// Its text, expanded.
    pub text: String,
}

/// An exact fraction, its denominator positive and the two coprime. `i128::MIN` is never held,
/// so every negation is exact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Q {
    n: i128,
    d: i128,
}

/// The most significant digits a value is written to where no decimal holds it exactly: with
/// any i128 denominator, enough that the one rounding into a float64 is the reader's (S59).
const DIGITS: usize = 60;

fn gcd(a: u128, b: u128) -> u128 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.max(1)
}

/// `a/b` against `c/d`, `b` and `d` positive, exactly: by their integer parts, then their
/// remainders' reciprocals in reverse, as Euclid's algorithm runs.
fn cmp_frac(a: u128, b: u128, c: u128, d: u128) -> Ordering {
    let (qa, ra, qc, rc) = (a / b, a % b, c / d, c % d);
    match qa.cmp(&qc) {
        Ordering::Equal => match (ra, rc) {
            (0, 0) => Ordering::Equal,
            (0, _) => Ordering::Less,
            (_, 0) => Ordering::Greater,
            _ => cmp_frac(d, rc, b, ra),
        },
        other => other,
    }
}

impl Q {
    fn new(n: i128, d: i128) -> Option<Q> {
        if d == 0 || n == i128::MIN || d == i128::MIN {
            return None;
        }
        let g = gcd(n.unsigned_abs(), d.unsigned_abs()) as i128;
        let s = d.signum();
        Some(Q {
            n: s * (n / g),
            d: s * (d / g),
        })
    }

    fn int(n: i128) -> Q {
        Q { n, d: 1 }
    }

    fn add(self, o: Q) -> Option<Q> {
        Q::new(
            self.n
                .checked_mul(o.d)?
                .checked_add(o.n.checked_mul(self.d)?)?,
            self.d.checked_mul(o.d)?,
        )
    }

    fn neg(self) -> Q {
        Q {
            n: -self.n,
            d: self.d,
        }
    }

    fn mul(self, o: Q) -> Option<Q> {
        // Cross-reduced first, so a product that fits is never refused.
        let (g1, g2) = (
            gcd(self.n.unsigned_abs(), o.d.unsigned_abs()) as i128,
            gcd(o.n.unsigned_abs(), self.d.unsigned_abs()) as i128,
        );
        Q::new(
            (self.n / g1).checked_mul(o.n / g2)?,
            (self.d / g2).checked_mul(o.d / g1)?,
        )
    }

    fn div(self, o: Q) -> Option<Q> {
        if o.n == 0 {
            return None;
        }
        self.mul(Q::new(o.d, o.n)?)
    }

    fn cmp(self, o: Q) -> Ordering {
        match (self.n.signum(), o.n.signum()) {
            (a, b) if a != b => a.cmp(&b),
            (0, _) => Ordering::Equal,
            (1, _) => cmp_frac(
                self.n.unsigned_abs(),
                self.d.unsigned_abs(),
                o.n.unsigned_abs(),
                o.d.unsigned_abs(),
            ),
            _ => cmp_frac(
                o.n.unsigned_abs(),
                o.d.unsigned_abs(),
                self.n.unsigned_abs(),
                self.d.unsigned_abs(),
            ),
        }
    }

    /// A decimal: exact where the denominator is of 2s and 5s, else `DIGITS` significant digits.
    /// The long division never overflows: each digit is found by adding the remainder ten times,
    /// reduced as it goes, and the remainder and the denominator are below 2^127.
    fn decimal(self) -> String {
        let (mut n, d) = (self.n.unsigned_abs(), self.d.unsigned_abs());
        let whole = n / d;
        n %= d;
        let mut out = format!("{}{whole}", if self.n < 0 { "-" } else { "" });
        if n == 0 {
            return out;
        }
        out.push('.');
        let mut significant = if whole == 0 {
            0
        } else {
            whole.to_string().len()
        };
        while n != 0 && significant < DIGITS {
            let (mut r, mut digit) = (0u128, 0u8);
            for _ in 0..10 {
                r += n;
                if r >= d {
                    r -= d;
                    digit += 1;
                }
            }
            n = r;
            out.push(char::from(b'0' + digit));
            if significant > 0 || digit != 0 {
                significant += 1;
            }
        }
        out
    }
}

/// A computed value.
#[derive(Clone, Debug, PartialEq)]
enum Val {
    /// A number in its dimension's internal unit, but for the power of its angle `deg` held in
    /// degrees (`35deg` is 35 with `deg` 1; `35deg*2m` keeps it, `35deg/1deg` is a number).
    Num {
        q: Q,
        dim: Dim,
        deg: i8,
    },
    Bool(bool),
    Word(String),
    /// A handle, as written in the expanded text.
    Handle(String),
}

impl Val {
    fn num(n: i128) -> Val {
        Val::Num {
            q: Q::int(n),
            dim: Dim::NONE,
            deg: 0,
        }
    }

    /// As written in the expanded text.
    fn text(&self) -> Result<String, String> {
        Ok(match self {
            Val::Num { q, dim, deg } => {
                let unit = match deg {
                    0 if *dim == Dim::NONE => "",
                    0 => dim
                        .named()
                        .and_then(|d| d.units().first())
                        .map(|u| u.suffix())
                        .ok_or_else(|| {
                            format!("the value is {}, which no property takes", dim.describe())
                        })?,
                    1 if *dim == Dim::of(Dimension::Angle) => "deg",
                    _ => {
                        return Err(format!(
                            "the value is {} with degrees in it, which has no exact form: \
                             write its angles in one unit",
                            dim.describe()
                        ));
                    }
                };
                format!("{}{unit}", q.decimal())
            }
            Val::Bool(b) => if *b { "on" } else { "off" }.to_string(),
            Val::Word(w) | Val::Handle(w) => w.clone(),
        })
    }

    /// A value written as a plain word (a default, an item of a list): a number, `on` or
    /// `off`, a handle, or a word.
    fn read(text: &str) -> Val {
        match text {
            "on" => Val::Bool(true),
            "off" => Val::Bool(false),
            t if t.starts_with('@') => Val::Handle(t.to_string()),
            t => literal(t).unwrap_or_else(|| Val::Word(t.to_string())),
        }
    }

    /// The kind, as a message names it.
    fn kind(&self) -> String {
        match self {
            Val::Num { dim, deg, .. } if *deg != 0 => format!("{} in degrees", dim.describe()),
            Val::Num { dim, .. } => dim.describe(),
            Val::Bool(_) => "`on` or `off`".into(),
            Val::Word(w) => format!("the word `{w}`"),
            Val::Handle(h) => format!("the handle `{h}`"),
        }
    }
}

/// A number as written (`25cm`, `2.4`, `-1e3`, `35deg`), exactly.
fn literal(text: &str) -> Option<Val> {
    let b = text.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'-' | b'+')));
    let digits_from = i;
    while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.' || b[i] == b'_') {
        i += 1;
    }
    if !b[digits_from..i].iter().any(u8::is_ascii_digit) {
        return None;
    }
    let mantissa_end = i;
    let mut exp: i32 = 0;
    if i < b.len() && matches!(b[i], b'e' | b'E') {
        let mut j = i + 1;
        if j < b.len() && matches!(b[j], b'-' | b'+') {
            j += 1;
        }
        let start = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > start {
            exp = text[i + 1..j].parse().ok()?;
            i = j;
        }
    }
    let mantissa = text[digits_from..mantissa_end].replace('_', "");
    let (whole, frac) = mantissa.split_once('.').unwrap_or((&mantissa, ""));
    let digits = format!("{whole}{frac}");
    let mut q = Q::new(
        digits.parse().ok()?,
        10i128.checked_pow(u32::try_from(frac.len()).ok()?)?,
    )?;
    let ten = Q::int(10i128.checked_pow(exp.unsigned_abs())?);
    q = if exp >= 0 { q.mul(ten)? } else { q.div(ten)? };
    if b.first() == Some(&b'-') {
        q = q.neg();
    }
    let unit = &text[i..];
    if unit.is_empty() {
        return Some(Val::Num {
            q,
            dim: Dim::NONE,
            deg: 0,
        });
    }
    let u = Unit::from_suffix(unit, Dimension::None)?;
    if u.is_degrees() {
        return Some(Val::Num {
            q,
            dim: Dim::of(Dimension::Angle),
            deg: 1,
        });
    }
    let (n, d) = u.exact_factor()?;
    Some(Val::Num {
        q: q.mul(Q::new(i128::try_from(n).ok()?, i128::try_from(d).ok()?)?)?,
        dim: Dim::of(u.dimension()),
        deg: 0,
    })
}

const TOO_LARGE: &str = "the value is too large to compute exactly";

/// Two numbers of one kind, for adding or comparing: the same dimension, their angles in the
/// same unit.
fn same_kind(a: &Val, b: &Val, what: &str) -> Result<(Q, Q), String> {
    match (a, b) {
        (
            Val::Num {
                q: qa,
                dim: da,
                deg: ga,
            },
            Val::Num {
                q: qb,
                dim: db,
                deg: gb,
            },
        ) if da == db && ga == gb => Ok((*qa, *qb)),
        (Val::Num { dim: da, .. }, Val::Num { dim: db, .. }) if da == db => Err(format!(
            "degrees and radians do not {what} exactly (by π/180): write both in one"
        )),
        _ => Err(format!(
            "only values of one kind {what}: {} and {}",
            a.kind(),
            b.kind()
        )),
    }
}

fn eval(e: &Expr, scope: &HashMap<String, Val>) -> Result<Val, String> {
    match e {
        Expr::Number { lit, .. } => {
            literal(lit).ok_or_else(|| format!("`{lit}` cannot be computed exactly"))
        }
        Expr::Name(n) => scope
            .get(n)
            .cloned()
            .ok_or_else(|| format!("no parameter is called `{n}`")),
        Expr::Neg(x) => match eval(x, scope)? {
            Val::Num { q, dim, deg } => Ok(Val::Num {
                q: q.neg(),
                dim,
                deg,
            }),
            _ => Err("only a number is negated".into()),
        },
        Expr::Op(op, a, b) => {
            let (a, b) = (eval(a, scope)?, eval(b, scope)?);
            if matches!(op, '+' | '-') {
                let what = if *op == '+' { "add" } else { "subtract" };
                let (qa, qb) = same_kind(&a, &b, what)?;
                let Val::Num { dim, deg, .. } = a else {
                    unreachable!()
                };
                let q = if *op == '+' {
                    qa.add(qb)
                } else {
                    qa.add(qb.neg())
                };
                return Ok(Val::Num {
                    q: q.ok_or(TOO_LARGE)?,
                    dim,
                    deg,
                });
            }
            let (
                Val::Num {
                    q: qa,
                    dim: da,
                    deg: ga,
                },
                Val::Num {
                    q: qb,
                    dim: db,
                    deg: gb,
                },
            ) = (a, b)
            else {
                return Err("only numbers are multiplied and divided".into());
            };
            let (q, dim, deg) = match op {
                '*' => (qa.mul(qb).ok_or(TOO_LARGE)?, da.times(db), ga + gb),
                _ if qb.n == 0 => return Err("a division by zero".into()),
                _ => (qa.div(qb).ok_or(TOO_LARGE)?, da.over(db), ga - gb),
            };
            Ok(Val::Num { q, dim, deg })
        }
    }
}

fn holds(c: &expr::Condition, scope: &HashMap<String, Val>) -> Result<bool, String> {
    // A name that is no parameter is a word of a list (`state == liquid`), or `on`/`off`.
    let side = |e: &Expr| -> Result<Val, String> {
        match e {
            Expr::Name(n) if !scope.contains_key(n) => Ok(Val::read(n)),
            _ => eval(e, scope),
        }
    };
    let (a, b) = (side(&c.left)?, side(&c.right)?);
    if !matches!((&a, &b), (Val::Num { .. }, Val::Num { .. })) {
        return match c.op {
            "==" => Ok(a == b),
            "!=" => Ok(a != b),
            _ => Err("only numbers are ordered".into()),
        };
    }
    let (x, y) = same_kind(&a, &b, "compare")?;
    let ord = x.cmp(y);
    Ok(match c.op {
        "==" => ord == Ordering::Equal,
        "!=" => ord != Ordering::Equal,
        "<" => ord == Ordering::Less,
        ">" => ord == Ordering::Greater,
        "<=" => ord != Ordering::Greater,
        _ => ord != Ordering::Less,
    })
}

/// What names mean inside one copy.
#[derive(Clone, Debug, Default)]
struct Ctx {
    values: HashMap<String, Val>,
    /// A local handle to its copy's handle (`@plank` to `@plank.7`, `@plank-mesh` to
    /// `@plank.7/@plank-mesh`).
    handles: HashMap<String, String>,
}

impl Ctx {
    /// `v` as written in the expanded text: an expression computed, a local handle made the
    /// copy's, anything else as written.
    fn value(&self, v: &Value<'_>) -> Result<String, String> {
        match v {
            Value::List(_) => {
                let items = v
                    .items()
                    .iter()
                    .map(|i| self.value(i))
                    .collect::<Result<Vec<String>, String>>()?;
                Ok(format!("[{}]", items.join(" ")))
            }
            Value::Missing(_) => Ok(String::new()),
            _ => match self.computed(v)? {
                Some(val) => val.text(),
                None => Ok(v.text().to_string()),
            },
        }
    }

    /// `v` as a value, exactly, for a parameter, a count or a list's item: what
    /// [`Ctx::computed`] makes of it, else read as written.
    fn val(&self, v: &Value<'_>) -> Result<Val, String> {
        Ok(match self.computed(v)? {
            Some(val) => val,
            None => match v {
                Value::Word(t) => Val::read(t.text),
                other => Val::Word(other.text().to_string()),
            },
        })
    }

    /// What the copy makes of `v`, where it makes anything: a local handle, or an expression
    /// of the names in scope (a word, or a quoted one with an operator); `None` for a value
    /// that stays as written.
    fn computed(&self, v: &Value<'_>) -> Result<Option<Val>, String> {
        match v {
            Value::Word(t) => {
                let w = t.text;
                if let Some(h) = self.handles.get(w) {
                    return Ok(Some(Val::Handle(h.clone())));
                }
                // A path from a local handle (`@plank-mesh/@bevel`).
                if let Some((head, rest)) = w.split_once('/')
                    && let Some(h) = self.handles.get(head)
                {
                    return Ok(Some(Val::Handle(format!("{h}/{rest}"))));
                }
                if expr::is_expression(w)
                    && let Ok(e) = expr::parse(w, false)
                    && names_known(&e, &self.values)
                {
                    return eval(&e, &self.values).map(Some);
                }
                Ok(None)
            }
            Value::Str(_) | Value::Raw(_) => {
                let text = v.as_string().unwrap_or_default();
                match expr::parse(&text, true) {
                    Ok(e) if !matches!(e, Expr::Name(_)) && names_known(&e, &self.values) => {
                        eval(&e, &self.values).map(Some)
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }
}

fn names_known(e: &Expr, scope: &HashMap<String, Val>) -> bool {
    match e {
        Expr::Name(n) => scope.contains_key(n),
        Expr::Neg(x) => names_known(x, scope),
        Expr::Op(_, a, b) => names_known(a, scope) && names_known(b, scope),
        Expr::Number { .. } => true,
    }
}

fn first_word<'a>(node: Node<'a>) -> Option<&'a str> {
    node.args().next().and_then(|v| v.as_word())
}

/// A node's text from `span`, which lies inside it.
fn text_in<'a>(node: Node<'a>, span: Span) -> &'a str {
    let start = node.span().start;
    &node.0.text()[span.start - start..span.end - start]
}

/// The lines that may repeat in a datablock, which an override's line of the same name replaces
/// only where its handle is the same, and is added to otherwise.
const REPEATED: [&str; 3] = ["modifier", "constraint", "bone"];

/// Whether line `a` laid over a block replaces line `b` there: the same name, and for a line
/// that repeats, the same handle.
fn same_line(a: Node<'_>, b: Node<'_>) -> bool {
    let name = a.name();
    name == b.name() && (!REPEATED.contains(&name.as_ref()) || first_word(a) == first_word(b))
}

/// A line laid over a datablock, with what names mean where it was written.
type Over<'a> = (Node<'a>, Ctx);

struct Ex<'a, 's> {
    schema: &'s Schema,
    comps: HashMap<String, Node<'a>>,
    /// Override lines by the path they name, as written.
    overrides: HashMap<String, Vec<Node<'a>>>,
    made: usize,
    out: Vec<Diagnostic>,
    stopped: bool,
    /// The components being expanded, with their countdown's value, innermost last.
    stack: Vec<(String, Option<Q>)>,
}

impl<'a> Ex<'a, '_> {
    fn error(&mut self, file: FileId, span: Span, message: impl Into<String>) {
        self.out
            .push(Diagnostic::error(codes::TYPE, file, span, message.into()));
    }

    fn is_datablock(&self, kind: &str) -> bool {
        self.schema.datablock_kinds().any(|s| s.kind == kind)
    }

    /// A block's lines, conditions decided: an `if`'s when it holds, else its `else`'s.
    fn lines(&mut self, file: FileId, node: Node<'a>, ctx: &Ctx) -> Vec<Node<'a>> {
        let mut out = Vec::new();
        let mut last_if: Option<bool> = None;
        for c in node.children() {
            match c.name().as_ref() {
                "if" => {
                    let held = self.condition(file, c, ctx);
                    if held {
                        out.extend(self.lines(file, c, ctx));
                    }
                    last_if = Some(held);
                }
                "else" => {
                    if last_if == Some(false) {
                        out.extend(self.lines(file, c, ctx));
                    }
                    last_if = None;
                }
                _ => {
                    out.push(c);
                    last_if = None;
                }
            }
        }
        out
    }

    fn condition(&mut self, file: FileId, node: Node<'a>, ctx: &Ctx) -> bool {
        let Some(text) = node.args().next().and_then(|v| v.as_string()) else {
            return false; // the checker says so
        };
        match expr::parse_condition(&text).and_then(|c| holds(&c, &ctx.values)) {
            Ok(b) => b,
            Err(e) => {
                self.error(file, node.code_span(), format!("`{text}`: {e}"));
                false
            }
        }
    }

    /// The override lines that name `path`, their conditions decided, in the project's terms.
    fn overrides_of(&mut self, file: FileId, path: &str) -> Vec<Over<'a>> {
        let nodes = self.overrides.get(path).cloned().unwrap_or_default();
        let project = Ctx::default();
        let mut out = Vec::new();
        for o in nodes {
            out.extend(
                self.lines(file, o, &project)
                    .into_iter()
                    .map(|l| (l, Ctx::default())),
            );
        }
        out
    }

    /// One node as written in a copy: its line, its entries in the copy's terms (its first
    /// argument `head` where given), its block's lines with `over` laid on them. `owner` is the
    /// datablock's path, which its sub-objects' overrides name (`@plank.7/@fluid`).
    #[allow(clippy::too_many_arguments)]
    fn emit(
        &mut self,
        file: FileId,
        node: Node<'a>,
        ctx: &Ctx,
        depth: usize,
        over: &[Over<'a>],
        head: Option<&str>,
        owner: &str,
        out: &mut String,
    ) {
        let pad = "  ".repeat(depth);
        let mut line = node.name().into_owned();
        for (i, e) in node.entries().enumerate() {
            let text = match (e, head) {
                (Entry::Arg(_), Some(h)) if i == 0 => Ok(h.to_string()),
                (Entry::Arg(v), _) => ctx.value(&v),
                (Entry::Prop { key, value }, _) => {
                    ctx.value(&value).map(|v| format!("{}={v}", key.text))
                }
                (Entry::Keyword { key, value }, _) => {
                    ctx.value(&value).map(|v| format!("{} {v}", key.text))
                }
            };
            match text {
                Ok(t) => {
                    line.push(' ');
                    line.push_str(&t);
                }
                Err(m) => self.error(file, e.value().span(), m),
            }
        }
        let children = self.lines(file, node, ctx);
        // The layers in order (the placement's lines, then each override's): a line replaces
        // the copy's line, and an earlier layer's, of its name; one that repeats (`modifier`),
        // those of its name and handle; else it is added.
        let mut layers: Vec<Over<'a>> = Vec::new();
        for (o, octx) in over {
            layers.retain(|(p, _)| !same_line(*p, *o));
            layers.push((*o, octx.clone()));
        }
        let over = layers.as_slice();
        let replaced = |c: &Node<'a>| over.iter().any(|(o, _)| same_line(*o, *c));
        let kept: Vec<Node<'a>> = children.into_iter().filter(|c| !replaced(c)).collect();
        if kept.is_empty() && over.is_empty() && node.block().is_none() {
            let _ = writeln!(out, "{pad}{line}");
            return;
        }
        let _ = writeln!(out, "{pad}{line} {{");
        for c in kept {
            let path = first_word(c)
                .filter(|h| h.starts_with('@'))
                .map(|h| format!("{owner}/{h}"));
            let lay = match &path {
                Some(p) if self.overrides.contains_key(p) => self.overrides_of(file, p),
                _ => Vec::new(),
            };
            self.emit(file, c, ctx, depth + 1, &lay, None, owner, out);
        }
        for (o, octx) in over {
            self.emit(file, *o, octx, depth + 1, &[], None, owner, out);
        }
        let _ = writeln!(out, "{pad}}}");
    }

    /// A placement (`use`), in `ctx` (the placing copy's, or the project's), its copies' handles
    /// under `prefix`: their datablocks into `out`, their roots' handles into `members`.
    fn placement(
        &mut self,
        file: FileId,
        node: Node<'a>,
        ctx: &Ctx,
        prefix: Option<&str>,
        out: &mut String,
        members: &mut Vec<String>,
    ) {
        if self.stopped {
            return;
        }
        let args: Vec<Value<'a>> = node.args().collect();
        let (Some(name), Some(handle)) = (
            args.first().and_then(|v| v.as_word()),
            args.get(1).and_then(|v| v.as_word()),
        ) else {
            return;
        };
        let Some(&comp) = self.comps.get(name) else {
            return; // the checker says so
        };
        let full = |h: String| prefix.map_or(h.clone(), |p| format!("{p}/{h}"));
        let prop = |k: &str| {
            node.entries().find_map(|e| match e {
                Entry::Prop { key, value } if key.text == k => Some(value),
                _ => None,
            })
        };
        // The copies, each with `n` and the item where it is a pattern's.
        let mut copies: Vec<(String, Vec<(String, Val)>)> = Vec::new();
        if let Some(c) = prop("count") {
            let count = match ctx.val(&c) {
                Ok(Val::Num { q, dim, deg: 0 }) if dim == Dim::NONE && q.d == 1 && q.n >= 0 => q.n,
                _ => return, // the checker says so
            };
            // Each copy makes at least its component's own datablocks.
            let each = comp
                .children()
                .filter(|c| self.is_datablock(&c.name()))
                .count()
                .max(1) as i128;
            if (self.made as i128).saturating_add(count.saturating_mul(each)) > LIMIT as i128 {
                return self.limit(file, node);
            }
            for k in 1..=count {
                copies.push((
                    full(format!("{handle}.{k}")),
                    vec![("n".into(), Val::num(k))],
                ));
            }
        } else if let Some(list) = prop("each") {
            let Some(item) = prop("as").and_then(|v| v.as_word().map(str::to_string)) else {
                return; // the checker says so
            };
            for (i, v) in list.items().iter().enumerate() {
                let value = match ctx.val(v) {
                    Ok(t) => t,
                    Err(m) => {
                        self.error(file, v.span(), m);
                        continue;
                    }
                };
                let k = i as i128 + 1;
                copies.push((
                    full(format!("{handle}.{k}")),
                    vec![("n".into(), Val::num(k)), (item.clone(), value)],
                ));
            }
        } else {
            copies.push((full(handle.to_string()), Vec::new()));
        }
        for (copy, extra) in copies {
            if self.stopped {
                return;
            }
            // The block's names: the placing copy's, and `n` and the item.
            let mut outer = ctx.clone();
            outer.values.extend(extra);
            self.copy(file, comp, node, name, &copy, &outer, out, members);
        }
    }

    fn limit(&mut self, file: FileId, node: Node<'a>) {
        self.error(
            file,
            node.code_span(),
            format!(
                "this expands the project past {LIMIT} datablocks, Null's limit (S38): place fewer \
                 copies, or make them in geometry nodes"
            ),
        );
        self.stopped = true;
    }

    /// The parameters `lines` give, into `values`, read in `ctx`; the other lines are the root's.
    fn given(
        &mut self,
        file: FileId,
        lines: Vec<Node<'a>>,
        ctx: &Ctx,
        params: &[String],
        values: &mut HashMap<String, Val>,
    ) -> Vec<Over<'a>> {
        let mut root = Vec::new();
        for l in lines {
            let name = l.name().into_owned();
            if !params.contains(&name) {
                root.push((l, ctx.clone()));
                continue;
            }
            let Some(v) = l.args().next() else { continue };
            match ctx.val(&v) {
                Ok(value) => {
                    values.insert(name, value);
                }
                Err(m) => self.error(file, v.span(), m),
            }
        }
        root
    }

    /// One copy of component `comp` (handle `name`), placed by `placement` as `copy`.
    #[allow(clippy::too_many_arguments)]
    fn copy(
        &mut self,
        file: FileId,
        comp: Node<'a>,
        placement: Node<'a>,
        name: &str,
        copy: &str,
        outer: &Ctx,
        out: &mut String,
        members: &mut Vec<String>,
    ) {
        // Parameters: defaults, then the placement's, then an override's of this copy.
        let mut values: HashMap<String, Val> = HashMap::new();
        let mut params: Vec<String> = Vec::new();
        let mut countdown = None;
        for p in comp.children().filter(|c| c.name() == "param") {
            let args: Vec<Value<'_>> = p.args().collect();
            let Some(pname) = args.first().and_then(|v| v.as_word()) else {
                continue;
            };
            params.push(pname.to_string());
            if p.children().any(|c| c.name() == "countdown") {
                countdown = Some(pname.to_string());
            }
            let one_of = p.children().any(|c| c.name() == "one-of");
            let default = match args.as_slice() {
                // `param finish material @oak`
                [_, _, h] => h.as_word().map(Val::read),
                [_, v] => v.as_word().and_then(|w| match Val::read(w) {
                    // A type's name (`param length length`) is no value; a word of a list is.
                    Val::Word(w) if !one_of || w == "word" => None,
                    value => Some(value),
                }),
                _ => None,
            };
            if let Some(d) = default {
                values.insert(pname.to_string(), d);
            }
        }
        let placed = self.lines(file, placement, outer);
        let mut root_over = self.given(file, placed, outer, &params, &mut values);
        let project = Ctx::default();
        let mut overridden = Vec::new();
        for o in self.overrides.get(copy).cloned().unwrap_or_default() {
            overridden.extend(self.lines(file, o, &project));
        }
        root_over.extend(self.given(file, overridden, &project, &params, &mut values));
        // Recursion counts down (S38): this component's countdown smaller than where it was
        // last placed, and the chain no deeper than `DEPTH`.
        let value = countdown.as_ref().and_then(|c| match values.get(c) {
            Some(Val::Num { q, .. }) => Some(*q),
            _ => None,
        });
        if let Some(&(_, before)) = self.stack.iter().rev().find(|(c, _)| c == name) {
            let smaller =
                matches!((before, value), (Some(b), Some(v)) if v.cmp(b) == Ordering::Less);
            if !smaller {
                self.error(
                    file,
                    placement.code_span(),
                    format!(
                        "`{name}` is placed inside itself and does not count down: its \
                         `{{ countdown }}` parameter must be smaller here than where it was \
                         placed (S38)"
                    ),
                );
                self.stopped = true;
                return;
            }
        }
        if self.stack.len() >= DEPTH {
            self.error(
                file,
                placement.code_span(),
                format!("copies inside copies nest deeper than {DEPTH} here: count down from less"),
            );
            self.stopped = true;
            return;
        }
        // The copy's handles: its root is the copy, its other datablocks paths from it.
        let mut ctx = Ctx {
            values,
            handles: HashMap::new(),
        };
        let mut root = None;
        for c in comp.children() {
            if let Some(h) = first_word(c).filter(|h| h.starts_with('@'))
                && self.is_datablock(&c.name())
            {
                if c.name() == "object" && root.is_none() {
                    root = Some(h.to_string());
                    ctx.handles.insert(h.to_string(), copy.to_string());
                } else {
                    ctx.handles.insert(h.to_string(), format!("{copy}/{h}"));
                }
            }
        }
        members.push(copy.to_string());
        self.stack.push((name.to_string(), value));
        for c in self.lines(file, comp, &ctx) {
            if self.stopped {
                break;
            }
            let kind = c.name().into_owned();
            if kind == "use" {
                self.placement(file, c, &ctx, Some(copy), out, members);
                continue;
            }
            if !self.is_datablock(&kind) {
                continue; // `param`
            }
            if self.made >= LIMIT {
                self.limit(file, placement);
                break;
            }
            self.made += 1;
            let Some(h) = first_word(c).filter(|h| h.starts_with('@')) else {
                continue; // the checker says so
            };
            let new = ctx.handles.get(h).cloned().unwrap_or_else(|| h.to_string());
            let over = if root.as_deref() == Some(h) {
                root_over.clone()
            } else {
                self.overrides_of(file, &new)
            };
            out.push('\n');
            self.emit(file, c, &ctx, 0, &over, Some(&new), &new, out);
        }
        self.stack.pop();
    }

    /// A scene or a collection with placements: its lines as written, each `use` made into its
    /// copies' members (the copies' datablocks into `made`), its `override` lines taken out.
    fn container(
        &mut self,
        file: FileId,
        node: Node<'a>,
        depth: usize,
        out: &mut String,
        made: &mut String,
    ) {
        let pad = "  ".repeat(depth);
        let start = node.code_span().start;
        let own = node
            .block()
            .map_or(node.code_span().end, |b| b.span().start);
        let _ = writeln!(
            out,
            "{pad}{} {{",
            text_in(node, Span::new(start, own)).trim_end()
        );
        for c in node.children() {
            match c.name().as_ref() {
                "use" => {
                    let mut members = Vec::new();
                    self.placement(file, c, &Ctx::default(), None, made, &mut members);
                    for m in members {
                        let _ = writeln!(out, "{pad}  object {m}");
                    }
                }
                "override" => {}
                "collection" if c.block().is_some() && has_placements(c) => {
                    self.container(file, c, depth + 1, out, made);
                }
                _ => {
                    let _ = writeln!(out, "{pad}  {}", text_in(c, c.code_span()));
                }
            }
        }
        let _ = writeln!(out, "{pad}}}");
    }
}

fn has_placements(node: Node<'_>) -> bool {
    node.children().any(|c| {
        matches!(c.name().as_ref(), "use" | "override")
            || (c.name() == "collection" && has_placements(c))
    })
}

fn collect_overrides<'a>(node: Node<'a>, out: &mut HashMap<String, Vec<Node<'a>>>) {
    for c in node.children() {
        match c.name().as_ref() {
            "override" => {
                if let Some(w) = first_word(c) {
                    out.entry(w.to_string()).or_default().push(c);
                }
            }
            "collection" => collect_overrides(c, out),
            _ => {}
        }
    }
}

/// Every file of a project expanded: components taken out, each placement made into its copies'
/// datablocks (after the scene or collection that places them, in its file) and its copies'
/// roots as members there; overrides laid over the copies they name; every other line as
/// written. The problems are the expansion's own (a condition or value that cannot be computed,
/// a recursion that does not count down, the limits); the checker's are [`crate::check`]'s, on
/// the files as written, which should be clean before the expansion is trusted.
pub fn expand(schema: &Schema, files: &[(FileId, &Parse)]) -> (Vec<Expanded>, Vec<Diagnostic>) {
    let mut ex = Ex {
        schema,
        comps: HashMap::new(),
        overrides: HashMap::new(),
        made: 0,
        out: Vec::new(),
        stopped: false,
        stack: Vec::new(),
    };
    for (_, p) in files {
        for node in p.document().nodes() {
            match node.name().as_ref() {
                "component" => {
                    if let Some(h) = first_word(node) {
                        ex.comps.entry(h.to_string()).or_insert(node);
                    }
                }
                "scene" | "collection" => collect_overrides(node, &mut ex.overrides),
                _ => {}
            }
        }
    }
    let mut result = Vec::new();
    for (file, p) in files {
        // The text as written, but for components (taken out) and the scenes and collections
        // that place copies (written with their copies, which follow them).
        let text = p.print();
        let mut out = String::new();
        let mut at = 0;
        for node in p.document().nodes() {
            let kind = node.name();
            let placing = matches!(kind.as_ref(), "scene" | "collection") && has_placements(node);
            if kind != "component" && !placing {
                continue;
            }
            let span = node.span();
            out.push_str(&text[at..span.start]);
            at = span.end;
            if placing {
                let mut made = String::new();
                ex.container(*file, node, 0, &mut out, &mut made);
                out.push_str(&made);
            }
        }
        out.push_str(&text[at..]);
        result.push(Expanded {
            file: *file,
            text: out,
        });
    }
    let mut problems = ex.out;
    problems.sort_by_key(|d| (d.primary.file.0, d.primary.span.start, d.primary.span.end));
    (result, problems)
}

/// The expanded files checked: their syntax, and [`crate::check`] on them, their copy handles
/// (`@plank.7`, `@plank.7/@plank-mesh`) taken as the handles they are. The spans are the
/// expanded texts'.
pub fn check_expanded(schema: &Schema, expanded: &[Expanded]) -> Vec<Diagnostic> {
    let parses: Vec<(FileId, Parse)> = expanded.iter().map(|e| (e.file, parse(&e.text))).collect();
    let files: Vec<(FileId, &Parse)> = parses.iter().map(|(f, p)| (*f, p)).collect();
    let mut out: Vec<Diagnostic> = files
        .iter()
        .flat_map(|(f, p)| null_syntax::diagnostics(p, *f))
        .collect();
    out.extend(crate::check::check_flat(schema, &files));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(n: i128, d: i128) -> Q {
        Q::new(n, d).unwrap()
    }

    #[test]
    fn decimals_are_exact_or_60_significant_digits() {
        assert_eq!(q(1, 4).decimal(), "0.25");
        assert_eq!(q(-5, 1).decimal(), "-5");
        assert_eq!(q(7, 50).decimal(), "0.14");
        assert_eq!(q(1, 3).decimal(), format!("0.{}", "3".repeat(60)));
        assert_eq!(q(1, 3000).decimal(), format!("0.000{}", "3".repeat(60)));
        assert_eq!(q(10, 3).decimal(), format!("3.{}", "3".repeat(59)));
        // A denominator above u128::MAX / 10 is divided without overflow.
        let big = i128::MAX - 1;
        let d = q(1, big).decimal();
        assert!(
            d.starts_with(
                "0.00000000000000000000000000000000000000587747175411143753984368268611122838"
            ),
            "{d}"
        );
        assert_eq!(q(big - 1, big).decimal().len(), 2 + 60);
    }

    #[test]
    fn comparisons_are_exact_however_large() {
        let a = q(i128::MAX - 2, i128::MAX - 1);
        let b = q(i128::MAX - 3, i128::MAX - 2);
        assert_eq!(a.cmp(b), Ordering::Greater);
        assert_eq!(b.cmp(a), Ordering::Less);
        assert_eq!(a.neg().cmp(b.neg()), Ordering::Less);
        assert_eq!(a.cmp(a), Ordering::Equal);
        assert_eq!(q(-1, 2).cmp(q(1, 3)), Ordering::Less);
    }

    #[test]
    fn literals_are_exact_in_the_internal_unit() {
        let len = Dim::of(Dimension::Length);
        let num = |q, dim| Some(Val::Num { q, dim, deg: 0 });
        assert_eq!(literal("25cm"), num(q(1, 4), len));
        assert_eq!(literal("-1e3"), num(Q::int(-1000), Dim::NONE));
        assert_eq!(literal("2.5e-1m"), num(q(1, 4), len));
        assert_eq!(literal("1in"), num(q(254, 10000), len));
        assert!(matches!(literal("35deg"), Some(Val::Num { deg: 1, .. })));
        assert_eq!(literal("liquid"), None);
        assert_eq!(literal("-"), None);
    }

    #[test]
    fn arithmetic_keeps_units_and_degrees_exact() {
        let mut scope = HashMap::new();
        scope.insert("n".to_string(), Val::num(7));
        scope.insert("length".to_string(), literal("2.4m").unwrap());
        let get = |text: &str, quoted: bool| {
            eval(&expr::parse(text, quoted).unwrap(), &scope).and_then(|v| v.text())
        };
        assert_eq!(get("n*25cm", false).as_deref(), Ok("1.75m"));
        assert_eq!(get("length/1m", false).as_deref(), Ok("2.4"));
        assert_eq!(get("length*0.7", false).as_deref(), Ok("1.68m"));
        assert_eq!(get("(n - 1) * 25cm", true).as_deref(), Ok("1.5m"));
        assert_eq!(get("2*35deg", false).as_deref(), Ok("70deg"));
        // Degrees are kept through a step that is no angle.
        assert_eq!(get("35deg*2m/1m", false).as_deref(), Ok("70deg"));
        assert_eq!(get("35deg/5deg", false).as_deref(), Ok("7"));
        assert!(get("35deg + 1rad", true).is_err());
        assert!(get("35deg/1rad", false).is_err());
        assert!(get("35deg*35deg", false).is_err());
        assert!(get("length/0", false).is_err());
        assert!(get("length + 1", true).is_err());
        assert!(get("length*length", false).is_ok_and(|t| t.starts_with("5.76")));
    }

    #[test]
    fn conditions_compare_one_kind_in_one_unit() {
        let mut scope = HashMap::new();
        scope.insert("tilt".to_string(), literal("30deg").unwrap());
        scope.insert("length".to_string(), literal("1m").unwrap());
        let holds_ = |text: &str| holds(&expr::parse_condition(text).unwrap(), &scope);
        assert_eq!(holds_("tilt > 20deg"), Ok(true));
        assert!(holds_("tilt > 1rad").is_err());
        assert!(holds_("tilt > length").is_err());
        assert_eq!(holds_("length > 99cm"), Ok(true));
    }
}
