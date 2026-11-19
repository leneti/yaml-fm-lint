//! Compares what 2.0 reports for every fixture with what 1.7.1 reported (recorded in
//! `tests/parity/expected-1x/`). Every difference must be explained in
//! `tests/parity/allowed-diffs.toml`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;

use common::{project, yfl};

/// A problem as a `(rule, line)` pair.
type Problem = (String, usize);

#[derive(Deserialize)]
struct AllowedDiffs {
    #[serde(default)]
    diff: Vec<Diff>,
}

#[derive(Deserialize)]
struct Diff {
    id: String,
    decision: String,
    reason: String,
    #[serde(default)]
    matches: Vec<Match>,
}

#[derive(Deserialize)]
struct Match {
    fixture: String,
    rule: String,
    line: usize,
    /// `removed` (in 1.x, not in 2.0) or `added`.
    change: String,
}

fn repo_file(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn allowed() -> AllowedDiffs {
    let text = fs::read_to_string(repo_file("tests/parity/allowed-diffs.toml")).unwrap();
    toml::from_str(&text).unwrap()
}

/// The fixtures as `group/name.md`, sorted.
fn fixtures() -> Vec<String> {
    let mut found = Vec::new();
    for group in ["1x", "edge"] {
        for entry in fs::read_dir(repo_file(&format!("tests/fixtures/{group}"))).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            if name.ends_with(".md") {
                found.push(format!("{group}/{name}"));
            }
        }
    }
    found.sort();
    found
}

/// The rule that 2.0 reports for what 1.x called `message`.
fn rule_of_1x(message: &str) -> &'static str {
    match message {
        "missing required attributes" => "required-keys",
        "there must be no empty lines" => "no-blank-lines",
        "there must be no whitespace before colons" => "no-space-before-colon",
        "there must be no quotes in the front matter" => "no-quotes",
        "there must be no trailing spaces" => "no-trailing-spaces",
        "there must be no brackets" => "no-flow-sequence",
        "there must be no curly braces" => "no-flow-mapping",
        "lines cannot be indented more than 2 spaces from the previous line" => "indentation",
        "there must be no trailing commas" => "no-trailing-commas",
        "possibly unintended whitespace" => "whitespace",
        "possibly unintended commas" => "no-commas",
        other => panic!("unknown 1.x message: {other}"),
    }
}

/// The problems recorded for a fixture by 1.7.1.
fn expected(fixture: &str) -> BTreeSet<Problem> {
    let name = fixture.replace('/', "__");
    let text =
        fs::read_to_string(repo_file(&format!("tests/parity/expected-1x/{name}.json"))).unwrap();
    let record: Value = serde_json::from_str(&text).unwrap();
    assert!(
        record["crashed"].is_null(),
        "1.x crashed on {fixture}: {}",
        record["crashed"]
    );
    let mut set = BTreeSet::new();
    for section in ["errors", "warnings"] {
        for (key, value) in record[section].as_object().unwrap() {
            match key.as_str() {
                "customError" => {
                    set.insert((
                        "yaml-syntax".to_string(),
                        value["row"].as_u64().unwrap() as usize,
                    ));
                }
                "noFrontMatter" => {
                    set.insert(("require-front-matter".to_string(), 1));
                }
                message => {
                    let rule = rule_of_1x(message);
                    for item in value.as_array().unwrap() {
                        let row = item.as_u64().or_else(|| item["row"].as_u64()).unwrap();
                        set.insert((rule.to_string(), row as usize));
                    }
                }
            }
        }
    }
    set
}

/// What 2.0 reports for every fixture, by fixture name.
fn actual_2x() -> BTreeMap<String, BTreeSet<Problem>> {
    let dir = project(&[]);
    for fixture in fixtures() {
        let target = dir.path().join(&fixture);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(repo_file(&format!("tests/fixtures/{fixture}")), target).unwrap();
    }
    let output = yfl(dir.path())
        .args(["--format", "json", "."])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let report: Value = serde_json::from_slice(&output).unwrap();
    let mut by_fixture = BTreeMap::new();
    for file in report["files"].as_array().unwrap() {
        let path = file["path"]
            .as_str()
            .unwrap()
            .trim_start_matches("./")
            .to_string();
        let set = file["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| {
                let rule = match d["rule"].as_str().unwrap() {
                    "no-repeated-spaces" | "single-space-after-colon" => "whitespace",
                    other => other,
                };
                (
                    rule.to_string(),
                    d["start"]["line"].as_u64().unwrap() as usize,
                )
            })
            .collect();
        by_fixture.insert(path, set);
    }
    by_fixture
}

#[test]
fn parity_with_1x() {
    let actual = actual_2x();
    let allowed = allowed();
    let mut allowed_matches: BTreeSet<(String, String, usize, String)> = BTreeSet::new();
    for m in allowed.diff.iter().flat_map(|d| &d.matches) {
        allowed_matches.insert((m.fixture.clone(), m.rule.clone(), m.line, m.change.clone()));
    }

    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for fixture in fixtures() {
        let before = expected(&fixture);
        let after = actual.get(&fixture).cloned().unwrap_or_default();
        for (change, sign, set) in [
            ("removed", '-', before.difference(&after)),
            ("added", '+', after.difference(&before)),
        ] {
            for (rule, line) in set {
                let key = (fixture.clone(), rule.clone(), *line, change.to_string());
                if allowed_matches.contains(&key) {
                    seen.insert(key);
                } else {
                    problems.push(format!("{fixture}: {sign}{rule}@{line}"));
                }
            }
        }
    }
    for key in allowed_matches.difference(&seen) {
        let (fixture, rule, line, change) = key;
        let sign = if change == "added" { '+' } else { '-' };
        problems.push(format!(
            "allowed but did not happen: {fixture}: {sign}{rule}@{line}"
        ));
    }
    assert!(
        problems.is_empty(),
        "{} parity problems:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

#[test]
fn allowed_diffs_are_well_formed() {
    let allowed = allowed();
    let kebab = regex::Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").unwrap();
    let mut ids = BTreeSet::new();
    for diff in &allowed.diff {
        assert!(
            kebab.is_match(&diff.id),
            "id is not kebab-case: {}",
            diff.id
        );
        assert!(ids.insert(diff.id.clone()), "duplicate id: {}", diff.id);
        assert!(
            !diff.decision.trim().is_empty(),
            "{}: empty decision",
            diff.id
        );
        assert!(!diff.reason.trim().is_empty(), "{}: empty reason", diff.id);
        for m in &diff.matches {
            assert!(
                repo_file(&format!("tests/fixtures/{}", m.fixture)).is_file(),
                "{}: no fixture {}",
                diff.id,
                m.fixture
            );
            assert!(
                m.change == "removed" || m.change == "added",
                "{}: bad change {}",
                diff.id,
                m.change
            );
        }
    }
}
