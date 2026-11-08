//! Snapshots of what `lint` and `fix` do to every fixture, using the default configuration.
//!
//! Review a changed snapshot before accepting it; see "Snapshot tests" in AGENTS.md.

use yaml_fm_lint_core::{Config, Diagnostic, FixError, LineIndex, Linter, Severity};

/// One line per diagnostic: `L:C-L:C severity [rule] message` (1-based, character columns).
fn show(index: &LineIndex<'_>, diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|d| {
            let (start, end) = (index.position(d.range.start), index.position(d.range.end));
            let severity = match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            format!(
                "{}:{}-{}:{} {} [{}] {}\n",
                start.line + 1,
                start.chars + 1,
                end.line + 1,
                end.chars + 1,
                severity,
                d.rule,
                d.message
            )
        })
        .collect()
}

/// The text with `\r` shown as `␍` and a BOM as `<BOM>`, so snapshot files stay plain LF text.
fn visible(text: &str) -> String {
    text.replace('\r', "\u{240D}").replace('\u{FEFF}', "<BOM>")
}

#[test]
fn fixtures() {
    let linter = Linter::new(Config::default(), None).expect("default config is valid");
    insta::glob!("../../../tests/fixtures", "{1x,edge}/*.md", |path| {
        let source = std::fs::read_to_string(path).expect("fixture is UTF-8");
        let index = LineIndex::new(&source);

        let report = linter.lint(&source);
        let fixable = report
            .diagnostics
            .iter()
            .filter(|d| d.fix.is_some())
            .count();
        insta::assert_snapshot!(
            "lint",
            format!("{}fixable: {fixable}\n", show(&index, &report.diagnostics))
        );

        let fixed = match linter.fix(&source) {
            Err(FixError::Unparseable(_)) => "Unparseable\n".to_string(),
            Ok(outcome) => {
                let out_index = LineIndex::new(&outcome.output);
                format!(
                    "{}\napplied: {}\nrefused:\n{}remaining:\n{}",
                    visible(&outcome.output),
                    outcome.applied,
                    show(&index, &outcome.refused),
                    show(&out_index, &outcome.remaining)
                )
            }
        };
        insta::assert_snapshot!("fix", fixed);
    });
}
