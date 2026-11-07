//! What must hold for any text, broken or not: the checks the fuzz target and the tests share.
//!
//! [`check`] parses a text, edits it, and panics if the tree loses a byte, a span
//! leaves the text, a syntax error goes unreported, a message is one Null has not
//! read ([`crate::messages::KNOWN`]), or an incremental reparse differs from a
//! fresh parse. The fuzz target (`fuzz/fuzz_targets/parse.rs`) calls it on
//! arbitrary input; a panic there is a crash.

use scowr_errors::{FileId, SourceMap, Span};

use crate::{diagnostics, dump, messages, parse};

/// Checks every invariant on `text`, then on `text` with `start..end` (moved to
/// character boundaries, clamped to the text) replaced by `replacement`.
///
/// # Panics
///
/// Panics, naming the invariant, if one does not hold.
pub fn check(text: &str, start: usize, end: usize, replacement: &str) {
    let mut file = parse(text);
    assert_eq!(file.print(), text, "the tree prints back the text");

    let mut at = 0;
    for t in file.root().tokens() {
        assert_eq!(t.span.start, at, "tokens are contiguous");
        at = t.span.end;
    }
    assert_eq!(at, text.len(), "tokens cover the text");

    for e in file.errors() {
        assert!(
            messages::is_known(&e.message),
            "unread message: {}",
            e.message
        );
    }
    let found = diagnostics(&file, FileId(0));
    for d in &found {
        assert_span(text, d.primary.span);
        for fix in &d.fixes {
            for edit in &fix.edits {
                assert_span(text, edit.span);
            }
        }
        for word in ["vel=", "track {", "266A"] {
            assert!(
                !d.message.contains(word),
                "Scowr's example in {}",
                d.message
            );
        }
    }
    let syntax = found
        .iter()
        .filter(|d| d.code == crate::codes::SYNTAX)
        .count();
    assert_eq!(
        syntax,
        file.errors().len(),
        "every syntax error is reported"
    );
    let mut map = SourceMap::new();
    map.add("fuzz.noll", text);
    let _ = map.render_all(&found);
    let _ = dump(&file);

    // An edit reparses to the same tree and errors as a fresh parse of the new text.
    let floor = |mut i: usize| {
        i = i.min(text.len());
        while !text.is_char_boundary(i) {
            i -= 1;
        }
        i
    };
    let (a, b) = (floor(start), floor(end));
    let span = Span::new(a.min(b), a.max(b));
    let mut edited = text.to_string();
    edited.replace_range(span.range(), replacement);
    file.edit(span, replacement);
    let fresh = parse(&edited);
    assert_eq!(
        file.print(),
        edited,
        "the edited tree prints the edited text"
    );
    assert_eq!(
        dump(&file),
        dump(&fresh),
        "an edit reparses as a fresh parse"
    );
    assert_eq!(
        file.errors(),
        fresh.errors(),
        "an edit finds a fresh parse's errors"
    );
}

fn assert_span(text: &str, span: Span) {
    assert!(
        span.start <= span.end && span.end <= text.len(),
        "span in the text"
    );
    assert!(
        text.is_char_boundary(span.start) && text.is_char_boundary(span.end),
        "span on character boundaries"
    );
}
