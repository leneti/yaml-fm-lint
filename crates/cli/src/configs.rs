//! Finding the config files that apply to a folder, and the linter they make.

use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use anyhow::Context;
use yaml_fm_lint_core::{Config, ConfigLayer, Linter, resolve_config};

use crate::paths::slash;

const CONFIG_FILE: &str = ".yaml-fm-lint.json";

/// The config files of every folder met so far, and the linters built from them.
///
/// Shared by the worker threads. A folder's file is read once, and files that end up with the
/// same list of config files share a linter.
pub struct Configs {
    extra: Option<ConfigLayer>,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    /// For a folder, the config files from there up to the project root, outermost first.
    chains: HashMap<PathBuf, Result<Arc<Vec<ConfigLayer>>, String>>,
    /// For a folder, its linter.
    by_dir: HashMap<PathBuf, Result<Arc<Linter>, String>>,
    /// For a list of config file paths, its linter.
    by_layers: HashMap<Vec<String>, Result<Arc<Linter>, String>>,
}

impl Configs {
    /// Reads the `--config` file, if there is one; relative paths in it resolve against `cwd`.
    pub fn new(extra: Option<&Path>, cwd: &Path) -> anyhow::Result<Configs> {
        let extra = extra
            .map(|path| {
                let path = crate::paths::absolute(path, cwd);
                let source = std::fs::read_to_string(&path)
                    .with_context(|| format!("can't read config `{}`", slash(&path)))?;
                anyhow::Ok(ConfigLayer {
                    dir: slash(path.parent().unwrap_or(&path)),
                    path: slash(&path),
                    source,
                })
            })
            .transpose()?;
        Ok(Configs {
            extra,
            state: Mutex::new(State::default()),
        })
    }

    /// The linter for files in `dir` (absolute), or the line to print on stderr.
    // shortcut: one lock for the whole call, so a thread that builds a new linter (reads and
    // compiles a schema) holds the others up; split the lock if that ever shows in a profile.
    pub fn linter_for(&self, dir: &Path) -> Result<Arc<Linter>, String> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(found) = state.by_dir.get(dir) {
            return found.clone();
        }
        let built = self.build(&mut state, dir);
        state.by_dir.insert(dir.to_path_buf(), built.clone());
        built
    }

    fn build(&self, state: &mut State, dir: &Path) -> Result<Arc<Linter>, String> {
        let chain = chain(&mut state.chains, dir)?;
        let config = resolve_config(&chain, self.extra.as_ref())
            .map_err(|e| format!("config error: {e}"))?;
        if let Some(found) = state.by_layers.get(&config.layers) {
            return found.clone();
        }
        let key = config.layers.clone();
        let linter = make_linter(config).map_err(|e| format!("config error: {e}"));
        state.by_layers.insert(key, linter.clone());
        linter
    }
}

/// Reads the schema the config names, if any, and builds the linter.
fn make_linter(config: Config) -> Result<Arc<Linter>, String> {
    let schema = config
        .schema
        .as_ref()
        .map(|path| {
            std::fs::read_to_string(path).map_err(|e| format!("can't read schema `{path}`: {e}"))
        })
        .transpose()?;
    Linter::new(config, schema.as_deref())
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

/// The config files from `dir` up to the first folder with a `.git` (a folder, or a file for
/// worktrees and submodules) or the filesystem root, outermost first.
fn chain(
    cache: &mut HashMap<PathBuf, Result<Arc<Vec<ConfigLayer>>, String>>,
    dir: &Path,
) -> Result<Arc<Vec<ConfigLayer>>, String> {
    if let Some(found) = cache.get(dir) {
        return found.clone();
    }
    let mut layers = match dir.parent() {
        Some(parent) if dir.join(".git").symlink_metadata().is_err() => {
            chain(cache, parent)?.as_ref().clone()
        }
        _ => Vec::new(),
    };
    let result = read_layer(dir).map(|layer| {
        layers.extend(layer);
        Arc::new(layers)
    });
    cache.insert(dir.to_path_buf(), result.clone());
    result
}

/// The config file in `dir`, if it has one.
fn read_layer(dir: &Path) -> Result<Option<ConfigLayer>, String> {
    let path = dir.join(CONFIG_FILE);
    match std::fs::read_to_string(&path) {
        Ok(source) => Ok(Some(ConfigLayer {
            dir: slash(dir),
            path: slash(&path),
            source,
        })),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!(
            "config error: can't read config `{}`: {e}",
            slash(&path)
        )),
    }
}
