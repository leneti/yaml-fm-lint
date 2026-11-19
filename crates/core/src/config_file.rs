//! Parsing and validating `.yaml-fm-lint.json` files.
//!
//! The core does no I/O: the caller reads each file and passes its text in a [`ConfigLayer`].

use std::collections::BTreeMap;

use jsonc_parser::{ParseOptions, parse_to_serde_value};
use serde_json::{Map, Value};

use crate::files::check_pattern;
use crate::{Config, ConfigError, ConfigErrorKind, FilePattern, Level, RuleId};

/// One config file, already read by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigLayer {
    /// The absolute, `/`-separated folder of the file; relative paths in it resolve against this.
    pub dir: String,
    /// The path of the file, used to name it in errors and in [`Config::layers`].
    pub path: String,
    /// The text of the file.
    pub source: String,
}

/// The settings of one config file, validated but not yet merged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Layer {
    root: Option<bool>,
    extensions: Option<Vec<String>>,
    include: Option<Vec<FilePattern>>,
    exclude: Option<Vec<FilePattern>>,
    ignore_keys: Option<Vec<String>>,
    rules: BTreeMap<RuleId, RuleSetting>,
    /// Already absolute.
    schema: Option<String>,
}

/// A rule's value in a config file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RuleSetting {
    /// `"error"`, `"warn"` or `"off"`.
    Level(Level),
    /// `["error" | "warn", {"keys": [...]}]` for `required-keys`.
    RequiredKeys(Level, Vec<String>),
    /// `["error" | "warn", {"size": n}]` for `indentation`.
    Indentation(Level, u8),
}

/// Keys of 1.x configs, which 2.0 no longer reads.
pub(crate) const LEGACY_KEYS: [&str; 8] = [
    "requiredAttributes",
    "disabledAttributes",
    "mandatory",
    "extensions",
    "excludeDirs",
    "extraExcludeDirs",
    "excludeFiles",
    "includeDirs",
];

const BAD_LEVEL: &str = r#"must be "error", "warn" or "off", or ["error" | "warn", {options}]"#;

/// Builds the configuration from config files, lowest priority first, and an optional extra file
/// that has the highest priority.
///
/// Files are parsed from the last to the first and parsing stops after the first one with
/// `root: true`, so an invalid file above it is no error. A `root: true` in `extra` drops every
/// other file.
pub fn resolve_config(
    layers: &[ConfigLayer],
    extra: Option<&ConfigLayer>,
) -> Result<Config, ConfigError> {
    let extra = extra
        .map(|file| parse_layer(file).map(|layer| (file, layer)))
        .transpose()?;
    let mut parsed = Vec::new();
    if extra
        .as_ref()
        .is_none_or(|(_, layer)| layer.root != Some(true))
    {
        for file in layers.iter().rev() {
            let layer = parse_layer(file)?;
            let is_root = layer.root == Some(true);
            parsed.push((file, layer));
            if is_root {
                break;
            }
        }
        parsed.reverse();
    }
    parsed.extend(extra);
    let mut config = Config::default();
    for (file, layer) in &parsed {
        apply(&mut config, layer);
        config.layers.push(file.path.clone());
    }
    Ok(config)
}

/// Merges `layer` into `config`: each setting that is present replaces the existing one, and a
/// rule's value replaces everything that rule had, options included.
pub(crate) fn apply(config: &mut Config, layer: &Layer) {
    if let Some(extensions) = &layer.extensions {
        config.files.extensions.clone_from(extensions);
    }
    if let Some(include) = &layer.include {
        config.files.include.clone_from(include);
    }
    if let Some(exclude) = &layer.exclude {
        config.files.exclude.clone_from(exclude);
    }
    if let Some(ignore_keys) = &layer.ignore_keys {
        config.ignore_keys.clone_from(ignore_keys);
    }
    if let Some(schema) = &layer.schema {
        config.schema = Some(schema.clone());
    }
    for (rule, setting) in &layer.rules {
        let (RuleSetting::Level(level)
        | RuleSetting::RequiredKeys(level, _)
        | RuleSetting::Indentation(level, _)) = setting;
        config.rules.insert(*rule, *level);
        match (rule, setting) {
            (RuleId::RequiredKeys, RuleSetting::RequiredKeys(_, keys)) => {
                config.required_keys.clone_from(keys);
            }
            (RuleId::RequiredKeys, _) => config.required_keys.clear(),
            (RuleId::Indentation, RuleSetting::Indentation(_, size)) => config.indent_size = *size,
            (RuleId::Indentation, _) => config.indent_size = Config::default().indent_size,
            _ => {}
        }
    }
}

/// Parses and validates the text of one config file.
pub(crate) fn parse_layer(layer: &ConfigLayer) -> Result<Layer, ConfigError> {
    let options = ParseOptions {
        allow_comments: true,
        allow_loose_object_property_names: false,
        allow_trailing_commas: true,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
        allow_bare_decimal_point_numbers: false,
        allow_non_finite_numbers: false,
        allow_extended_string_escapes: false,
    };
    let parser = Parser {
        file: &layer.path,
        dir: &layer.dir,
    };
    let value =
        parse_to_serde_value::<Value>(&layer.source, &options).map_err(|e| ConfigError {
            line: Some(e.line_display()),
            column: Some(e.column_display()),
            ..parser.failure(None, format!("invalid JSON: {}", e.kind()))
        })?;
    let Value::Object(object) = value else {
        return Err(parser.failure(None, "config must be a JSON object".into()));
    };
    parser.layer(&object)
}

/// Validates one config file; `file` names it in errors and `dir` is what its paths are
/// relative to.
struct Parser<'a> {
    file: &'a str,
    dir: &'a str,
}

/// A JSON pointer segment for `key`.
fn segment(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

impl Parser<'_> {
    fn failure(&self, pointer: Option<&str>, message: String) -> ConfigError {
        ConfigError {
            kind: ConfigErrorKind::Config,
            file: Some(self.file.to_string()),
            pointer: pointer.map(str::to_string),
            line: None,
            column: None,
            message,
        }
    }

    fn error<T>(&self, pointer: &str, message: &str) -> Result<T, ConfigError> {
        Err(self.failure(Some(pointer), message.to_string()))
    }

    fn unknown_key<T>(&self, pointer: &str, key: &str, legacy: bool) -> Result<T, ConfigError> {
        let hint = if legacy && LEGACY_KEYS.contains(&key) {
            "; this looks like a 1.x config; run `yaml-fm-lint migrate`"
        } else {
            ""
        };
        self.error(pointer, &format!("unknown key `{key}`{hint}"))
    }

    fn boolean(&self, value: &Value, pointer: &str) -> Result<bool, ConfigError> {
        match value {
            Value::Bool(b) => Ok(*b),
            _ => self.error(pointer, "must be true or false"),
        }
    }

    fn string(&self, value: &Value, pointer: &str) -> Result<String, ConfigError> {
        match value {
            Value::String(s) => Ok(s.clone()),
            _ => self.error(pointer, "must be a string"),
        }
    }

    fn strings(&self, value: &Value, pointer: &str) -> Result<Vec<String>, ConfigError> {
        let Value::Array(items) = value else {
            return self.error(pointer, "must be an array of strings");
        };
        items
            .iter()
            .enumerate()
            .map(|(i, item)| self.string(item, &format!("{pointer}/{i}")))
            .collect()
    }

    fn object<'v>(
        &self,
        value: &'v Value,
        pointer: &str,
    ) -> Result<&'v Map<String, Value>, ConfigError> {
        match value {
            Value::Object(object) => Ok(object),
            _ => self.error(pointer, "must be an object"),
        }
    }

    /// `list` is `include` or `exclude`.
    fn patterns(
        &self,
        value: &Value,
        pointer: &str,
        list: &str,
    ) -> Result<Vec<FilePattern>, ConfigError> {
        self.strings(value, pointer)?
            .into_iter()
            .enumerate()
            .map(|(index, pattern)| {
                check_pattern(&pattern, list, index).map_err(|e| ConfigError {
                    file: Some(self.file.to_string()),
                    ..e
                })?;
                Ok(FilePattern {
                    dir: self.dir.to_string(),
                    pattern,
                })
            })
            .collect()
    }

    fn layer(&self, object: &Map<String, Value>) -> Result<Layer, ConfigError> {
        let mut layer = Layer {
            root: None,
            extensions: None,
            include: None,
            exclude: None,
            ignore_keys: None,
            rules: BTreeMap::new(),
            schema: None,
        };
        for (key, value) in object {
            let pointer = format!("/{}", segment(key));
            match key.as_str() {
                "$schema" => {
                    self.string(value, &pointer)?;
                }
                "root" => layer.root = Some(self.boolean(value, &pointer)?),
                "ignoreKeys" => layer.ignore_keys = Some(self.strings(value, &pointer)?),
                "schema" => {
                    layer.schema = Some(join_path(self.dir, &self.string(value, &pointer)?));
                }
                "files" => self.files(&mut layer, value, &pointer)?,
                "rules" => {
                    for (name, setting) in self.object(value, &pointer)? {
                        let (rule, setting) = self.rule(name, setting)?;
                        layer.rules.insert(rule, setting);
                    }
                }
                _ => return self.unknown_key(&pointer, key, true),
            }
        }
        Ok(layer)
    }

    fn files(&self, layer: &mut Layer, value: &Value, pointer: &str) -> Result<(), ConfigError> {
        for (key, value) in self.object(value, pointer)? {
            let pointer = format!("{pointer}/{}", segment(key));
            match key.as_str() {
                "extensions" => layer.extensions = Some(self.strings(value, &pointer)?),
                "include" => layer.include = Some(self.patterns(value, &pointer, "include")?),
                "exclude" => layer.exclude = Some(self.patterns(value, &pointer, "exclude")?),
                _ => return self.unknown_key(&pointer, key, false),
            }
        }
        Ok(())
    }

    fn rule(&self, name: &str, value: &Value) -> Result<(RuleId, RuleSetting), ConfigError> {
        let pointer = format!("/rules/{}", segment(name));
        let Some(rule) = RuleId::from_name(name) else {
            return self.error(&pointer, &format!("unknown rule `{name}`"));
        };
        if !rule.is_configurable() {
            return self.error(&pointer, &format!("rule `{rule}` can't be configured"));
        }
        let (level, options) = match value {
            Value::String(s) => (self.level(s, true, &pointer)?, None),
            Value::Array(items) => match items.as_slice() {
                [Value::String(s), options] => (self.level(s, false, &pointer)?, Some(options)),
                _ => return self.error(&pointer, BAD_LEVEL),
            },
            _ => return self.error(&pointer, BAD_LEVEL),
        };
        let Some(options) = options else {
            return Ok((rule, RuleSetting::Level(level)));
        };
        let options_pointer = format!("{pointer}/1");
        match rule {
            RuleId::RequiredKeys => {
                let mut keys = Vec::new();
                for (key, value) in self.object(options, &options_pointer)? {
                    let pointer = format!("{options_pointer}/{}", segment(key));
                    match key.as_str() {
                        "keys" => keys = self.strings(value, &pointer)?,
                        _ => return self.error(&pointer, &format!("unknown option `{key}`")),
                    }
                }
                Ok((rule, RuleSetting::RequiredKeys(level, keys)))
            }
            RuleId::Indentation => {
                let mut size = Config::default().indent_size;
                for (key, value) in self.object(options, &options_pointer)? {
                    let pointer = format!("{options_pointer}/{}", segment(key));
                    match key.as_str() {
                        "size" => {
                            size = match value.as_u64().and_then(|n| u8::try_from(n).ok()) {
                                Some(n @ 1..=8) => n,
                                _ => return self.error(&pointer, "must be an integer from 1 to 8"),
                            };
                        }
                        _ => return self.error(&pointer, &format!("unknown option `{key}`")),
                    }
                }
                Ok((rule, RuleSetting::Indentation(level, size)))
            }
            _ => self.error(&pointer, &format!("rule `{rule}` takes no options")),
        }
    }

    /// `"off"` is only allowed when the level stands alone, not in `[level, options]`.
    fn level(&self, text: &str, allow_off: bool, pointer: &str) -> Result<Level, ConfigError> {
        match text {
            "error" => Ok(Level::Error),
            "warn" => Ok(Level::Warn),
            "off" if allow_off => Ok(Level::Off),
            _ => self.error(pointer, BAD_LEVEL),
        }
    }
}

/// Resolves `rel` against the absolute folder `dir`, folding `.` and `..` without touching the
/// file system. `\` counts as `/`, and an absolute `rel` (`/…`, `//…` or `X:/…`) is returned as
/// it is.
pub(crate) fn join_path(dir: &str, rel: &str) -> String {
    let rel = rel.replace('\\', "/");
    if root_len(&rel) > 0 {
        return rel;
    }
    let joined = format!("{}/{rel}", dir.trim_end_matches('/'));
    let (root, rest) = joined
        .split_at_checked(root_len(&joined))
        .unwrap_or(("", &joined));
    let mut parts: Vec<&str> = Vec::new();
    for part in rest.split('/') {
        match part {
            "" | "." => {}
            // shortcut: `..` at the root is dropped, and so is one that climbs out of a UNC
            // share's name; fine while `dir` is a real folder, upgrade if shares matter
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    format!("{root}{}", parts.join("/"))
}

/// The length of the root in `path` (`/`, `//` or `X:/`), or 0 for a relative path.
fn root_len(path: &str) -> usize {
    let mut chars = path.chars();
    match (chars.next(), chars.next(), chars.next()) {
        (Some('/'), Some('/'), _) => 2,
        (Some('/'), _, _) => 1,
        (Some(drive), Some(':'), Some('/')) if drive.is_ascii_alphabetic() => 3,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Config, FilePattern, Level, RuleId};
    use rstest::rstest;

    const PATH: &str = "/r/.yaml-fm-lint.json";

    fn layer(src: &str) -> ConfigLayer {
        ConfigLayer {
            dir: "/r".into(),
            path: PATH.into(),
            source: src.into(),
        }
    }

    fn resolve(src: &str) -> Config {
        resolve_config(&[layer(src)], None).unwrap()
    }

    fn err(src: &str) -> crate::ConfigError {
        resolve_config(&[layer(src)], None).unwrap_err()
    }

    const BAD_LEVEL: &str = r#"must be "error", "warn" or "off", or ["error" | "warn", {options}]"#;

    #[test]
    fn empty_object_gives_defaults() {
        assert_eq!(
            resolve("{}"),
            Config {
                layers: vec![PATH.into()],
                ..Config::default()
            }
        );
    }

    #[test]
    fn jsonc_comments_and_trailing_commas() {
        let config = resolve("{\n // c\n \"ignoreKeys\": [\"a\",],\n}");
        assert_eq!(config.ignore_keys, ["a"]);
    }

    #[rstest]
    #[case::single_quotes("{'root': true}")]
    #[case::bare_key("{root: true}")]
    #[case::missing_comma(r#"{"root": true "ignoreKeys": []}"#)]
    #[case::hex(r#"{"root": 0x1}"#)]
    fn json5_extras_rejected(#[case] src: &str) {
        let e = err(src);
        assert!(e.message.starts_with("invalid JSON: "), "{}", e.message);
        assert!(e.line.is_some() && e.column.is_some());
        if src.starts_with("{'") {
            assert_eq!((e.line, e.column), (Some(1), Some(2)));
        }
    }

    #[rstest]
    #[case("")]
    #[case("null")]
    #[case("[]")]
    fn not_an_object(#[case] src: &str) {
        let e = err(src);
        assert_eq!(e.message, "config must be a JSON object");
        assert_eq!(e.pointer, None);
    }

    #[rstest]
    #[case(r#"{"rulez": {}}"#, "/rulez", "unknown key `rulez`")]
    #[case(
        r#"{"excludeDirs": ["x"]}"#,
        "/excludeDirs",
        "unknown key `excludeDirs`; this looks like a 1.x config; run `yaml-fm-lint migrate`"
    )]
    #[case(r#"{"root": "yes"}"#, "/root", "must be true or false")]
    #[case(r#"{"ignoreKeys": "a"}"#, "/ignoreKeys", "must be an array of strings")]
    #[case(r#"{"ignoreKeys": [1]}"#, "/ignoreKeys/0", "must be a string")]
    #[case(r#"{"files": []}"#, "/files", "must be an object")]
    #[case(r#"{"files": {"globs": []}}"#, "/files/globs", "unknown key `globs`")]
    #[case(
        r#"{"files": {"extensions": [".md", 2]}}"#,
        "/files/extensions/1",
        "must be a string"
    )]
    #[case(r#"{"schema": 1}"#, "/schema", "must be a string")]
    #[case(r#"{"$schema": 1}"#, "/$schema", "must be a string")]
    #[case(r#"{"rules": []}"#, "/rules", "must be an object")]
    fn key_errors(#[case] src: &str, #[case] pointer: &str, #[case] message: &str) {
        let e = err(src);
        assert_eq!(e.pointer.as_deref(), Some(pointer));
        assert_eq!(e.message, message);
        assert_eq!(e.kind, crate::ConfigErrorKind::Config);
        assert_eq!(e.file.as_deref(), Some(PATH));
    }

    #[rstest]
    #[case(r#""nope": "off""#, "/rules/nope", "unknown rule `nope`")]
    #[case(
        r#""yaml-syntax": "off""#,
        "/rules/yaml-syntax",
        "rule `yaml-syntax` can't be configured"
    )]
    #[case(r#""no-quotes": "warning""#, "/rules/no-quotes", BAD_LEVEL)]
    #[case(
        r#""no-quotes": ["error", {}]"#,
        "/rules/no-quotes",
        "rule `no-quotes` takes no options"
    )]
    #[case(
        r#""required-keys": ["off", {"keys": []}]"#,
        "/rules/required-keys",
        BAD_LEVEL
    )]
    #[case(r#""required-keys": ["error"]"#, "/rules/required-keys", BAD_LEVEL)]
    #[case(
        r#""required-keys": ["error", 1]"#,
        "/rules/required-keys/1",
        "must be an object"
    )]
    #[case(
        r#""required-keys": ["error", {"key": []}]"#,
        "/rules/required-keys/1/key",
        "unknown option `key`"
    )]
    #[case(
        r#""required-keys": ["error", {"keys": "a"}]"#,
        "/rules/required-keys/1/keys",
        "must be an array of strings"
    )]
    #[case(
        r#""indentation": ["warn", {"size": 9}]"#,
        "/rules/indentation/1/size",
        "must be an integer from 1 to 8"
    )]
    #[case(
        r#""indentation": ["warn", {"size": 2.5}]"#,
        "/rules/indentation/1/size",
        "must be an integer from 1 to 8"
    )]
    fn rule_errors(#[case] rule: &str, #[case] pointer: &str, #[case] message: &str) {
        let e = err(&format!(r#"{{"rules": {{{rule}}}}}"#));
        assert_eq!(e.pointer.as_deref(), Some(pointer));
        assert_eq!(e.message, message);
        assert_eq!(e.kind, crate::ConfigErrorKind::Config);
        assert_eq!(e.file.as_deref(), Some(PATH));
    }

    #[test]
    fn every_error_names_the_file() {
        for src in [
            "{",
            "[]",
            r#"{"rulez": 1}"#,
            r#"{"rules": {"nope": "off"}}"#,
        ] {
            assert_eq!(err(src).file.as_deref(), Some(PATH), "{src}");
        }
    }

    #[test]
    fn pointers_escape_keys() {
        let e = err(r#"{"rules": {"a/b~c": "off"}}"#);
        assert_eq!(e.pointer.as_deref(), Some("/rules/a~1b~0c"));
    }

    #[test]
    fn rule_values_parse() {
        let config = resolve(
            r#"{"rules": {"no-commas": "off", "required-keys": ["warn", {"keys": ["title"]}],
                "indentation": ["error", {"size": 4}]}}"#,
        );
        assert_eq!(config.level(RuleId::NoCommas), Level::Off);
        assert_eq!(config.level(RuleId::RequiredKeys), Level::Warn);
        assert_eq!(config.required_keys, ["title"]);
        assert_eq!(config.indent_size, 4);

        let config = resolve(r#"{"rules": {"required-keys": ["error", {}]}}"#);
        assert!(config.required_keys.is_empty());
        assert_eq!(config.indent_size, 2);
    }

    #[rstest]
    #[case("/r", "./s/fm.json", "/r/s/fm.json")]
    #[case("/r/sub", "../x.json", "/r/x.json")]
    #[case("/r", "/abs.json", "/abs.json")]
    #[case("C:/p", "s\\fm.json", "C:/p/s/fm.json")]
    #[case("C:/p", "D:/x.json", "D:/x.json")]
    #[case("/", "../a", "/a")]
    fn paths_are_absolute(#[case] dir: &str, #[case] rel: &str, #[case] expected: &str) {
        assert_eq!(join_path(dir, rel), expected);
    }

    #[test]
    fn schema_and_patterns_resolve_against_the_layer_dir() {
        let config = resolve(r#"{"schema": "./fm.json", "files": {"exclude": ["a/**"]}}"#);
        assert_eq!(config.schema.as_deref(), Some("/r/fm.json"));
        assert_eq!(
            config.files.exclude,
            [FilePattern {
                dir: "/r".into(),
                pattern: "a/**".into()
            }]
        );
    }

    fn l(dir: &str, src: &str) -> ConfigLayer {
        ConfigLayer {
            dir: dir.into(),
            path: format!("{dir}/.yaml-fm-lint.json"),
            source: src.into(),
        }
    }

    #[test]
    fn later_layers_override_per_key() {
        let config = resolve_config(
            &[
                l(
                    "/r",
                    r#"{"rules": {"no-commas": "off", "no-quotes": "warn"},
                        "files": {"extensions": [".md", ".mdx"], "exclude": ["a/**"]}}"#,
                ),
                l(
                    "/r/sub",
                    r#"{"rules": {"no-quotes": "error"}, "files": {"exclude": ["b/**"]}}"#,
                ),
            ],
            None,
        )
        .unwrap();
        assert_eq!(config.level(RuleId::NoCommas), Level::Off);
        assert_eq!(config.level(RuleId::NoQuotes), Level::Error);
        assert_eq!(config.files.extensions, [".md", ".mdx"]);
        assert_eq!(
            config.files.exclude,
            [FilePattern {
                dir: "/r/sub".into(),
                pattern: "b/**".into()
            }]
        );
    }

    #[test]
    fn rule_value_replaced_whole() {
        let config = resolve_config(
            &[
                l(
                    "/r",
                    r#"{"rules": {"required-keys": ["error", {"keys": ["a"]}]}}"#,
                ),
                l("/r/s", r#"{"rules": {"required-keys": "warn"}}"#),
            ],
            None,
        )
        .unwrap();
        assert_eq!(config.level(RuleId::RequiredKeys), Level::Warn);
        assert!(config.required_keys.is_empty());
    }

    #[test]
    fn arrays_replaced() {
        let config = resolve_config(
            &[
                l("/r", r#"{"ignoreKeys": ["a"]}"#),
                l("/r/s", r#"{"ignoreKeys": ["b"]}"#),
            ],
            None,
        )
        .unwrap();
        assert_eq!(config.ignore_keys, ["b"]);
    }

    #[test]
    fn root_drops_layers_above_unparsed() {
        let config = resolve_config(
            &[
                l("/r", "{"),
                l("/r/s", r#"{"root": true, "rules": {"no-commas": "off"}}"#),
                l("/r/s/d", "{}"),
            ],
            None,
        )
        .unwrap();
        assert_eq!(
            config.layers,
            ["/r/s/.yaml-fm-lint.json", "/r/s/d/.yaml-fm-lint.json"]
        );
        assert_eq!(config.level(RuleId::NoCommas), Level::Off);
    }

    #[test]
    fn invalid_layer_below_root_is_an_error() {
        let e =
            resolve_config(&[l("/r/s", r#"{"root": true}"#), l("/r/s/d", "{")], None).unwrap_err();
        assert_eq!(e.file.as_deref(), Some("/r/s/d/.yaml-fm-lint.json"));
    }

    fn extra(src: &str) -> ConfigLayer {
        ConfigLayer {
            dir: "/home/x".into(),
            path: "/home/x/ci.json".into(),
            source: src.into(),
        }
    }

    #[test]
    fn extra_is_last_and_highest() {
        let config = resolve_config(
            &[l("/r", r#"{"rules": {"no-quotes": "warn"}}"#)],
            Some(&extra(r#"{"rules": {"no-quotes": "off"}}"#)),
        )
        .unwrap();
        assert_eq!(config.level(RuleId::NoQuotes), Level::Off);
        assert_eq!(config.layers, ["/r/.yaml-fm-lint.json", "/home/x/ci.json"]);
    }

    #[test]
    fn extra_root_drops_all_discovered() {
        let config = resolve_config(&[l("/r", "{")], Some(&extra(r#"{"root": true}"#))).unwrap();
        assert_eq!(config.layers, ["/home/x/ci.json"]);
    }

    #[test]
    fn invalid_extra_is_an_error() {
        let e = resolve_config(&[], Some(&extra("{"))).unwrap_err();
        assert_eq!(e.file.as_deref(), Some("/home/x/ci.json"));
    }

    #[test]
    fn to_json_shape() {
        use serde_json::json;
        let json = Config::default().to_json();
        assert_eq!(json["rules"].as_object().map(|r| r.len()), Some(14));
        assert_eq!(json["rules"]["no-commas"], "warn");
        assert_eq!(
            json["rules"]["required-keys"],
            json!(["error", {"keys": []}])
        );
        assert_eq!(json["rules"]["indentation"], json!(["error", {"size": 2}]));
        assert!(json["rules"].get("yaml-syntax").is_none());
        assert!(json["schema"].is_null());
        assert_eq!(json["files"]["extensions"], json!([".md"]));
        assert!(json.get("layers").is_none());

        let json = resolve(r#"{"files": {"exclude": ["a/**"]}, "rules": {"indentation": "off"}}"#)
            .to_json();
        assert_eq!(
            json["files"]["exclude"],
            json!([{"dir": "/r", "pattern": "a/**"}])
        );
        assert_eq!(json["rules"]["indentation"], "off");
    }
}

/// The JSON Schema of the config file, generated from test-only types and checked in at
/// `schemas/config.schema.json`.
#[cfg(test)]
mod schema_tests {
    use super::*;
    use schemars::{JsonSchema, Schema, SchemaGenerator, transform::RecursiveTransform};
    use serde_json::json;

    const SCHEMA_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../schemas/config.schema.json"
    );

    /// Configuration of yaml-fm-lint, from a `.yaml-fm-lint.json` file.
    #[derive(JsonSchema)]
    #[schemars(
        title = "yaml-fm-lint configuration",
        extend("$id" = "https://unpkg.com/yaml-fm-lint@2/config.schema.json"),
        deny_unknown_fields,
        transform = RecursiveTransform(all_optional)
    )]
    #[expect(dead_code, reason = "the fields only describe the file format")]
    struct ConfigFileSchema {
        /// The URL of this schema. Optional and ignored.
        #[schemars(rename = "$schema")]
        schema_url: Option<String>,
        /// Stop looking for config files in parent folders; discovery also stops at a `.git` folder.
        root: Option<bool>,
        /// Which files are linted.
        files: Option<FilesSchema>,
        /// Top-level keys whose subtrees the style rules skip.
        #[schemars(rename = "ignoreKeys")]
        ignore_keys: Option<Vec<String>>,
        /// The level of each rule: "error", "warn" or "off"; `required-keys` and `indentation` also take `[level, {options}]`.
        #[schemars(schema_with = "rules_schema")]
        rules: Option<Vec<String>>,
        /// A JSON Schema file the front matter must match, relative to this config file.
        schema: Option<String>,
    }

    /// The `files` key.
    #[derive(JsonSchema)]
    #[schemars(deny_unknown_fields, transform = RecursiveTransform(all_optional))]
    #[expect(dead_code, reason = "the fields only describe the file format")]
    struct FilesSchema {
        /// Extensions of the files to lint, each with its leading dot. Default: `[".md"]`.
        extensions: Option<Vec<String>>,
        /// Glob patterns, relative to this config file. If there are any, a file must match one to be linted.
        include: Option<Vec<String>>,
        /// Glob patterns, relative to this config file, of files to skip. A leading `!` re-includes; the last match wins.
        exclude: Option<Vec<String>>,
    }

    /// Every key is optional, and none accepts `null`: the config parser rejects it.
    fn all_optional(schema: &mut Schema) {
        schema.remove("required");
        if let Some(serde_json::Value::Array(types)) = schema.get_mut("type") {
            types.retain(|t| t != "null");
            if let [only] = types.as_slice() {
                let only = only.clone();
                schema.insert("type".into(), only);
            }
        }
        if let Some(serde_json::Value::Array(branches)) = schema.get_mut("anyOf") {
            branches.retain(|b| *b != json!({"type": "null"}));
            if let [only] = branches.as_slice() {
                let only = only.clone();
                schema.remove("anyOf");
                if let serde_json::Value::Object(entries) = only {
                    schema.as_object_mut().unwrap().extend(entries);
                }
            }
        }
    }

    fn rules_schema(_: &mut SchemaGenerator) -> Schema {
        let level = json!({"enum": ["error", "warn", "off"]});
        let tuple = |options: serde_json::Value| {
            json!({"oneOf": [level, {
                "type": "array",
                "prefixItems": [{"enum": ["error", "warn"]}, options],
                "minItems": 2,
                "maxItems": 2,
                "items": false,
            }]})
        };
        let mut properties = serde_json::Map::new();
        for rule in RuleId::ALL.into_iter().filter(|r| r.is_configurable()) {
            let schema = match rule {
                RuleId::RequiredKeys => tuple(json!({
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {"keys": {"type": "array", "items": {"type": "string"}}},
                })),
                RuleId::Indentation => tuple(json!({
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {"size": {"type": "integer", "minimum": 1, "maximum": 8}},
                })),
                _ => level.clone(),
            };
            properties.insert(rule.as_str().into(), schema);
        }
        Schema::try_from(json!({
            "type": "object",
            "additionalProperties": false,
            "properties": properties,
        }))
        .unwrap()
    }

    fn generated() -> String {
        let mut text =
            serde_json::to_string_pretty(&schemars::schema_for!(ConfigFileSchema)).unwrap();
        text.push('\n');
        text
    }

    #[test]
    fn config_schema_is_current() {
        let text = generated();
        if std::env::var("UPDATE_SCHEMAS").is_ok_and(|v| v == "1") {
            std::fs::create_dir_all(std::path::Path::new(SCHEMA_PATH).parent().unwrap()).unwrap();
            std::fs::write(SCHEMA_PATH, &text).unwrap();
        }
        let on_disk = std::fs::read_to_string(SCHEMA_PATH).unwrap_or_default();
        assert!(
            on_disk == text,
            "schemas/config.schema.json is out of date; run with UPDATE_SCHEMAS=1"
        );
        let schema: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            schema["$id"],
            "https://unpkg.com/yaml-fm-lint@2/config.schema.json"
        );
        assert_eq!(schema["title"], "yaml-fm-lint configuration");
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["properties"]["rules"]["properties"]
                .as_object()
                .map(|rules| rules.len()),
            Some(14)
        );
    }

    #[test]
    fn schema_agrees_with_resolve_config() {
        let schema = serde_json::from_str(&generated()).unwrap();
        let validator = jsonschema::draft202012::new(&schema).unwrap();
        for (source, accepted) in [
            ("{}", true),
            (
                r#"{"$schema": "https://unpkg.com/yaml-fm-lint@2/config.schema.json",
                    "root": false,
                    "files": {"extensions": [".md"], "include": ["docs/**"], "exclude": ["!a.md"]},
                    "ignoreKeys": ["x"],
                    "schema": "./s.json",
                    "rules": {"no-quotes": "warn", "indentation": ["error", {"size": 4}]}}"#,
                true,
            ),
            (r#"{"rulez": 1}"#, false),
            (r#"{"rules": {"no-quotes": ["error", {}]}}"#, false),
            (
                r#"{"rules": {"indentation": ["warn", {"size": 9}]}}"#,
                false,
            ),
            (r#"{"rules": {"yaml-syntax": "off"}}"#, false),
            (
                r#"{"rules": {"required-keys": ["off", {"keys": []}]}}"#,
                false,
            ),
            (r#"{"rules": {"required-keys": ["error"]}}"#, false),
            (r#"{"files": {"extensions": [1]}}"#, false),
            (r#"{"root": null}"#, false),
            (r#"{"root": true, "ignoreKeys": ["a"]}"#, true),
        ] {
            let value: serde_json::Value = serde_json::from_str(source).unwrap();
            let layer = ConfigLayer {
                dir: "/r".into(),
                path: "/r/.yaml-fm-lint.json".into(),
                source: source.into(),
            };
            assert_eq!(validator.is_valid(&value), accepted, "schema: {source}");
            assert_eq!(resolve_config(&[layer], None).is_ok(), accepted, "{source}");
        }
    }

    /// Known gap: the schema checks shapes, not what a string means or how a number is spelled.
    #[test]
    fn schema_accepts_what_resolve_config_rejects() {
        let schema = serde_json::from_str(&generated()).unwrap();
        let validator = jsonschema::draft202012::new(&schema).unwrap();
        for source in [
            // glob syntax
            r#"{"files": {"exclude": ["a/[b"]}}"#,
            // an empty pattern
            r#"{"files": {"include": [""]}}"#,
            // `2.0` is an integer to JSON Schema, but not to `resolve_config`
            r#"{"rules": {"indentation": ["error", {"size": 2.0}]}}"#,
        ] {
            let value: serde_json::Value = serde_json::from_str(source).unwrap();
            let layer = ConfigLayer {
                dir: "/r".into(),
                path: "/r/.yaml-fm-lint.json".into(),
                source: source.into(),
            };
            assert!(validator.is_valid(&value), "schema: {source}");
            assert!(resolve_config(&[layer], None).is_err(), "{source}");
        }
    }
}
