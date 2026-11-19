//! Linting the files named on the command line.

use std::io::{ErrorKind, Write};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use anyhow::Context;
use yaml_fm_lint_core::{Applicability, Diagnostic, Severity};

use crate::cli::Cli;
use crate::configs::Configs;
use crate::output;
use crate::read::{ReadError, read_for_lint};
use crate::walk::{Target, collect};

/// What happened to one file.
#[derive(Debug)]
pub struct FileResult {
    /// How the file is named in output.
    pub display: String,
    /// The file's text, kept only when there are diagnostics.
    pub text: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
    /// Fixes applied to the file.
    pub fixed: usize,
    /// Whether the file was rewritten.
    pub written: bool,
}

/// The counts of a run.
#[derive(Debug)]
pub struct Summary {
    pub errors: usize,
    pub warnings: usize,
    /// Problems with an automatic fix.
    pub fixable: usize,
    pub files: usize,
    pub files_with_problems: usize,
    pub fixed_problems: usize,
    pub fixed_files: usize,
    pub elapsed_ms: u128,
}

impl Summary {
    fn new(results: &[FileResult], started: Instant) -> Summary {
        let count = |severity| {
            results
                .iter()
                .flat_map(|r| &r.diagnostics)
                .filter(|d| d.severity == severity)
                .count()
        };
        Summary {
            errors: count(Severity::Error),
            warnings: count(Severity::Warning),
            fixable: results
                .iter()
                .flat_map(|r| &r.diagnostics)
                .filter(|d| {
                    d.fix
                        .as_ref()
                        .is_some_and(|f| f.applicability == Applicability::Automatic)
                })
                .count(),
            files: results.len(),
            files_with_problems: results.iter().filter(|r| !r.diagnostics.is_empty()).count(),
            fixed_problems: results.iter().map(|r| r.fixed).sum(),
            fixed_files: results.iter().filter(|r| r.written).count(),
            elapsed_ms: started.elapsed().as_millis(),
        }
    }
}

/// 2 when `fatal` (a config error, or a file that couldn't be read or written), else 1 when
/// there are errors or more warnings than `max_warnings`, else 0.
pub fn exit_code(summary: &Summary, fatal: bool, max_warnings: Option<usize>) -> u8 {
    if fatal {
        2
    } else if summary.errors > 0 || max_warnings.is_some_and(|max| summary.warnings > max) {
        1
    } else {
        0
    }
}

/// A file that could not be processed: the line to print on stderr.
struct Failure {
    line: String,
    is_config: bool,
}

struct Item {
    display: String,
    outcome: Result<FileResult, Failure>,
}

/// Lints the files named in `cli.paths` and prints the report; returns the exit code.
pub fn lint(cli: &Cli) -> anyhow::Result<u8> {
    // shortcut: these are refused until Tasks 6 to 10 add them.
    if cli.command.is_some() || cli.fix || cli.stdin {
        anyhow::bail!("this option is not available yet");
    }
    let started = Instant::now();
    let cwd = std::env::current_dir().context("can't find the current folder")?;
    let configs = Configs::new(cli.config.as_deref(), &cwd)?;
    let collected = collect(&cli.paths, &cwd, &configs);
    for line in &collected.errors {
        write_stderr(line);
    }
    if let Some(line) = &collected.fatal {
        write_stderr(line);
        return Ok(2);
    }
    if collected.targets.is_empty() && collected.errors.is_empty() {
        write_stderr("no files matched");
        return Ok(2);
    }
    let items = process(&collected.targets, &configs, cli.quiet);

    let mut results = Vec::new();
    let mut config_error = None;
    let mut fatal = !collected.errors.is_empty();
    for item in items {
        match item.outcome {
            Ok(result) => results.push(result),
            Err(failure) => {
                fatal = true;
                if failure.is_config {
                    // The first one, by sorted path.
                    config_error.get_or_insert(failure.line);
                } else {
                    write_stderr(&failure.line);
                }
            }
        }
    }
    if let Some(line) = config_error {
        write_stderr(&line);
        return Ok(2);
    }
    let summary = Summary::new(&results, started);
    let text = output::render(cli.format, &results, &summary);
    let mut stdout = anstream::AutoStream::new(std::io::stdout().lock(), cli.color.choice());
    match stdout
        .write_all(text.as_bytes())
        .and_then(|()| stdout.flush())
    {
        Ok(()) => {}
        // The reader went away (`| head`): nothing more to say.
        Err(e) if e.kind() == ErrorKind::BrokenPipe => {}
        Err(e) => anyhow::bail!("can't write output: {e}"),
    }
    Ok(exit_code(&summary, fatal, cli.max_warnings))
}

fn write_stderr(line: &str) {
    // Nobody to tell if stderr is closed.
    let _ = writeln!(std::io::stderr().lock(), "{line}");
}

/// Lints every target on worker threads; the items come back sorted by `display`.
fn process(targets: &[Target], configs: &Configs, quiet: bool) -> Vec<Item> {
    let next = AtomicUsize::new(0);
    let items = Mutex::new(Vec::with_capacity(targets.len()));
    let workers = std::thread::available_parallelism().map_or(1, usize::from);
    std::thread::scope(|scope| {
        for _ in 0..workers.min(targets.len()) {
            scope.spawn(|| {
                while let Some(target) = targets.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let item = lint_file(target, configs, quiet);
                    items
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .push(item);
                }
            });
        }
    });
    let mut items = items
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    items.sort_by(|a, b| a.display.cmp(&b.display));
    items
}

fn lint_file(target: &Target, configs: &Configs, quiet: bool) -> Item {
    let Target { path, display, .. } = target;
    let display = display.clone();
    let read_error = |line: String| Failure {
        line,
        is_config: false,
    };
    let outcome = (|| {
        let text = std::fs::File::open(path)
            .map_err(ReadError::Io)
            .and_then(read_for_lint)
            .map_err(|e| {
                read_error(match e {
                    ReadError::Io(e) if e.kind() == ErrorKind::NotFound => {
                        format!("{display}: no such file or directory")
                    }
                    ReadError::Io(e) => format!("{display}: can't read: {e}"),
                    ReadError::NotUtf8 => format!("{display}: not valid UTF-8"),
                })
            })?;
        let linter = configs
            .linter_for(path.parent().unwrap_or(path))
            .map_err(|line| Failure {
                line,
                is_config: true,
            })?;
        let mut diagnostics = linter.lint(&text).diagnostics;
        if quiet {
            diagnostics.retain(|d| d.severity == Severity::Error);
        }
        Ok(FileResult {
            display: display.clone(),
            text: (!diagnostics.is_empty()).then_some(text),
            diagnostics,
            fixed: 0,
            written: false,
        })
    })();
    Item { display, outcome }
}
