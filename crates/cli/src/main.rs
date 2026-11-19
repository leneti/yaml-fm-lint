//! The `yaml-fm-lint` command; see AGENTS.md.

mod cli;
mod configs;
mod migrate;
mod output;
mod paths;
mod print_config;
mod read;
mod run;
mod walk;
mod write;

use std::io::Write;
use std::process::ExitCode;

use anyhow::Context;
use clap::Parser;

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            // Nobody to tell if stderr is closed.
            let _ = writeln!(std::io::stderr().lock(), "error: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> anyhow::Result<u8> {
    let cli = cli::Cli::parse();
    match &cli.command {
        None => run::lint(&cli),
        Some(cli::Command::PrintConfig { file }) => {
            let cwd = std::env::current_dir().context("can't find the current folder")?;
            let configs = configs::Configs::new(cli.config.as_deref(), &cwd)?;
            print_config::run(file, &configs)
        }
        Some(cli::Command::Migrate { path, write }) => migrate::run(path.as_deref(), *write),
    }
}
