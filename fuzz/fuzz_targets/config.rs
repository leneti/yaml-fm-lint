//! Config files, however malformed, never panic: not when resolving them, not when building a
//! linter from the result, not when matching a path or printing the config.
#![no_main]

use libfuzzer_sys::fuzz_target;
use yaml_fm_lint_core::{ConfigLayer, Linter, resolve_config};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    // A `\0` splits the input into the config of `/r` and the config of `/r/sub`.
    let sources = match text.split_once('\0') {
        Some((outer, inner)) => vec![("/r", outer), ("/r/sub", inner)],
        None => vec![("/r", text)],
    };
    let layers: Vec<ConfigLayer> = sources
        .into_iter()
        .map(|(dir, source)| ConfigLayer {
            dir: dir.to_string(),
            path: format!("{dir}/.yaml-fm-lint.json"),
            source: source.to_string(),
        })
        .collect();
    let Ok(config) = resolve_config(&layers, None) else {
        return;
    };
    if let Ok(linter) = Linter::new(config.clone(), config.schema.as_ref().map(|_| "{}")) {
        linter.matches("/r/sub/a.md");
        config.to_json();
    }
});
