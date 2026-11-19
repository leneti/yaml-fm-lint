//! The `json`, `sarif` and `github` formats.
//!
//! `schemas/sarif-2.1.0.json` is the SARIF 2.1.0 JSON Schema from SchemaStore,
//! <https://json.schemastore.org/sarif-2.1.0.json> (draft-07, 112,469 bytes, sha256
//! `c96eb2d311c37b0a38cbd18c52d79a68f778bbd2d831abed7412d9850740f785`). The SchemaStore
//! repository is licensed under Apache-2.0. The file is vendored unchanged.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

mod common;

use std::path::Path;

use common::{BAD, OK, project, yfl};
use serde_json::{Value, json};

fn run(dir: &Path, format: &str, args: &[&str]) -> std::process::Output {
    yfl(dir)
        .args(["--format", format])
        .args(args)
        .output()
        .unwrap()
}

fn run_json(dir: &Path, format: &str, args: &[&str]) -> Value {
    let out = run(dir, format, args);
    serde_json::from_slice(&out.stdout).unwrap()
}

fn read_json(path: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A project holding a copy of `tests/fixtures/{1x,edge}`.
fn fixtures_project() -> tempfile::TempDir {
    let dir = project(&[]);
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    for name in ["1x", "edge"] {
        copy_dir(&fixtures.join(name), &dir.path().join(name));
    }
    dir
}

#[test]
fn json_exact() {
    let dir = project(&[("bad.md", BAD)]);
    let out = run(dir.path(), "json", &["bad.md"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.ends_with(b"}\n"));
    let expected: Value = serde_json::from_str(
        r#"{"version":1,"files":[{"path":"bad.md","diagnostics":[{"rule":"no-quotes","severity":"error","message":"unnecessary quotes","start":{"line":2,"column":8,"offset":11},"end":{"line":2,"column":15,"offset":18},"fix":{"applicability":"automatic","edits":[{"start":{"line":2,"column":8,"offset":11},"end":{"line":2,"column":15,"offset":18},"newText":"Hello"}]}}]}],"summary":{"errors":1,"warnings":0,"fixable":1,"files":1}}"#,
    )
    .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap(),
        expected
    );
}

#[test]
fn json_lists_clean_files() {
    let dir = project(&[("bad.md", BAD), ("ok.md", OK)]);
    let json = run_json(dir.path(), "json", &["bad.md", "ok.md"]);
    let files = json["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[1]["path"], "ok.md");
    assert_eq!(files[1]["diagnostics"], json!([]));
    assert_eq!(json["summary"]["files"], 2);
}

#[test]
fn json_matches_output_schema() {
    let dir = fixtures_project();
    let json = run_json(dir.path(), "json", &["."]);
    assert!(!json["files"].as_array().unwrap().is_empty());
    let schema = read_json("../../schemas/output.schema.json");
    let validator = jsonschema::draft202012::new(&schema).unwrap();
    let errors: Vec<_> = validator
        .iter_errors(&json)
        .map(|e| e.to_string())
        .collect();
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn sarif_valid() {
    let dir = fixtures_project();
    let sarif = run_json(dir.path(), "sarif", &["."]);
    let schema = read_json("tests/schemas/sarif-2.1.0.json");
    let validator = jsonschema::draft7::new(&schema).unwrap();
    let errors: Vec<_> = validator
        .iter_errors(&sarif)
        .map(|e| e.to_string())
        .collect();
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(sarif["runs"][0]["columnKind"], "unicodeCodePoints");
    assert_eq!(
        sarif["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .unwrap()
            .len(),
        17
    );
    assert!(!sarif["runs"][0]["results"].as_array().unwrap().is_empty());

    let dir = project(&[("bad.md", BAD)]);
    let sarif = run_json(dir.path(), "sarif", &["bad.md"]);
    let run = &sarif["runs"][0];
    let rules = run["tool"]["driver"]["rules"].as_array().unwrap();
    let index = rules.iter().position(|r| r["id"] == "no-quotes").unwrap();
    assert_eq!(
        rules[index]["helpUri"],
        "https://github.com/leneti/yaml-fm-lint/blob/main/docs/rules.md#no-quotes"
    );
    let result = &run["results"][0];
    assert_eq!(result["ruleId"], "no-quotes");
    assert_eq!(result["ruleIndex"], index);
    assert_eq!(result["level"], "error");
    assert_eq!(
        result["locations"][0]["physicalLocation"]["region"],
        json!({"startLine": 2, "startColumn": 8, "endLine": 2, "endColumn": 15})
    );
}

#[test]
fn github_line() {
    let dir = project(&[("bad.md", BAD)]);
    let out = run(dir.path(), "github", &["bad.md"]);
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "::error file=bad.md,line=2,col=8,endLine=2,endColumn=15,title=no-quotes::unnecessary quotes\n"
    );
}

#[test]
fn github_escapes() {
    let dup: &[u8] = b"---\na%: 1\na%: 2\n---\n";
    let dir = project(&[("a,b.md", dup)]);
    let out = run(dir.path(), "github", &["a,b.md"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.starts_with("::error file=a%2Cb.md,line=3,col=1,"),
        "{text}"
    );
    assert!(text.ends_with("::duplicate key `a%25`\n"), "{text}");

    #[cfg(unix)]
    {
        let dir = project(&[("a:b.md", dup)]);
        let out = run(dir.path(), "github", &["a:b.md"]);
        let text = String::from_utf8(out.stdout).unwrap();
        assert!(text.contains("file=a%3Ab.md,"), "{text}");
    }
}

#[test]
fn awkward_paths_in_every_format() {
    let dir = project(&[("docs/my file ü.md", BAD)]);
    let path = "docs/my file ü.md";
    let out = run(dir.path(), "compact", &[path]);
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .starts_with("docs/my file ü.md:2:8:")
    );
    let json = run_json(dir.path(), "json", &[path]);
    assert_eq!(json["files"][0]["path"], path);
    let sarif = run_json(dir.path(), "sarif", &[path]);
    assert_eq!(
        sarif["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
        "docs/my%20file%20%C3%BC.md"
    );
    let out = run(dir.path(), "github", &[path]);
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("file=docs/my file ü.md,line=2,")
    );
}
