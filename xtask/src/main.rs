//! Developer tasks, run with `cargo xtask <command>`.

use std::process::ExitCode;
use xtask::parity;

/// Names of the commands that `cargo xtask` understands.
const COMMANDS: &[&str] = &["parity-record"];

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("parity-record") => match parity::record() {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("error: {err:#}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("usage: cargo xtask <command>");
            eprintln!("available commands: {}", COMMANDS.join(", "));
            ExitCode::from(2)
        }
    }
}
