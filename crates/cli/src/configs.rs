//! Finding the config files that apply to a folder, and the linter they make.

use std::collections::HashMap;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use yaml_fm_lint_core::{Config, ConfigLayer, Linter, resolve_config};

use crate::paths::slash;
use crate::reason::reason;

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
                let source = read_text_file(&path, "config").map_err(anyhow::Error::msg)?;
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

    /// The settings for files in `dir` (absolute), or the line to print on stderr. Reads no
    /// schema.
    pub fn config_for(&self, dir: &Path) -> Result<Config, String> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        self.config_in(&mut state, dir)
    }

    fn config_in(&self, state: &mut State, dir: &Path) -> Result<Config, String> {
        let chain = chain(&mut state.chains, dir)?;
        resolve_config(&chain, self.extra.as_ref()).map_err(|e| format!("config error: {e}"))
    }

    fn build(&self, state: &mut State, dir: &Path) -> Result<Arc<Linter>, String> {
        let config = self.config_in(state, dir)?;
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
        .map(|path| read_text_file(Path::new(path), "schema"))
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
    if path
        .metadata()
        .is_err_and(|e| e.kind() == ErrorKind::NotFound)
    {
        return Ok(None);
    }
    let source = read_text_file(&path, "config").map_err(|e| format!("config error: {e}"))?;
    Ok(Some(ConfigLayer {
        dir: slash(dir),
        path: slash(&path),
        source,
    }))
}

/// The most a config or schema file may hold.
const MAX_FILE_BYTES: u64 = 1 << 20;

/// The text of the config or schema file (`what` says which) at `path`. Only regular files are
/// read (a FIFO would block, `/dev/zero` never ends), and at most 1 MiB of them.
pub(crate) fn read_text_file(path: &Path, what: &str) -> Result<String, String> {
    let shown = slash(path);
    let failed = |e: std::io::Error| format!("can't read {what} `{shown}`: {}", reason(&e));
    if !std::fs::metadata(path).map_err(failed)?.is_file() {
        return Err(format!("{shown}: not a regular file"));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|file| file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes))
        .map_err(failed)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(format!("{shown}: {what} file is larger than 1 MiB"));
    }
    String::from_utf8(bytes).map_err(|_| format!("{shown}: {what} file is not valid UTF-8"))
}
