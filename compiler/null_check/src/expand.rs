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
use std::sync::Arc;

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

/// One file, expanded, with the map from its text back to the files as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expanded {
    /// The file, as given.
    pub file: FileId,
    /// Its text, expanded.
    pub text: String,
    /// Its pieces in order, each with where it came from ([`Expanded::origin`]).
    pieces: Vec<Piece>,
    /// Every copy the expansion made, which pieces name by index.
    copies: Arc<Vec<String>>,
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
        (Val::Num { dim: da, .. }, Val::Num { dim: db, .. }) if da == db => {
            Err(expr::mixed_angles(what))
        }
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
                '*' => (
                    qa.mul(qb).ok_or(TOO_LARGE)?,
                    da.checked_times(db).ok_or(TOO_LARGE)?,
                    ga.checked_add(gb).ok_or(TOO_LARGE)?,
                ),
                _ if qb.n == 0 => return Err("a division by zero".into()),
                _ => (
                    qa.div(qb).ok_or(TOO_LARGE)?,
                    da.checked_over(db).ok_or(TOO_LARGE)?,
                    ga.checked_sub(gb).ok_or(TOO_LARGE)?,
                ),
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

/// The handle a datablock's line defines: written, or `@` and the kebab-case form of its display
/// name (S18), as the checker reads it.
fn handle_of(node: Node<'_>) -> Option<String> {
    if let Some(h) = node.handle() {
        return Some(h.text.to_string());
    }
    let name = node.args().find_map(|v| match v {
        Value::Str(_) | Value::Raw(_) => v.as_string(),
        _ => None,
    })?;
    Some(format!("@{}", null_syntax::names::to_kebab_case(&name)?))
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

/// One piece of expanded text and where it came from: `start..end` in the expanded text, `span`
/// in `file` as written, and the copy whose expansion wrote it (an index into the copies, plus
/// one; 0 for text outside every copy).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Piece {
    start: usize,
    end: usize,
    file: FileId,
    span: Span,
    copy: usize,
}

/// Expanded text being written, with the map from its pieces back to the files as written.
#[derive(Clone, Debug, Default)]
struct Text {
    s: String,
    map: Vec<Piece>,
}

impl Text {
    fn put(&mut self, s: &str) {
        self.s.push_str(s);
    }

    /// `s`, written for `span` of `file` (in copy `copy`, see [`Piece`]).
    fn put_from(&mut self, s: &str, file: FileId, span: Span, copy: usize) {
        let start = self.s.len();
        self.s.push_str(s);
        self.map.push(Piece {
            start,
            end: self.s.len(),
            file,
            span,
            copy,
        });
    }

    fn append(&mut self, other: Text) {
        let shift = self.s.len();
        self.s.push_str(&other.s);
        self.map.extend(other.map.into_iter().map(|p| Piece {
            start: p.start + shift,
            end: p.end + shift,
            ..p
        }));
    }
}

/// Where a span of expanded text came from (task 1d-03h): its place in the files as written, and
/// the copy whose expansion wrote it, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin {
    /// The file as written.
    pub file: FileId,
    /// The span there: the same text where the expansion wrote it as it stands, else the whole
    /// of what made it (an expression's value, a copy's handle, a member line's placement).
    pub span: Span,
    /// The copy whose expansion wrote it (`@plank.7`), none for text outside every copy.
    pub copy: Option<String>,
}

/// A line laid over a datablock, with what names mean where it was written.
type Over<'a> = (Node<'a>, Ctx);

struct Ex<'a, 's> {
    schema: &'s Schema,
    /// Each file's text, to find the file a node is in.
    texts: Vec<(FileId, &'a str)>,
    comps: HashMap<String, Node<'a>>,
    /// Override lines by the path they name, as written.
    overrides: HashMap<String, Vec<Node<'a>>>,
    made: usize,
    /// The problems, each with the copy it was found in.
    out: Vec<(Diagnostic, Option<String>)>,
    stopped: bool,
    /// The components being expanded, with their countdown's value, innermost last.
    stack: Vec<(String, Option<Q>)>,
    /// Every copy made, in order; and the one being written, as an index plus one.
    copies: Vec<String>,
    current: usize,
}

impl<'a> Ex<'a, '_> {
    /// The file `node` is in.
    fn file_of(&self, node: Node<'a>) -> FileId {
        let text = node.0.file_text();
        self.texts
            .iter()
            .find(|(_, t)| std::ptr::eq(t.as_ptr(), text.as_ptr()))
            .map_or(FileId(0), |(f, _)| *f)
    }

    /// A problem at `span` in the file of `node`, in the copy being written.
    fn error(&mut self, node: Node<'a>, span: Span, message: impl Into<String>) {
        let file = self.file_of(node);
        let copy = self.copy_name();
        self.out.push((
            Diagnostic::error(codes::TYPE, file, span, message.into()),
            copy,
        ));
    }

    fn copy_name(&self) -> Option<String> {
        self.current.checked_sub(1).map(|i| self.copies[i].clone())
    }

    /// `s`, written for `span` of `node`'s file in the copy being written.
    fn put(&self, out: &mut Text, s: &str, node: Node<'a>, span: Span) {
        out.put_from(s, self.file_of(node), span, self.current);
    }

    fn is_datablock(&self, kind: &str) -> bool {
        self.schema.datablock_kinds().any(|s| s.kind == kind)
    }

    /// A component's datablocks with their handles, those under its `if`s and `else`s
    /// included, in order.
    fn datablocks(&self, node: Node<'a>, out: &mut Vec<(Node<'a>, String)>) {
        for c in node.children() {
            match c.name().as_ref() {
                "if" | "else" => self.datablocks(c, out),
                kind if self.is_datablock(kind) => {
                    if let Some(h) = handle_of(c) {
                        out.push((c, h));
                    }
                }
                _ => {}
            }
        }
    }

    /// A block's lines, conditions decided: an `if`'s when it holds, else its `else`'s.
    fn lines(&mut self, node: Node<'a>, ctx: &Ctx) -> Vec<Node<'a>> {
        let mut out = Vec::new();
        let mut last_if: Option<bool> = None;
        for c in node.children() {
            match c.name().as_ref() {
                "if" => {
                    let held = self.condition(c, ctx);
                    if held {
                        out.extend(self.lines(c, ctx));
                    }
                    last_if = Some(held);
                }
                "else" => {
                    if last_if == Some(false) {
                        out.extend(self.lines(c, ctx));
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

    fn condition(&mut self, node: Node<'a>, ctx: &Ctx) -> bool {
        let Some(text) = node.args().next().and_then(|v| v.as_string()) else {
            return false; // the checker says so
        };
        match expr::parse_condition(&text).and_then(|c| holds(&c, &ctx.values)) {
            Ok(b) => b,
            Err(e) => {
                self.error(node, node.code_span(), format!("`{text}`: {e}"));
                false
            }
        }
    }

    /// The override lines that name `path`, their conditions decided, in the project's terms.
    fn overrides_of(&mut self, path: &str) -> Vec<Over<'a>> {
        let nodes = self.overrides.get(path).cloned().unwrap_or_default();
        let project = Ctx::default();
        let mut out = Vec::new();
        for o in nodes {
            out.extend(
                self.lines(o, &project)
                    .into_iter()
                    .map(|l| (l, Ctx::default())),
            );
        }
        out
    }

    /// One node as written in a copy: its line, its entries in the copy's terms (its first
    /// argument `head` where given), its block's lines with `over` laid on them. `owner` is the
    /// datablock's path, which its sub-objects' overrides name (`@plank.7/@fluid`). Each name
    /// and entry is mapped to where it is written (task 1d-03h).
    #[allow(clippy::too_many_arguments)]
    fn emit(
        &mut self,
        node: Node<'a>,
        ctx: &Ctx,
        depth: usize,
        over: &[Over<'a>],
        head: Option<&str>,
        owner: &str,
        out: &mut Text,
    ) {
        let pad = "  ".repeat(depth);
        out.put(&pad);
        let name = node.name().into_owned();
        let name_span = node.name_token().map_or(node.span(), |t| t.span);
        self.put(out, &name, node, name_span);
        // A datablock whose handle is made from its name (S18) has its copy's handle written.
        let written = node.handle().is_some();
        if let Some(h) = head
            && !written
        {
            out.put(" ");
            self.put(out, h, node, name_span);
        }
        for (i, e) in node.entries().enumerate() {
            let text = match (e, head) {
                (Entry::Arg(_), Some(h)) if i == 0 && written => Ok(h.to_string()),
                (Entry::Arg(v), _) => ctx.value(&v),
                (Entry::Prop { key, value }, _) => {
                    ctx.value(&value).map(|v| format!("{}={v}", key.text))
                }
                (Entry::Keyword { key, value }, _) => {
                    ctx.value(&value).map(|v| format!("{} {v}", key.text))
                }
            };
            let span = match e {
                Entry::Arg(v) => v.span(),
                Entry::Prop { key, value } | Entry::Keyword { key, value } => {
                    key.span.cover(value.span())
                }
            };
            match text {
                Ok(t) => {
                    out.put(" ");
                    self.put(out, &t, node, span);
                }
                Err(m) => self.error(node, e.value().span(), m),
            }
        }
        let children = self.lines(node, ctx);
        // The layers in order (the placement's lines, then each override's): a line replaces
        // the copy's line, and an earlier layer's, of its name; one that repeats (`modifier`),
        // those of its name and handle; else it is added.
        let mut layers: Vec<Over<'a>> = Vec::new();
        for (o, octx) in over {
            layers.retain(|(p, _)| !same_line(*p, *o));
            layers.push((*o, octx.clone()));
        }
        if children.is_empty() && layers.is_empty() && node.block().is_none() {
            out.put("\n");
            return;
        }
        out.put(" {\n");
        // A line laid over takes the place of the first line it replaces, so a stack keeps its
        // order; the copy's other lines it replaces go, and a line that replaces none is added.
        let mut used = vec![false; layers.len()];
        for c in children {
            match layers.iter().position(|(o, _)| same_line(*o, c)) {
                Some(i) if used[i] => {}
                Some(i) => {
                    used[i] = true;
                    let (o, octx) = &layers[i];
                    self.child(*o, octx, depth + 1, owner, out);
                }
                None => self.child(c, ctx, depth + 1, owner, out),
            }
        }
        for (i, (o, octx)) in layers.iter().enumerate() {
            if !used[i] {
                self.child(*o, octx, depth + 1, owner, out);
            }
        }
        out.put(&format!("{pad}}}\n"));
    }

    /// A line inside a datablock `owner` (`@rig.2`) or a sub-object of one: a sub-object's line
    /// with the overrides that name its path (`@rig.2/@spine`) laid over it, its own lines then
    /// inside that path, so an override reaches a sub-object of a sub-object
    /// (`@rig.2/@spine/@reach`, task 1d-03i). A node tree's nodes, and a zone's, are their
    /// owner's (section 7).
    fn child(&mut self, node: Node<'a>, ctx: &Ctx, depth: usize, owner: &str, out: &mut Text) {
        let path = first_word(node)
            .filter(|h| h.starts_with('@'))
            .map(|h| format!("{owner}/{h}"));
        let lay = match &path {
            Some(p) if self.overrides.contains_key(p) => self.overrides_of(p),
            _ => Vec::new(),
        };
        let inner = match &path {
            Some(p) if node.name() != "zone" => p.clone(),
            _ => owner.to_string(),
        };
        self.emit(node, ctx, depth, &lay, None, &inner, out);
    }

    /// A placement (`use`), in `ctx` (the placing copy's, or the project's), its copies' handles
    /// under `prefix`: their datablocks into `out`, their roots' handles into `members`, each
    /// with its index among the copies.
    fn placement(
        &mut self,
        node: Node<'a>,
        ctx: &Ctx,
        prefix: Option<&str>,
        out: &mut Text,
        members: &mut Vec<(String, usize)>,
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
                return self.limit(node);
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
                        self.error(node, v.span(), m);
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
            self.copy(comp, node, name, &copy, &outer, out, members);
        }
    }

    fn limit(&mut self, node: Node<'a>) {
        self.error(
            node,
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
                Err(m) => self.error(l, v.span(), m),
            }
        }
        root
    }

    /// One copy of component `comp` (handle `name`), placed by `placement` as `copy`.
    #[allow(clippy::too_many_arguments)]
    fn copy(
        &mut self,
        comp: Node<'a>,
        placement: Node<'a>,
        name: &str,
        copy: &str,
        outer: &Ctx,
        out: &mut Text,
        members: &mut Vec<(String, usize)>,
    ) {
        // The copy's place among the copies, which every piece it writes is tagged with.
        self.copies.push(copy.to_string());
        let index = self.copies.len();
        let placing = self.current;
        self.current = index;
        self.copy_body(comp, placement, name, copy, outer, out, members, index);
        self.current = placing;
    }

    #[allow(clippy::too_many_arguments)]
    fn copy_body(
        &mut self,
        comp: Node<'a>,
        placement: Node<'a>,
        name: &str,
        copy: &str,
        outer: &Ctx,
        out: &mut Text,
        members: &mut Vec<(String, usize)>,
        index: usize,
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
        let placed = self.lines(placement, outer);
        let mut root_over = self.given(placed, outer, &params, &mut values);
        let project = Ctx::default();
        let mut overridden = Vec::new();
        for o in self.overrides.get(copy).cloned().unwrap_or_default() {
            overridden.extend(self.lines(o, &project));
        }
        root_over.extend(self.given(overridden, &project, &params, &mut values));
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
                    placement,
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
                placement,
                placement.code_span(),
                format!(
                    "copies inside copies nest deeper than {DEPTH} here (S76): count down from less"
                ),
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
        let mut datablocks = Vec::new();
        self.datablocks(comp, &mut datablocks);
        for (c, h) in datablocks {
            if c.name() == "object" && root.is_none() {
                root = Some(h.clone());
                ctx.handles.insert(h, copy.to_string());
            } else {
                ctx.handles.insert(h.clone(), format!("{copy}/{h}"));
            }
        }
        members.push((copy.to_string(), index));
        self.stack.push((name.to_string(), value));
        for c in self.lines(comp, &ctx) {
            if self.stopped {
                break;
            }
            let kind = c.name().into_owned();
            if kind == "use" {
                self.placement(c, &ctx, Some(copy), out, members);
                continue;
            }
            if !self.is_datablock(&kind) {
                continue; // `param`
            }
            if self.made >= LIMIT {
                self.limit(placement);
                break;
            }
            self.made += 1;
            let Some(h) = handle_of(c) else {
                continue; // the checker says so
            };
            let new = ctx.handles.get(&h).cloned().unwrap_or_else(|| h.clone());
            let over = if root.as_deref() == Some(h.as_str()) {
                root_over.clone()
            } else {
                self.overrides_of(&new)
            };
            out.put("\n");
            self.emit(c, &ctx, 0, &over, Some(&new), &new, out);
        }
        self.stack.pop();
    }

    /// A scene or a collection with placements: its lines as written, each `use` made into its
    /// copies' members (the copies' datablocks into `made`), its `override` lines taken out.
    fn container(&mut self, node: Node<'a>, depth: usize, out: &mut Text, made: &mut Text) {
        let pad = "  ".repeat(depth);
        let start = node.code_span().start;
        let own = node
            .block()
            .map_or(node.code_span().end, |b| b.span().start);
        let header = text_in(node, Span::new(start, own)).trim_end();
        out.put(&pad);
        self.put(out, header, node, Span::new(start, start + header.len()));
        out.put(" {\n");
        for c in node.children() {
            match c.name().as_ref() {
                "use" => {
                    let mut members = Vec::new();
                    self.placement(c, &Ctx::default(), None, made, &mut members);
                    // A member line is its placement's: the `use` line's handle.
                    let at = c.args().nth(1).map_or(c.code_span(), |v| v.span());
                    let file = self.file_of(c);
                    for (m, index) in members {
                        out.put(&format!("{pad}  object "));
                        out.put_from(&m, file, at, index);
                        out.put("\n");
                    }
                }
                "override" => {}
                "collection" if c.block().is_some() && has_placements(c) => {
                    self.container(c, depth + 1, out, made);
                }
                _ => {
                    out.put(&format!("{pad}  "));
                    self.put(out, text_in(c, c.code_span()), c, c.code_span());
                    out.put("\n");
                }
            }
        }
        out.put(&format!("{pad}}}\n"));
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

/// The copies a problem was found in, as a note: "in copy `@plank.7`", or "in 20 copies:
/// `@plank.1`, `@plank.2`, … and 15 more".
fn copies_note(copies: &[String]) -> String {
    let shown: Vec<String> = copies.iter().take(5).map(|c| format!("`{c}`")).collect();
    match copies.len() {
        1 => format!("in copy {}", shown[0]),
        n if n <= 5 => format!("in {n} copies: {}", shown.join(", ")),
        n => format!("in {n} copies: {} and {} more", shown.join(", "), n - 5),
    }
}

/// Problems found once per copy, gathered: one per code and place, with the copies it was found
/// in as a note (task 1d-03h), in the files' text order.
fn gather(found: Vec<(Diagnostic, Option<String>)>) -> Vec<Diagnostic> {
    let mut out: Vec<(Diagnostic, Vec<String>)> = Vec::new();
    let mut at: HashMap<(String, u32, usize, usize), usize> = HashMap::new();
    for (d, copy) in found {
        let key = (
            d.code.to_string(),
            d.primary.file.0,
            d.primary.span.start,
            d.primary.span.end,
        );
        let i = *at.entry(key).or_insert_with(|| {
            out.push((d, Vec::new()));
            out.len() - 1
        });
        if let Some(c) = copy
            && !out[i].1.contains(&c)
        {
            out[i].1.push(c);
        }
    }
    let mut out: Vec<Diagnostic> = out
        .into_iter()
        .map(|(d, copies)| {
            if copies.is_empty() {
                d
            } else {
                d.with_note(copies_note(&copies))
            }
        })
        .collect();
    out.sort_by_key(|d| (d.primary.file.0, d.primary.span.start, d.primary.span.end));
    out
}

/// Every file of a project expanded: components taken out, each placement made into its copies'
/// datablocks (after the scene or collection that places them, in its file) and its copies'
/// roots as members there; overrides laid over the copies they name; every other line as
/// written. Each file keeps a map back to the files as written ([`Expanded::origin`]). The
/// problems are the expansion's own (a condition or value that cannot be computed, a recursion
/// that does not count down, the limits), each once where it is written, with the copies it was
/// found in; the checker's are [`crate::check`]'s, on the files as written, which should be
/// clean before the expansion is trusted.
pub fn expand(schema: &Schema, files: &[(FileId, &Parse)]) -> (Vec<Expanded>, Vec<Diagnostic>) {
    let mut ex = Ex {
        schema,
        texts: files.iter().map(|(f, p)| (*f, p.text())).collect(),
        comps: HashMap::new(),
        overrides: HashMap::new(),
        made: 0,
        out: Vec::new(),
        stopped: false,
        stack: Vec::new(),
        copies: Vec::new(),
        current: 0,
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
    let mut texts = Vec::new();
    for (file, p) in files {
        // The text as written, but for components (taken out) and the scenes and collections
        // that place copies (written with their copies, which follow them).
        let text = p.text();
        let mut out = Text::default();
        let mut at = 0;
        for node in p.document().nodes() {
            let kind = node.name();
            let placing = matches!(kind.as_ref(), "scene" | "collection") && has_placements(node);
            if kind != "component" && !placing {
                continue;
            }
            let span = node.span();
            out.put_from(&text[at..span.start], *file, Span::new(at, span.start), 0);
            at = span.end;
            if placing {
                let mut made = Text::default();
                ex.container(node, 0, &mut out, &mut made);
                out.append(made);
            }
        }
        out.put_from(&text[at..], *file, Span::new(at, text.len()), 0);
        texts.push((*file, out));
    }
    let copies = Arc::new(std::mem::take(&mut ex.copies));
    let result = texts
        .into_iter()
        .map(|(file, t)| Expanded {
            file,
            text: t.s,
            pieces: t.map.into_iter().filter(|p| p.start < p.end).collect(),
            copies: Arc::clone(&copies),
        })
        .collect();
    (result, gather(ex.out))
}

impl Expanded {
    /// An expanded file with no map: its spans are its own (for text made otherwise, such as a
    /// test's).
    pub fn unmapped(file: FileId, text: String) -> Expanded {
        Expanded {
            file,
            text,
            pieces: Vec::new(),
            copies: Arc::new(Vec::new()),
        }
    }

    /// Where `span` of the expanded text came from (task 1d-03h): the piece that holds its start,
    /// else the last before it on its way (a brace, a line's end). Within a piece written as it
    /// stands the place is exact; within one the expansion made (a computed value, a copy's
    /// handle) it is the whole of what made it. `None` for an unmapped file.
    pub fn origin(&self, span: Span) -> Option<Origin> {
        let i = self.pieces.partition_point(|p| p.start <= span.start);
        let p = self.pieces.get(i.checked_sub(1)?)?;
        let exact = p.end - p.start == p.span.end - p.span.start;
        let span = if exact && span.start < p.end {
            let start = p.span.start + (span.start - p.start);
            Span::new(
                start,
                (start + span.end.saturating_sub(span.start)).min(p.span.end),
            )
        } else {
            p.span
        };
        Some(Origin {
            file: p.file,
            span,
            copy: p.copy.checked_sub(1).map(|c| self.copies[c].clone()),
        })
    }
}

/// The expanded files checked: their syntax, and [`crate::check`] on them, their copy handles
/// (`@plank.7`, `@plank.7/@plank-mesh`) taken as the handles they are. Each problem is reported
/// where it is written in the files as given, through each file's map ([`Expanded::origin`]):
/// a mistake in a component's line once, at that line, with the copies it affects as a note
/// (task 1d-03h). A fix is kept where every edit maps exactly. A file with no map keeps its
/// expanded spans.
pub fn check_expanded(schema: &Schema, expanded: &[Expanded]) -> Vec<Diagnostic> {
    let parses: Vec<(FileId, Parse)> = expanded.iter().map(|e| (e.file, parse(&e.text))).collect();
    let files: Vec<(FileId, &Parse)> = parses.iter().map(|(f, p)| (*f, p)).collect();
    let mut found: Vec<Diagnostic> = files
        .iter()
        .flat_map(|(f, p)| null_syntax::diagnostics(p, *f))
        .collect();
    found.extend(crate::check::check_flat(schema, &files));
    let of = |file: FileId| expanded.iter().find(|e| e.file == file);
    let mapped = found
        .into_iter()
        .map(|mut d| {
            let Some(e) = of(d.primary.file).filter(|e| !e.pieces.is_empty()) else {
                return (d, None);
            };
            let Some(o) = e.origin(d.primary.span) else {
                return (d, None);
            };
            d.primary.file = o.file;
            d.primary.span = o.span;
            for l in &mut d.secondary {
                if let Some(lo) = of(l.location.file).and_then(|e| e.origin(l.location.span)) {
                    l.location.file = lo.file;
                    l.location.span = lo.span;
                }
            }
            // A fix's edits must land exactly where they are written, else it is left out.
            d.fixes.retain_mut(|f| {
                f.edits.iter_mut().all(|edit| {
                    let Some(target) = of(edit.file) else {
                        return false;
                    };
                    let Some(eo) = target.origin(edit.span) else {
                        return false;
                    };
                    let exact = target.pieces.iter().any(|p| {
                        p.start <= edit.span.start
                            && edit.span.end <= p.end
                            && p.end - p.start == p.span.end - p.span.start
                    });
                    edit.file = eo.file;
                    edit.span = eo.span;
                    exact
                })
            });
            (d, o.copy)
        })
        .collect();
    gather(mapped)
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
    fn a_power_past_what_its_counter_holds_is_an_error() {
        // 128 degrees multiplied: the power of degrees passes an i8, and the length's too.
        let scope = HashMap::new();
        for unit in ["deg", "m"] {
            let text = vec![format!("1{unit}"); 128].join("*");
            let e = expr::parse(&text, false).unwrap();
            assert!(eval(&e, &scope).is_err(), "{unit}");
        }
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
