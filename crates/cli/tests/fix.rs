//! `--fix` writes, and `--stdin`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

mod common;

use common::{BAD, OK, project, yfl};
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

const FIXED: &str = "---\ntitle: Hello\n---\n";

fn names(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn fix_writes_and_reports() {
    let dir = project(&[("bad.md", BAD)]);
    let out = yfl(dir.path())
        .args(["--fix", "--format", "human", "--color", "never", "bad.md"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("bad.md")).unwrap(),
        FIXED
    );
    let out = String::from_utf8(out).unwrap();
    assert_eq!(out.lines().next(), Some("fixed 1 problem in 1 file"));
}

#[test]
fn fixed_line_is_printed_with_fix_only() {
    let dir = project(&[("ok.md", OK)]);
    let human = ["--format", "human", "--color", "never", "ok.md"];
    yfl(dir.path())
        .arg("--fix")
        .args(human)
        .assert()
        .code(0)
        .stdout(contains("fixed 0 problems in 0 files\n"));
    yfl(dir.path())
        .args(human)
        .assert()
        .code(0)
        .stdout(contains("fixed").not());
}

#[cfg(unix)]
#[test]
fn fix_leaves_clean_files_alone() {
    use std::os::unix::fs::MetadataExt;

    let dir = project(&[("ok.md", OK)]);
    let path = dir.path().join("ok.md");
    let before = std::fs::metadata(&path).unwrap();
    yfl(dir.path())
        .args(["--fix", "--format", "compact", "ok.md"])
        .assert()
        .code(0);
    let after = std::fs::metadata(&path).unwrap();
    assert_eq!(before.ino(), after.ino());
    assert_eq!(before.modified().unwrap(), after.modified().unwrap());
}

#[cfg(unix)]
#[test]
fn fix_preserves_permissions_and_leaves_no_temp_files() {
    use std::os::unix::fs::PermissionsExt;

    let dir = project(&[("bad.md", BAD)]);
    let path = dir.path().join("bad.md");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
    let before = names(dir.path());
    yfl(dir.path())
        .args(["--fix", "--format", "compact", "bad.md"])
        .assert()
        .code(0);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), FIXED);
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o640);
    assert_eq!(names(dir.path()), before);
}

#[cfg(unix)]
#[test]
fn fix_skips_symlinks() {
    let dir = project(&[("bad.md", BAD)]);
    std::os::unix::fs::symlink("bad.md", dir.path().join("link.md")).unwrap();
    yfl(dir.path())
        .args(["--fix", "--format", "compact", "link.md"])
        .assert()
        .code(0)
        .stderr(contains("link.md: skipped: symlink"));
    assert_eq!(std::fs::read(dir.path().join("bad.md")).unwrap(), BAD);
    assert!(
        std::fs::symlink_metadata(dir.path().join("link.md"))
            .unwrap()
            .is_symlink()
    );
}

#[cfg(unix)]
#[test]
fn fix_unwritable_dir_keeps_original() {
    use std::os::unix::fs::PermissionsExt;

    let dir = project(&[("ro/bad.md", BAD), ("rw/bad.md", BAD)]);
    let ro = dir.path().join("ro");
    std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o555)).unwrap();
    // Running as root: the folder can be written anyway.
    if std::fs::File::create(ro.join("probe")).is_ok() {
        return;
    }
    let output = yfl(dir.path())
        .args(["--fix", "--format", "compact", "ro/bad.md", "rw/bad.md"])
        .output()
        .unwrap();
    std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ro/bad.md: can't write:"), "{stderr}");
    assert_eq!(std::fs::read(ro.join("bad.md")).unwrap(), BAD);
    assert_eq!(names(&ro), ["bad.md"]);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("rw/bad.md")).unwrap(),
        FIXED
    );
}

#[test]
fn fix_unparseable_untouched() {
    let input = b"---\na: [\n---\n";
    let dir = project(&[("bad.md", input)]);
    yfl(dir.path())
        .args(["--fix", "--format", "compact", "bad.md"])
        .assert()
        .code(1)
        .stdout(contains("[yaml-syntax]"));
    assert_eq!(std::fs::read(dir.path().join("bad.md")).unwrap(), input);
}

#[test]
fn fix_checks_body_utf8() {
    let input = b"---\ntitle: Hello\n---\n\xff";
    let dir = project(&[("body.md", input)]);
    yfl(dir.path())
        .args(["--format", "compact", "body.md"])
        .assert()
        .code(0);
    yfl(dir.path())
        .args(["--fix", "--format", "compact", "body.md"])
        .assert()
        .code(2)
        .stderr(contains("not valid UTF-8"));
    assert_eq!(std::fs::read(dir.path().join("body.md")).unwrap(), input);
}

#[test]
fn fix_same_file_twice() {
    let dir = project(&[("bad.md", BAD)]);
    let out = yfl(dir.path())
        .args(["--fix", "--format", "human", "--color", "never"])
        .args(["bad.md", "./bad.md"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(out).unwrap();
    assert_eq!(out.lines().next(), Some("fixed 1 problem in 1 file"));
}

#[test]
fn fix_keeps_crlf() {
    let dir = project(&[("crlf.md", b"---\r\ntitle: \"Hello\"\r\n---\r\n")]);
    yfl(dir.path())
        .args(["--fix", "--format", "compact", "crlf.md"])
        .assert()
        .code(0);
    assert_eq!(
        std::fs::read(dir.path().join("crlf.md")).unwrap(),
        b"---\r\ntitle: Hello\r\n---\r\n"
    );
}

#[test]
fn fix_machine_formats_print_only_the_document() {
    let dir = project(&[("bad.md", BAD)]);
    let out = yfl(dir.path())
        .args(["--fix", "--format", "json", "bad.md"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["summary"]["errors"], 0);
    assert!(!text.contains("fixed"), "{text}");
}

#[test]
fn stdin_lint() {
    let dir = project(&[]);
    yfl(dir.path())
        .args(["--stdin", "--format", "compact"])
        .write_stdin(BAD)
        .assert()
        .code(1)
        .stdout("<stdin>:2:8: error [no-quotes] unnecessary quotes\n");

    let dir = project(&[(
        "docs/.yaml-fm-lint.json",
        br#"{"rules": {"no-quotes": "off"}}"#,
    )]);
    yfl(dir.path())
        .args([
            "--stdin",
            "--stdin-filepath",
            "docs/x.md",
            "--format",
            "compact",
        ])
        .write_stdin(BAD)
        .assert()
        .code(0)
        .stdout("");
}

#[test]
fn stdin_filepath_names_the_file() {
    let dir = project(&[]);
    yfl(dir.path())
        .args([
            "--stdin",
            "--stdin-filepath",
            "docs/x.md",
            "--format",
            "compact",
        ])
        .write_stdin(BAD)
        .assert()
        .code(1)
        .stdout("docs/x.md:2:8: error [no-quotes] unnecessary quotes\n");
}

#[test]
fn stdin_fix() {
    let dir = project(&[]);
    yfl(dir.path())
        .args(["--stdin", "--fix"])
        .write_stdin(BAD)
        .assert()
        .code(0)
        .stdout(FIXED);

    let broken = "---\na: [\n---\n";
    yfl(dir.path())
        .args(["--stdin", "--fix"])
        .write_stdin(broken)
        .assert()
        .code(1)
        .stdout(broken);

    let dir = project(&[(
        ".yaml-fm-lint.json",
        br#"{"rules": {"required-keys": ["error", {"keys": ["date"]}]}}"#,
    )]);
    yfl(dir.path())
        .args(["--stdin", "--fix"])
        .write_stdin(BAD)
        .assert()
        .code(1)
        .stdout(FIXED);
}

#[test]
fn stdin_invalid_utf8_is_fatal() {
    let dir = project(&[]);
    yfl(dir.path())
        .arg("--stdin")
        .write_stdin(&b"---\na: \xff\n---\n"[..])
        .assert()
        .code(2)
        .stderr(contains("<stdin>: not valid UTF-8"));
}

#[test]
fn stdin_with_paths_is_a_usage_error() {
    let dir = project(&[("a.md", OK)]);
    yfl(dir.path())
        .args(["--stdin", "a.md"])
        .assert()
        .code(2)
        .stderr(contains("--stdin can't be combined with paths"));
}
