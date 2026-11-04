use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use xtask::corpus::{SplitMix64, generate};

fn read_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path.strip_prefix(root).unwrap();
                let rel = rel.to_string_lossy().replace('\\', "/");
                out.insert(rel, fs::read(&path).unwrap());
            }
        }
    }
    out
}

#[test]
fn splitmix64_reference_values() {
    let mut rng = SplitMix64::new(0);
    assert_eq!(rng.next_u64(), 0xe220a8397b1dcdaf);
    assert_eq!(rng.next_u64(), 0x6e789e6aa1b965f4);
    assert_eq!(rng.next_u64(), 0x06c45d188009454f);
}

#[test]
fn same_seed_same_bytes() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    generate(50, 42, a.path()).unwrap();
    generate(50, 42, b.path()).unwrap();
    assert_eq!(read_tree(a.path()), read_tree(b.path()));
}

#[test]
fn different_seed_differs() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    generate(50, 1, a.path()).unwrap();
    generate(50, 2, b.path()).unwrap();
    assert_ne!(read_tree(a.path()), read_tree(b.path()));
}

#[test]
fn files_have_expected_shape() {
    let dir = tempfile::tempdir().unwrap();
    generate(200, 3, dir.path()).unwrap();
    let skipped = [
        "build",
        "dist",
        "coverage",
        "logs",
        "typings",
        "node_modules",
    ];
    let mut count = 0;
    for (rel, bytes) in read_tree(dir.path()) {
        if rel == "manifest.json" {
            continue;
        }
        count += 1;
        for part in rel.split('/') {
            assert!(!skipped.contains(&part), "{rel}");
        }
        assert!(!bytes.contains(&b'\r'), "{rel} has CR");
        let text = String::from_utf8(bytes).unwrap();
        let rest = text
            .strip_prefix("---\n")
            .unwrap_or_else(|| panic!("{rel}: no opener"));
        let (fm, body) = rest
            .split_once("\n---\n")
            .unwrap_or_else(|| panic!("{rel}: no closer"));
        let keys = fm
            .lines()
            .filter(|l| !l.starts_with(' ') && !l.starts_with('-') && l.contains(':'))
            .count();
        assert!((5..=20).contains(&keys), "{rel}: {keys} keys");
        assert!(
            (2048..=5120).contains(&body.len()),
            "{rel}: body {}",
            body.len()
        );
    }
    assert_eq!(count, 200);
}

#[test]
fn error_rate_is_about_ten_percent() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = generate(1000, 5, dir.path()).unwrap();
    let n = manifest.error_files.len();
    assert!((70..=130).contains(&n), "{n} error files");
    let mut sorted = manifest.error_files.clone();
    sorted.sort();
    assert_eq!(sorted, manifest.error_files);
    assert!(dir.path().join("manifest.json").is_file());
}

#[test]
fn refuses_non_empty_out_dir() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("x"), "x").unwrap();
    assert!(generate(1, 1, dir.path()).is_err());
}
