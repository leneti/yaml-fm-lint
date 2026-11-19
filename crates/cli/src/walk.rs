//! Finding the files to lint: folders, globs and explicit files.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use globset::{GlobBuilder, GlobMatcher};
use ignore::{WalkBuilder, WalkState};

use crate::configs::Configs;
use crate::paths::{absolute, display, slash};

/// A file to lint.
#[derive(Debug)]
pub struct Target {
    /// Absolute.
    pub path: PathBuf,
    /// How the file is named in output.
    pub display: String,
    /// Named on the command line: the extension and `files` patterns don't apply.
    pub explicit: bool,
}

/// What the arguments stand for.
#[derive(Debug)]
pub struct Collected {
    /// Sorted by `display`, each file once.
    pub targets: Vec<Target>,
    /// Lines to print on stderr, sorted: arguments and folders that couldn't be used.
    pub errors: Vec<String>,
    /// The config error to report, if any file met one: the first by config path, so that the
    /// answer doesn't depend on thread order.
    pub fatal: Option<String>,
}

#[derive(Default)]
struct Found {
    targets: Vec<Target>,
    errors: Vec<String>,
    config_errors: Vec<String>,
}

impl Found {
    fn push(found: &Mutex<Found>, add: impl FnOnce(&mut Found)) {
        add(&mut found.lock().unwrap_or_else(PoisonError::into_inner));
    }
}

const GLOB_CHARS: [char; 4] = ['*', '?', '[', '{'];

/// The files named by `args` (the current folder when there are none), in `cwd`.
pub fn collect(args: &[String], cwd: &Path, configs: &Configs) -> Collected {
    let default = [".".to_string()];
    let args = if args.is_empty() {
        default.as_slice()
    } else {
        args
    };
    let found = Mutex::new(Found::default());
    for arg in args {
        let path = absolute(Path::new(arg), cwd);
        // What exists under that name is that file or folder (`blog/[slug].md`), whatever
        // characters the name has; only otherwise is a glob character a pattern.
        match std::fs::metadata(&path) {
            Ok(meta) if meta.is_dir() => walk(&path, None, cwd, configs, &found),
            Ok(meta) if meta.is_file() => Found::push(&found, |f| {
                f.targets.push(Target {
                    display: display(&path, cwd),
                    path,
                    explicit: true,
                });
            }),
            _ if arg.contains(GLOB_CHARS) => walk_glob(arg, cwd, configs, &found),
            _ => Found::push(&found, |f| {
                f.errors.push(format!(
                    "{}: no such file or directory",
                    display(&path, cwd)
                ));
            }),
        }
    }
    let Found {
        mut targets,
        mut errors,
        config_errors,
    } = found.into_inner().unwrap_or_else(PoisonError::into_inner);
    // The same file reached twice is linted once; an explicit mention wins.
    targets.sort_by(|a, b| a.display.cmp(&b.display).then(b.explicit.cmp(&a.explicit)));
    targets.dedup_by(|later, first| later.path == first.path);
    errors.sort();
    errors.dedup();
    // Every line starts with the same words, then the config path, so the smallest line is
    // the one whose config path sorts first.
    let fatal = config_errors.into_iter().min();
    Collected {
        targets,
        errors,
        fatal,
    }
}

/// Walks from the folder a glob starts in and keeps the files it matches.
fn walk_glob(arg: &str, cwd: &Path, configs: &Configs, found: &Mutex<Found>) {
    let (prefix, rest) = split_glob(&glob_separators(arg, cfg!(windows)));
    let root = absolute(Path::new(&prefix), cwd);
    // A folder that isn't there has no files in it (and is not an error).
    if !std::fs::metadata(&root).is_ok_and(|meta| meta.is_dir()) {
        return;
    }
    match GlobBuilder::new(&rest).literal_separator(true).build() {
        Ok(glob) => walk(&root, Some(&glob.compile_matcher()), cwd, configs, found),
        Err(e) => Found::push(found, |f| {
            f.errors.push(format!("{arg}: invalid glob: {}", e.kind()));
        }),
    }
}

/// On Windows `\` separates folders (`C:\repo\docs\*.md`) and can't escape a glob character, so
/// it becomes `/`. Everywhere else it is left alone. (`windows` is a parameter so that a test on
/// any OS covers the Windows rule.)
fn glob_separators(arg: &str, windows: bool) -> String {
    if windows {
        arg.replace('\\', "/")
    } else {
        arg.to_string()
    }
}

/// Splits a glob into the folder before its first component with a glob character (`.` when
/// that is the first one) and the rest. Only `/` separates components.
fn split_glob(arg: &str) -> (String, String) {
    let parts: Vec<&str> = arg.split('/').collect();
    let at = parts
        .iter()
        .position(|part| part.contains(GLOB_CHARS))
        .unwrap_or(0);
    let join = |parts: Option<&[&str]>| parts.unwrap_or_default().join("/");
    let prefix = match join(parts.get(..at)) {
        _ if at == 0 => ".".to_string(),
        // `/*.md`: the root; `C:/*.md`: the root of a drive, not its current folder.
        prefix if prefix.is_empty() || is_drive(&prefix) => format!("{prefix}/"),
        prefix => prefix,
    };
    (prefix, join(parts.get(at..)))
}

/// `C:`.
fn is_drive(prefix: &str) -> bool {
    matches!(prefix.as_bytes(), [letter, b':'] if letter.is_ascii_alphabetic())
}

/// Walks `root` and records the files to lint: with `glob`, the ones it matches (relative to
/// `root`) that pass `files.include` and `files.exclude`; without, the ones the config selects.
fn walk(
    root: &Path,
    glob: Option<&GlobMatcher>,
    cwd: &Path,
    configs: &Configs,
    found: &Mutex<Found>,
) {
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .follow_links(false)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .require_git(false)
        .ignore(false)
        .parents(true)
        .threads(std::thread::available_parallelism().map_or(1, usize::from))
        .filter_entry(|entry| {
            let skipped = entry.file_type().is_some_and(|t| t.is_dir())
                && (entry.file_name() == "node_modules" || entry.file_name() == ".git");
            entry.depth() == 0 || !skipped
        });
    builder.build_parallel().run(|| {
        Box::new(|entry| {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    // Other errors (a bad line in a `.gitignore`) are not about a file.
                    if let Some(io) = e.io_error() {
                        let line = match error_path(&e) {
                            Some(path) => format!("{}: can't read: {io}", display(path, cwd)),
                            None => format!("can't read: {io}"),
                        };
                        Found::push(found, |f| f.errors.push(line));
                    }
                    return WalkState::Continue;
                }
            };
            // Symlinks are not followed, and a link to a file is not a file here.
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                return WalkState::Continue;
            }
            let path = entry.into_path();
            let slashed = slash(&path);
            let selected = glob.is_none_or(|glob| {
                path.strip_prefix(root)
                    .is_ok_and(|relative| glob.is_match(slash(relative)))
            });
            if !selected {
                return WalkState::Continue;
            }
            match configs.linter_for(path.parent().unwrap_or(&path)) {
                Ok(linter) => {
                    let keep = if glob.is_some() {
                        linter.matches_patterns(&slashed)
                    } else {
                        linter.matches(&slashed)
                    };
                    if keep {
                        Found::push(found, |f| {
                            f.targets.push(Target {
                                display: display(&path, cwd),
                                path,
                                explicit: false,
                            });
                        });
                    }
                }
                // Keep walking: which error is reported must not depend on thread order.
                Err(line) => Found::push(found, |f| f.config_errors.push(line)),
            }
            WalkState::Continue
        })
    });
}

/// The path a walk error is about.
fn error_path(e: &ignore::Error) -> Option<&Path> {
    match e {
        ignore::Error::WithPath { path, .. } => Some(path),
        ignore::Error::WithDepth { err, .. } | ignore::Error::WithLineNumber { err, .. } => {
            error_path(err)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs_split_at_the_first_glob_component() {
        let split = |arg: &str| split_glob(arg);
        assert_eq!(split("*.md"), (".".into(), "*.md".into()));
        assert_eq!(split("docs/*.txt"), ("docs".into(), "*.txt".into()));
        assert_eq!(split("./a/b/**/*.md"), ("./a/b".into(), "**/*.md".into()));
        assert_eq!(split("/*.md"), ("/".into(), "*.md".into()));
        assert_eq!(split("/x/y/*.md"), ("/x/y".into(), "*.md".into()));
        assert_eq!(split("x/{a/b,c}"), ("x".into(), "{a/b,c}".into()));
        assert_eq!(split("{a/b,c}/*.md"), (".".into(), "{a/b,c}/*.md".into()));
    }

    #[test]
    fn windows_globs_split_like_slash_globs() {
        let split = |arg: &str| split_glob(&glob_separators(arg, true));
        assert_eq!(
            split(r"C:\repo\docs\*.md"),
            ("C:/repo/docs".into(), "*.md".into())
        );
        assert_eq!(
            split(r"..\other\**\*.md"),
            ("../other".into(), "**/*.md".into())
        );
        assert_eq!(split(r"docs\*.md"), ("docs".into(), "*.md".into()));
        // A drive root is a folder, not the drive's current folder.
        assert_eq!(split(r"C:\*.md"), ("C:/".into(), "*.md".into()));
        // Elsewhere `\` is a file name character (and a glob escape), so it stays.
        assert_eq!(glob_separators(r"a\[b].md", false), r"a\[b].md");
    }
}
