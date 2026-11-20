//! Inputs the fuzzer found: every reported range is ordered, inside the text and on character
//! boundaries.

use yaml_fm_lint_core::{Config, LineIndex, Linter};

#[allow(clippy::expect_used)]
fn assert_ranges_sound(text: &str) {
    let linter = Linter::new(Config::default(), None).expect("default config is valid");
    let report = linter.lint(text);
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
}

#[test]
fn lone_cr_line_breaks_in_front_matter() {
    // crash-392164c1: a tag line, then lone CR line breaks and a comment.
    assert_ranges_sound(
        "---\n!defaults*est&====tag===tag==\r\r\r\r\r\r\r\r\r\r\r\r\r\r\r\r\r\r\r\r\r\r# f\r\r=== p je\n---\n\nBdoy.\n",
    );
}
