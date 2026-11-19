//! `migrate`: converting a 1.x config to the 2.0 format.

use std::io::ErrorKind;
use std::path::Path;

use anyhow::Context;
use serde_json::{Map, Value, json};
use yaml_fm_lint_core::LEGACY_KEYS;

use crate::paths::{absolute, display};
use crate::run::{emit, write_stderr};
use crate::write::write_atomic;

const SCHEMA_URL: &str = "https://unpkg.com/yaml-fm-lint@2/config.schema.json";

/// Keys that only 2.0 configs have.
const V2_KEYS: [&str; 6] = ["$schema", "root", "ignoreKeys", "schema", "files", "rules"];

/// The folders 1.x excluded by default (its `config/default.json`); 2.0 skips what `.gitignore`
/// lists instead.
const V1_DEFAULT_EXCLUDES: [&str; 20] = [
    "__mocks__",
    "__tests__",
    ".DS_Store",
    ".eslintcache",
    ".expo",
    ".expo-shared",
    ".git",
    ".idea",
    ".node_repl_history",
    ".npm",
    ".tgz",
    ".vscode",
    ".yarn-integrity",
    "build",
    "coverage",
    "dist",
    "jspm_packages",
    "logs",
    "node_modules",
    "typings",
];

/// A converted config, and what the conversion left out.
pub struct Migrated {
    pub config: Value,
    pub notes: Vec<String>,
}

fn strings(object: &Map<String, Value>, key: &str) -> Result<Option<Vec<String>>, String> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    value
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .map(Some)
        .ok_or_else(|| format!("/{key}: must be an array of strings"))
}

/// Converts the parsed 1.x config `v1` (spec §7.5).
pub fn convert(v1: &Value) -> Result<Migrated, String> {
    let object = v1.as_object().ok_or("must be a JSON object")?;
    if let Some(key) = V2_KEYS.iter().find(|key| object.contains_key(**key)) {
        return Err(format!(
            "already looks like a 2.0 config (it has `{key}`); nothing to migrate"
        ));
    }
    let required = strings(object, "requiredAttributes")?.unwrap_or_default();
    let disabled = strings(object, "disabledAttributes")?.unwrap_or_default();
    let mandatory = match object.get("mandatory") {
        None => true,
        Some(value) => value.as_bool().ok_or("/mandatory: must be true or false")?,
    };
    let extensions = strings(object, "extensions")?;
    let exclude_dirs = strings(object, "excludeDirs")?.unwrap_or_default();
    let extra_exclude_dirs = strings(object, "extraExcludeDirs")?.unwrap_or_default();
    let exclude_files = strings(object, "excludeFiles")?.unwrap_or_default();
    let include_dirs = strings(object, "includeDirs")?.unwrap_or_default();

    let mut notes: Vec<String> = object
        .keys()
        .filter(|key| !LEGACY_KEYS.contains(&key.as_str()))
        .map(|key| format!("dropped unknown key `{key}`"))
        .collect();

    let (defaults, dirs): (Vec<String>, Vec<String>) = exclude_dirs
        .into_iter()
        .chain(extra_exclude_dirs)
        .partition(|dir| V1_DEFAULT_EXCLUDES.contains(&dir.as_str()));
    if !defaults.is_empty() {
        notes.push(format!(
            "dropped 1.x default excludes ({}); 2.0 skips what .gitignore lists",
            defaults.join(", ")
        ));
    }
    let exclude: Vec<String> = dirs
        .iter()
        .map(|dir| format!("**/{dir}/**"))
        .chain(exclude_files.iter().map(|file| format!("**/{file}")))
        .chain(include_dirs.iter().map(|dir| format!("!**/{dir}/**")))
        .collect();

    let mut config = Map::new();
    config.insert("$schema".to_string(), json!(SCHEMA_URL));
    let mut files = Map::new();
    if let Some(extensions) = extensions {
        files.insert("extensions".to_string(), json!(extensions));
    }
    if !exclude.is_empty() {
        files.insert("exclude".to_string(), json!(exclude));
    }
    if !files.is_empty() {
        config.insert("files".to_string(), Value::Object(files));
    }
    if !disabled.is_empty() {
        config.insert("ignoreKeys".to_string(), json!(disabled));
    }
    let mut rules = Map::new();
    if !mandatory {
        rules.insert("require-front-matter".to_string(), json!("warn"));
    }
    if !required.is_empty() {
        rules.insert(
            "required-keys".to_string(),
            json!(["error", {"keys": required}]),
        );
    }
    if !rules.is_empty() {
        config.insert("rules".to_string(), Value::Object(rules));
    }
    Ok(Migrated {
        config: Value::Object(config),
        notes,
    })
}

/// Converts the 1.x config at `path` (default `./.yaml-fm-lint.json`) and prints it, or with
/// `write` replaces the file; returns the exit code.
pub fn run(path: Option<&Path>, write: bool) -> anyhow::Result<u8> {
    let cwd = std::env::current_dir().context("can't find the current folder")?;
    let path = absolute(path.unwrap_or(Path::new("./.yaml-fm-lint.json")), &cwd);
    let shown = display(&path, &cwd);
    let folder = path.parent().unwrap_or(&path);
    // Only looked for, never read or run.
    if folder.join(".yaml-fm-lint.js").symlink_metadata().is_ok() {
        write_stderr(
            "found `.yaml-fm-lint.js`: yaml-fm-lint 2 never runs JavaScript config. Rewrite it as \
             `.yaml-fm-lint.json` by hand (see the migration guide), then delete the .js file.",
        );
        return Ok(2);
    }
    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            anyhow::bail!("{shown}: no such file or directory")
        }
        Err(e) => anyhow::bail!("{shown}: can't read: {e}"),
    };
    let v1: Value =
        serde_json::from_str(&source).map_err(|e| anyhow::anyhow!("{shown}: invalid JSON: {e}"))?;
    let migrated = convert(&v1).map_err(|e| anyhow::anyhow!("{shown}: {e}"))?;
    for note in &migrated.notes {
        write_stderr(note);
    }
    let text = format!("{}\n", serde_json::to_string_pretty(&migrated.config)?);
    if write {
        write_atomic(&path, &text).map_err(|e| anyhow::anyhow!("{shown}: can't write: {e}"))?;
        write_stderr(&format!("wrote {shown}"));
    } else {
        emit(std::io::stdout().lock(), &text)?;
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fails when core learns another 1.x key that `convert` doesn't handle.
    #[test]
    fn every_legacy_key_is_handled() {
        let v1: Map<String, Value> = LEGACY_KEYS
            .iter()
            .map(|key| {
                let value = if *key == "mandatory" {
                    json!(true)
                } else {
                    json!([])
                };
                ((*key).to_string(), value)
            })
            .collect();
        let migrated = convert(&Value::Object(v1)).unwrap();
        assert!(migrated.notes.is_empty(), "{:?}", migrated.notes);
    }

    #[test]
    fn two_point_zero_keys_are_refused() {
        for key in V2_KEYS {
            let err = convert(&json!({ key: null })).err().unwrap();
            assert!(err.contains(&format!("(it has `{key}`)")), "{err}");
        }
    }
}
