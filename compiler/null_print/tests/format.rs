//! The formatter's layout pass: canonical, idempotent, and never changes a file's meaning.

use std::path::Path;

use null_print::format;
use null_syntax::{TokenKind, lex, parse};
use proptest::prelude::*;

/// Every slice file: the syntax's and the checker's.
fn files() -> Vec<String> {
    let mut out = Vec::new();
    for dir in ["../null_syntax/tests/slice", "../null_check/tests/slice"] {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
        for e in std::fs::read_dir(dir).unwrap() {
            out.push(std::fs::read_to_string(e.unwrap().path()).unwrap());
        }
    }
    out.sort();
    out
}

/// The tokens that carry meaning, comments included: everything but whitespace and line
/// feeds outside comments and strings.
fn meaning(text: &str) -> Vec<(TokenKind, String)> {
    let (tokens, _) = lex(text);
    let mut at = 0;
    let mut out = Vec::new();
    for t in tokens {
        let s = &text[at..at + t.len as usize];
        at += t.len as usize;
        match t.kind {
            TokenKind::Whitespace => {}
            TokenKind::LineComment => out.push((t.kind, s.trim_end().to_string())),
            _ => out.push((t.kind, s.to_string())),
        }
    }
    // Blank lines and the file's last line feeds carry nothing.
    while out.last().is_some_and(|(k, _)| *k == TokenKind::Newline) {
        out.pop();
    }
    out.dedup_by(|b, a| a.0 == TokenKind::Newline && b.0 == TokenKind::Newline);
    if out.first().is_some_and(|(k, _)| *k == TokenKind::Newline) {
        out.remove(0);
    }
    out
}

/// `text` with its whitespace scrambled: runs of spaces and tabs of other lengths,
/// indentation anywhere, trailing whitespace, extra blank lines.
fn scramble(text: &str, seed: &[u8]) -> String {
    let mut out = String::new();
    let mut k = 0;
    let mut next = || {
        k += 1;
        seed[k % seed.len()]
    };
    let (tokens, _) = lex(text);
    let mut at = 0;
    let mut last = None;
    for t in tokens {
        let s = &text[at..at + t.len as usize];
        at += t.len as usize;
        let blank = last == Some(TokenKind::Newline) && t.kind == TokenKind::Newline;
        if t.kind != TokenKind::Whitespace {
            last = Some(t.kind);
        }
        match t.kind {
            TokenKind::Whitespace => {
                let n = 1 + next() as usize % 4;
                for _ in 0..n {
                    out.push(if next() % 3 == 0 { '\t' } else { ' ' });
                }
            }
            TokenKind::Newline => {
                if next() % 4 == 0 {
                    out.push_str("  ");
                }
                out.push('\n');
                // A blank line may grow, never appear: the formatter keeps one.
                if blank && next() % 2 == 0 {
                    out.push_str("\n\n");
                }
                for _ in 0..next() % 6 {
                    out.push(' ');
                }
            }
            _ => out.push_str(s),
        }
    }
    out
}

#[test]
fn the_slice_files_format_and_stay_formatted() {
    for text in files() {
        let once = format(&text);
        assert_eq!(format(&once), once, "not idempotent:\n{once}");
        assert_eq!(meaning(&once), meaning(&text));
        assert_eq!(parse(&once).errors().len(), parse(&text).errors().len());
    }
}

#[test]
fn layout_is_canonical() {
    let text = "scene   @main\t\"Main\"{\n\n\n    frame-start 1 ;frame-end 2   // two\n camera @cam\n\t\tvertices [ \n[0 0 0]\n  [1 0 0] ]\n}   \n\n";
    let want = "scene @main \"Main\" {\n\n  frame-start 1; frame-end 2 // two\n  camera @cam\n  vertices [\n    [0 0 0]\n    [1 0 0]]\n}\n";
    assert_eq!(format(text), want);
}

proptest! {
    #[test]
    fn scrambled_whitespace_formats_back_to_one_form(
        pick in any::<prop::sample::Index>(),
        seed in prop::collection::vec(any::<u8>(), 1..64),
    ) {
        let files = files();
        let text = &files[pick.index(files.len())];
        let scrambled = scramble(text, &seed);
        let a = format(text);
        let b = format(&scrambled);
        prop_assert_eq!(&a, &b);
        prop_assert_eq!(meaning(&b), meaning(text));
        prop_assert_eq!(format(&b), b.clone());
    }
}
