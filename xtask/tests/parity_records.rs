use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use xtask::parity::record_file_name;

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
}

/// Fixtures as `group/name.md`.
fn fixture_rel_paths() -> Vec<String> {
    let mut found = Vec::new();
    for group in ["1x", "edge"] {
        let dir = tests_dir().join("fixtures").join(group);
        for entry in fs::read_dir(&dir).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            if name.ends_with(".md") {
                found.push(format!("{group}/{name}"));
            }
        }
    }
    found.sort();
    found
}

#[test]
fn every_fixture_has_exactly_one_record() {
    let dir = tests_dir().join("parity").join("expected-1x");
    let mut on_disk = BTreeSet::new();
    for entry in fs::read_dir(&dir).unwrap() {
        on_disk.insert(entry.unwrap().file_name().to_string_lossy().into_owned());
    }
    let expected: BTreeSet<String> = fixture_rel_paths()
        .iter()
        .map(|rel| record_file_name(rel))
        .collect();
    assert_eq!(
        on_disk, expected,
        "records and fixtures differ; re-run `cargo xtask parity-record`"
    );
}

#[test]
fn records_have_the_expected_shape() {
    let dir = tests_dir().join("parity").join("expected-1x");
    for rel in fixture_rel_paths() {
        let path = dir.join(record_file_name(&rel));
        let text = fs::read_to_string(&path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("{} is not valid JSON: {e}", path.display()));
        let object = value
            .as_object()
            .unwrap_or_else(|| panic!("{} is not a JSON object", path.display()));
        assert_eq!(
            object.get("fixture").and_then(|v| v.as_str()),
            Some(rel.as_str())
        );
        assert_eq!(
            object.get("version").and_then(|v| v.as_str()),
            Some("1.7.1"),
            "{rel}"
        );
        assert!(
            object.get("errors").is_some_and(|v| v.is_object()),
            "{rel}: errors"
        );
        assert!(
            object.get("warnings").is_some_and(|v| v.is_object()),
            "{rel}: warnings"
        );
        assert!(
            object
                .get("crashed")
                .is_some_and(|v| v.is_null() || v.is_string()),
            "{rel}: crashed"
        );
    }
}
