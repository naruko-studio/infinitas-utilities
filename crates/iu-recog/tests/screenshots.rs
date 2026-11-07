//! Screenshot regression tests. They run only where the screenshot folder
//! exists (see `iu-fixtures`); elsewhere they print a notice and pass.

/// Harness smoke test: every screenshot must decode and match the size of
/// its resolution folder. Recognizer checks are added here from P1 on.
#[test]
fn screenshots_decode_at_folder_resolution() {
    for screen in ["play", "result", "select", "option"] {
        iu_fixtures::run(screen, |_, _| Ok(()));
    }
}
