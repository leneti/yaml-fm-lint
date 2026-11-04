//! Developer tasks, run with `cargo xtask <command>`.
//!
//! There are no commands yet. Later tasks add them as `match` arms in `main`.

use std::process::ExitCode;

/// Names of the commands that `cargo xtask` understands.
const COMMANDS: &[&str] = &[];

fn main() -> ExitCode {
    // No command exists yet, so every invocation is an unknown command.
    // When the first command is added, match on `std::env::args().nth(1)` here
    // and keep this usage output as the fallback arm.
    eprintln!("usage: cargo xtask <command>");
    if COMMANDS.is_empty() {
        eprintln!("available commands: (none yet)");
    } else {
        eprintln!("available commands: {}", COMMANDS.join(", "));
    }
    ExitCode::from(2)
}
