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
//! codes (`null-syntax`, spec section 18) instead of Scowr's. The meaning of a
//! word (numbers, units, handles, kinds) comes from Null's schema, never from
//! the shared syntax.
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
    TextEdit,
};
pub use scowr_syntax::{
    GreenElement, GreenNode, GreenToken, NodeKind, Parse, ParseError, Reparsed, SyntaxElement,
    SyntaxFix, SyntaxNode, SyntaxToken, TokenKind, ast, escape, raw_value, unescape,
};

/// Null's diagnostic codes for the syntax (spec, section 18). They are stable, so
/// tools and AI can act on them.
pub mod codes {
    /// Text that does not fit the node syntax.
    pub const SYNTAX: &str = "null-syntax";
}

/// Parses a whole Null file into its lossless tree, recovering from errors.
pub fn parse(text: &str) -> Parse {
    scowr_syntax::parse(text)
}

/// The syntax problems of `parse` as diagnostics in `file`, with Null's code
/// [`codes::SYNTAX`] and the shared front end's fixes.
pub fn diagnostics(parse: &Parse, file: FileId) -> Vec<Diagnostic> {
    parse
        .diagnostics(file)
        .into_iter()
        .map(|d| Diagnostic {
            code: codes::SYNTAX,
            ..d
        })
        .collect()
}

#[cfg(test)]
mod tests;
