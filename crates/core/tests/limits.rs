//! Input limits through the public API.

use yaml_fm_lint_core::{Config, FixError, Linter, RuleId};

#[test]
fn oversized_front_matter() {
    let source = format!("---\n{}---\nBody.\n", "a: 1\n".repeat(210_000));
    assert!(source.len() > 1_050_000);
    let linter = Linter::new(Config::default(), None).expect("default config is valid");
    let report = linter.lint(&source);
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].rule, RuleId::InputLimit);
    assert!(matches!(linter.fix(&source), Err(FixError::Unparseable(_))));
}

#[test]
fn deep_flow_nesting() {
    // A small stack proves that neither the parser nor the tree builder recurses per level.
    let source = format!("---\na: {}\n---\n", "[".repeat(10_000));
    let linter = Linter::new(Config::default(), None).expect("default config is valid");
    let report = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || linter.lint(&source))
        .expect("thread starts")
        .join()
        .expect("no stack overflow");
    assert_eq!(report.diagnostics.len(), 1);
    assert!(matches!(
        report.diagnostics[0].rule,
        RuleId::YamlSyntax | RuleId::InputLimit
    ));
}
