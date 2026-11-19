//! Source snippets with the problem underlined, and a summary line.

use std::fmt::Write;
use std::ops::Range;

use annotate_snippets::{AnnotationKind, Level, Renderer, Snippet};
use yaml_fm_lint_core::{Severity, TextRange};

use super::{escape, positions, push_escaped};
use crate::run::{FileResult, Summary};

/// The snippets, then the summary line. The output carries colour codes; the stream it is
/// written to removes them when colour is off.
pub fn render(results: &[FileResult], summary: &Summary) -> String {
    let renderer = Renderer::styled();
    let mut out = String::new();
    let mut groups = 0;
    for result in results {
        let text = result.text.as_deref().unwrap_or_default();
        for (d, (start, _)) in result.diagnostics.iter().zip(positions(result)) {
            let level = match d.severity {
                Severity::Error => Level::ERROR,
                Severity::Warning => Level::WARNING,
            };
            let (source, span) = snippet(text, d.range);
            let group = level
                .primary_title(escape(&d.message))
                .id(d.rule.to_string())
                .element(
                    Snippet::source(source)
                        .line_start(start.line)
                        .path(escape(&result.display))
                        .fold(true)
                        .annotation(AnnotationKind::Primary.span(span)),
                );
            if groups > 0 {
                out.push('\n');
            }
            groups += 1;
            out.push_str(&renderer.render(&[group]));
            out.push('\n');
        }
    }
    if groups > 0 {
        out.push('\n');
    }
    summary_lines(summary, &mut out);
    out
}

/// The lines of `text` that `range` covers, escaped, and the range within them.
fn snippet(text: &str, range: TextRange) -> (String, Range<usize>) {
    let start = range.start.min(text.len());
    let end = range.end.clamp(start, text.len());
    let line_start = text
        .get(..start)
        .and_then(|before| before.rfind('\n'))
        .map_or(0, |i| i + 1);
    // A range that ends with a line break covers that line, not the next one.
    let ends_with_break = end > start && text.get(..end).is_some_and(|t| t.ends_with('\n'));
    let from = if ends_with_break { end - 1 } else { end };
    let mut line_end = text
        .get(from..)
        .and_then(|after| after.find('\n'))
        .map_or(text.len(), |i| from + i);
    // The `\r` of a `\r\n` line break is not part of the line.
    if text.get(..line_end).is_some_and(|t| t.ends_with('\r')) && line_end < text.len() {
        line_end -= 1;
    }
    let lines = text.get(line_start..line_end).unwrap_or_default();

    let mut source = String::with_capacity(lines.len());
    let (mut span_start, mut span_end) = (None, None);
    for (i, c) in lines.char_indices() {
        if i == start - line_start {
            span_start = Some(source.len());
        }
        if i == end.min(line_end) - line_start {
            span_end = Some(source.len());
        }
        match c {
            '\n' | '\t' => source.push(c),
            '\r' if lines.as_bytes().get(i + 1) == Some(&b'\n') => {}
            _ => push_escaped(&mut source, c),
        }
    }
    let span = span_start.unwrap_or(source.len())..span_end.unwrap_or(source.len());
    (source, span)
}

fn summary_lines(summary: &Summary, out: &mut String) {
    let Summary {
        errors,
        warnings,
        fixable,
        files,
        files_with_problems,
        fixed_problems,
        fixed_files,
        fix,
        incomplete,
        elapsed_ms,
    } = summary;
    let total = format!("{} in {elapsed_ms} ms", count(*files, "file"));
    if *fix {
        let _ = writeln!(
            out,
            "fixed {} in {}",
            count(*fixed_problems, "problem"),
            count(*fixed_files, "file")
        );
    }
    if errors + warnings == 0 {
        if !incomplete {
            let _ = writeln!(out, "✔ no problems · {total}");
        }
        return;
    }
    let _ = write!(
        out,
        "✘ {}, {} in {}",
        count(*errors, "error"),
        count(*warnings, "warning"),
        count(*files_with_problems, "file")
    );
    if *fixable > 0 {
        let _ = write!(out, " ({fixable} fixable with --fix)");
    }
    let _ = writeln!(out, " · {total}");
}

/// `1 file`, `2 files`.
fn count(n: usize, noun: &str) -> String {
    format!("{n} {noun}{}", if n == 1 { "" } else { "s" })
}
