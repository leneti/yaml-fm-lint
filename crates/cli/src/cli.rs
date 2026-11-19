//! Command-line arguments.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// Lints and fixes the YAML front matter of markdown files.
#[derive(Debug, Parser)]
#[command(version, args_conflicts_with_subcommands = true)]
// `allow`, not `expect`: Rust 1.88 and current stable disagree on which fields count as read.
#[allow(dead_code, reason = "fields are read by later tasks")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Files, folders or globs to lint (default: the current folder)
    pub paths: Vec<String>,

    /// Apply the automatic fixes
    #[arg(long)]
    pub fix: bool,

    /// Config file with the highest priority
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Output format
    #[arg(long, value_enum, default_value_t = Format::Human)]
    pub format: Format,

    /// When to use colours
    #[arg(long, value_enum, default_value_t = ColorWhen::Auto)]
    pub color: ColorWhen,

    /// Fail when there are more warnings than this
    #[arg(long, value_name = "N")]
    pub max_warnings: Option<usize>,

    /// Report errors only
    #[arg(short, long)]
    pub quiet: bool,

    /// Read the text from standard input
    #[arg(long)]
    pub stdin: bool,

    /// The path standard input stands for, to find its config
    #[arg(long, requires = "stdin", value_name = "PATH")]
    pub stdin_filepath: Option<PathBuf>,

    /// Accepted for compatibility with 1.x and ignored
    #[arg(short, long, hide = true)]
    pub recursive: bool,
}

/// What `yaml-fm-lint` does besides linting.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print the settings that apply to a file
    PrintConfig {
        /// The file to look up
        file: PathBuf,
    },
    /// Convert a 1.x config file to the 2.0 format
    Migrate {
        /// The 1.x config file (default: ./.yaml-fm-lint.json)
        path: Option<PathBuf>,
        /// Overwrite the file instead of printing the result
        #[arg(long)]
        write: bool,
    },
}

/// How problems are printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// Source snippets and a summary line.
    Human,
    /// One line per problem.
    Compact,
    /// A JSON document.
    Json,
    /// SARIF 2.1.0.
    Sarif,
    /// GitHub Actions annotations.
    Github,
}

/// The `--color` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorWhen {
    /// Colours on a terminal, unless `NO_COLOR` is set.
    Auto,
    /// Always.
    Always,
    /// Never.
    Never,
}

impl ColorWhen {
    /// The matching `anstream` choice.
    pub fn choice(self) -> anstream::ColorChoice {
        match self {
            ColorWhen::Auto => anstream::ColorChoice::Auto,
            ColorWhen::Always => anstream::ColorChoice::Always,
            ColorWhen::Never => anstream::ColorChoice::Never,
        }
    }
}
