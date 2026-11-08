//! Every diagnostic (spec, section 18; task 1d-02c): each of Null's codes, `null_check::codes::ALL`,
//! has its cases here, each with its message and, where one exists, its fix, as one snapshot per
//! code. The test fails when a code has no case, and when the spec's list of codes and `ALL`
//! differ.

use null_check::{Schema, check, codes};
use null_syntax::{SourceMap, parse};

const PROJECT: &str = include_str!("../../../schema/null-schema-project.json");
const SPEC: &str = include_str!("../../../docs/spec/null-v0.md");

fn schema() -> &'static Schema {
    static S: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
    S.get_or_init(|| Schema::from_json(PROJECT).unwrap())
}

/// Each code with the files that raise it: one or more cases, each one file.
const CASES: &[(&str, &[&str])] = &[
    (
        codes::SYNTAX,
        &[
            "object @o \"O\" {\n  location [0m 0m 1m\n}\n",
            "object @o \"O\" {\n  name = \"x\"\n}\n",
        ],
    ),
    (
        codes::NAME,
        &[
            "object @Big_Cube \"O\"\n",
            "object @o \"O\" { rotation-mode XYZ }\n",
            "object @o \"O\" { pass_index 1 }\n",
        ],
    ),
    (
        codes::KIND,
        &[
            "objeet @o \"O\"\n",
            "mesh @m \"M\" { primitive cuboid size=1m }\n",
            "material @m \"M\" {\n  nodes {\n    node @a bsdf-principle\n  }\n}\n",
        ],
    ),
    (
        codes::UNKNOWN_PROPERTY,
        &[
            "object @o \"O\" { locaton [0m 0m 1m] }\n",
            "object @o \"O\" {\n  modifier @b bevel { wdth 4mm }\n}\n",
        ],
    ),
    (
        codes::TYPE,
        &[
            "object @o \"O\" { pass-index \"three\" }\n",
            "object @o \"O\" { location [0m 0m] }\n",
            "object @o \"O\" { hide-render maybe }\n",
            // A driver's expression (task 1d-07f): a variable misspelt, a function with the wrong
            // number of arguments.
            "object @d \"D\"\nlight @l \"L\" {\n  type point\n  energy driver=\"spn * 100W\" {\n    var spin transforms { target @d transform-type=rot-z }\n  }\n}\n",
            "object @d \"D\"\nlight @l \"L\" {\n  type point\n  energy driver=\"atan2(spin)\" {\n    var spin transforms { target @d transform-type=rot-z }\n  }\n}\n",
        ],
    ),
    (
        codes::UNIT,
        &[
            "object @o \"O\" { location [0m 0m 1deg] }\n",
            "camera @c \"C\" { clip-start 1s }\n",
            "camera @c \"C\" { lens 5zz }\n",
            // A driver's units, as a value's (task 1d-07f): a unit that is none, and a unit in a
            // Python expression.
            "object @d \"D\"\nlight @l \"L\" {\n  type point\n  energy driver=\"spin * 100Q\" {\n    var spin transforms { target @d transform-type=rot-z }\n  }\n}\n",
            "object @d \"D\"\nlight @l \"L\" {\n  type point\n  energy driver=\"bpy.context.scene.frame_current * 1W\" {\n    var spin transforms { target @d transform-type=rot-z }\n  }\n}\n",
        ],
    ),
    (
        codes::RANGE,
        &[
            "material @m \"M\" { roughness 2 }\n",
            "object @o \"O\" { pass-index -1 }\n",
            "material @m \"M\" { roughness nan }\n",
        ],
    ),
    (
        codes::ENUM,
        &[
            "object @o \"O\" { rotation-mode xyy }\n",
            "object @o \"O\" { empty-display-type cubee }\n",
        ],
    ),
    (
        codes::HANDLE,
        &[
            "object @o \"O\" { parent @lamp }\nobject @lamb \"Lamb\"\n",
            "object @o \"O\" { data mesh @nothing }\n",
            "object @o \"O\"\nobject @o \"Again\"\n",
        ],
    ),
    (
        codes::READONLY,
        &[
            "object @o \"O\" { mode edit }\n",
            "object @o \"O\" {\n  mode edit\n  pass-index 1\n}\n",
        ],
    ),
    (
        codes::PYTHON,
        &[
            "object @d \"D\"\nlight @l \"L\" {\n  type point\n  energy driver=\"bpy.data.objects['D'].location.z * 100\" {\n    var spin transforms { target @d transform-type=rot-z }\n  }\n}\n",
        ],
    ),
    (
        codes::REPEATED,
        &["object @o \"O\" {\n  pass-index 1\n  pass-index 2\n}\n"],
    ),
];

/// The case's diagnostics, syntax and checker together, as they print, fixes included.
fn rendered(text: &str) -> (String, Vec<String>) {
    let mut map = SourceMap::new();
    let id = map.add("case.noll", text);
    let file = parse(text);
    let mut found = null_syntax::diagnostics(&file, id);
    if file.errors().is_empty() {
        found.extend(check(schema(), &[(id, &file)]));
    }
    let codes = found.iter().map(|d| d.code.to_string()).collect();
    // What each fix makes of the text, so a snapshot holds the edit as well as its words.
    let mut out = map.render_all(&found);
    for fix in found.iter().flat_map(|d| &d.fixes) {
        let edits: Vec<_> = fix
            .edits
            .iter()
            .map(|e| (e.span, e.replacement.as_str()))
            .collect();
        let after = null_syntax::apply_edits(text, &edits).replace('\n', "\\n");
        out.push_str(&format!("  `{}` gives: {after}\n", fix.message));
    }
    (out, codes)
}

#[test]
fn every_code_has_its_cases_each_with_its_message_and_fix() {
    for code in codes::ALL {
        let Some((_, cases)) = CASES.iter().find(|(c, _)| *c == code) else {
            panic!("{code} has no case: add one to CASES");
        };
        let mut out = String::new();
        for text in *cases {
            let (shown, found) = rendered(text);
            assert!(
                found.iter().any(|c| c == code),
                "a case of {code} does not raise it:\n{text}\n{shown}"
            );
            out.push_str("--- ");
            out.push_str(&text.replace('\n', "\\n"));
            out.push('\n');
            out.push_str(&shown);
        }
        insta::assert_snapshot!(code, out);
    }
}

#[test]
fn every_case_is_of_a_listed_code() {
    for (code, _) in CASES {
        assert!(codes::ALL.contains(code), "{code} is not in codes::ALL");
    }
}

#[test]
fn the_specs_list_of_codes_is_codes_all() {
    // Section 18 names each code once, in backticks.
    let start = SPEC.find("## 18. Diagnostics").unwrap();
    let end = start + SPEC[start..].find("\n## 19.").unwrap();
    let mut listed: Vec<&str> = Vec::new();
    for piece in SPEC[start..end].split('`').skip(1).step_by(2) {
        if piece.starts_with("null-") && !listed.contains(&piece) {
            listed.push(piece);
        }
    }
    assert_eq!(listed, codes::ALL);
}
