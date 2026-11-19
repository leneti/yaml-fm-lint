//! Linting the files named on the command line.

use std::io::{ErrorKind, Write};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use anyhow::Context;
use yaml_fm_lint_core::{Applicability, Diagnostic, FixError, Linter, Severity};

use crate::cli::Cli;
use crate::configs::Configs;
use crate::output;
use crate::paths::{absolute, display};
use crate::read::{ReadError, read_for_lint, read_whole};
use crate::walk::{Target, collect};
use crate::write::write_atomic;

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
    /// Whether the run was asked to fix files.
    pub fix: bool,
    pub elapsed_ms: u128,
}

impl Summary {
    fn new(results: &[FileResult], started: Instant, fix: bool) -> Summary {
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
            fix,
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

/// Why a file has no result.
#[derive(PartialEq, Eq)]
enum Failure {
    /// The config that applies to it is wrong.
    Config,
    /// It couldn't be read or written.
    Io,
    /// It was left alone on purpose.
    Skipped,
}

/// A file that could not be processed: the line to print on stderr, and why.
struct Failed {
    line: String,
    kind: Failure,
}

struct Item {
    display: String,
    outcome: Result<FileResult, Failed>,
}

/// Lints the files named in `cli.paths` and prints the report; returns the exit code.
pub fn lint(cli: &Cli) -> anyhow::Result<u8> {
    if cli.stdin && !cli.paths.is_empty() {
        anyhow::bail!("--stdin can't be combined with paths");
    }
    let started = Instant::now();
    let cwd = std::env::current_dir().context("can't find the current folder")?;
    let configs = Configs::new(cli.config.as_deref(), &cwd)?;
    if cli.stdin {
        return lint_stdin(cli, &cwd, &configs, started);
    }
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
    let items = process(&collected.targets, &configs, cli.quiet, cli.fix);

    let mut results = Vec::new();
    let mut config_error = None;
    let mut fatal = !collected.errors.is_empty();
    for item in items {
        match item.outcome {
            Ok(result) => results.push(result),
            Err(Failed { line, kind }) => {
                fatal |= kind != Failure::Skipped;
                if kind == Failure::Config {
                    // The first one, by sorted path.
                    config_error.get_or_insert(line);
                } else {
                    write_stderr(&line);
                }
            }
        }
    }
    if let Some(line) = config_error {
        write_stderr(&line);
        return Ok(2);
    }
    let summary = Summary::new(&results, started, cli.fix);
    let text = output::render(cli.format, &results, &summary);
    // `anstream` removes the colour codes when colour is off.
    let stdout = anstream::AutoStream::new(std::io::stdout().lock(), cli.color.choice());
    emit(stdout, &text)?;
    Ok(exit_code(&summary, fatal, cli.max_warnings))
}

/// Writes `text` to `out`. A reader that went away (`| head`) is not an error.
pub fn emit(mut out: impl Write, text: &str) -> anyhow::Result<()> {
    match out.write_all(text.as_bytes()).and_then(|()| out.flush()) {
        Err(e) if e.kind() != ErrorKind::BrokenPipe => anyhow::bail!("can't write output: {e}"),
        _ => Ok(()),
    }
}

/// Lints (or with `--fix`, fixes) the text on standard input.
fn lint_stdin(cli: &Cli, cwd: &Path, configs: &Configs, started: Instant) -> anyhow::Result<u8> {
    let path = cli
        .stdin_filepath
        .as_deref()
        .map(|path| absolute(path, cwd));
    let shown = path
        .as_deref()
        .map_or_else(|| "<stdin>".to_string(), |path| display(path, cwd));
    let text = match read_whole(std::io::stdin().lock()) {
        Ok(text) => text,
        Err(e) => {
            write_stderr(&match e {
                ReadError::Io(e) => format!("{shown}: can't read: {e}"),
                ReadError::NotUtf8 => format!("{shown}: not valid UTF-8"),
            });
            return Ok(2);
        }
    };
    let dir = path.as_deref().and_then(Path::parent).unwrap_or(cwd);
    let linter = match configs.linter_for(dir) {
        Ok(linter) => linter,
        Err(line) => {
            write_stderr(&line);
            return Ok(2);
        }
    };
    let (result, output) = check(&linter, shown, &text, cli.quiet, cli.fix);
    let results = [result];
    let summary = Summary::new(&results, started, cli.fix);
    if cli.fix {
        // Exactly the text, so an editor can pipe a buffer through; no problems are printed.
        emit(std::io::stdout().lock(), output.as_deref().unwrap_or(&text))?;
    } else {
        let stdout = anstream::AutoStream::new(std::io::stdout().lock(), cli.color.choice());
        emit(stdout, &output::render(cli.format, &results, &summary))?;
    }
    Ok(exit_code(&summary, false, cli.max_warnings))
}

/// Lints `text`, or with `fix` fixes it. The second value is the fixed text, when it differs
/// from `text`; the result's positions refer to the text as it ends up (nothing is written
/// here, so `written` is false).
fn check(
    linter: &Linter,
    display: String,
    text: &str,
    quiet: bool,
    fix: bool,
) -> (FileResult, Option<String>) {
    let (mut diagnostics, fixed, changed) = if fix {
        match linter.fix(text) {
            // The text stays as it is, and so do its problems.
            Err(FixError::Unparseable(found)) => (found, 0, None),
            Ok(o) if o.output == text => (o.remaining, 0, None),
            Ok(o) => (o.remaining, o.applied, Some(o.output)),
        }
    } else {
        (linter.lint(text).diagnostics, 0, None)
    };
    let shown = changed.as_deref().unwrap_or(text);
    if quiet {
        diagnostics.retain(|d| d.severity == Severity::Error);
    }
    let result = FileResult {
        display,
        text: (!diagnostics.is_empty()).then(|| shown.to_string()),
        diagnostics,
        fixed,
        written: false,
    };
    (result, changed)
}

pub fn write_stderr(line: &str) {
    // Nobody to tell if stderr is closed.
    let _ = writeln!(std::io::stderr().lock(), "{line}");
}

/// Lints (or fixes) every target on worker threads; the items come back sorted by `display`.
fn process(targets: &[Target], configs: &Configs, quiet: bool, fix: bool) -> Vec<Item> {
    let next = AtomicUsize::new(0);
    let items = Mutex::new(Vec::with_capacity(targets.len()));
    let workers = std::thread::available_parallelism().map_or(1, usize::from);
    std::thread::scope(|scope| {
        for _ in 0..workers.min(targets.len()) {
            scope.spawn(|| {
                while let Some(target) = targets.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let item = lint_file(target, configs, quiet, fix);
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

fn lint_file(target: &Target, configs: &Configs, quiet: bool, fix: bool) -> Item {
    let Target { path, display, .. } = target;
    let display = display.clone();
    let failed = |kind, line| Failed { line, kind };
    let outcome = (|| {
        if fix && std::fs::symlink_metadata(path).is_ok_and(|m| m.is_symlink()) {
            // Replacing a link would replace the link, not the file it points to.
            return Err(failed(
                Failure::Skipped,
                format!("{display}: skipped: symlink"),
            ));
        }
        let text = std::fs::File::open(path)
            .map_err(ReadError::Io)
            // `--fix` reads everything, so it also catches invalid UTF-8 after the front matter.
            .and_then(if fix { read_whole } else { read_for_lint })
            .map_err(|e| {
                failed(
                    Failure::Io,
                    match e {
                        ReadError::Io(e) if e.kind() == ErrorKind::NotFound => {
                            format!("{display}: no such file or directory")
                        }
                        ReadError::Io(e) => format!("{display}: can't read: {e}"),
                        ReadError::NotUtf8 => format!("{display}: not valid UTF-8"),
                    },
                )
            })?;
        let linter = configs
            .linter_for(path.parent().unwrap_or(path))
            .map_err(|line| failed(Failure::Config, line))?;
        let (mut result, output) = check(&linter, display.clone(), &text, quiet, fix);
        if let Some(output) = output {
            write_atomic(path, &output)
                .map_err(|e| failed(Failure::Io, format!("{display}: can't write: {e}")))?;
            result.written = true;
        }
        Ok(result)
    })();
    Item { display, outcome }
}
