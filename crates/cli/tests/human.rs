//! The human format, colour, `--quiet`, control characters and closed pipes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

mod common;

use std::io::{BufRead, BufReader};
use std::process::Stdio;

use common::{BAD, OK, WARN, project, yfl};
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use regex::Regex;

/// The timing changes from run to run.
fn normalise(stdout: &[u8]) -> String {
    let text = String::from_utf8(stdout.to_vec()).unwrap();
    Regex::new(r" in \d+ ms")
        .unwrap()
        .replace_all(&text, " in <T> ms")
        .into_owned()
}

fn three_files() -> tempfile::TempDir {
    project(&[("b.md", BAD), ("a.md", WARN), ("ok.md", OK)])
}

#[test]
fn human_snapshot() {
    let dir = three_files();
    let out = yfl(dir.path()).args(["--color", "never"]).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    insta::assert_snapshot!(normalise(&out.stdout));
}

#[test]
fn summary_lines() {
    let dir = three_files();
    let out = yfl(dir.path()).args(["--color", "never"]).output().unwrap();
    let text = normalise(&out.stdout);
    assert_eq!(
        text.lines().last().unwrap(),
        "✘ 1 error, 1 warning in 2 files (2 fixable with --fix) · 3 files in <T> ms"
    );

    let dir = project(&[("ok.md", OK)]);
    let out = yfl(dir.path()).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(normalise(&out.stdout), "✔ no problems · 1 file in <T> ms\n");
}

#[test]
fn colour() {
    let dir = project(&[("bad.md", BAD)]);
    let run = |args: &[&str], env: &[(&str, &str)]| {
        yfl(dir.path())
            .args(args)
            .envs(env.iter().copied())
            .assert()
    };
    run(&["--color", "always"], &[]).stdout(contains("\x1b["));
    run(&["--color", "never"], &[]).stdout(contains("\x1b[").not());
    run(&[], &[("NO_COLOR", "1")]).stdout(contains("\x1b[").not());
    run(&[], &[("CLICOLOR_FORCE", "1")]).stdout(contains("\x1b["));
    // Not a terminal.
    run(&[], &[]).stdout(contains("\x1b[").not());
}

#[test]
fn quiet_drops_warnings() {
    let dir = project(&[("a.md", WARN)]);
    let out = yfl(dir.path())
        .args(["-q", "--max-warnings", "0", "a.md"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        normalise(&out.stdout).lines().last().unwrap(),
        "✔ no problems · 1 file in <T> ms"
    );

    let dir = three_files();
    yfl(dir.path())
        .args(["-q", "--format", "compact"])
        .assert()
        .code(1)
        .stdout("b.md:2:8: error [no-quotes] unnecessary quotes\n");
}

/// A name with a raw escape and a line break must not reach the terminal as such.
#[cfg(unix)]
#[test]
fn control_characters_are_escaped() {
    let dir = project(&[("a\x1b[31mb\nc.md", BAD)]);
    let expected_path = r"a\u{1b}[31mb\u{a}c.md";
    let compact = yfl(dir.path())
        .args(["--format", "compact", "--color", "always"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8(compact.stdout).unwrap(),
        format!("{expected_path}:2:8: error [no-quotes] unnecessary quotes\n")
    );
    let human = yfl(dir.path()).args(["--color", "never"]).output().unwrap();
    let human = String::from_utf8(human.stdout).unwrap();
    assert!(human.contains(expected_path), "{human}");
    assert!(!human.contains('\x1b'), "{human}");
}

/// The message of `duplicate-keys` quotes the key, which the file's author controls.
#[test]
fn control_characters_in_messages_are_escaped() {
    let dir = project(&[("d.md", b"---\n\"a\\e[2J\": 1\n\"a\\e[2J\": 2\n---\n")]);
    for format in ["compact", "human"] {
        let out = yfl(dir.path())
            .args(["--format", format, "--color", "always"])
            .output()
            .unwrap();
        let text = String::from_utf8(out.stdout).unwrap();
        assert!(text.contains(r"a\u{1b}[2J"), "{format}: {text}");
        assert!(!text.contains("a\x1b[2J"), "{format}: {text}");
    }
}

/// `yaml-fm-lint . | head -1`: stop quietly, keep the exit code.
#[test]
fn closed_stdout_is_quiet() {
    // Long names make the output far bigger than a pipe buffer, so the write really fails.
    let long = "x".repeat(200);
    let names: Vec<String> = (0..700).map(|i| format!("{long}{i}.md")).collect();
    let files: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), BAD)).collect();
    let dir = project(&files);

    let mut child = std::process::Command::new(assert_cmd::cargo::cargo_bin("yaml-fm-lint"))
        .current_dir(dir.path())
        .args(["--format", "compact"])
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path())
        .env("GIT_CONFIG_GLOBAL", dir.path().join("no-git-config"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut first = String::new();
    BufReader::new(stdout).read_line(&mut first).unwrap();
    assert!(first.contains("[no-quotes]"), "{first}");
    // The reader is gone now.
    let out = child.wait_with_output().unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert_eq!(stderr, "");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn crlf_files_show_clean_snippets() {
    let dir = project(&[("crlf.md", b"---\r\ntitle: \"H\xc3\xa9llo\"\r\n---\r\n")]);
    let out = yfl(dir.path()).args(["--color", "never"]).output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("\n2 | title: \"Héllo\"\n  |        ^^^^^^^\n"),
        "{text}"
    );
}
