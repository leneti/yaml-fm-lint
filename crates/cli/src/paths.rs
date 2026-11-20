//! Paths as users see them.

use std::path::{Component, Path, PathBuf};

/// `path` with `/` separators, on every OS.
pub fn slash(path: &Path) -> String {
    let text = path.to_string_lossy();
    if cfg!(windows) {
        backslashes_to_slashes(&text)
    } else {
        text.into_owned()
    }
}

/// Kept apart from [`slash`] so a Unix test can cover the Windows rule (a `\` is a file name
/// character on Unix, so `slash` must not touch it there).
fn backslashes_to_slashes(text: &str) -> String {
    text.replace('\\', "/")
}

/// `path` made absolute against `cwd`, with `.` and `..` removed lexically; symlinks are not
/// resolved.
pub fn absolute(path: &Path, cwd: &Path) -> PathBuf {
    let joined = cwd.join(path);
    let joined = std::path::absolute(&joined).unwrap_or(joined);
    let mut out = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// How a file is named in output: relative to `cwd` when it is under it, else absolute; `/`
/// separated, without a leading `./`. `path` is absolute.
pub fn display(path: &Path, cwd: &Path) -> String {
    match path.strip_prefix(cwd) {
        Ok(rest) if rest.as_os_str().is_empty() => ".".to_string(),
        Ok(rest) => slash(rest),
        Err(_) => slash(path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_separators() {
        assert_eq!(backslashes_to_slashes(r"a\b\c.md"), "a/b/c.md");
    }

    #[cfg(unix)]
    #[test]
    fn absolute_resolves_dots() {
        let cwd = Path::new("/w/repo/sub");
        assert_eq!(
            absolute(Path::new("../a.md"), cwd),
            Path::new("/w/repo/a.md")
        );
        assert_eq!(
            absolute(Path::new("./x/./y.md"), cwd),
            Path::new("/w/repo/sub/x/y.md")
        );
    }

    #[cfg(unix)]
    #[test]
    fn display_is_relative_under_cwd() {
        let cwd = Path::new("/w/repo");
        assert_eq!(display(Path::new("/w/repo/a/b.md"), cwd), "a/b.md");
        assert_eq!(display(Path::new("/w/other/b.md"), cwd), "/w/other/b.md");
        assert_eq!(display(Path::new("/w/repo"), cwd), ".");
        assert_eq!(
            display(Path::new("/w/repository/b.md"), cwd),
            "/w/repository/b.md"
        );
    }
}
