//! `cargo xtask parity-record`: runs yaml-fm-lint 1.7.1 on every fixture and
//! stores the raw results in `tests/parity/expected-1x/`.
//!
//! Output is deterministic: `record-1x.cjs` prints pretty JSON with a fixed
//! field order. This code only checks that the output parses as JSON and then
//! writes the exact bytes it received (the script already ends them with one
//! newline), so running the command twice gives identical files.

use anyhow::{Context, Result, bail};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// The 1.x version whose behaviour is recorded.
const VERSION: &str = "1.7.1";
/// How long one fixture may take before it is recorded as a timeout.
const TIMEOUT: Duration = Duration::from_secs(30);
/// Fixture groups under `tests/fixtures/`.
const GROUPS: &[&str] = &["1x", "edge"];

/// Turns `1x/testQuotes.md` into `1x__testQuotes.md.json`.
pub fn record_file_name(fixture_rel: &str) -> String {
    format!("{}.json", fixture_rel.replace('/', "__"))
}

/// The repository root: the parent of the xtask crate.
fn repo_root() -> Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .context("xtask has no parent directory")?;
    Ok(root.to_path_buf())
}

/// Installs yaml-fm-lint 1.7.1 into `target/parity-1x`, unless it is already there.
fn install_1x(pkg_dir: &Path) -> Result<()> {
    let manifest = pkg_dir
        .join("node_modules")
        .join("yaml-fm-lint")
        .join("package.json");
    if let Ok(text) = fs::read_to_string(&manifest)
        && let Ok(json) = serde_json::from_str::<serde_json::Value>(&text)
        && json["version"] == VERSION
    {
        return Ok(());
    }
    println!(
        "installing yaml-fm-lint@{VERSION} into {}",
        pkg_dir.display()
    );
    fs::create_dir_all(pkg_dir)?;
    let status = Command::new("npm")
        .arg("install")
        .arg("--prefix")
        .arg(pkg_dir)
        .args(["--ignore-scripts", "--no-audit", "--no-fund"])
        .arg(format!("yaml-fm-lint@{VERSION}"))
        .status()
        .context("could not run npm")?;
    if !status.success() {
        bail!("npm install failed: {status}");
    }
    Ok(())
}

/// Lists fixtures as `group/name.md`, sorted.
fn list_fixtures(fixtures_dir: &Path) -> Result<Vec<String>> {
    let mut found = Vec::new();
    for group in GROUPS {
        let dir = fixtures_dir.join(group);
        for entry in fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))? {
            let name = entry?.file_name().to_string_lossy().into_owned();
            if name.ends_with(".md") {
                found.push(format!("{group}/{name}"));
            }
        }
    }
    found.sort();
    Ok(found)
}

/// What running the recorder script on one fixture produced.
struct Recorded {
    /// The JSON text to store.
    json: String,
    /// Whether 1.7.1 crashed (or timed out) on this fixture.
    crashed: bool,
}

/// A JSON document for a fixture that could not be recorded normally.
fn crash_json(fixture_rel: &str, message: &str) -> String {
    let value = serde_json::json!({
        "fixture": fixture_rel,
        "version": VERSION,
        "errors": {},
        "warnings": {},
        "crashed": message,
    });
    // Same format as record-1x.cjs: two-space indent and one trailing newline.
    let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
    text.push('\n');
    text
}

/// Runs the recorder script with a timeout. Never fails the whole run: any
/// problem becomes a `crashed` record for this fixture.
fn record_one(root: &Path, pkg_dir: &Path, fixture_rel: &str) -> Recorded {
    let script = root.join("xtask").join("assets").join("record-1x.cjs");
    let fixture_abs = root.join("tests").join("fixtures").join(fixture_rel);

    let spawned = Command::new("node")
        .arg(&script)
        .arg(pkg_dir)
        .arg(&fixture_abs)
        .arg(fixture_rel)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(err) => {
            return Recorded {
                json: crash_json(fixture_rel, &format!("could not start node: {err}")),
                crashed: true,
            };
        }
    };

    // Read stdout on a thread so a full pipe can never block the child.
    let reader = child.stdout.take().map(|mut out| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = out.read_to_end(&mut bytes);
            bytes
        })
    });

    let deadline = Instant::now() + TIMEOUT;
    let mut timed_out = false;
    let mut exit_ok = false;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                exit_ok = status.success();
                break;
            }
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                timed_out = true;
                break;
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(_) => break,
        }
    }

    let bytes = match reader {
        Some(handle) => handle.join().unwrap_or_default(),
        None => Vec::new(),
    };

    if timed_out {
        return Recorded {
            json: crash_json(fixture_rel, "timeout"),
            crashed: true,
        };
    }
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let parsed = serde_json::from_str::<serde_json::Value>(&text);
    match parsed {
        Ok(value) if exit_ok => {
            let crashed = !value["crashed"].is_null();
            Recorded {
                json: text,
                crashed,
            }
        }
        _ => Recorded {
            json: crash_json(fixture_rel, "recorder produced no valid JSON"),
            crashed: true,
        },
    }
}

/// Runs `cargo xtask parity-record`.
pub fn record() -> Result<()> {
    let root = repo_root()?;
    let pkg_dir = root.join("target").join("parity-1x");
    let out_dir = root.join("tests").join("parity").join("expected-1x");
    install_1x(&pkg_dir)?;
    fs::create_dir_all(&out_dir)?;

    let fixtures = list_fixtures(&root.join("tests").join("fixtures"))?;
    let mut crashed_count = 0;
    let mut wanted = Vec::new();
    for fixture_rel in &fixtures {
        let recorded = record_one(&root, &pkg_dir, fixture_rel);
        if recorded.crashed {
            crashed_count += 1;
            println!("crashed: {fixture_rel}");
        }
        let name = record_file_name(fixture_rel);
        fs::write(out_dir.join(&name), recorded.json)?;
        wanted.push(name);
    }

    // Remove records whose fixture is gone.
    for entry in fs::read_dir(&out_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".json") && !wanted.contains(&name) {
            fs::remove_file(entry.path())?;
        }
    }

    println!(
        "recorded {} fixtures ({crashed_count} crashed)",
        fixtures.len()
    );
    Ok(())
}
