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
