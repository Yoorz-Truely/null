//! Null's syntax: the lexer and lossless syntax tree Null shares with Scowr, with Null's codes.
//!
//! Null writes its files in Scowr's node syntax (spec, section 5; Sau's decision
//! S15), and so uses Scowr's front-end crates, `scowr_syntax` and `scowr_errors`,
//! by path from `../Scowr`. This crate is the only place Null names them: every
//! other Null crate reads syntax through it, so when the shared crates move to
//! their own repository only this crate's manifest changes. What is shared and
//! what is Null's own is written in `AGENTS.md` ("The shared front end").
//!
//! What this crate adds is Null's side of the boundary: diagnostics carry Null's
//! codes (`null-syntax`, spec section 18) instead of Scowr's; names are checked
//! to be kebab-case (module [`names`], Sau's decision S16); and [`dump`] prints a
//! tree for snapshot tests. The meaning of a word (numbers, units, handles,
//! kinds) comes from Null's schema, never from the shared syntax.
//!
//! # Use
//!
//! ```
//! use null_syntax::{FileId, parse};
//!
//! let text = "object @pier \"Pier\" {\n  location [0m -4m 0.35m] // on the water\n}\n";
//! let file = parse(text);
//! assert!(null_syntax::diagnostics(&file, FileId(0)).is_empty());
//! assert_eq!(file.print(), text); // lossless
//!
//! let pier = file.document().nodes().next().unwrap();
//! assert_eq!(pier.name(), "object");
//! assert_eq!(pier.handle().unwrap().text, "@pier");
//! ```

pub use scowr_errors::{
    Diagnostic, FileId, Fix, Label, LineCol, Location, Severity, SourceFile, SourceMap, Span,
    TextEdit, apply_edits,
};
pub use scowr_syntax::lexer::{LexError, Token, lex};
pub use scowr_syntax::{
    GreenElement, GreenNode, GreenToken, NodeKind, Parse, ParseError, Reparsed, SyntaxElement,
    SyntaxFix, SyntaxNode, SyntaxToken, TokenKind, ast, escape, raw_value, unescape,
};

pub mod invariants;
pub mod messages;
pub mod names;

/// Null's diagnostic codes for the syntax (spec, section 18). They are stable, so
/// tools and AI can act on them.
pub mod codes {
    /// Text that does not fit the node syntax.
    pub const SYNTAX: &str = "null-syntax";
    /// A node name or property key that is not kebab-case.
    pub const NAME: &str = "null-name";
}

/// Parses a whole Null file into its lossless tree, recovering from errors.
pub fn parse(text: &str) -> Parse {
    scowr_syntax::parse(text)
}

/// The problems of `parse` as diagnostics in `file`, in text order: the syntax
/// errors, with Null's code [`codes::SYNTAX`], the shared front end's fixes and
/// Null's wording ([`messages`]),
/// and the names that are not kebab-case ([`codes::NAME`], [`names::check`]).
pub fn diagnostics(parse: &Parse, file: FileId) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = parse
        .errors()
        .iter()
        .map(|e| {
            let (message, fix) = messages::reword(parse, e, parse.fix(e));
            let d = Diagnostic::error(codes::SYNTAX, file, e.span, message);
            match fix {
                Some(f) => d.with_fix_replace(f.message, f.span, f.replacement),
                None => d,
            }
        })
        .collect();
    out.extend(names::check(parse, file));
    out.sort_by_key(|d| (d.primary.span.start, d.primary.span.end));
    out
}

/// The tree of `parse` as indented text, one element per line: each node's kind
/// and span, each token's kind, span and text (escaped). Every byte appears in
/// exactly one token, so the dump shows the whole file. For snapshot tests.
pub fn dump(parse: &Parse) -> String {
    let mut out = String::new();
    dump_node(parse.root(), 0, &mut out);
    out
}

fn dump_node(node: SyntaxNode<'_>, depth: usize, out: &mut String) {
    use std::fmt::Write;
    let span = node.span();
    let indent = "  ".repeat(depth);
    let _ = writeln!(
        out,
        "{indent}{:?}@{}..{}",
        node.kind(),
        span.start,
        span.end
    );
    for child in node.children() {
        match child {
            SyntaxElement::Node(n) => dump_node(n, depth + 1, out),
            SyntaxElement::Token(t) => {
                let _ = writeln!(
                    out,
                    "{indent}  {:?}@{}..{} {:?}",
                    t.kind, t.span.start, t.span.end, t.text
                );
            }
        }
    }
}

#[cfg(test)]
mod tests;
