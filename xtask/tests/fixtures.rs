use std::fs;
use std::path::{Path, PathBuf};

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
}

fn fixture_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for group in ["1x", "edge"] {
        let dir = fixtures_root().join(group);
        let entries = fs::read_dir(&dir).unwrap_or_else(|e| panic!("cannot read {dir:?}: {e}"));
        for entry in entries {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|ext| ext == "md") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

#[test]
fn fixture_line_endings_are_preserved() {
    let files = fixture_files();
    assert_eq!(
        files.len(),
        36,
        "fixture count changed: update this count and re-run `cargo xtask parity-record`"
    );
    for path in files {
        let bytes = fs::read(&path).unwrap();
        if path.file_name().is_some_and(|n| n == "crlf.md") {
            let lf_count = bytes.iter().filter(|b| **b == b'\n').count();
            assert!(lf_count >= 1, "crlf.md has no line breaks");
            for (i, b) in bytes.iter().enumerate() {
                if *b == b'\n' {
                    assert!(i > 0 && bytes[i - 1] == b'\r', "bare LF in crlf.md at {i}");
                }
            }
        } else {
            assert!(!bytes.contains(&b'\r'), "{path:?} contains a CR");
        }
    }
}

#[test]
fn bom_fixture_starts_with_bom() {
    let bytes = fs::read(fixtures_root().join("edge").join("bom.md")).unwrap();
    assert!(bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
}

#[test]
fn record_file_name_flattens_groups() {
    assert_eq!(
        xtask::parity::record_file_name("1x/testQuotes.md"),
        "1x__testQuotes.md.json"
    );
}
