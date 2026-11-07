//! Kebab-case names (Sau's decision S16): node names and property keys, checked with a fix.
//!
//! Every name in Null is a Blender RNA identifier with `_` written as `-`, so
//! node names and property keys are lower-case kebab-case words: a letter, then
//! letters and digits, in segments joined by single hyphens (`rotation-euler`,
//! `use-2d`). A node may also be named by a handle, `@` and a kebab-case name, as
//! the entries of a `layout` block are (`@tex [-420 0]`). Unlike Scowr, Null never
//! needs a string as a name: S16 found every RNA identifier maps to kebab-case.
//! Values are not names: enum values and handles in arguments are the checker's.

use scowr_errors::{Diagnostic, FileId, Span};
use scowr_syntax::ast::{Entry, Node};
use scowr_syntax::{Parse, SyntaxToken, TokenKind};

use crate::codes;

/// Whether `name` is lower-case kebab-case: `[a-z][a-z0-9]*` then any `-[a-z0-9]+`.
pub fn is_kebab_case(name: &str) -> bool {
    let mut segments = name.split('-');
    let first = segments.next().unwrap_or("");
    let lower_alnum = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    };
    first.starts_with(|c: char| c.is_ascii_lowercase())
        && lower_alnum(first)
        && segments.all(lower_alnum)
}

/// The kebab-case spelling of `name`, if it has one: `_`, spaces and `.` become
/// `-`, a capital after a lower-case letter starts a segment
/// (`frameStart` is `frame-start`, `use2D` is `use2d`), and everything is lowered. `None` when the
/// result is still not kebab-case (it starts with a digit, or holds other symbols).
pub fn to_kebab_case(name: &str) -> Option<String> {
    let mut out = String::with_capacity(name.len() + 4);
    let mut prev: Option<char> = None;
    for c in name.chars() {
        if matches!(c, '_' | ' ' | '.' | '-') {
            if !out.is_empty() && !out.ends_with('-') {
                out.push('-');
            }
        } else {
            if c.is_ascii_uppercase()
                && prev.is_some_and(|p| p.is_ascii_lowercase())
                && !out.ends_with('-')
            {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
        }
        prev = Some(c);
    }
    let out = out.trim_end_matches('-').to_string();
    is_kebab_case(&out).then_some(out)
}

/// Diagnostics, code [`codes::NAME`], for every node name and property key in
/// `parse` that is not kebab-case, each with the kebab-case spelling as a fix
/// where there is one. Nodes commented out with `/-` are left alone.
pub fn check(parse: &Parse, file: FileId) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let mut stack: Vec<Node<'_>> = parse.document().nodes().collect();
    stack.reverse();
    while let Some(node) = stack.pop() {
        if let Some(name) = node.name_token() {
            check_name(name, true, file, &mut out);
        }
        for entry in node.entries() {
            if let Entry::Prop { key, .. } = entry {
                check_name(key, false, file, &mut out);
            }
        }
        let mut children: Vec<_> = node.children().collect();
        children.reverse();
        stack.extend(children);
    }
    out
}

fn check_name(token: SyntaxToken<'_>, node_name: bool, file: FileId, out: &mut Vec<Diagnostic>) {
    let what = if node_name {
        "a node name"
    } else {
        "a property key"
    };
    let text = token.text;
    let span = token.span;
    match token.kind {
        TokenKind::Word => {
            if is_kebab_case(text) {
                return;
            }
            if node_name && let Some(rest) = text.strip_prefix('@') {
                if is_kebab_case(rest) {
                    return;
                }
                let d = Diagnostic::error(
                    codes::NAME,
                    file,
                    span,
                    format!(
                        "`{text}` is not a kebab-case handle: `@`, then lower-case words joined by `-`"
                    ),
                );
                out.push(with_fix(
                    d,
                    span,
                    to_kebab_case(rest).map(|k| format!("@{k}")),
                ));
                return;
            }
            let d = Diagnostic::error(
                codes::NAME,
                file,
                span,
                format!("`{text}` is not kebab-case: {what} is lower-case words joined by `-`"),
            );
            out.push(with_fix(d, span, to_kebab_case(text)));
        }
        TokenKind::String | TokenKind::RawString => {
            let value = scowr_syntax::ast::scalar_text(token);
            let d = Diagnostic::error(
                codes::NAME,
                file,
                span,
                format!("{what} is a kebab-case word, not a string"),
            )
            .with_note("strings are for values: names, paths and free-text enum items");
            out.push(with_fix(d, span, to_kebab_case(&value)));
        }
        _ => {}
    }
}

fn with_fix(d: Diagnostic, span: Span, fix: Option<String>) -> Diagnostic {
    match fix {
        Some(k) => d.with_fix_replace(format!("write `{k}`"), span, k),
        None => d,
    }
}
