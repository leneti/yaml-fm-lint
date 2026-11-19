//! Arguments, config discovery, explicit files, the compact format and exit codes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

mod common;

use common::{BAD, OK, WARN, project, project_without_git, yfl};
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

#[test]
fn version() {
    let dir = project(&[]);
    yfl(dir.path())
        .arg("--version")
        .assert()
        .code(0)
        .stdout("yaml-fm-lint 2.0.0-dev\n");
}

#[test]
fn help() {
    let dir = project(&[]);
    yfl(dir.path())
        .arg("--help")
        .assert()
        .code(0)
        .stdout(contains("--format"))
        .stdout(contains("--recursive").not());
}

#[test]
fn usage_errors_exit_2() {
    let dir = project(&[("x.md", OK)]);
    yfl(dir.path()).args(["--format", "xml"]).assert().code(2);
    yfl(dir.path())
        .args(["--stdin-filepath", "x.md"])
        .assert()
        .code(2);
}

#[test]
fn recursive_is_a_no_op() {
    let dir = project(&[("bad.md", BAD)]);
    let plain = yfl(dir.path())
        .args(["--format", "compact", "bad.md"])
        .output()
        .unwrap();
    let recursive = yfl(dir.path())
        .args(["--format", "compact", "-r", "bad.md"])
        .output()
        .unwrap();
    assert_eq!(plain.stdout, recursive.stdout);
    assert_eq!(recursive.status.code(), Some(1));
}

#[test]
fn compact_line() {
    let dir = project(&[("bad.md", BAD)]);
    yfl(dir.path())
        .args(["--format", "compact", "bad.md"])
        .assert()
        .code(1)
        .stdout("bad.md:2:8: error [no-quotes] unnecessary quotes\n");
}

#[test]
fn clean_exit_0() {
    let dir = project(&[("ok.md", OK)]);
    yfl(dir.path())
        .args(["--format", "compact", "ok.md"])
        .assert()
        .code(0)
        .stdout("");
}

#[test]
fn max_warnings() {
    let dir = project(&[("warn.md", WARN)]);
    let run = |extra: &[&str]| {
        yfl(dir.path())
            .args(["--format", "compact", "warn.md"])
            .args(extra)
            .assert()
            .stdout(
                "warn.md:2:5: warning [single-space-after-colon] more than one space after \":\"\n",
            )
    };
    run(&[]).code(0);
    run(&["--max-warnings", "0"]).code(1);
    run(&["--max-warnings", "1"]).code(0);
}

#[test]
fn cascading() {
    let no_quotes = |level: &str| format!(r#"{{"rules": {{"no-quotes": "{level}"}}}}"#);
    let dir = project_without_git(&[
        // Invalid, so reaching it past the `.git` folder would exit 2.
        ("t/.yaml-fm-lint.json", br#"{"excludeDirs": []}"#),
        ("t/repo/.git/HEAD", b"ref: refs/heads/main\n"),
        ("t/repo/.yaml-fm-lint.json", no_quotes("warn").as_bytes()),
        ("t/repo/a.md", BAD),
        ("t/repo/sub/.yaml-fm-lint.json", br#"{"root": true}"#),
        ("t/repo/sub/b.md", BAD),
        ("t/repo/wt/.git", b"gitdir: ../x"),
        ("t/repo/wt/c.md", BAD),
    ]);
    yfl(&dir.path().join("t/repo"))
        .args(["--format", "compact", "a.md", "sub/b.md", "wt/c.md"])
        .assert()
        .code(1)
        .stdout(
            "a.md:2:8: warning [no-quotes] unnecessary quotes\n\
             sub/b.md:2:8: error [no-quotes] unnecessary quotes\n\
             wt/c.md:2:8: error [no-quotes] unnecessary quotes\n",
        );
}

#[test]
fn config_flag_is_highest() {
    let dir = project(&[
        ("bad.md", BAD),
        ("ci.json", br#"{"rules": {"no-quotes": "off"}}"#),
    ]);
    yfl(dir.path())
        .args(["--format", "compact", "--config", "ci.json", "bad.md"])
        .assert()
        .code(0)
        .stdout("");
}

#[test]
fn config_error_is_fatal() {
    let dir = project(&[
        (".yaml-fm-lint.json", br#"{"excludeDirs": []}"#),
        ("bad.md", BAD),
    ]);
    yfl(dir.path())
        .args(["--format", "compact", "bad.md"])
        .assert()
        .code(2)
        .stdout("")
        .stderr(contains("config error: "))
        .stderr(contains(
            "/excludeDirs: unknown key `excludeDirs`; this looks like a 1.x config; \
             run `yaml-fm-lint migrate`",
        ));
}

#[test]
fn schema_file_is_loaded() {
    let dir = project(&[
        (".yaml-fm-lint.json", br#"{"schema": "./fm.schema.json"}"#),
        ("fm.schema.json", br#"{"required": ["title"]}"#),
        ("x.md", b"---\na: 1\n---\n"),
    ]);
    yfl(dir.path())
        .args(["--format", "compact", "x.md"])
        .assert()
        .code(1)
        .stdout(contains("[schema]"));
    std::fs::remove_file(dir.path().join("fm.schema.json")).unwrap();
    yfl(dir.path())
        .args(["--format", "compact", "x.md"])
        .assert()
        .code(2)
        .stdout("")
        .stderr(contains("can't read schema `"))
        .stderr(contains("fm.schema.json"));
}

#[test]
fn missing_path() {
    let dir = project(&[("bad.md", BAD)]);
    yfl(dir.path())
        .args(["--format", "compact", "nope.md", "bad.md"])
        .assert()
        .code(2)
        .stderr(contains("nope.md: no such file or directory"))
        .stdout("bad.md:2:8: error [no-quotes] unnecessary quotes\n");
}

#[test]
fn explicit_file_bypasses_filters() {
    let dir = project(&[
        (
            ".yaml-fm-lint.json",
            br#"{"files": {"exclude": ["a.txt"]}}"#,
        ),
        ("a.txt", BAD),
    ]);
    yfl(dir.path())
        .args(["--format", "compact", "a.txt"])
        .assert()
        .code(1)
        .stdout("a.txt:2:8: error [no-quotes] unnecessary quotes\n");
}

#[test]
fn invalid_utf8_is_a_read_error() {
    let dir = project(&[("u.md", b"---\nt: \xff\n---\n")]);
    yfl(dir.path())
        .args(["--format", "compact", "u.md"])
        .assert()
        .code(2)
        .stderr("u.md: not valid UTF-8\n");
}

#[test]
fn read_errors_are_listed_before_a_config_error() {
    let dir = project(&[
        (".yaml-fm-lint.json", br#"{"excludeDirs": []}"#),
        ("bad.md", BAD),
    ]);
    yfl(dir.path())
        .args(["--format", "compact", "bad.md", "nope.md"])
        .assert()
        .code(2)
        .stdout("")
        .stderr(
            predicates::str::is_match("^nope.md: no such file or directory\nconfig error: ")
                .unwrap(),
        );
}
