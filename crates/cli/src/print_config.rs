//! `print-config`: the settings that apply to a file.

use std::path::Path;

use anyhow::Context;
use serde_json::json;

use crate::configs::Configs;
use crate::paths::absolute;
use crate::run::{emit, write_stderr};

/// Prints the layers and the settings for `file` (or for the files in it, if it's a folder);
/// returns the exit code. The schema file isn't read.
pub fn run(file: &Path, configs: &Configs) -> anyhow::Result<u8> {
    let cwd = std::env::current_dir().context("can't find the current folder")?;
    let file = absolute(file, &cwd);
    let dir = if file.is_dir() {
        file.as_path()
    } else {
        file.parent().unwrap_or(&file)
    };
    let config = match configs.config_for(dir) {
        Ok(config) => config,
        Err(line) => {
            write_stderr(&line);
            return Ok(2);
        }
    };
    let text = serde_json::to_string_pretty(&json!({
        "layers": config.layers,
        "config": config.to_json(),
    }))?;
    emit(std::io::stdout().lock(), &format!("{text}\n"))?;
    Ok(0)
}
