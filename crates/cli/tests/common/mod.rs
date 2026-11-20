//! Helpers shared by the integration tests.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

/// One `no-quotes` error at 2:8-2:15.
pub const BAD: &[u8] = b"---\ntitle: \"Hello\"\n---\n";
/// One `single-space-after-colon` warning at 2:5.
pub const WARN: &[u8] = b"---\nkey:  value\n---\n";
/// No problems.
pub const OK: &[u8] = b"---\ntitle: Hello\n---\n";

/// A temporary folder holding `files`, with a `.git/` folder at its root so that config
/// discovery never leaves it.
pub fn project(files: &[(&str, &[u8])]) -> TempDir {
    let dir = project_without_git(files);
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    dir
}

/// Like [`project`], without the `.git/` folder.
pub fn project_without_git(files: &[(&str, &[u8])]) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, content) in files {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
    dir
}

/// `dir` as an absolute path the binary and the test agree on: symlinks resolved (macOS `/var`),
/// and without the `\\?\` prefix that Windows' `canonicalize` adds (its `?` would be a glob
/// character in a path argument).
///
/// On Windows it is `dir` itself: the child's working directory is `dir` as given (possibly an
/// 8.3 short name like `RUNNER~1`), and the binary only prints relative paths when its paths are
/// in that same form, which `canonicalize` would change.
pub fn plain_path(dir: &Path) -> String {
    if cfg!(windows) {
        return dir.display().to_string();
    }
    let canonical = std::fs::canonicalize(dir).unwrap();
    let text = canonical.display().to_string();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_string()
}

/// The binary, run in `dir`, with colour settings and the user's git configuration kept out.
pub fn yfl(dir: &Path) -> Command {
    let mut command = Command::cargo_bin("yaml-fm-lint").unwrap();
    command
        .current_dir(dir)
        .env_remove("NO_COLOR")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("CLICOLOR")
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .env("GIT_CONFIG_GLOBAL", dir.join("no-git-config"));
    command
}
