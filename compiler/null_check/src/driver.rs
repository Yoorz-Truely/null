//! Drivers in Null's expression syntax (spec, section 11; Sau's decisions S38 and S54; task
//! 1d-07f).
//!
//! A scripted driver's expression is written in quotes, `driver="spin * 100W"`. Null's
//! expression syntax is Blender's simple expressions, the Python subset Blender evaluates without
//! Python (`BLI_expr_pylike_eval`), written as Python writes them, with Null's units on numbers:
//! - numbers, `pi`, `True` and `False`, the driver's variables and `frame`;
//! - `+ - * /`, the comparisons (chained, as Python's), `and`, `or`, `not` and `a if c else b`,
//!   with parentheses;
//! - the subset's functions, each with its number of arguments: `min` and `max` of any number,
//!   `abs`, `fabs`, `floor`, `ceil`, `trunc`, `round`, `int`, `sin`, `cos`, `tan`, `asin`,
//!   `acos`, `atan`, `atan2`, `exp`, `log`, `sqrt`, `pow`, `fmod`, `lerp`, `clamp`, `smoothstep`,
//!   `radians` and `degrees`;
//! - a number may carry a unit (`100W`, `35deg`, `2m`), read as a value is (section 6) in the
//!   internal unit, through the scene's unit scale.
//!
//! [`compile`] gives the expression Blender is given: the text with each number in a unit
//! written as its internal value, exactly. An expression outside the subset is Python: it is kept
//! as written, and [`Compiled::python`] says why, which the checker reports as a note
//! (`null-python`), since it runs only where Blender may run Python. Units are Null's, so a
//! Python expression takes none.

use null_value::{Dimension, Scene, Unit, print_bare, read_in};

/// The subset's functions, each with the numbers of arguments it takes; `None` is any number
/// from one (`min`, `max`). Blender's `builtin_ops`, `expr_pylike_eval.cc`.
pub const FUNCTIONS: [(&str, Option<&[usize]>); 26] = [
    ("min", None),
    ("max", None),
    ("radians", Some(&[1])),
    ("degrees", Some(&[1])),
    ("abs", Some(&[1])),
    ("fabs", Some(&[1])),
    ("floor", Some(&[1])),
    ("ceil", Some(&[1])),
    ("trunc", Some(&[1])),
    ("round", Some(&[1])),
    ("int", Some(&[1])),
    ("sin", Some(&[1])),
    ("cos", Some(&[1])),
    ("tan", Some(&[1])),
    ("asin", Some(&[1])),
    ("acos", Some(&[1])),
    ("atan", Some(&[1])),
    ("atan2", Some(&[2])),
    ("exp", Some(&[1])),
    ("log", Some(&[1, 2])),
    ("sqrt", Some(&[1])),
    ("pow", Some(&[2])),
    ("fmod", Some(&[2])),
    ("lerp", Some(&[3])),
    ("clamp", Some(&[1, 3])),
    ("smoothstep", Some(&[3])),
];

/// The subset's constants.
pub const CONSTANTS: [&str; 3] = ["pi", "True", "False"];

/// The one variable every driver has: the frame it is evaluated at.
pub const FRAME: &str = "frame";

const KEYWORDS: [&str; 5] = ["and", "or", "not", "if", "else"];

/// A driver's expression, compiled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compiled {
    /// The expression Blender is given.
    pub expression: String,
    /// Why the expression is Python, outside Blender's simple expressions; `None` when it is
    /// one of them.
    pub python: Option<String>,
}

/// A mistake in an expression, at a byte range of its text, with the text to write there where
/// one is near.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExprError {
    /// What is wrong.
    pub message: String,
    /// Where, in the expression's text.
    pub at: (usize, usize),
    /// The word to write there instead.
    pub fix: Option<String>,
    /// The mistake is a unit's (one that is none, seconds without a frame rate, a scale the
    /// scenes do not agree on), which the checker reports as `null-unit`, as a value's; else
    /// `null-type`.
    pub unit: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    /// A number, with the unit written after it.
    Num {
        unit: Option<Unit>,
    },
    Name(String),
    Op(&'static str),
    Open,
    Close,
    Comma,
    End,
}

#[derive(Clone, Debug)]
struct Token {
    tok: Tok,
    at: (usize, usize),
}

/// Why an expression is not one of the simple ones (it is Python), or a mistake in it.
enum Fail {
    Python(String),
    Error(ExprError),
}

fn error(message: impl Into<String>, at: (usize, usize), fix: Option<String>) -> Fail {
    Fail::Error(ExprError {
        message: message.into(),
        at,
        fix,
        unit: false,
    })
}

fn tokens(text: &str) -> Result<Vec<Token>, Fail> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let start = i;
        // Blender's `isspace`, between any two tokens: an expression may run over lines.
        if matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || (c == b'.' && b.get(i + 1).is_some_and(u8::is_ascii_digit)) {
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            if i < b.len() && b[i] == b'.' {
                i += 1;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i < b.len() && matches!(b[i], b'e' | b'E') {
                let mut j = i + 1;
                if j < b.len() && matches!(b[j], b'+' | b'-') {
                    j += 1;
                }
                if j < b.len() && b[j].is_ascii_digit() {
                    i = j;
                    while i < b.len() && b[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let number = &text[start..i];
            // C-style octal, as Blender forbids it: a whole number led by 0 and holding another
            // digit (`00` is 0, in Blender as in Python).
            if number.len() > 1
                && number.starts_with('0')
                && number.bytes().all(|d| d.is_ascii_digit())
                && number.bytes().any(|d| d != b'0')
            {
                return Err(Fail::Python(format!(
                    "`{number}` is a whole number with a leading 0, which Python refuses"
                )));
            }
            // Python's own number forms (`0x1F`, `0b11`, `0o7`, `1_000`, `1j`): Python, not a unit.
            if let Some(form) = python_number(b, start, i) {
                return Err(Fail::Python(format!(
                    "`{}` is a number in Python's form, which Blender's simple expressions do not read",
                    &text[start..form]
                )));
            }
            // A unit right after the number: letters, as `100W`, `35deg`, `2m`.
            let unit_start = i;
            while i < b.len() && (b[i].is_ascii_alphabetic() || b[i] == b'_') {
                i += 1;
            }
            let unit = if unit_start == i {
                None
            } else {
                let suffix = &text[unit_start..i];
                match Unit::from_suffix(suffix, Dimension::None) {
                    Some(u) => Some(u),
                    None => {
                        return Err(Fail::Error(ExprError {
                            message: format!(
                                "`{suffix}` is not a unit, after the number `{number}`"
                            ),
                            at: (unit_start, i),
                            fix: None,
                            unit: true,
                        }));
                    }
                }
            };
            out.push(Token {
                tok: Tok::Num { unit },
                at: (start, i),
            });
            continue;
        }
        if c.is_ascii_alphabetic() || c == b'_' {
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            out.push(Token {
                tok: Tok::Name(text[start..i].to_string()),
                at: (start, i),
            });
            continue;
        }
        let two = text.get(i..i + 2).unwrap_or("");
        let op: Option<&'static str> = match two {
            "==" => Some("=="),
            "!=" => Some("!="),
            "<=" => Some("<="),
            ">=" => Some(">="),
            "**" | "//" => {
                return Err(Fail::Python(format!(
                    "`{two}` is Python's, not one of Blender's simple expressions' operators"
                )));
            }
            _ => None,
        };
        if let Some(op) = op {
            out.push(Token {
                tok: Tok::Op(op),
                at: (i, i + 2),
            });
            i += 2;
            continue;
        }
        let tok = match c {
            b'+' => Tok::Op("+"),
            b'-' => Tok::Op("-"),
            b'*' => Tok::Op("*"),
            b'/' => Tok::Op("/"),
            b'<' => Tok::Op("<"),
            b'>' => Tok::Op(">"),
            b'(' => Tok::Open,
            b')' => Tok::Close,
            b',' => Tok::Comma,
            _ => {
                let ch = text[i..].chars().next().unwrap_or('?');
                return Err(Fail::Python(format!(
                    "`{ch}` is not in Blender's simple expressions"
                )));
            }
        };
        out.push(Token {
            tok,
            at: (i, i + 1),
        });
        i += 1;
    }
    out.push(Token {
        tok: Tok::End,
        at: (text.len(), text.len()),
    });
    Ok(out)
}

/// Where Python's own form of the number at `b[start..i]` ends, if it is in one: a `0x`, `0b`
/// or `0o` prefix (either case), a `_` between digits, or a `j` (imaginary) after it. `None`
/// for a number Blender reads, or one followed by letters that are none of these (a unit, or
/// a misspelt one).
fn python_number(b: &[u8], start: usize, i: usize) -> Option<usize> {
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let digits = &b[start..i];
    let next = *b.get(i)?;
    if digits == b"0" && matches!(next, b'x' | b'X' | b'b' | b'B' | b'o' | b'O') {
        let mut j = i + 1;
        while j < b.len() && word(b[j]) {
            j += 1;
        }
        return (j > i + 1).then_some(j);
    }
    if next == b'_' && b.get(i + 1).is_some_and(u8::is_ascii_digit) {
        let mut j = i + 1;
        while j < b.len() && (b[j].is_ascii_digit() || matches!(b[j], b'_' | b'.' | b'e' | b'E')) {
            j += 1;
        }
        if j < b.len() && matches!(b[j], b'j' | b'J') {
            j += 1;
        }
        return Some(j);
    }
    if matches!(next, b'j' | b'J') && !b.get(i + 1).is_some_and(|&c| word(c)) {
        return Some(i + 1);
    }
    None
}

/// Where the string literal starting at `b[i]` (its quote, `'` or `"`) ends, past its closing
/// quote, a triple quote's and a `\` escape's included; the text's end if it never closes.
fn string_end(b: &[u8], i: usize) -> usize {
    let q = b[i];
    let triple = b.get(i..i + 3).is_some_and(|t| t == [q, q, q]);
    let mut j = i + if triple { 3 } else { 1 };
    while j < b.len() {
        if b[j] == b'\\' {
            j += 2;
            continue;
        }
        if triple {
            if b.get(j..j + 3).is_some_and(|t| t == [q, q, q]) {
                return j + 3;
            }
        } else if b[j] == q {
            return j + 1;
        }
        j += 1;
    }
    b.len()
}

/// The nearest of `names` to `word`, by Scowr's rule for suggestions.
fn near<'a>(word: &str, names: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    null_syntax::closest(word, names)
}

struct Parser<'v> {
    toks: Vec<Token>,
    at: usize,
    vars: &'v [&'v str],
}

impl Parser<'_> {
    fn peek(&self) -> &Tok {
        &self.toks[self.at].tok
    }

    fn here(&self) -> (usize, usize) {
        self.toks[self.at].at
    }

    fn bump(&mut self) -> Token {
        let t = self.toks[self.at].clone();
        if self.at + 1 < self.toks.len() {
            self.at += 1;
        }
        t
    }

    fn is_name(&self, word: &str) -> bool {
        matches!(self.peek(), Tok::Name(n) if n == word)
    }

    fn not_simple<T>(&self) -> Result<T, Fail> {
        let (s, e) = self.here();
        Err(Fail::Python(if s == e {
            "it ends where Blender's simple expressions go on".into()
        } else {
            format!(
                "Blender's simple expressions take no `{}` there",
                self.text_at(s, e)
            )
        }))
    }

    fn text_at(&self, _s: usize, _e: usize) -> String {
        match self.peek() {
            Tok::Name(n) => n.clone(),
            Tok::Op(o) => (*o).to_string(),
            Tok::Open => "(".into(),
            Tok::Close => ")".into(),
            Tok::Comma => ",".into(),
            Tok::Num { .. } => "number".into(),
            Tok::End => String::new(),
        }
    }

    fn expr(&mut self) -> Result<(), Fail> {
        self.or()?;
        if self.is_name("if") {
            self.bump();
            self.or()?;
            if !self.is_name("else") {
                return self.not_simple();
            }
            self.bump();
            self.expr()?;
        }
        Ok(())
    }

    fn or(&mut self) -> Result<(), Fail> {
        self.and()?;
        while self.is_name("or") {
            self.bump();
            self.and()?;
        }
        Ok(())
    }

    fn and(&mut self) -> Result<(), Fail> {
        self.not()?;
        while self.is_name("and") {
            self.bump();
            self.not()?;
        }
        Ok(())
    }

    fn not(&mut self) -> Result<(), Fail> {
        if self.is_name("not") {
            self.bump();
            return self.not();
        }
        self.cmp()
    }

    fn cmp(&mut self) -> Result<(), Fail> {
        self.add()?;
        while matches!(self.peek(), Tok::Op("==" | "!=" | "<" | "<=" | ">" | ">=")) {
            self.bump();
            self.add()?;
        }
        Ok(())
    }

    fn add(&mut self) -> Result<(), Fail> {
        self.mul()?;
        while matches!(self.peek(), Tok::Op("+" | "-")) {
            self.bump();
            self.mul()?;
        }
        Ok(())
    }

    fn mul(&mut self) -> Result<(), Fail> {
        self.unary()?;
        while matches!(self.peek(), Tok::Op("*" | "/")) {
            self.bump();
            self.unary()?;
        }
        Ok(())
    }

    fn unary(&mut self) -> Result<(), Fail> {
        match self.peek().clone() {
            Tok::Op("+" | "-") => {
                self.bump();
                self.unary()
            }
            Tok::Open => {
                self.bump();
                self.expr()?;
                if *self.peek() != Tok::Close {
                    return self.not_simple();
                }
                self.bump();
                Ok(())
            }
            Tok::Num { .. } => {
                self.bump();
                Ok(())
            }
            Tok::Name(n) if !KEYWORDS.contains(&n.as_str()) => {
                let at = self.here();
                self.bump();
                if *self.peek() == Tok::Open {
                    return self.call(&n, at);
                }
                self.name(&n, at)
            }
            _ => self.not_simple(),
        }
    }

    /// A name standing alone: a variable, `frame` or a constant. A name near one is a mistake;
    /// any other is Python's (`bpy`, `self`).
    fn name(&self, n: &str, at: (usize, usize)) -> Result<(), Fail> {
        if self.vars.contains(&n) || n == FRAME || CONSTANTS.contains(&n) {
            return Ok(());
        }
        let known = self.vars.iter().copied().chain([FRAME]).chain(CONSTANTS);
        if let Some(fix) = near(n, known) {
            return Err(error(
                format!("this driver has no variable `{n}`"),
                at,
                Some(fix.to_string()),
            ));
        }
        Err(Fail::Python(format!(
            "`{n}` is no variable of this driver, nor a name Blender's simple expressions know"
        )))
    }

    /// A function's call: one of the subset's, with as many arguments as it takes.
    fn call(&mut self, n: &str, at: (usize, usize)) -> Result<(), Fail> {
        let Some((_, takes)) = FUNCTIONS.iter().find(|(f, _)| *f == n) else {
            if let Some(fix) = near(n, FUNCTIONS.iter().map(|(f, _)| *f)) {
                return Err(error(
                    format!("`{n}` is no function of Blender's simple expressions"),
                    at,
                    Some(fix.to_string()),
                ));
            }
            return Err(Fail::Python(format!(
                "`{n}` is no function of Blender's simple expressions"
            )));
        };
        self.bump(); // `(`
        let mut count = 0;
        if *self.peek() != Tok::Close {
            loop {
                self.expr()?;
                count += 1;
                match self.peek() {
                    Tok::Comma => {
                        self.bump();
                    }
                    Tok::Close => break,
                    _ => return self.not_simple(),
                }
            }
        }
        self.bump(); // `)`
        let fits = match takes {
            None => count >= 1,
            Some(ns) => ns.contains(&count),
        };
        if !fits {
            let want = match takes {
                None => "one or more".to_string(),
                Some(ns) => ns
                    .iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join(" or "),
            };
            return Err(error(
                format!(
                    "`{n}` takes {want} argument{}, not {count}",
                    if want == "1" { "" } else { "s" }
                ),
                at,
                None,
            ));
        }
        Ok(())
    }
}

/// Where a driver is compiled: the scene its values are read in, and what its property and its
/// place say of how a number in a unit is read (task 1d-07f).
#[derive(Clone, Copy, Debug, Default)]
pub struct Context<'a> {
    /// The scene the numbers are read in: its unit scale and frame rate.
    pub scene: Scene,
    /// The driver is inside a scene, so a time in seconds can be read through its frame rate.
    pub in_scene: bool,
    /// The driven property counts frames (section 6, "Frames"): a time in seconds is read as
    /// frames, through the scene's frame rate.
    pub frames: bool,
    /// Outside every scene where the scenes' unit scales disagree and the project sets no
    /// `units`: the message for a number in a scaled unit, as a value's (section 6, "Scene
    /// scale").
    pub disagreement: Option<&'a str>,
}

/// Where a datablock's drivers are read, as the checker reads them (task 1d-07f): inside a
/// scene, that scene's unit scale (its `units` over the project's) and frame rate; outside every
/// scene, the scale the scenes agree on, or the message for a scaled unit where they disagree
/// and the project sets no `units`. Sau's apply and write-back take a driver's [`Context`] from
/// it, so they read a driver as the checker did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Place {
    /// The scene the numbers are read in.
    pub scene: Scene,
    /// Inside a scene.
    pub in_scene: bool,
    /// Outside every scene where the scenes' scales disagree: the message (section 6).
    pub disagreement: Option<String>,
}

impl Place {
    /// The context of a driver here on a property that counts frames (`frames`) or not.
    pub fn context(&self, frames: bool) -> Context<'_> {
        Context {
            scene: self.scene,
            in_scene: self.in_scene,
            frames,
            disagreement: self.disagreement.as_deref(),
        }
    }
}

/// The [`Place`] of a datablock in `files` (a project's parsed files): inside `scene`, the
/// scene node it is written in, or outside every scene when `None`.
pub fn place<'a>(
    files: &[&'a null_syntax::Parse],
    scene: Option<null_syntax::ast::Node<'a>>,
) -> Place {
    let project = crate::units::project(files.iter().copied()).unwrap_or_default();
    match scene {
        Some(node) => {
            let mut rate = crate::check::frame_rate(node);
            rate.scale_length = crate::units::Units::of(node)
                .unwrap_or_default()
                .over(&project)
                .scale();
            Place {
                scene: rate,
                in_scene: true,
                disagreement: None,
            }
        }
        None => {
            let outside = crate::units::outside(files.iter().copied());
            Place {
                scene: Scene {
                    scale_length: outside.scale(),
                    ..Scene::default()
                },
                in_scene: false,
                disagreement: crate::check::disagreement(&outside),
            }
        }
    }
}

/// `text` compiled in `scene` for a property that does not count frames, as a driver inside
/// that scene is: [`compile_in`] with that context.
pub fn compile(text: &str, vars: &[&str], scene: &Scene) -> Result<Compiled, ExprError> {
    compile_in(
        text,
        vars,
        &Context {
            scene: *scene,
            in_scene: true,
            frames: false,
            disagreement: None,
        },
    )
}

/// `text`, a driver's expression whose variables are `vars`, compiled for Blender: in Blender's
/// simple expressions, with its numbers in units written in the internal unit (read in `scene`);
/// else kept as written, with why it is Python. A mistake in it (a unit that is none, a
/// variable or a function misspelt, a function given the wrong number of arguments, a unit in
/// a Python expression) is an error.
pub fn compile_in(text: &str, vars: &[&str], cx: &Context<'_>) -> Result<Compiled, ExprError> {
    let scene = &cx.scene;
    let python = |why: String| -> Result<Compiled, ExprError> {
        Ok(Compiled {
            expression: text.to_string(),
            python: Some(why),
        })
    };
    let toks = match tokens(text) {
        Ok(t) => t,
        Err(Fail::Error(e)) => return Err(e),
        Err(Fail::Python(why)) => return check_python(text, why),
    };
    let mut p = Parser {
        toks: toks.clone(),
        at: 0,
        vars,
    };
    let parsed = p.expr().and_then(|()| {
        if *p.peek() == Tok::End {
            Ok(())
        } else {
            p.not_simple()
        }
    });
    match parsed {
        Err(Fail::Error(e)) => return Err(e),
        Err(Fail::Python(why)) => return check_python_tokens(&toks, text, why).and_then(python),
        Ok(()) => {}
    }
    // Each number in a unit, as its value in the internal unit.
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for t in &toks {
        if let Tok::Num { unit: Some(u) } = t.tok {
            let word = &text[t.at.0..t.at.1];
            let mut dimension = u.dimension();
            // A time in seconds on a property that counts frames: frames, through the scene's
            // frame rate, as a value is (task 1c-01h); outside a scene there is none to take.
            if dimension == Dimension::Seconds && cx.frames {
                if !cx.in_scene {
                    return Err(ExprError {
                        message: format!(
                            "`{word}` is in seconds, and this driver's property counts frames: seconds need a scene's frame rate, and this is not inside a scene"
                        ),
                        at: t.at,
                        fix: None,
                        unit: true,
                    });
                }
                dimension = Dimension::Frames;
            }
            // Outside every scene, a scaled unit needs the one scale the scenes agree on.
            if let Some(message) = cx.disagreement
                && dimension.scale_power() > 0
            {
                return Err(ExprError {
                    message: message.to_string(),
                    at: t.at,
                    fix: None,
                    unit: true,
                });
            }
            let value = read_in::<f64>(word, dimension, scene).map_err(|_| ExprError {
                message: format!("`{word}` does not read as a value"),
                at: t.at,
                fix: None,
                unit: true,
            })?;
            out.push_str(&text[last..t.at.0]);
            out.push_str(&print_bare(value));
            last = t.at.1;
        }
    }
    out.push_str(&text[last..]);
    Ok(Compiled {
        expression: out,
        python: None,
    })
}

/// A Python expression may hold no unit, which only Null reads.
fn check_python_tokens(toks: &[Token], text: &str, why: String) -> Result<String, ExprError> {
    match toks
        .iter()
        .find(|t| matches!(t.tok, Tok::Num { unit: Some(_) }))
    {
        Some(t) => Err(units_in_python(text, t.at, &why)),
        None => Ok(why),
    }
}

/// A Python expression whose tokens did not all lex: look for a unit in it, outside its strings
/// and Python's own number forms, which are Python's to read (a name `'3D Text'`, `0x1F`).
fn check_python(text: &str, why: String) -> Result<Compiled, ExprError> {
    let mut at = 0;
    let b = text.as_bytes();
    while at < b.len() {
        if matches!(b[at], b'\'' | b'"') {
            at = string_end(b, at);
            continue;
        }
        if b[at].is_ascii_digit() {
            let start = at;
            while at < b.len() && (b[at].is_ascii_digit() || b[at] == b'.') {
                at += 1;
            }
            if let Some(end) = python_number(b, start, at) {
                at = end;
                continue;
            }
            let unit_start = at;
            while at < b.len() && b[at].is_ascii_alphabetic() {
                at += 1;
            }
            if unit_start < at
                && !matches!(&text[unit_start..unit_start + 1], "e" | "E")
                && Unit::from_suffix(&text[unit_start..at], Dimension::None).is_some()
            {
                return Err(units_in_python(text, (start, at), &why));
            }
            continue;
        }
        if b[at].is_ascii_alphabetic() || b[at] == b'_' {
            while at < b.len() && (b[at].is_ascii_alphanumeric() || b[at] == b'_') {
                at += 1;
            }
            continue;
        }
        at += 1;
    }
    Ok(Compiled {
        expression: text.to_string(),
        python: Some(why),
    })
}

fn units_in_python(text: &str, at: (usize, usize), why: &str) -> ExprError {
    ExprError {
        message: format!(
            "`{}` has a unit, which Null compiles only in Blender's simple expressions; this one is Python ({why}), so write its numbers as Python does",
            &text[at.0..at.1]
        ),
        at,
        fix: None,
        unit: true,
    }
}

/// Whether Blender's expression `blender` is what `text` compiles to, so write-back leaves the
/// text as written (`spin * 100W` for Blender's `spin * 100`).
pub fn reads_as(text: &str, blender: &str, vars: &[&str], scene: &Scene) -> bool {
    text == blender || compile(text, vars, scene).is_ok_and(|c| c.expression == blender)
}

/// [`reads_as`] in a context of the caller's (a frame property, a place outside every scene).
pub fn reads_as_in(text: &str, blender: &str, vars: &[&str], cx: &Context<'_>) -> bool {
    text == blender || compile_in(text, vars, cx).is_ok_and(|c| c.expression == blender)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple(text: &str) -> String {
        let c = compile(text, &["spin", "dist"], &Scene::default()).unwrap();
        assert_eq!(c.python, None, "{text}");
        c.expression
    }

    fn python(text: &str) -> String {
        let c = compile(text, &["spin"], &Scene::default()).unwrap();
        assert_eq!(c.expression, text);
        c.python.unwrap_or_else(|| panic!("{text} is simple"))
    }

    fn wrong(text: &str) -> ExprError {
        compile(text, &["spin"], &Scene::default()).unwrap_err()
    }

    #[test]
    fn the_subset_compiles_as_written() {
        for t in [
            "spin * 100",
            "-spin + 2 * (frame / 24)",
            "sin(frame * pi / 12) if spin > 0.5 else 0",
            "min(spin, dist, 1) + max(0.5, spin)",
            "clamp(spin) + clamp(spin, 0, 1) + log(spin) + log(spin, 2)",
            "not spin < 1 <= dist and True or False",
            ".5 + 1e3 + 2.5E-2",
        ] {
            assert_eq!(simple(t), t);
        }
    }

    #[test]
    fn units_are_written_in_the_internal_unit() {
        assert_eq!(simple("spin * 100W"), "spin * 100");
        assert_eq!(simple("spin * 2kW"), "spin * 2000");
        assert_eq!(simple("spin + 90deg"), "spin + 1.5707963267948966");
        assert_eq!(simple("dist * 25cm"), "dist * 0.25");
        // Through the scene's unit scale, as a value is (section 6): the float Blender stores, so
        // 0.01 is not exactly a hundredth.
        let scene = |scale_length| Scene {
            scale_length,
            ..Scene::default()
        };
        let at = |s: f32| {
            compile("dist * 1m", &["dist"], &scene(s))
                .unwrap()
                .expression
        };
        assert_eq!(at(0.5), "dist * 2");
        assert_eq!(at(0.01), "dist * 100.00000223517424");
    }

    #[test]
    fn a_number_is_read_as_its_place_reads_it() {
        // Task 1d-07f. Seconds on a property that counts frames: through the scene's frame rate
        // inside a scene, refused outside one.
        let at = |in_scene, frames, disagreement| Context {
            scene: Scene {
                fps: 25,
                ..Scene::default()
            },
            in_scene,
            frames,
            disagreement,
        };
        let c = |text: &str, cx: &Context<'_>| compile_in(text, &["t"], cx);
        assert_eq!(
            c("t * 2s", &at(true, true, None)).unwrap().expression,
            "t * 50"
        );
        assert_eq!(
            c("t * 2s", &at(true, false, None)).unwrap().expression,
            "t * 2"
        );
        let e = c("t * 2s", &at(false, true, None)).unwrap_err();
        assert!(e.unit && e.at == (4, 6), "{e:?}");
        // Outside every scene where the scenes' scales disagree: a scaled unit is that error,
        // at the number; an angle, which no scale touches, is read as written.
        let e = c("t * 1m", &at(false, false, Some("they disagree"))).unwrap_err();
        assert_eq!(
            (e.message.as_str(), e.at, e.unit),
            ("they disagree", (4, 6), true)
        );
        assert_eq!(
            c("t * 90deg", &at(false, false, Some("they disagree")))
                .unwrap()
                .expression,
            "t * 1.5707963267948966"
        );
    }

    #[test]
    fn a_python_drivers_own_numbers_and_strings_are_pythons() {
        // From a .blend (task 1d-07f, round 2): Python's number forms and anything in quotes are
        // Python's, not units, so a working Python driver is written in Null as it is.
        for text in [
            "spin + 0x1F",
            "spin + 0b11",
            "spin * 0o7",
            "spin * 0XFF",
            "spin * 1_000",
            "spin * 1j",
            "spin & 0x1F",
            "bpy.data.objects['3D Text'].location.x",
            "bpy.data.objects[\"2m\"].location.x * spin",
            "obj['2m'] + obj[\"\"\"5cm\"\"\"]",
            "obj['it\\'s 2m'] * 2",
        ] {
            let c = compile(text, &["spin"], &Scene::default())
                .unwrap_or_else(|e| panic!("{text}: {e:?}"));
            assert!(c.python.is_some(), "{text}");
            assert_eq!(c.expression, text);
        }
        // A unit in a Python expression, outside its strings, stays the error it is; so does a
        // suffix that is no unit and no Python form.
        let e = wrong("spin & 2m");
        assert!(e.unit && e.message.contains("`2m` has a unit"), "{e:?}");
        let e = wrong("spin * 2mtr");
        assert!(e.message.contains("`mtr` is not a unit"), "{e:?}");
    }

    #[test]
    fn whitespace_and_zeros_are_blenders() {
        // Blender's `isspace` between tokens, so an expression over lines stays simple; `00` is
        // a whole number, `01` C's octal, which Blender and Python refuse.
        assert_eq!(
            simple("spin *\n  2 +\r\n\u{b}3\u{c}"),
            "spin *\n  2 +\r\n\u{b}3\u{c}"
        );
        assert_eq!(simple("spin * 00"), "spin * 00");
        assert!(python("spin * 01").contains("leading 0"));
    }

    #[test]
    fn python_is_kept_and_says_why() {
        assert!(python("bpy.data.objects['A'].location.x").contains('.'));
        assert!(python("spin ** 2").contains("**"));
        assert!(python("noise.random() * spin").contains('.'));
        assert!(python("self.location.x").contains('.'));
        assert!(python("spin % 2").contains('%'));
        assert!(python("var_list[0]").contains('['));
        assert!(python("random() * spin").contains("random"));
        assert!(python("spin if spin else").contains("ends"));
    }

    #[test]
    fn mistakes_are_errors_with_their_place() {
        let e = wrong("spn * 100");
        assert_eq!((e.at, e.fix.as_deref()), ((0, 3), Some("spin")));
        let e = wrong("sinn(spin)");
        assert_eq!(e.fix.as_deref(), Some("sin"));
        let e = wrong("atan2(spin)");
        assert!(
            e.message.contains("takes 2 arguments, not 1"),
            "{}",
            e.message
        );
        let e = wrong("spin * 100Q");
        assert_eq!(e.at, (10, 11));
        let e = wrong("bpy.data.x * 100W");
        assert!(e.message.contains("is Python"), "{}", e.message);
    }

    #[test]
    fn write_back_reads_a_compiled_expression_as_its_text() {
        let s = Scene::default();
        assert!(reads_as("spin * 100W", "spin * 100", &["spin"], &s));
        assert!(!reads_as("spin * 100W", "spin * 101", &["spin"], &s));
        assert!(reads_as("bpy.x", "bpy.x", &[], &s));
    }
}
