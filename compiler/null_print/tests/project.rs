//! Write-back across a project (task 1a-14a): one changed line per edit, comments and line endings kept.

use std::path::Path;

use null_check::{Schema, check};
use null_print::{FileEdit, ProjectFile, Transform, WriteBackError, format, write_transform};
use null_syntax::{FileId, TokenKind, lex, parse};

/// The checker's clean slice project: (path, text).
fn corpus() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../null_check/tests/slice");
    let mut out: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            (
                p.file_name().unwrap().to_str().unwrap().to_string(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    out.sort();
    out
}

fn objects(files: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (_, text) in files {
        for n in parse(text).document().nodes() {
            if n.name() == "object" {
                out.push(n.args().next().unwrap().text().to_string());
            }
        }
    }
    out
}

fn refs(files: &[(String, String)]) -> Vec<ProjectFile<'_>> {
    files
        .iter()
        .map(|(p, t)| ProjectFile { path: p, text: t })
        .collect()
}

/// `files` with `edits` applied, each to its own file.
fn apply(files: &[(String, String)], edits: &[FileEdit]) -> Vec<(String, String)> {
    files
        .iter()
        .map(|(path, text)| {
            let mut mine: Vec<_> = edits.iter().filter(|e| &e.path == path).collect();
            mine.sort_by_key(|e| e.start);
            let mut out = String::new();
            let mut at = 0;
            for e in mine {
                out.push_str(&text[at..e.start]);
                out.push_str(&e.replacement);
                at = e.end;
            }
            out.push_str(&text[at..]);
            (path.clone(), out)
        })
        .collect()
}

/// How many lines differ between `a` and `b`: the old and new lines between their common
/// first and last lines.
fn changed_lines(a: &str, b: &str) -> (usize, usize) {
    let a: Vec<&str> = a.split_inclusive('\n').collect();
    let b: Vec<&str> = b.split_inclusive('\n').collect();
    let head = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let tail = a[head..]
        .iter()
        .rev()
        .zip(b[head..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    (a.len() - head - tail, b.len() - head - tail)
}

fn comments(text: &str) -> Vec<String> {
    let (tokens, _) = lex(text);
    let mut at = 0;
    let mut out = Vec::new();
    for t in tokens {
        let s = &text[at..at + t.len as usize];
        at += t.len as usize;
        if matches!(t.kind, TokenKind::LineComment | TokenKind::BlockComment) {
            out.push(s.trim_end_matches('\r').to_string());
        }
    }
    out
}

fn checks_clean(files: &[(String, String)]) {
    let schema = Schema::from_json(include_str!("../../../schema/null-schema.json")).unwrap();
    let parses: Vec<_> = files.iter().map(|(_, t)| parse(t)).collect();
    let refs: Vec<_> = parses
        .iter()
        .enumerate()
        .map(|(i, p)| (FileId(i as u32), p))
        .collect();
    let found = check(&schema, &refs);
    assert!(found.is_empty(), "{found:?}");
}

const MOVES: [(Transform, [f32; 3]); 3] = [
    (Transform::Location, [1.5, -2.25, 0.125]),
    (
        Transform::RotationEuler,
        [0.1, 0.0, std::f32::consts::FRAC_PI_2],
    ),
    (Transform::Scale, [2.0, 2.0, 0.5]),
];

/// Every object, every transform: one line changed or added, nothing else.
fn move_everything(files: Vec<(String, String)>) -> usize {
    let mut edits_made = 0;
    for handle in objects(&files) {
        for (transform, values) in MOVES {
            let edits = write_transform(&refs(&files), &handle, transform, values).unwrap();
            assert!(!edits.is_empty(), "{handle} {transform:?}");
            edits_made += 1;
            let after = apply(&files, &edits);
            let mut lines_touched = 0;
            for ((path, before), (_, now)) in files.iter().zip(&after) {
                let (old, new) = changed_lines(before, now);
                if (old, new) != (0, 0) {
                    assert!(
                        old <= 1 && new == 1,
                        "{path}: {handle} {transform:?} touched {old} lines, made {new}:\n{now}"
                    );
                    lines_touched += 1;
                }
                assert_eq!(comments(before), comments(now), "{path}: a comment changed");
                if before.contains("\r\n") {
                    assert_eq!(
                        now.matches('\n').count(),
                        now.matches("\r\n").count(),
                        "{path}: a bare LF in a CRLF file"
                    );
                }
            }
            assert_eq!(lines_touched, 1, "{handle} {transform:?}");
            // The values now read back: writing them again changes nothing.
            let again = write_transform(&refs(&after), &handle, transform, values).unwrap();
            assert!(again.is_empty(), "{handle} {transform:?}: {again:?}");
            checks_clean(&after);
        }
    }
    edits_made
}

#[test]
fn every_object_moves_with_one_changed_line_per_edit() {
    let files = corpus();
    let made = move_everything(files);
    // Five objects, three transforms each, some of them added rather than changed.
    assert_eq!(made, 15);
}

#[test]
fn a_crlf_project_stays_crlf() {
    let files: Vec<_> = corpus()
        .into_iter()
        .map(|(p, t)| (p, t.replace('\n', "\r\n")))
        .collect();
    assert_eq!(move_everything(files), 15);
}

#[test]
fn units_follow_the_line_and_new_lines_the_project() {
    let files = corpus();
    // @boat writes location=[20cm 1.5m 0m]: each word keeps its unit.
    let edits = write_transform(
        &refs(&files),
        "@boat",
        Transform::Location,
        [0.25, 1.5, 0.0],
    )
    .unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].replacement, "25cm");
    // @sun has no location: added in the project's `length=m`.
    let edits =
        write_transform(&refs(&files), "@sun", Transform::Location, [1.0, 2.0, 3.0]).unwrap();
    assert!(
        edits[0].replacement.contains("location [1m 2m 3m]"),
        "{edits:?}"
    );
    // @boat has no scale: added bare.
    let edits = write_transform(&refs(&files), "@boat", Transform::Scale, [2.0, 2.0, 2.0]).unwrap();
    assert!(edits[0].replacement.contains("scale [2 2 2]"), "{edits:?}");
}

#[test]
fn objects_are_found_by_handle_or_not_at_all() {
    let files = corpus();
    assert_eq!(
        write_transform(&refs(&files), "@nothing", Transform::Location, [0.0; 3]),
        Err(WriteBackError::NoSuchObject)
    );
    let twice = vec![
        ("a.noll".to_string(), "object @x \"X\"\n".to_string()),
        ("b.noll".to_string(), "object @x \"X\"\n".to_string()),
    ];
    assert_eq!(
        write_transform(&refs(&twice), "@x", Transform::Location, [0.0; 3]),
        Err(WriteBackError::DefinedTwice)
    );
    let named = vec![(
        "a.noll".to_string(),
        "object \"Cube.001\" {\n}\n".to_string(),
    )];
    let edits = write_transform(
        &refs(&named),
        "@cube-001",
        Transform::Scale,
        [1.0, 2.0, 3.0],
    )
    .unwrap();
    assert_eq!(edits.len(), 1);
}

// The 1a-07 review's three findings.

#[test]
fn a_comment_on_the_opening_line_stays_there() {
    let text = "object @a \"A\" { // only a comment\n}\n";
    let files = vec![("a.noll".to_string(), text.to_string())];
    let edits = write_transform(&refs(&files), "@a", Transform::Scale, [2.0, 2.0, 2.0]).unwrap();
    let after = apply(&files, &edits);
    assert_eq!(
        after[0].1,
        "object @a \"A\" { // only a comment\n  scale [2 2 2]\n}\n"
    );
}

#[test]
fn an_insertion_into_a_crlf_file_uses_crlf() {
    for text in [
        "object @a \"A\" {\r\n  data mesh @m\r\n}\r\n",
        "object @a \"A\" { // c\r\n}\r\n",
    ] {
        let files = vec![("a.noll".to_string(), text.to_string())];
        let edits =
            write_transform(&refs(&files), "@a", Transform::Scale, [2.0, 2.0, 2.0]).unwrap();
        let after = &apply(&files, &edits)[0].1;
        assert_eq!(
            after.matches('\n').count(),
            after.matches("\r\n").count(),
            "{after:?}"
        );
        assert!(after.contains("  scale [2 2 2]\r\n"), "{after:?}");
    }
}

#[test]
fn a_continued_line_is_indented_one_level_deeper() {
    let text = "object @a {\n  location \\\n[1m 2m 3m]\n  scale \\ // c\n      [1 1 1]\n}\n";
    assert_eq!(
        format(text),
        "object @a {\n  location \\\n    [1m 2m 3m]\n  scale \\ // c\n    [1 1 1]\n}\n"
    );
    // And a CRLF file formats to LF lines (spec, section 15).
    assert_eq!(format("a 1\r\nb 2\r\n"), "a 1\nb 2\n");
}
