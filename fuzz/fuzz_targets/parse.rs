//! Fuzz target: any text, and an edit of it, keeps every invariant of Null's syntax.
//!
//! The first three bytes choose an edit (its start and end as fractions of the
//! text, and what to write); the rest, read as UTF-8 with bad bytes replaced, is
//! the text. `null_syntax::invariants::check` panics on any broken invariant,
//! which libFuzzer reports as a crash. Run: `cargo +nightly fuzz run parse`.
#![no_main]

use libfuzzer_sys::fuzz_target;

const PIECES: [&str; 12] = [
    "", "{", "}", "[", "]", "\"", "#\"", "\\\n", "/*", "\n", "=", "x 1",
];

fuzz_target!(|data: &[u8]| {
    let Some((head, body)) = data.split_first_chunk::<3>() else {
        return;
    };
    let text = String::from_utf8_lossy(body);
    let len = text.len();
    let start = head[0] as usize * len / 255;
    let end = head[1] as usize * len / 255;
    let replacement = PIECES[head[2] as usize % PIECES.len()];
    null_syntax::invariants::check(&text, start, end, replacement);
});
