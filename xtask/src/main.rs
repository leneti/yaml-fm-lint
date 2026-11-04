//! Developer tasks, run with `cargo xtask <command>`.

use std::process::ExitCode;
use xtask::{corpus, parity};

/// Names of the commands that `cargo xtask` understands.
const COMMANDS: &[&str] = &["parity-record", "bench-corpus"];

fn main() -> ExitCode {
    let result = match std::env::args().nth(1).as_deref() {
        Some("parity-record") => parity::record(),
        Some("bench-corpus") => {
            let args: Vec<String> = std::env::args().skip(2).collect();
            corpus::run(&args)
        }
        _ => {
            eprintln!("usage: cargo xtask <command>");
            eprintln!("available commands: {}", COMMANDS.join(", "));
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}
