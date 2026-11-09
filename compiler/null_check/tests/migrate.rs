//! Migrations between Null formats (task 1d-04d, Sau's S45): the steps from a format to the
//! newest, a generated rename's step, and what cannot be migrated.

use null_check::migrate::{
    FORMAT, MIGRATIONS, MigrateError, Migration, Rename, chained, migrate, oldest,
    rename_properties, steps,
};

/// A made-up format 0, before format 1: a camera's `lens` was `focal-length`, and a scene's
/// `render { fps … }` was `frame-rate`.
fn zero_to_one(text: &str) -> String {
    rename_properties(
        text,
        &[
            Rename {
                kind: "camera",
                path: &[],
                old: "focal-length",
                new: "lens",
            },
            Rename {
                kind: "scene",
                path: &["render"],
                old: "frame-rate",
                new: "fps",
            },
        ],
    )
}

const TEST: [Migration; 1] = [Migration {
    from: 0,
    what: "a camera's focal-length is lens; a scene's render frame-rate is fps",
    run: zero_to_one,
}];

#[test]
fn v0_is_format_1_and_ships_no_migration() {
    assert_eq!(FORMAT, 1);
    assert!(MIGRATIONS.is_empty());
    assert_eq!(oldest(MIGRATIONS), 1);
    assert_eq!(oldest(&TEST), 0);
}

#[test]
fn a_rename_changes_the_property_it_names_and_nothing_else() {
    let old = "\
camera @eye \"Eye\" focal-length=35mm {
  focal-length 50mm // the focal-length stays in a comment
  sensor-width 36mm
}
object @focal-length \"focal-length\" {
  focal-length 1
}
scene @main \"Main\" {
  frame-rate 1
  render {
    frame-rate 24
    resolution-x 1920
  }
}
";
    let new = migrate(old, 0, 1, &TEST).unwrap();
    assert_eq!(
        new,
        "\
camera @eye \"Eye\" lens=35mm {
  lens 50mm // the focal-length stays in a comment
  sensor-width 36mm
}
object @focal-length \"focal-length\" {
  focal-length 1
}
scene @main \"Main\" {
  frame-rate 1
  render {
    fps 24
    resolution-x 1920
  }
}
"
    );
    // A file migrated already, or written in part, is left as it is.
    assert_eq!(migrate(&new, 0, 1, &TEST).unwrap(), new);
}

#[test]
fn the_steps_run_in_order_and_a_gap_or_a_newer_format_is_refused() {
    fn one_to_two(text: &str) -> String {
        rename_properties(
            text,
            &[Rename {
                kind: "camera",
                path: &[],
                old: "lens",
                new: "focal",
            }],
        )
    }
    let two = [
        TEST[0],
        Migration {
            from: 1,
            what: "a camera's lens is focal",
            run: one_to_two,
        },
    ];
    let old = "camera @eye \"Eye\" {\n  focal-length 50mm\n}\n";
    assert_eq!(
        migrate(old, 0, 2, &two).unwrap(),
        "camera @eye \"Eye\" {\n  focal 50mm\n}\n"
    );
    assert_eq!(steps(0, 2, &two).unwrap().len(), 2);
    assert_eq!(steps(1, 1, &two).unwrap().len(), 0);
    assert_eq!(
        migrate(old, 2, 1, &TEST),
        Err(MigrateError::Newer { from: 2, to: 1 })
    );
    let err = migrate(old, 0, 1, MIGRATIONS).unwrap_err();
    assert_eq!(err, MigrateError::NoStep { at: 0, oldest: 1 });
    assert_eq!(
        err.to_string(),
        "no migration reads Null format 0: this Null reads format 1 and migrates no older format"
    );
}

#[test]
fn a_file_with_syntax_errors_is_not_migrated() {
    assert_eq!(
        migrate("camera @eye {\n  focal-length 50mm\n", 0, 1, &TEST),
        Err(MigrateError::Unparsed)
    );
}

#[test]
fn a_step_may_not_chain_or_swap_its_renames() {
    let r = |old, new| Rename {
        kind: "camera",
        path: &[],
        old,
        new,
    };
    let chain = [r("a", "b"), r("b", "c")];
    let swap = [r("a", "b"), r("b", "a")];
    assert!(chained(&chain).is_some());
    assert!(chained(&swap).is_some());
    // The same names in other places are no chain.
    let apart = [
        r("a", "b"),
        Rename {
            kind: "light",
            path: &[],
            old: "b",
            new: "c",
        },
    ];
    assert!(chained(&apart).is_none());
    for renames in [&chain[..], &swap[..]] {
        let refused =
            std::panic::catch_unwind(|| rename_properties("camera @c {\n  a 1\n}\n", renames));
        let message = refused.unwrap_err();
        let message = message.downcast_ref::<String>().unwrap();
        assert!(
            message.contains("a chain or swap is two steps, one format each"),
            "{message}"
        );
    }
}

#[test]
fn null_ships_no_step_with_a_chain() {
    // Each shipped step is run on an empty file, so a chained rename list panics here first.
    for m in MIGRATIONS {
        let _ = (m.run)("");
    }
}
