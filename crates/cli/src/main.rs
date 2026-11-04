use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() == 1 && args.first().map(String::as_str) == Some("--version") {
        println!("yaml-fm-lint {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    eprintln!("yaml-fm-lint 2.0 is under development; use yaml-fm-lint 1.x from npm");
    ExitCode::from(2)
}
