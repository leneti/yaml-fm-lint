//! Selecting files with `files.include` and `files.exclude`.

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use crate::linter::config_error;
use crate::{ConfigError, FilePattern, Files};

/// One compiled pattern.
#[derive(Debug, Clone)]
struct Compiled {
    /// The folder the pattern is relative to, without a trailing `/`.
    dir: String,
    /// A `!` pattern, which re-includes.
    negated: bool,
    /// The pattern and everything below it.
    globs: GlobSet,
}

impl Compiled {
    fn new(pattern: &FilePattern, list: &str, index: usize) -> Result<Compiled, ConfigError> {
        let (negated, globs) = compile(&pattern.pattern, list, index)?;
        Ok(Compiled {
            dir: pattern.dir.trim_end_matches('/').to_string(),
            negated,
            globs,
        })
    }

    fn matches(&self, path: &str) -> bool {
        path.strip_prefix(self.dir.as_str())
            .and_then(|rest| rest.strip_prefix('/'))
            .is_some_and(|relative| self.globs.is_match(relative))
    }
}

/// The compiled `files.include` and `files.exclude` patterns.
#[derive(Debug, Clone)]
pub(crate) struct FileMatcher {
    include: Vec<Compiled>,
    exclude: Vec<Compiled>,
}

impl FileMatcher {
    pub(crate) fn new(files: &Files) -> Result<FileMatcher, ConfigError> {
        let compile_all = |patterns: &[FilePattern], list: &str| {
            patterns
                .iter()
                .enumerate()
                .map(|(index, pattern)| Compiled::new(pattern, list, index))
                .collect::<Result<Vec<_>, _>>()
        };
        Ok(FileMatcher {
            include: compile_all(&files.include, "include")?,
            exclude: compile_all(&files.exclude, "exclude")?,
        })
    }

    /// Whether `path` is selected by the include and exclude patterns.
    pub(crate) fn matches(&self, path: &str) -> bool {
        let included =
            self.include.is_empty() || self.include.iter().any(|pattern| pattern.matches(path));
        // The last exclude pattern that matches decides; `!` re-includes.
        let excluded = self
            .exclude
            .iter()
            .rev()
            .find(|pattern| pattern.matches(path))
            .is_some_and(|pattern| !pattern.negated);
        included && !excluded
    }
}

/// Checks that `pattern`, item `index` of `files.{list}`, is allowed and compiles.
pub(crate) fn check_pattern(pattern: &str, list: &str, index: usize) -> Result<(), ConfigError> {
    compile(pattern, list, index).map(drop)
}

/// Compiles a pattern into whether it is negated and the globs for it and its contents.
///
/// A pattern with a leading `/` or a `/` inside it is anchored to its folder. Any other pattern
/// matches at every depth, like in a `.gitignore`.
fn compile(pattern: &str, list: &str, index: usize) -> Result<(bool, GlobSet), ConfigError> {
    let pointer = format!("/files/{list}/{index}");
    let rest = pattern.strip_prefix('!');
    if rest.is_some() && list == "include" {
        return Err(config_error(
            &pointer,
            "`!` is only allowed in files.exclude".to_string(),
        ));
    }
    let negated = rest.is_some();
    let rest = rest.unwrap_or(pattern);
    let body = rest.strip_suffix('/').unwrap_or(rest);
    let anchored = rest.starts_with('/') || body.contains('/');
    let body = body.strip_prefix('/').unwrap_or(body);
    let globs = if anchored {
        [body.to_string(), format!("{body}/**")]
    } else {
        [format!("**/{body}"), format!("**/{body}/**")]
    };
    let mut set = GlobSetBuilder::new();
    for glob in globs {
        let glob = GlobBuilder::new(&glob)
            .literal_separator(true)
            .backslash_escape(true)
            .build()
            .map_err(|e| config_error(&pointer, format!("invalid pattern: {e}")))?;
        set.add(glob);
    }
    let set = set
        .build()
        .map_err(|e| config_error(&pointer, format!("invalid pattern: {e}")))?;
    Ok((negated, set))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::{Config, ConfigLayer, FilePattern, Files, Linter, resolve_config};

    fn patterns(dir: &str, list: &[&str]) -> Vec<FilePattern> {
        list.iter()
            .map(|pattern| FilePattern {
                dir: dir.to_string(),
                pattern: (*pattern).to_string(),
            })
            .collect()
    }

    fn linter(include: &[&str], exclude: &[&str]) -> Linter {
        let config = Config {
            files: Files {
                include: patterns("/r", include),
                exclude: patterns("/r", exclude),
                ..Files::default()
            },
            ..Config::default()
        };
        Linter::new(config, None).unwrap()
    }

    fn m(include: &[&str], exclude: &[&str], path: &str) -> bool {
        linter(include, exclude).matches(path)
    }

    #[test]
    fn extension_check() {
        assert!(m(&[], &[], "/r/a.md"));
        assert!(!m(&[], &[], "/r/a.txt"));
        assert!(!m(&[], &[], "/r/a.MD"));
    }

    #[rstest]
    #[case::bare_name_dir(&["drafts"], "/r/drafts/a.md", false)]
    #[case::bare_name_nested(&["drafts"], "/r/x/drafts/a.md", false)]
    #[case::bare_name_not_prefix(&["drafts"], "/r/drafts.md", true)]
    #[case::bare_glob(&["*.draft.md"], "/r/a/b.draft.md", false)]
    #[case::leading_slash_top(&["/top.md"], "/r/top.md", false)]
    #[case::leading_slash_nested(&["/top.md"], "/r/a/top.md", true)]
    #[case::inner_slash(&["docs/*.md"], "/r/docs/a.md", false)]
    #[case::inner_slash_star_stops(&["docs/*.md"], "/r/docs/sub/a.md", true)]
    #[case::trailing_slash(&["build/"], "/r/build/a.md", false)]
    #[case::trailing_slash_nested(&["build/"], "/r/x/build/a.md", false)]
    #[case::reinclude(&["**/drafts/**", "!**/drafts/keep.md"], "/r/drafts/keep.md", true)]
    #[case::reinclude_others(&["**/drafts/**", "!**/drafts/keep.md"], "/r/drafts/x.md", false)]
    #[case::last_wins(&["!a.md", "a.md"], "/r/a.md", false)]
    fn exclude_table(#[case] exclude: &[&str], #[case] path: &str, #[case] expected: bool) {
        assert_eq!(m(&[], exclude, path), expected);
    }

    #[test]
    fn pattern_dir_scopes_it() {
        let config = Config {
            files: Files {
                exclude: patterns("/r/sub", &["a.md"]),
                ..Files::default()
            },
            ..Config::default()
        };
        let linter = Linter::new(config, None).unwrap();
        assert!(linter.matches("/r/a.md"));
        assert!(!linter.matches("/r/sub/a.md"));
    }

    #[test]
    fn include_patterns() {
        assert!(m(&["docs/**"], &[], "/r/docs/a.md"));
        assert!(!m(&["docs/**"], &[], "/r/blog/a.md"));
    }

    #[test]
    fn matches_patterns_skips_extension() {
        let linter = linter(&[], &[]);
        assert!(!linter.matches("/r/a.txt"));
        assert!(linter.matches_patterns("/r/a.txt"));
    }

    #[rstest]
    #[case::bad_glob(
        r#"{"files": {"exclude": ["a/[b"]}}"#,
        "/files/exclude/0",
        "invalid pattern: "
    )]
    #[case::negated_include(
        r#"{"files": {"include": ["!a"]}}"#,
        "/files/include/0",
        "`!` is only allowed in files.exclude"
    )]
    fn bad_patterns_name_the_file(#[case] src: &str, #[case] pointer: &str, #[case] message: &str) {
        let layer = ConfigLayer {
            dir: "/r".into(),
            path: "/r/.yaml-fm-lint.json".into(),
            source: src.into(),
        };
        let error = resolve_config(&[layer], None).unwrap_err();
        assert_eq!(error.file.as_deref(), Some("/r/.yaml-fm-lint.json"));
        assert_eq!(error.pointer.as_deref(), Some(pointer));
        assert!(error.message.starts_with(message), "{}", error.message);
    }
}
