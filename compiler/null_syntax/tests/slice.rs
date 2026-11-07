//! The slice's syntax: every construct lexes, parses, prints back byte for byte, and its tree
//! is a snapshot; kebab-case names are enforced with fixes.

use std::path::{Path, PathBuf};

use null_syntax::{
    FileId, SourceMap, TokenKind, apply_edits, diagnostics, dump, lex, names, parse,
};

fn slice_files() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/slice");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "noll"))
        .collect();
    files.sort();
    files
}

#[test]
fn every_slice_file_lexes_parses_and_prints_back() {
    let files = slice_files();
    assert_eq!(files.len(), 7);
    for path in files {
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();
        let text = std::fs::read_to_string(&path).unwrap();

        // Lexing: no errors, and the tokens cover the text exactly.
        let (tokens, errors) = lex(&text);
        assert!(errors.is_empty(), "{name}: {errors:?}");
        let lexed: usize = tokens.iter().map(|t| t.len as usize).sum();
        assert_eq!(lexed, text.len(), "{name}");

        // Parsing: no diagnostics, the tree's tokens are the lexer's, the text comes back.
        let file = parse(&text);
        let found = diagnostics(&file, FileId(0));
        assert!(found.is_empty(), "{name}: {found:?}");
        let tree_tokens: Vec<_> = file
            .root()
            .tokens()
            .iter()
            .map(|t| (t.kind, t.text.len()))
            .collect();
        let lexer_tokens: Vec<_> = tokens.iter().map(|t| (t.kind, t.len as usize)).collect();
        assert_eq!(tree_tokens, lexer_tokens, "{name}");
        assert_eq!(file.print(), text, "{name}");

        insta::assert_snapshot!(name, dump(&file));
    }
}

#[test]
fn the_slice_covers_its_constructs() {
    // The constructs of phase 1a (Sau's roadmap): scene, object with location,
    // rotation, scale and parent, mesh as a primitive and as inline vertices and
    // faces, material with a Principled BSDF's base colour and roughness, camera, light.
    let mut names = std::collections::BTreeSet::new();
    for path in slice_files() {
        let text = std::fs::read_to_string(&path).unwrap();
        let file = parse(&text);
        let mut stack: Vec<_> = file.document().nodes().collect();
        while let Some(node) = stack.pop() {
            names.insert(node.name().into_owned());
            for entry in node.entries() {
                if let Some(key) = entry.key() {
                    names.insert(key.into_owned());
                }
            }
            for arg in node.args() {
                if let Some(word) = arg.as_word() {
                    names.insert(word.to_string());
                }
            }
            stack.extend(node.children());
        }
    }
    for construct in [
        "project",
        "scene",
        "collection",
        "object",
        "location",
        "rotation-euler",
        "scale",
        "parent",
        "mesh",
        "primitive",
        "vertices",
        "faces",
        "material",
        "principled-bsdf",
        "base-color",
        "roughness",
        "camera",
        "light",
        "layout",
    ] {
        assert!(
            names.contains(construct),
            "no `{construct}` in the slice files"
        );
    }
}

#[test]
fn kebab_case_is_the_rule_for_names() {
    for good in ["location", "rotation-euler", "use-2d", "frame-start", "x"] {
        assert!(names::is_kebab_case(good), "{good}");
    }
    for bad in [
        "Location",
        "rotation_euler",
        "frameStart",
        "-x",
        "x-",
        "a--b",
        "2d",
        "",
        "base color",
    ] {
        assert!(!names::is_kebab_case(bad), "{bad}");
    }
    let cases = [
        ("rotation_euler", Some("rotation-euler")),
        ("Location", Some("location")),
        ("frameStart", Some("frame-start")),
        ("FRAME_START", Some("frame-start")),
        ("Base Color", Some("base-color")),
        ("use2D", Some("use2d")),
        ("2d", None),
        ("a%b", None),
    ];
    for (name, kebab) in cases {
        assert_eq!(names::to_kebab_case(name).as_deref(), kebab, "{name}");
    }
}

#[test]
fn names_that_are_not_kebab_case_are_errors_with_fixes() {
    let text = "\
object @pier \"Pier\" Rotation_Mode=xyz {
  rotation_euler [0 0 0]
  frameStart 1
  \"base color\" [1 1 1]
  layout { @Tex [0 0]; @out [1 1] }
  /- Commented_Out 1
  data mesh @pier_deck
}
";
    let file = parse(text);
    let mut map = SourceMap::new();
    let id = map.add("objects/pier.noll", text);
    let found = diagnostics(&file, id);
    assert!(found.iter().all(|d| d.code == "null-name"));
    // Handles in arguments (`@pier_deck`) and nodes commented out are not names.
    assert_eq!(found.len(), 5);
    insta::assert_snapshot!("kebab-case-errors", map.render_all(&found));
}

#[test]
fn the_fixes_make_the_file_kebab_case() {
    let text = "object @pier frameStart=1 { rotation_euler [0 0 0] }\n";
    let file = parse(text);
    let found = diagnostics(&file, FileId(0));
    let mut edits: Vec<_> = found
        .iter()
        .flat_map(|d| &d.fixes[0].edits)
        .map(|e| (e.span, e.replacement.as_str()))
        .collect();
    edits.sort_by_key(|e| e.0.start);
    let fixed = apply_edits(text, &edits);
    assert_eq!(
        fixed,
        "object @pier frame-start=1 { rotation-euler [0 0 0] }\n"
    );
    assert!(diagnostics(&parse(&fixed), FileId(0)).is_empty());
}

#[test]
fn the_dump_shows_every_byte() {
    let text = "scene @a { fps 24 } // x\n";
    let file = parse(text);
    let dumped = dump(&file);
    let words = dumped.lines().filter(|l| l.contains("Word@")).count();
    assert_eq!(words, 4);
    let tokens = file.root().tokens();
    assert!(tokens.iter().any(|t| t.kind == TokenKind::LineComment));
    assert_eq!(
        dumped.lines().filter(|l| l.contains('"')).count(),
        tokens.len()
    );
}
