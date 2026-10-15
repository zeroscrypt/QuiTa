//! Guards the documentation against references that go nowhere.
//!
//! Screenshots live in the repository rather than being fetched from somewhere
//! else, which makes them cheap but also means a renamed file leaves a broken
//! image in both READMEs with nothing to complain about.

use std::path::{Path, PathBuf};

const READMES: [(&str, &str); 2] = [
    ("README.md", include_str!("../../README.md")),
    ("README.ru.md", include_str!("../../README.ru.md")),
];

const SCREENSHOT_DIR: &str = "docs/screenshots";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri lives inside the repository")
        .to_path_buf()
}

/// Pulls `docs/screenshots/...` out of an `src="..."` attribute.
fn referenced_screenshots(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(at) = rest.find("src=\"") {
        rest = &rest[at + 5..];
        let Some(end) = rest.find('"') else { break };
        let value = &rest[..end];
        if value.starts_with(SCREENSHOT_DIR) {
            // Screenshots are stored as WebP. A stray .png reference would
            // render a broken image and the file check below would miss it.
            assert!(
                value.ends_with(".webp"),
                "скриншот должен быть .webp, а не .{}",
                value.rsplit('.').next().unwrap_or("?")
            );
            out.push(value.to_string());
        }
        rest = &rest[end + 1..];
    }
    out
}

#[test]
fn every_referenced_screenshot_exists() {
    let root = repo_root();
    for (file, text) in READMES {
        for (index, line) in text.lines().enumerate() {
            for reference in referenced_screenshots(line) {
                let path = root.join(&reference);
                assert!(
                    path.exists(),
                    "{file}:{} points at {reference}, which is not in the repository",
                    index + 1
                );
            }
        }
    }
}

#[test]
fn no_screenshot_is_left_unused() {
    // A file nobody links to is usually a renamed one whose references were
    // only half updated.
    let directory = repo_root().join(SCREENSHOT_DIR);
    let mut referenced: Vec<String> = Vec::new();
    for (_, text) in READMES {
        for line in text.lines() {
            referenced.extend(referenced_screenshots(line));
        }
    }

    let entries =
        std::fs::read_dir(&directory).unwrap_or_else(|e| panic!("cannot read {directory:?}: {e}"));
    for entry in entries {
        let name = entry.expect("directory entry").file_name();
        let name = name.to_string_lossy().to_string();
        if !referenced.iter().any(|r| r.ends_with(&name)) {
            panic!("{SCREENSHOT_DIR}/{name} is in the repository but linked from no README");
        }
    }
}

#[test]
fn screenshots_are_small_enough_to_load_in_a_readme() {
    // Six full-resolution window shots would add megabytes to every page view
    // of the repository, which is what makes a README slow to open.
    let directory = repo_root().join(SCREENSHOT_DIR);
    let entries =
        std::fs::read_dir(&directory).unwrap_or_else(|e| panic!("cannot read {directory:?}: {e}"));
    for entry in entries {
        let path = entry.expect("directory entry").path();
        let size = std::fs::metadata(&path).expect("file metadata").len();
        assert!(
            size < 700 * 1024,
            "{} weighs {} bytes; a README of six such images would make the \
             repository page crawl on a phone connection",
            path.display(),
            size
        );
    }
}

/// Counts the test attributes in one file, or in a whole directory.
fn count_tests_in(relative: &str) -> usize {
    fn walk(path: &Path) -> usize {
        if path.is_dir() {
            let mut total = 0;
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    total += walk(&entry.path());
                }
            }
            return total;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            return 0;
        }
        let text = std::fs::read_to_string(path).unwrap_or_default();
        // Matched per line, not as a substring: an attribute named inside a
        // comment or a string literal is not a test, and this very file holds
        // the marker twice that way.
        text.lines().filter(|line| line.trim() == "#[test]").count()
    }
    walk(&repo_root().join(relative))
}

/// Counts `it(` in the TypeScript suites.
fn count_ts_tests() -> usize {
    let dir = repo_root().join("tests");
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("ts") {
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                // `it(` followed by a quote, not any substring ending in "it(":
                // `await taskStore.init()` contains one too.
                total += text
                    .lines()
                    .filter(|line| {
                        line.contains("it('") || line.contains("it(\"") || line.contains("it(`")
                    })
                    .count();
            }
        }
    }
    total
}

/// The counts stated in the documentation, checked against the source tree.
///
/// This number changes on nearly every commit and is the one nobody notices
/// going stale: the plan said 44 tests where there were 84, and the README
/// agreed with the plan, so the two agreed with each other and both were
/// wrong.
#[test]
fn the_test_counts_in_the_docs_match_the_code() {
    let rust_unit = count_tests_in("src-tauri/src");
    let rust_integration = count_tests_in("src-tauri/tests/integration_tests.rs");
    let rust_docs = count_tests_in("src-tauri/tests/documentation.rs");
    let ts = count_ts_tests();
    let rust_total = rust_unit + rust_integration + rust_docs;

    let plan = std::fs::read_to_string(repo_root().join("docs/TEST_PLAN.md")).unwrap();
    for (label, actual) in [
        ("Rust unit", rust_unit),
        ("Rust integration", rust_integration),
        ("TypeScript", ts),
    ] {
        assert!(
            plan_has_count(&plan, label, actual),
            "docs/TEST_PLAN.md does not state {actual} for {label}"
        );
    }

    let readme = std::fs::read_to_string(repo_root().join("README.ru.md")).unwrap();
    // The noun form changes with the number — 119 модульных тестов, 122
    // модульных теста — so the marker is the part that stays put. Matching the
    // full phrase would have forced bad grammar into the README to satisfy the
    // guard.
    assert!(
        readme_has(&readme, "модульных тест", ts),
        "README.ru.md does not state the TypeScript count {ts}"
    );
    assert!(
        readme_has(&readme, "Rust-тестов", rust_total),
        "README.ru.md does not state the Rust total {rust_total}"
    );
}

/// Finds a table row for `label` and checks the number in it.
fn plan_has_count(plan: &str, label: &str, expected: usize) -> bool {
    plan.lines().any(|line| {
        line.starts_with(&format!("| {label} |"))
            && line
                .split('|')
                .any(|cell| cell.trim() == expected.to_string())
    })
}

/// Looks for `label` next to `expected` anywhere in the text.
fn readme_has(text: &str, label: &str, expected: usize) -> bool {
    text.lines()
        .any(|line| line.contains(label) && line.contains(&expected.to_string()))
}
