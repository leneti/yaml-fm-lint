//! Every range `lint` reports, and every fix edit range, lies inside the text, on character
//! boundaries.
#![no_main]

use libfuzzer_sys::fuzz_target;
use std::sync::OnceLock;
use yaml_fm_lint_core::{Config, LineIndex, Linter};

fn linter() -> &'static Linter {
    static LINTER: OnceLock<Linter> = OnceLock::new();
    LINTER
        .get_or_init(|| Linter::new(Config::default(), None).expect("the default config is valid"))
}

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let report = linter().lint(text);
    let index = LineIndex::new(text);
    let ranges = report
        .front_matter
        .into_iter()
        .chain(report.diagnostics.iter().map(|d| d.range))
        .chain(
            (report.diagnostics.iter())
                .filter_map(|d| d.fix.as_ref())
                .flat_map(|fix| fix.edits.iter().map(|e| e.range)),
        );
    for range in ranges {
        assert!(
            range.start <= range.end && range.end <= text.len(),
            "range {range:?} is outside the {} byte text",
            text.len()
        );
        assert!(
            text.is_char_boundary(range.start) && text.is_char_boundary(range.end),
            "range {range:?} splits a character"
        );
        index.position(range.start);
        index.position(range.end);
    }
});
