//! The layout pass of the formatter: canonical whitespace and indentation, nothing else.

use null_syntax::{TokenKind, lex};

/// `text` with canonical layout (spec, section 15):
///
/// - each line indented two spaces per open block and open list, a closing `}` or `]`
///   at the start of a line one level less, a line a `\` continues one level more;
/// - one space between entries, none just inside `[ ]`, one on each side inside a
///   `{ }` that stays on one line, none before `;`, `=` left as written;
/// - no trailing whitespace, a comment's included, at most one blank line in a row, none at the start, and
///   exactly one line feed at the end; lines end in LF, a CRLF file's included.
///
/// Every token but whitespace is kept as written, comments and strings included, so
/// the file means the same; formatting twice gives the same text.
pub fn format(text: &str) -> String {
    let (tokens, _) = lex(text);
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    let mut depth: usize = 0;
    let mut line_start = true;
    let mut pending_space = false;
    let mut blank_lines = 0;
    let mut prev: Option<TokenKind> = None;
    // A `\` continuation on this line, and so on the line it leads to.
    let mut continues = false;
    let mut continued = false;
    for token in tokens {
        let kind = token.kind;
        let t = &text[at..at + token.len as usize];
        at += token.len as usize;
        match kind {
            TokenKind::Whitespace => {
                if !line_start {
                    pending_space = true;
                }
            }
            TokenKind::Newline => {
                if line_start {
                    blank_lines += 1;
                    if blank_lines == 1 && !out.is_empty() {
                        out.push('\n');
                    }
                } else {
                    out.push('\n');
                    blank_lines = 0;
                }
                line_start = true;
                pending_space = false;
                continued = continues;
                continues = false;
                prev = Some(kind);
            }
            _ => {
                if line_start {
                    let closing = matches!(kind, TokenKind::RBrace | TokenKind::RBracket);
                    let level = if closing {
                        depth.saturating_sub(1)
                    } else {
                        depth
                    };
                    // A line a `\` continues sits one level deeper (the 1a-07 review).
                    let level = level + usize::from(continued);
                    out.push_str(&"  ".repeat(level));
                    line_start = false;
                } else if space_between(prev, kind, pending_space) {
                    out.push(' ');
                }
                // A line comment runs to the end of its line, trailing spaces too.
                let t = if kind == TokenKind::LineComment {
                    t.trim_end_matches([' ', '\t', '\r'])
                } else {
                    t
                };
                out.push_str(t);
                if kind == TokenKind::Continuation {
                    continues = true;
                }
                match kind {
                    TokenKind::LBrace | TokenKind::LBracket => depth += 1,
                    TokenKind::RBrace | TokenKind::RBracket => depth = depth.saturating_sub(1),
                    _ => {}
                }
                pending_space = false;
                blank_lines = 0;
                prev = Some(kind);
            }
        }
    }
    let trimmed = out.trim_end_matches(['\n', ' ']);
    if trimmed.is_empty() {
        return String::new();
    }
    format!("{trimmed}\n")
}

/// Whether a space goes between `prev` and `next` on one line; `written` is whether
/// the text had whitespace there.
fn space_between(prev: Option<TokenKind>, next: TokenKind, written: bool) -> bool {
    use TokenKind::*;
    match (prev, next) {
        // `=` is left as written: whitespace around it can change what parses.
        (Some(Equals), _) | (_, Equals) => written,
        (Some(LBracket), _) | (_, RBracket) => false,
        (_, Semicolon) => false,
        (Some(Semicolon), _) => true,
        (_, LBrace) | (Some(LBrace), _) | (_, RBrace) => true,
        (_, LineComment) => true,
        _ => written,
    }
}
