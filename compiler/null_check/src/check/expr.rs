//! Arithmetic and conditions in a component (spec, section 8; Sau's decision S74 (3); task
//! 1d-03b): what a value written as an expression is, before any value is computed.
//!
//! Inside a word, `+`, `*` and `/` join numbers with units and names (`n*25cm`, `length/1m`).
//! A quoted expression may also subtract, group and compare, with spaces around each operator
//! (`"depth - 1"`, `"(n - 1) * 25cm"`, `"state == liquid"`). A `-` inside a word is part of a
//! name, since names are kebab-case. The checker reads an expression's kind of value here: its
//! units, as powers of length, angle, mass and time (and the units Null keeps apart: a lens's,
//! power, temperature, wavelength), and whether it is a whole number. Computing the value,
//! exactly and rounded once into the property's type (S59), is the expansion's (task 1d-03c).

use null_value::{Dimension, Unit};

/// Powers of the base quantities: length, angle, mass, time, a lens's millimetre, power,
/// temperature, wavelength, colour temperature.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct Dim([i8; 9]);

const L: usize = 0;
const A: usize = 1;
const M: usize = 2;
const T: usize = 3;
const LENS: usize = 4;
const P: usize = 5;
const K: usize = 6;
const WAVE: usize = 7;
const CK: usize = 8;

impl Dim {
    pub(crate) const NONE: Dim = Dim([0; 9]);

    fn base(i: usize, power: i8) -> Dim {
        let mut d = [0; 9];
        d[i] = power;
        Dim(d)
    }

    fn times(self, o: Dim) -> Dim {
        let mut d = self.0;
        for (x, y) in d.iter_mut().zip(o.0) {
            *x += y;
        }
        Dim(d)
    }

    fn over(self, o: Dim) -> Dim {
        let mut d = self.0;
        for (x, y) in d.iter_mut().zip(o.0) {
            *x -= y;
        }
        Dim(d)
    }

    /// What a dimension of Null's units is, as powers.
    pub(crate) fn of(d: Dimension) -> Dim {
        match d {
            Dimension::None | Dimension::Frames => Dim::NONE,
            Dimension::Length => Dim::base(L, 1),
            Dimension::Area => Dim::base(L, 2),
            Dimension::Volume => Dim::base(L, 3),
            Dimension::Mass => Dim::base(M, 1),
            Dimension::Angle => Dim::base(A, 1),
            Dimension::Seconds => Dim::base(T, 1),
            Dimension::Velocity => Dim::base(L, 1).times(Dim::base(T, -1)),
            Dimension::Acceleration => Dim::base(L, 1).times(Dim::base(T, -2)),
            Dimension::Lens => Dim::base(LENS, 1),
            Dimension::Power => Dim::base(P, 1),
            Dimension::Temperature => Dim::base(K, 1),
            Dimension::Wavelength => Dim::base(WAVE, 1),
            Dimension::ColorTemperature => Dim::base(CK, 1),
            Dimension::Frequency => Dim::base(T, -1),
        }
    }

    /// The dimension of Null's that this is, if one is.
    pub(crate) fn named(self) -> Option<Dimension> {
        Dimension::ALL
            .into_iter()
            .filter(|d| *d != Dimension::Frames)
            .find(|d| Dim::of(*d) == self)
    }

    /// Whether a value of this dimension may stand where `want` goes: the same powers, or, for
    /// a time in frames, a bare number or seconds (section 6, "Frames").
    pub(crate) fn fits(self, want: Dimension) -> bool {
        self == Dim::of(want) || (want == Dimension::Frames && self == Dim::base(T, 1))
    }

    /// In words: "a length", "a number", "length squared times …".
    pub(crate) fn describe(self) -> String {
        if let Some(d) = self.named() {
            return match d {
                Dimension::None => "a number".into(),
                Dimension::ColorTemperature => "a colour temperature".into(),
                other => {
                    let w = format!("{other:?}").to_lowercase();
                    let an = w.starts_with(['a', 'e', 'i', 'o', 'u']);
                    format!("{} {w}", if an { "an" } else { "a" })
                }
            };
        }
        let names = ["m", "rad", "kg", "s", "lens-mm", "W", "K", "nm", "colour-K"];
        let parts: Vec<String> = self
            .0
            .iter()
            .zip(names)
            .filter(|(p, _)| **p != 0)
            .map(|(p, n)| {
                if *p == 1 {
                    n.to_string()
                } else {
                    format!("{n}^{p}")
                }
            })
            .collect();
        format!("a value in {}", parts.join("·"))
    }
}

/// What an expression's value is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A number with units, and whether it is a whole number.
    Quantity { dim: Dim, whole: bool },
    /// `on` or `off`.
    Bool,
    /// A handle of a kind of datablock.
    Handle(String),
    /// A word from a list.
    Word(Vec<String>),
}

impl Kind {
    pub(crate) fn number(dim: Dim) -> Kind {
        Kind::Quantity { dim, whole: false }
    }

    pub(crate) fn describe(&self) -> String {
        match self {
            Kind::Quantity { dim, whole: true } if *dim == Dim::NONE => "a whole number".into(),
            Kind::Quantity { dim, .. } => dim.describe(),
            Kind::Bool => "`on` or `off`".into(),
            Kind::Handle(k) => format!("a handle of a {k}"),
            Kind::Word(items) => format!("one of [{}]", items.join(" ")),
        }
    }
}

/// An expression as read.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Expr {
    /// A number, with its unit's dimension, whole if written as one.
    Number { dim: Dim, whole: bool },
    /// A name: a parameter, a pattern's `n`, an `each=` item, or a word.
    Name(String),
    /// `-x`.
    Neg(Box<Expr>),
    /// `a op b`, `op` one of `+ - * /`.
    Op(char, Box<Expr>, Box<Expr>),
}

/// A condition as read: `a <op> b`, `op` one of `== != < > <= >=`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Condition {
    pub left: Expr,
    pub op: &'static str,
    pub right: Expr,
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(Dim, bool),
    Name(String),
    Op(char),
    Cmp(&'static str),
    Open,
    Close,
}

/// The longest unit written at the start of `rest`, if any: units may hold `/` (`m/s`), which
/// is also division, so the longest known unit wins (`9.81m/s2`), and division otherwise
/// (`25cm/2`).
fn unit_at(rest: &str) -> Option<(Unit, usize)> {
    let end = rest
        .find(['+', '*', ' ', ')', '(', '=', '<', '>', '!'])
        .unwrap_or(rest.len());
    let candidate = &rest[..end];
    (1..=candidate.len())
        .rev()
        .filter(|&n| candidate.is_char_boundary(n))
        .find_map(|n| Unit::from_suffix(&candidate[..n], Dimension::None).map(|u| (u, n)))
}

fn tokens(text: &str, quoted: bool) -> Result<Vec<Tok>, String> {
    let mut out = Vec::new();
    let mut i = 0;
    let bytes = text.as_bytes();
    while i < text.len() {
        let c = bytes[i] as char;
        let rest = &text[i..];
        if c == ' ' {
            if !quoted {
                return Err("a space: write an expression with spaces in quotes".into());
            }
            i += 1;
        } else if c.is_ascii_digit()
            || (c == '.' && rest[1..].starts_with(|d: char| d.is_ascii_digit()))
        {
            let len = rest
                .find(|d: char| !(d.is_ascii_digit() || d == '.' || d == '_'))
                .unwrap_or(rest.len());
            // An exponent: `1e3`, `2.5e-4`.
            let mut len = len;
            if rest[len..].starts_with(['e', 'E'])
                && rest[len + 1..]
                    .trim_start_matches(['+', '-'])
                    .starts_with(|d: char| d.is_ascii_digit())
            {
                let after = &rest[len + 1..];
                let sign = after.len() - after.trim_start_matches(['+', '-']).len();
                let digits = after[sign..]
                    .find(|d: char| !d.is_ascii_digit())
                    .unwrap_or(after.len() - sign);
                len += 1 + sign + digits;
            }
            let number = &rest[..len];
            let whole = !number.contains(['.', 'e', 'E']);
            let (dim, used) = match unit_at(&rest[len..]) {
                Some((u, n)) if !rest[len..].starts_with(|d: char| d.is_ascii_digit()) => {
                    (Dim::of(u.dimension()), n)
                }
                _ if rest[len..].starts_with(|d: char| d.is_ascii_alphabetic()) => {
                    let end = rest[len..]
                        .find(|d: char| !d.is_ascii_alphanumeric())
                        .unwrap_or(rest.len() - len);
                    return Err(format!("`{}` is not a unit", &rest[len..len + end]));
                }
                _ => (Dim::NONE, 0),
            };
            out.push(Tok::Num(dim, whole && dim == Dim::NONE));
            i += len + used;
        } else if c.is_ascii_alphabetic() {
            // A name: kebab-case in a word; in quotes a `-` with spaces is subtraction.
            let len = rest
                .find(|d: char| !(d.is_ascii_alphanumeric() || d == '-'))
                .unwrap_or(rest.len());
            let name = rest[..len].trim_end_matches('-');
            out.push(Tok::Name(name.to_string()));
            i += name.len();
        } else if matches!(c, '+' | '*' | '/') {
            out.push(Tok::Op(c));
            i += 1;
        } else if c == '-' {
            if !quoted {
                return Err("a `-` inside a word is part of a name; write subtraction in quotes, with spaces (`\"depth - 1\"`)".into());
            }
            out.push(Tok::Op('-'));
            i += 1;
        } else if c == '(' || c == ')' {
            if !quoted {
                return Err("Scowr reserves `( )`: write grouping in quotes".into());
            }
            out.push(if c == '(' { Tok::Open } else { Tok::Close });
            i += 1;
        } else if let Some(op) = ["==", "!=", "<=", ">=", "<", ">"]
            .into_iter()
            .find(|o| rest.starts_with(o))
        {
            if !quoted {
                return Err("write a comparison in quotes".into());
            }
            out.push(Tok::Cmp(op));
            i += op.len();
        } else {
            return Err(format!("`{c}` is not part of an expression"));
        }
    }
    Ok(out)
}

struct Parser {
    toks: Vec<Tok>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.at)
    }

    fn sum(&mut self) -> Result<Expr, String> {
        let mut left = self.product()?;
        while let Some(Tok::Op(op @ ('+' | '-'))) = self.peek().cloned() {
            self.at += 1;
            left = Expr::Op(op, Box::new(left), Box::new(self.product()?));
        }
        Ok(left)
    }

    fn product(&mut self) -> Result<Expr, String> {
        let mut left = self.factor()?;
        while let Some(Tok::Op(op @ ('*' | '/'))) = self.peek().cloned() {
            self.at += 1;
            left = Expr::Op(op, Box::new(left), Box::new(self.factor()?));
        }
        Ok(left)
    }

    fn factor(&mut self) -> Result<Expr, String> {
        let t = self.peek().cloned();
        self.at += 1;
        match t {
            Some(Tok::Num(dim, whole)) => Ok(Expr::Number { dim, whole }),
            Some(Tok::Name(n)) => Ok(Expr::Name(n)),
            Some(Tok::Op('-')) => Ok(Expr::Neg(Box::new(self.factor()?))),
            Some(Tok::Open) => {
                let e = self.sum()?;
                match self.peek() {
                    Some(Tok::Close) => {
                        self.at += 1;
                        Ok(e)
                    }
                    _ => Err("a `(` with no `)`".into()),
                }
            }
            Some(other) => Err(format!("{} where a value goes", show(&other))),
            None => Err("the expression ends where a value goes".into()),
        }
    }
}

fn show(t: &Tok) -> String {
    match t {
        Tok::Op(c) => format!("`{c}`"),
        Tok::Cmp(c) => format!("`{c}`"),
        Tok::Open => "`(`".into(),
        Tok::Close => "`)`".into(),
        Tok::Num(..) => "a number".into(),
        Tok::Name(n) => format!("`{n}`"),
    }
}

/// `text` read as an expression: a word's (`quoted` false) or a quoted one's.
pub(crate) fn parse(text: &str, quoted: bool) -> Result<Expr, String> {
    let mut p = Parser {
        toks: tokens(text, quoted)?,
        at: 0,
    };
    let e = p.sum()?;
    match p.peek() {
        None => Ok(e),
        Some(t) => Err(format!("{} after a complete expression", show(t))),
    }
}

/// `text`, quoted, read as a condition.
pub(crate) fn parse_condition(text: &str) -> Result<Condition, String> {
    let toks = tokens(text, true)?;
    let Some(at) = toks.iter().position(|t| matches!(t, Tok::Cmp(_))) else {
        return Err("a condition compares two values: `==`, `!=`, `<`, `>`, `<=` or `>=`".into());
    };
    let Tok::Cmp(op) = toks[at] else {
        unreachable!()
    };
    let side = |toks: Vec<Tok>| -> Result<Expr, String> {
        let mut p = Parser { toks, at: 0 };
        let e = p.sum()?;
        match p.peek() {
            None => Ok(e),
            Some(t) => Err(format!("{} after a complete value", show(t))),
        }
    };
    Ok(Condition {
        left: side(toks[..at].to_vec())?,
        op,
        right: side(toks[at + 1..].to_vec())?,
    })
}

/// Whether a word is written as an expression rather than a plain value: it names something
/// (starts with a letter, `on` and `off` aside), or joins values with `+`, `*` or `/` (a unit
/// with a `/` in it, `9.81m/s2`, is a plain value).
pub(crate) fn is_expression(word: &str) -> bool {
    if word.starts_with('@') || word.starts_with('#') || matches!(word, "on" | "off") {
        return false;
    }
    if word.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return true;
    }
    if word.contains(['+', '*']) {
        return true;
    }
    // `/`: division unless the whole word is one number and a unit.
    word.contains('/') && null_value::unit_of(word, Dimension::None).is_none()
}

/// What `e` is, its names read through `lookup`; the error says what is wrong.
pub(crate) fn kind_of(e: &Expr, lookup: &dyn Fn(&str) -> Option<Kind>) -> Result<Kind, String> {
    match e {
        Expr::Number { dim, whole } => Ok(Kind::Quantity {
            dim: *dim,
            whole: *whole,
        }),
        Expr::Name(n) => lookup(n).ok_or_else(|| format!("no parameter is called `{n}`")),
        Expr::Neg(x) => match kind_of(x, lookup)? {
            q @ Kind::Quantity { .. } => Ok(q),
            other => Err(format!("{} cannot be negated", other.describe())),
        },
        Expr::Op(op, a, b) => {
            let (ka, kb) = (kind_of(a, lookup)?, kind_of(b, lookup)?);
            let (Kind::Quantity { dim: da, whole: wa }, Kind::Quantity { dim: db, whole: wb }) =
                (&ka, &kb)
            else {
                let bad = if matches!(ka, Kind::Quantity { .. }) {
                    &kb
                } else {
                    &ka
                };
                return Err(format!(
                    "{} is not a number to compute with",
                    bad.describe()
                ));
            };
            match op {
                '+' | '-' if da != db => Err(format!(
                    "{} {op} {}: only values of one kind add",
                    da.describe(),
                    db.describe()
                )),
                '+' | '-' => Ok(Kind::Quantity {
                    dim: *da,
                    whole: *wa && *wb,
                }),
                '*' => Ok(Kind::Quantity {
                    dim: da.times(*db),
                    whole: *wa && *wb,
                }),
                _ => Ok(Kind::number(da.over(*db))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(text: &str, quoted: bool) -> Result<Kind, String> {
        let e = parse(text, quoted)?;
        kind_of(&e, &|n| match n {
            "n" | "depth" => Some(Kind::Quantity {
                dim: Dim::NONE,
                whole: true,
            }),
            "length" => Some(Kind::number(Dim::of(Dimension::Length))),
            "state" => Some(Kind::Word(vec!["solid".into(), "liquid".into()])),
            _ => None,
        })
    }

    #[test]
    fn arithmetic_keeps_units() {
        let length = Dim::of(Dimension::Length);
        assert_eq!(
            kind("n*25cm", false),
            Ok(Kind::Quantity {
                dim: length,
                whole: false
            })
        );
        assert_eq!(kind("length/1m", false), Ok(Kind::number(Dim::NONE)));
        assert_eq!(kind("length*0.7", false), Ok(Kind::number(length)));
        assert_eq!(
            kind("depth - 1", true),
            Ok(Kind::Quantity {
                dim: Dim::NONE,
                whole: true
            })
        );
        assert_eq!(kind("(n - 1) * 25cm", true), Ok(Kind::number(length)));
        assert_eq!(
            kind("length*length", false).unwrap(),
            Kind::number(Dim::of(Dimension::Area))
        );
        assert_eq!(
            kind("9.81m/s2", false).unwrap(),
            Kind::number(Dim::of(Dimension::Acceleration))
        );
        assert_eq!(kind("25cm/2", false).unwrap(), Kind::number(length));
        assert!(
            kind("length+1", false)
                .unwrap_err()
                .contains("only values of one kind add")
        );
        assert!(
            kind("lenght*2", false)
                .unwrap_err()
                .contains("no parameter is called `lenght`")
        );
        assert!(kind("state*2", false).unwrap_err().contains("not a number"));
    }

    #[test]
    fn the_syntax_is_scowr_s() {
        assert!(parse("depth-1", false).is_ok()); // a name, `depth-1`
        assert!(parse("(n)", false).unwrap_err().contains("reserves"));
        assert!(parse("n * 2", false).unwrap_err().contains("space"));
        assert!(parse("2zz", false).unwrap_err().contains("not a unit"));
        assert!(parse("\"n +\"", true).is_err());
        let c = parse_condition("state == liquid").unwrap();
        assert_eq!(c.op, "==");
        assert!(parse_condition("depth").unwrap_err().contains("compares"));
    }

    #[test]
    fn plain_values_are_not_expressions() {
        for w in [
            "1m", "0.5", "9.81m/s2", "on", "@cube", "#ff0000", "-2", "1e3",
        ] {
            assert!(!is_expression(w), "{w}");
        }
        for w in ["n*25cm", "length", "length/1m", "2+2", "25cm/2"] {
            assert!(is_expression(w), "{w}");
        }
    }
}
