//! gyrus is generic (ADR-0038 §7): its dependencies are SQLite, JSON and
//! one error helper, never its host. A dependency on an application crate
//! (any `juggler-*`), or on anything not listed here, fails this test.

#![allow(missing_docs, reason = "a test")]

const ALLOWED: &[&str] = &["rusqlite", "serde_json", "thiserror"];
const ALLOWED_DEV: &[&str] = &["tempfile"];

/// The dependency names of one `[section]` of the manifest.
fn names(manifest: &str, section: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == section;
            continue;
        }
        if inside
            && !line.is_empty()
            && !line.starts_with('#')
            && let Some((name, _)) = line.split_once('=')
        {
            out.push(name.trim().to_owned());
        }
    }
    out
}

#[test]
fn gyrus_depends_on_nothing_of_its_host() {
    let manifest = include_str!("../Cargo.toml");
    let deps = names(manifest, "[dependencies]");
    let dev = names(manifest, "[dev-dependencies]");
    for name in deps.iter().chain(&dev) {
        assert!(
            !name.starts_with("juggler"),
            "gyrus depends on its host's crate {name}"
        );
    }
    for name in &deps {
        assert!(ALLOWED.contains(&name.as_str()), "a new dependency: {name}");
    }
    for name in &dev {
        assert!(
            ALLOWED_DEV.contains(&name.as_str()),
            "a new dev-dependency: {name}"
        );
    }
    assert!(
        !manifest.contains("[build-dependencies]"),
        "a build dependency"
    );
    assert!(manifest.contains("license = \"MIT OR Apache-2.0\""));
}

#[test]
fn no_source_file_names_the_host() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut stack = vec![dir];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                assert!(
                    !text.contains("juggler"),
                    "{} names its host",
                    path.display()
                );
            }
        }
    }
}
