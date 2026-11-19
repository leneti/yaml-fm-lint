//! `print-config` and `migrate`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

mod common;

use common::{project, yfl};
use serde_json::{Value, json};

const SCHEMA_URL: &str = "https://unpkg.com/yaml-fm-lint@2/config.schema.json";

const V1_ALL: &str = r#"{"requiredAttributes": ["title"], "disabledAttributes": ["tags"], "mandatory": false, "extensions": [".md", ".mdx"], "excludeDirs": ["node_modules", "build", "drafts"], "extraExcludeDirs": ["tmp"], "excludeFiles": ["README.md"], "includeDirs": ["drafts/keep"], "colour": true}"#;

fn v2_all() -> Value {
    json!({
        "$schema": SCHEMA_URL,
        "files": {
            "extensions": [".md", ".mdx"],
            "exclude": ["**/drafts/**", "**/tmp/**", "**/README.md", "!**/drafts/keep/**"]
        },
        "ignoreKeys": ["tags"],
        "rules": {
            "require-front-matter": "warn",
            "required-keys": ["error", {"keys": ["title"]}]
        }
    })
}

fn stdout_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn print_config_shape() {
    let dir = project(&[
        (
            ".yaml-fm-lint.json",
            br#"{"rules": {"no-commas": "off"}, "schema": "./s.json"}"#,
        ),
        ("a.md", common::OK),
    ]);
    let output = yfl(dir.path())
        .args(["print-config", "a.md"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let json = stdout_json(&output);
    let layers = json["layers"].as_array().unwrap();
    assert_eq!(layers.len(), 1);
    let layer = layers[0].as_str().unwrap();
    assert!(layer.ends_with("/.yaml-fm-lint.json"), "{layer}");
    assert!(!layer.contains('\\'));
    assert_eq!(json["config"]["rules"]["no-commas"], "off");
    assert_eq!(
        json["config"]["schema"].as_str().unwrap(),
        layer.replace(".yaml-fm-lint.json", "s.json")
    );
}

#[test]
fn print_config_with_extra() {
    let dir = project(&[
        (".yaml-fm-lint.json", b"{}"),
        ("ci.json", br#"{"ignoreKeys": ["x"]}"#),
        ("a.md", common::OK),
    ]);
    // Options go after the subcommand: before it, clap reads `print-config` as a path.
    let output = yfl(dir.path())
        .args(["print-config", "--config", "ci.json", "a.md"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let json = stdout_json(&output);
    let layers = json["layers"].as_array().unwrap();
    assert_eq!(layers.len(), 2);
    assert!(layers[1].as_str().unwrap().ends_with("/ci.json"));
    assert_eq!(json["config"]["ignoreKeys"], json!(["x"]));
}

#[test]
fn print_config_error() {
    let dir = project(&[(".yaml-fm-lint.json", b"{")]);
    let output = yfl(dir.path())
        .args(["print-config", "a.md"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(".yaml-fm-lint.json"),
        "{:?}",
        output.stderr
    );
}

#[test]
fn migrate_maps_every_key() {
    let dir = project(&[(".yaml-fm-lint.json", V1_ALL.as_bytes())]);
    let output = yfl(dir.path()).arg("migrate").output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout_json(&output), v2_all());
    assert!(output.stdout.ends_with(b"}\n"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("dropped unknown key `colour`"), "{stderr}");
    assert!(
        stderr.contains("dropped 1.x default excludes (node_modules, build)"),
        "{stderr}"
    );
}

#[test]
fn migrate_drops_defaults_from_extra_exclude_dirs_too() {
    let dir = project(&[(
        ".yaml-fm-lint.json",
        br#"{"extraExcludeDirs": ["dist", "tmp"]}"#,
    )]);
    let output = yfl(dir.path()).arg("migrate").output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout_json(&output)["files"]["exclude"],
        json!(["**/tmp/**"])
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("(dist)"));
}

#[test]
fn migrated_config_is_valid() {
    let dir = project(&[
        (".yaml-fm-lint.json", V1_ALL.as_bytes()),
        ("a.md", common::OK),
    ]);
    let migrated = yfl(dir.path()).arg("migrate").output().unwrap().stdout;
    std::fs::write(dir.path().join(".yaml-fm-lint.json"), migrated).unwrap();
    let output = yfl(dir.path())
        .args(["print-config", "a.md"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
}

#[test]
fn migrate_mandatory_true_dropped() {
    let dir = project(&[(
        ".yaml-fm-lint.json",
        br#"{"mandatory": true, "requiredAttributes": []}"#,
    )]);
    let output = yfl(dir.path()).arg("migrate").output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout_json(&output), json!({"$schema": SCHEMA_URL}));
}

#[test]
fn migrate_write() {
    let dir = project(&[(".yaml-fm-lint.json", V1_ALL.as_bytes())]);
    let output = yfl(dir.path())
        .args(["migrate", "--write"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.ends_with("wrote .yaml-fm-lint.json\n"), "{stderr}");
    let written = std::fs::read(dir.path().join(".yaml-fm-lint.json")).unwrap();
    assert!(written.ends_with(b"}\n"));
    assert_eq!(serde_json::from_slice::<Value>(&written).unwrap(), v2_all());
}

#[test]
fn migrate_never_runs_js() {
    let dir = project(&[
        (
            ".yaml-fm-lint.js",
            b"require('fs').writeFileSync('PWNED', 'x')",
        ),
        (".yaml-fm-lint.json", br#"{"mandatory": false}"#),
    ]);
    let output = yfl(dir.path()).arg("migrate").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("never runs JavaScript config"));
    assert!(!dir.path().join("PWNED").exists());
}

#[test]
fn migrate_refuses_2x_config() {
    let dir = project(&[(".yaml-fm-lint.json", br#"{"rules": {}}"#)]);
    let output = yfl(dir.path())
        .args(["migrate", "--write"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("already looks like a 2.0 config"));
    assert_eq!(
        std::fs::read(dir.path().join(".yaml-fm-lint.json")).unwrap(),
        br#"{"rules": {}}"#
    );
}

#[test]
fn migrate_errors() {
    let dir = project(&[]);
    let run = |content: Option<&str>, args: &[&str]| {
        match content {
            Some(c) => std::fs::write(dir.path().join(".yaml-fm-lint.json"), c).unwrap(),
            None => {
                let _ = std::fs::remove_file(dir.path().join(".yaml-fm-lint.json"));
            }
        }
        let output = yfl(dir.path()).args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        String::from_utf8_lossy(&output.stderr).into_owned()
    };
    assert!(run(None, &["migrate"]).contains(".yaml-fm-lint.json"));
    assert!(
        run(Some(r#"{"excludeDirs": "x"}"#), &["migrate"])
            .contains("/excludeDirs: must be an array of strings")
    );
    assert!(
        run(Some(r#"{"mandatory": 1}"#), &["migrate"])
            .contains("/mandatory: must be true or false")
    );
    assert!(run(Some("not json"), &["migrate"]).contains("invalid JSON"));
    assert!(run(Some("[]"), &["migrate"]).contains("must be a JSON object"));
}
