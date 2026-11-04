//! Deterministic benchmark data: `cargo xtask bench-corpus`.
//!
//! The same `--files` and `--seed` always give byte-identical files: one
//! SplitMix64 stream is drawn in file order, names are zero-padded and every
//! file uses `\n` line endings.

use anyhow::{Context, Result, bail};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// The SplitMix64 random number generator (no external crate needed).
pub struct SplitMix64(u64);

impl SplitMix64 {
    /// Starts a stream from `seed`.
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Returns the next 64-bit value.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Returns a value in `0..n` (plain modulo; `n` must be above zero).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

/// What `generate` produced; written to `<out>/manifest.json`.
pub struct Manifest {
    /// Number of Markdown files written.
    pub files: usize,
    /// The seed that was used.
    pub seed: u64,
    /// Files with a deliberate error, as sorted `/`-separated relative paths.
    pub error_files: Vec<String>,
}

const WORDS: &[&str] = &[
    "amber", "birch", "cedar", "delta", "ember", "fjord", "grove", "harbor", "island", "juniper",
    "kernel", "lantern", "meadow", "nectar", "orchid", "pebble", "quartz", "river", "summit",
    "timber", "upland", "valley", "willow", "yonder", "zephyr", "anchor", "breeze", "canyon",
    "dune", "echo", "forest", "glacier", "horizon", "ivory", "jungle", "koala", "lagoon", "marble",
    "north", "ocean",
];

/// Front-matter key order. A file with N keys (5 to 20) gets the first N of
/// this list, so `title`, `date`, `author`, `description` and `draft` are
/// always present.
const KEYS: &[&str] = &[
    "title",
    "date",
    "author",
    "description",
    "draft",
    "weight",
    "slug",
    "tags",
    "field_01",
    "field_02",
    "field_03",
    "field_04",
    "field_05",
    "field_06",
    "field_07",
    "field_08",
    "field_09",
    "field_10",
    "field_11",
    "field_12",
];

/// The word-valued keys an error can be injected into.
const ERROR_KEYS: &[&str] = &["title", "author", "description"];

fn word(rng: &mut SplitMix64) -> &'static str {
    WORDS
        .get(rng.below(WORDS.len() as u64) as usize)
        .copied()
        .unwrap_or("word")
}

fn phrase(rng: &mut SplitMix64, count: u64) -> String {
    let mut words = Vec::new();
    for _ in 0..count {
        words.push(word(rng));
    }
    words.join(" ")
}

/// Builds one file. Returns the text and whether it carries a deliberate error.
fn make_file(rng: &mut SplitMix64) -> (String, bool) {
    let key_count = 5 + rng.below(16) as usize;
    let has_error = rng.below(10) == 0;
    let error_kind = rng.below(3);
    let error_key = ERROR_KEYS
        .get(rng.below(ERROR_KEYS.len() as u64) as usize)
        .copied()
        .unwrap_or("title");

    let mut text = String::from("---\n");
    for key in KEYS.iter().take(key_count) {
        let line = match *key {
            "date" => format!(
                "date: 2026-{:02}-{:02}",
                1 + rng.below(12),
                1 + rng.below(28)
            ),
            "draft" => format!("draft: {}", rng.below(2) == 0),
            "weight" => format!("weight: {}", rng.below(1000)),
            "tags" => {
                let mut tags = String::from("tags:");
                for _ in 0..1 + rng.below(5) {
                    let _ = write!(tags, "\n  - {}", word(rng));
                }
                tags
            }
            _ => {
                let count = 1 + rng.below(3);
                let value = phrase(rng, count);
                if has_error && *key == error_key {
                    // The one word-valued line that gets the deliberate error.
                    match error_kind {
                        0 => format!("{key}: {value} "),
                        1 => format!("{key}: \"{}\"", word(rng)),
                        _ => format!("{key}: {value},"),
                    }
                } else {
                    format!("{key}: {value}")
                }
            }
        };
        text.push_str(&line);
        text.push('\n');
    }
    text.push_str("---\n");

    let target = 2048 + rng.below(2953) as usize; // 2048..=5000, below the 5120 cap
    let mut body = String::new();
    while body.len() < target {
        let count = 8 + rng.below(9);
        let line = phrase(rng, count);
        body.push_str(&line);
        body.push('\n');
        if rng.below(4) == 0 {
            body.push('\n');
        }
    }
    text.push_str(&body);
    (text, has_error)
}

/// Writes `files` Markdown files plus `manifest.json` into `out`.
///
/// `out` may be missing or empty; a non-empty directory is refused.
pub fn generate(files: usize, seed: u64, out: &Path) -> Result<Manifest> {
    if out.exists() {
        let mut entries =
            fs::read_dir(out).with_context(|| format!("reading {}", out.display()))?;
        if entries.next().is_some() {
            bail!("output directory {} is not empty", out.display());
        }
    }
    let mut rng = SplitMix64::new(seed);
    let mut error_files = Vec::new();
    for i in 0..files {
        let rel = format!("s{:03}/g{}/post-{i:06}.md", i / 1000, (i / 100) % 10);
        let (text, has_error) = make_file(&mut rng);
        let path = out.join(&rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
        if has_error {
            error_files.push(rel);
        }
    }
    error_files.sort();
    let manifest = Manifest {
        files,
        seed,
        error_files,
    };

    fs::create_dir_all(out)?;
    let json = serde_json::json!({
        "files": manifest.files,
        "seed": manifest.seed,
        "error_files": manifest.error_files,
    });
    let mut rendered = serde_json::to_string_pretty(&json)?;
    rendered.push('\n');
    fs::write(out.join("manifest.json"), rendered).context("writing manifest.json")?;
    Ok(manifest)
}

/// Parses `--files <N> --seed <S> --out <dir>` and runs `generate`.
pub fn run(args: &[String]) -> Result<()> {
    let mut files = None;
    let mut seed = None;
    let mut out = None;
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let value = it.next().with_context(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--files" => files = Some(value.parse::<usize>().context("--files must be a number")?),
            "--seed" => seed = Some(value.parse::<u64>().context("--seed must be a number")?),
            "--out" => out = Some(value.clone()),
            other => bail!("unknown option {other}"),
        }
    }
    let files = files.context("missing --files <N>")?;
    let seed = seed.context("missing --seed <S>")?;
    let out = out.context("missing --out <dir>")?;
    let manifest = generate(files, seed, Path::new(&out))?;
    println!(
        "wrote {} files ({} with deliberate errors) to {out}",
        manifest.files,
        manifest.error_files.len()
    );
    Ok(())
}
