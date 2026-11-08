//! Fuzz target: Null's whole front end on any text (task 1d-02d). Parsing, the syntax's
//! diagnostics and the v0 outline never panic; the formatter's layout is idempotent and keeps
//! every token and the file's constructs; and on text that parses clean, the formatter `sau fmt`
//! runs (`canonical`, then `without_defaults`) gives text that parses clean and formats to
//! itself, as the short forms' expansion does. Any broken property panics, which libFuzzer
//! reports as a crash. Run: `cargo +nightly fuzz run format`.
#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use null_check::Schema;
use null_syntax::{FileId, TokenKind, lex, parse, v0};

const PROJECT: &str = include_str!("../../schema/null-schema-project.json");

fn schema() -> &'static Schema {
    static S: OnceLock<Schema> = OnceLock::new();
    S.get_or_init(|| Schema::from_json(PROJECT).expect("the project schema reads"))
}

/// The tokens that mean something, and the comments with their lines' trailing whitespace
/// left out (and the file's last line feeds, for a comment that runs to its end), which is all
/// the layout may change of them.
fn meaning(text: &str) -> Vec<(TokenKind, String)> {
    let (tokens, _) = lex(text);
    let mut at = 0;
    let mut out = Vec::new();
    for t in tokens {
        let s = &text[at..at + t.len as usize];
        at += t.len as usize;
        match t.kind {
            TokenKind::Whitespace | TokenKind::Newline => {}
            k if k.is_comment() => {
                let lines: Vec<&str> = s.lines().map(str::trim_end).collect();
                // An unclosed `/*` runs to the end of the file, whose line feeds are layout.
                out.push((k, lines.join("\n").trim_end().to_string()));
            }
            // A string that is never closed runs to its line's end, whose trailing whitespace is
            // layout; a closed one ends in its quote, which no trimming touches.
            k @ (TokenKind::String | TokenKind::RawString) => {
                out.push((k, s.trim_end().to_string()));
            }
            k => out.push((k, s.to_string())),
        }
    }
    out
}

/// The outline without its line numbers, which blank lines may move, and each construct's words
/// as their tokens, since a list's words keep the layout they are written in.
fn shape(items: &[v0::Item]) -> Vec<(&'static str, String, usize)> {
    fn walk(items: &[v0::Item], depth: usize, out: &mut Vec<(&'static str, String, usize)>) {
        for i in items {
            let words: Vec<String> = meaning(&i.words).into_iter().map(|(_, t)| t).collect();
            out.push((i.what, words.join(" "), depth));
            walk(&i.children, depth + 1, out);
        }
    }
    let mut out = Vec::new();
    walk(items, 0, &mut out);
    out
}

/// `f` gives text that parses clean and is its own fixed point.
fn settles(name: &str, text: &str, f: impl Fn(&str) -> String) {
    let once = f(text);
    let p = parse(&once);
    assert!(
        p.errors().is_empty(),
        "{name} gave text that does not parse:\n{once}"
    );
    let twice = f(&once);
    assert_eq!(once, twice, "{name} twice is not {name} once, from\n{text}");
}

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let file = parse(&text);
    let _ = null_syntax::diagnostics(&file, FileId(0));
    let items = v0::outline(&file);
    let _ = v0::render(&items);

    // The layout: idempotent, every token kept, the constructs the same.
    let laid = null_print::format(&text);
    assert_eq!(
        null_print::format(&laid),
        laid,
        "the layout twice differs from once, for\n{text}"
    );
    assert_eq!(
        meaning(&laid),
        meaning(&text),
        "the layout changed a token of\n{text}"
    );
    let after = parse(&laid);
    assert_eq!(
        after.errors().is_empty(),
        file.errors().is_empty(),
        "the layout changed whether the text parses:\n{text}"
    );
    if !file.errors().is_empty() {
        return;
    }
    assert_eq!(
        shape(&v0::outline(&after)),
        shape(&items),
        "the layout changed a construct of\n{text}"
    );

    // `sau fmt`, and the short forms written in full.
    let schema = schema();
    settles("sau fmt", &text, |t| {
        null_print::without_defaults(&null_print::canonical(t, schema), schema)
    });
    settles("expand", &text, |t| {
        null_print::expand(&[t], schema).remove(0)
    });
});
