//! Folders, globs, file selection, duplicates and the order of output.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

mod common;

use common::{BAD, plain_path, project, project_without_git, yfl};

/// The `compact` lines of a run, as the files they name.
fn files(dir: &std::path::Path, args: &[&str]) -> Vec<String> {
    let out = yfl(dir)
        .args(["--format", "compact"])
        .args(args)
        .output()
        .unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|line| line.split(':').next().unwrap().to_string())
        .collect()
}

#[test]
fn default_is_current_dir() {
    let dir = project(&[("a.md", BAD)]);
    yfl(dir.path())
        .args(["--format", "compact"])
        .assert()
        .code(1)
        .stdout("a.md:2:8: error [no-quotes] unnecessary quotes\n");
}

fn selection_files() -> Vec<(&'static str, &'static [u8])> {
    vec![
        ("a.md", BAD),
        (".hidden/z.md", BAD),
        ("node_modules/x.md", BAD),
        ("sub/node_modules/y.md", BAD),
        ("ignored/w.md", BAD),
        (".gitignore", b"ignored/\n"),
        ("notes.txt", BAD),
        ("docs/d.md", BAD),
        ("docs/skip.md", BAD),
        (
            ".yaml-fm-lint.json",
            br#"{"files": {"exclude": ["docs/skip.md"]}}"#,
        ),
    ]
}

#[test]
fn walk_selection() {
    let dir = project(&selection_files());
    assert_eq!(
        files(dir.path(), &[]),
        [".hidden/z.md", "a.md", "docs/d.md"]
    );
}

#[test]
fn gitignore_without_git() {
    let dir = project_without_git(&selection_files());
    assert_eq!(
        files(dir.path(), &["."]),
        [".hidden/z.md", "a.md", "docs/d.md"]
    );
}

#[test]
fn globs() {
    let mut all = selection_files();
    all.push(("docs/n.txt", BAD));
    let dir = project(&all);
    assert_eq!(files(dir.path(), &["docs/*.txt"]), ["docs/n.txt"]);
    assert_eq!(files(dir.path(), &["*.md"]), ["a.md"]);
    assert_eq!(files(dir.path(), &["./*.md"]), ["a.md"]);
    assert_eq!(
        files(dir.path(), &["**/*.md"]),
        [".hidden/z.md", "a.md", "docs/d.md"]
    );
    assert_eq!(files(dir.path(), &["docs/**"]), ["docs/d.md", "docs/n.txt"]);
    assert_eq!(
        files(dir.path(), &["docs/*.{md,txt}"]),
        ["docs/d.md", "docs/n.txt"]
    );
    let absolute = format!("{}/docs/*.md", plain_path(dir.path()));
    assert_eq!(files(dir.path(), &[&absolute]), ["docs/d.md"]);
}

/// An existing file whose name has glob characters (`[slug].md`, `[...slug].md`) is that file,
/// not a pattern that happens to match something else.
#[test]
fn existing_names_with_glob_characters_are_literal() {
    let dir = project(&[
        ("blog/[slug].md", BAD),
        ("blog/s.md", BAD),
        ("[id]/a.md", BAD),
        ("i/b.md", BAD),
    ]);
    assert_eq!(files(dir.path(), &["blog/[slug].md"]), ["blog/[slug].md"]);
    let absolute = format!("{}/blog/[slug].md", plain_path(dir.path()));
    assert_eq!(files(dir.path(), &[&absolute]), ["blog/[slug].md"]);
    // A folder too.
    assert_eq!(files(dir.path(), &["[id]"]), ["[id]/a.md"]);
    // Nothing there: still a pattern.
    assert_eq!(files(dir.path(), &["blog/[s]*.md"]), ["blog/s.md"]);
}

#[test]
fn invalid_glob_exits_2() {
    let dir = project(&[("a.md", BAD)]);
    yfl(dir.path())
        .args(["--format", "compact", "[a"])
        .assert()
        .code(2)
        .stdout("")
        .stderr(predicates::str::contains("[a: invalid glob: "));
}

#[test]
fn glob_no_match_exit_2() {
    let dir = project(&[("a.md", BAD)]);
    yfl(dir.path())
        .args(["--format", "compact", "nothing/*.md"])
        .assert()
        .code(2)
        .stdout("")
        .stderr("no files matched\n");
    let empty = project(&[]);
    yfl(empty.path())
        .assert()
        .code(2)
        .stdout("")
        .stderr("no files matched\n");
}

#[test]
fn missing_argument_is_reported_next_to_the_others() {
    let dir = project(&[("a.md", BAD)]);
    yfl(dir.path())
        .args(["--format", "compact", "a.md", "nope"])
        .assert()
        .code(2)
        .stdout("a.md:2:8: error [no-quotes] unnecessary quotes\n")
        .stderr("nope: no such file or directory\n");
}

#[cfg(unix)]
#[test]
fn symlinks_skipped_while_walking() {
    let dir = project(&[("a.md", BAD), ("docs/d.md", BAD)]);
    std::os::unix::fs::symlink("a.md", dir.path().join("link.md")).unwrap();
    std::os::unix::fs::symlink("docs", dir.path().join("dl")).unwrap();
    assert_eq!(files(dir.path(), &["."]), ["a.md", "docs/d.md"]);
    assert_eq!(files(dir.path(), &["link.md"]), ["link.md"]);
}

#[cfg(unix)]
#[test]
fn unreadable_folder_is_reported() {
    use std::os::unix::fs::PermissionsExt;
    let dir = project(&[("a.md", BAD), ("locked/b.md", BAD)]);
    let locked = dir.path().join("locked");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let readable = std::fs::read_dir(&locked).is_ok();
    let out = yfl(dir.path())
        .args(["--format", "compact"])
        .output()
        .unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    if readable {
        return; // running as root: the folder can be read anyway
    }
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "a.md:2:8: error [no-quotes] unnecessary quotes\n"
    );
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        "locked: can't read: permission denied\n"
    );
}

#[test]
fn duplicates_linted_once() {
    let dir = project(&[("a.md", BAD)]);
    assert_eq!(files(dir.path(), &[".", "a.md", "./a.md"]), ["a.md"]);
}

#[test]
fn explicit_files_skip_the_checks() {
    let dir = project(&[("notes.txt", BAD), (".gitignore", b"notes.txt\n")]);
    assert_eq!(files(dir.path(), &["notes.txt"]), ["notes.txt"]);
    assert_eq!(files(dir.path(), &["."]), Vec::<String>::new());
}

#[test]
fn nested_config_error_is_fatal_and_deterministic() {
    let mut owned = Vec::new();
    for d in 0..30 {
        for f in 0..5 {
            owned.push(format!("d{d:02}/f{f}.md"));
        }
    }
    let mut files: Vec<(&str, &[u8])> = owned.iter().map(|p| (p.as_str(), BAD)).collect();
    files.push(("d17/.yaml-fm-lint.json", br#"{"rulez": 1}"#));
    files.push(("d23/.yaml-fm-lint.json", br#"{"root": 1}"#));
    let dir = project(&files);
    for _ in 0..5 {
        let out = yfl(dir.path())
            .args(["--format", "compact"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert_eq!(out.stdout, b"");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(stderr.contains("d17/.yaml-fm-lint.json"), "{stderr}");
        assert!(!stderr.contains("d23/"), "{stderr}");
    }
}

#[test]
fn parallel_output_is_stable() {
    let mut owned = Vec::new();
    for d in 0..20 {
        for f in 0..10 {
            owned.push(format!("d{d:02}/f{f}.md"));
        }
    }
    let mut files: Vec<(&str, &[u8])> = owned.iter().map(|p| (p.as_str(), BAD)).collect();
    files.push((
        ".yaml-fm-lint.json",
        br#"{"rules": {"no-quotes": "error"}}"#,
    ));
    files.push((
        "d05/.yaml-fm-lint.json",
        br#"{"rules": {"no-quotes": "warn"}}"#,
    ));
    let dir = project(&files);
    let run = || {
        yfl(dir.path())
            .args(["--format", "compact"])
            .output()
            .unwrap()
            .stdout
    };
    let first = run();
    let text = String::from_utf8(first.clone()).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 200);
    assert!(lines.is_sorted());
    for line in &lines {
        let severity = if line.starts_with("d05/") {
            ": warning ["
        } else {
            ": error ["
        };
        assert!(line.contains(severity), "{line}");
    }
    assert_eq!(run(), first);
    assert_eq!(run(), first);
}

#[test]
fn typed_ignored_folders_are_walked() {
    let dir = project(&[
        (".gitignore", b"ignored/\n"),
        ("a.md", BAD),
        ("ignored/w.md", BAD),
        ("ignored/deep/v.md", BAD),
        ("top/ignored/u.md", BAD),
    ]);
    // Found below a walk root: skipped.
    assert_eq!(files(dir.path(), &["."]), ["a.md"]);
    // Typed by the user: walked.
    assert_eq!(
        files(dir.path(), &["ignored"]),
        ["ignored/deep/v.md", "ignored/w.md"]
    );
    assert_eq!(files(dir.path(), &["ignored/*.md"]), ["ignored/w.md"]);
    assert_eq!(
        files(dir.path(), &["ignored/**/*.md"]),
        ["ignored/deep/v.md", "ignored/w.md"]
    );
    // Not typed, so still skipped.
    assert_eq!(files(dir.path(), &["top"]), Vec::<String>::new());
}
