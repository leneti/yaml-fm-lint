//! The `yaml-fm-lint` command; see AGENTS.md.

mod cli;
mod configs;
mod output;
mod paths;
mod read;
mod run;
mod walk;
mod write;

use std::io::Write;
use std::process::ExitCode;

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
    run::lint(&cli::Cli::parse())
}
