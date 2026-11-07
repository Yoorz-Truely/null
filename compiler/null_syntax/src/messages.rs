//! Null's wording of the shared syntax's messages: Null's examples where Scowr's give sound ones.
//!
//! The shared parser (`scowr_syntax`) writes its messages for Scowr, and three of
//! them carry Scowr's examples (`vel=96`, `track { ... }`, a musical note's
//! escape). Scowr is not changed for Null (`AGENTS.md`, "The shared front end"), so
//! Null rewords those here, by their exact text. [`KNOWN`] lists every message of
//! the shared syntax Null has read; a test fails on any other, so a new message
//! from Scowr is read before it reaches Null's users.

use scowr_errors::Span;
use scowr_syntax::ast::Node;
use scowr_syntax::{Parse, ParseError, SyntaxFix};

/// Every message the shared syntax gives, as read for Null on November 7, 2026
/// (Scowr `770a472`). `*` stands for any text.
pub const KNOWN: [&str; 22] = [
    "this } closes nothing; remove it or add the { it belongs to",
    "expected a node name before this block, such as track { ... }",
    "expected a node name before this list",
    "expected a node name, found `*`",
    "expected a node name after /-",
    "expected a key before =, such as vel=96",
    "/- comments out a whole node; put it before the node's name",
    "this ] closes no list",
    "`*` is reserved and cannot appear here",
    "a node ends after its block; start a new line for what follows",
    "expected a value after *=",
    "blocks are nested too deeply",
    "this { is never closed; add } at the end of its block",
    "lists are nested too deeply",
    "expected a value in this list, found `*`",
    "this [ is never closed; add ] after the last value",
    "this block comment is never closed; add */",
    "this string is never closed; add \"",
    "this string is not closed on its line; add \" or write a line break as \\n",
    "expected a Unicode escape such as \\u{266A}",
    "unknown escape; the escapes are \\\" \\\\ \\n \\t and \\u{...}",
    "this raw string is never closed; add *",
];

/// The example phrases of Scowr's that [`reword`] takes out of Null's messages,
/// whole, so a user's own key that ends the same way (`level=`, `travel=`) is not
/// mistaken for one.
pub const SCOWR_EXAMPLES: [&str; 3] = ["such as vel=96", "such as track {", "\\u{266A}"];

/// Whether `message` is one of [`KNOWN`].
pub fn is_known(message: &str) -> bool {
    KNOWN.iter().any(|p| matches(p, message))
}

fn matches(pattern: &str, text: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or("");
    let Some(mut rest) = text.strip_prefix(first) else {
        return false;
    };
    let parts: Vec<_> = parts.collect();
    let Some((last, middle)) = parts.split_last() else {
        return rest.is_empty();
    };
    for part in middle {
        match rest.find(part) {
            Some(i) => rest = &rest[i + part.len()..],
            None => return false,
        }
    }
    rest.ends_with(last)
}

/// `error`'s message and fix in Null's words: the shared front end's own where
/// they say nothing of Scowr.
pub(crate) fn reword(
    parse: &Parse,
    error: &ParseError,
    fix: Option<SyntaxFix>,
) -> (String, Option<SyntaxFix>) {
    match error.message.as_str() {
        "expected a node name before this block, such as track { ... }" => (
            "expected a node name before this block, such as object { ... }".into(),
            fix,
        ),
        "expected a Unicode escape such as \\u{266A}" => {
            ("expected a Unicode escape such as \\u{e9}".into(), fix)
        }
        "expected a key before =, such as vel=96" => match node_named_before(parse, error.span) {
            // `frame-start=1` on a line of its own: a child node takes a space.
            Some(name) => (
                format!(
                    "a child node is its name, a space and its value: `{name} …`; \
                     `{name}=…` goes on its owner's line"
                ),
                fix.or_else(|| {
                    Some(SyntaxFix {
                        message: "write a space for the =".into(),
                        span: error.span,
                        replacement: " ".into(),
                    })
                }),
            ),
            None => ("expected a key before =, such as frame-start=1".into(), fix),
        },
        _ => (error.message.clone(), fix),
    }
}

/// The name of the node whose name ends where `at` starts, if there is one.
fn node_named_before(parse: &Parse, at: Span) -> Option<String> {
    let mut stack: Vec<Node<'_>> = parse.document().all_nodes().collect();
    while let Some(node) = stack.pop() {
        let span = node.0.span();
        if span.start > at.start || span.end < at.start {
            continue;
        }
        if let Some(name) = node.name_token()
            && name.span.end == at.start
        {
            return Some(name.text.to_string());
        }
        if let Some(block) = node.block() {
            stack.extend(
                block
                    .child_nodes()
                    .filter(|n| n.kind() == scowr_syntax::NodeKind::Node)
                    .map(Node),
            );
        }
    }
    None
}
