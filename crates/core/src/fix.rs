//! The fix engine: applies automatic fixes, and keeps only those that provably leave the
//! front-matter data unchanged (spec §5.6).

use std::collections::BTreeMap;

use crate::data::{Data, build, data_eq, lookup_mut, resolve_plain};
use crate::edit::apply_edits;
use crate::extract::{Extraction, FrontMatter, extract};
use crate::parse::parse;
use crate::{
    Applicability, DataEffect, Diagnostic, Fix, FixError, FixOutcome, Linter, RuleId, TextEdit,
    TextRange,
};

/// The most fix passes `fix` runs; a pass applies the fixes that don't overlap.
pub(crate) const MAX_FIX_PASSES: usize = 10;

/// Appended to the message of a finding whose fix was refused.
const REFUSED_NOTE: &str = " (fix refused: would change data)";

/// A diagnostic and its automatic fix.
type Candidate = (Diagnostic, Fix);

/// The diagnostics with an `Automatic` fix, minus those whose `(rule, range)` was refused.
pub(crate) fn candidates(
    diagnostics: &[Diagnostic],
    refused: &[(RuleId, TextRange)],
) -> Vec<Candidate> {
    diagnostics
        .iter()
        .filter(|d| !refused.contains(&(d.rule, d.range)))
        .filter_map(|d| {
            let fix = d.fix.as_ref()?;
            let usable = fix.applicability == Applicability::Automatic && !fix.edits.is_empty();
            usable.then(|| (d.clone(), fix.clone()))
        })
        .collect()
}

/// Whether two edit ranges overlap; edits starting at the same offset always do.
fn overlaps(a: TextRange, b: TextRange) -> bool {
    (a.start < b.end && b.start < a.end) || a.start == b.start
}

/// Splits `fixes` into those to apply in one pass and those skipped for overlapping.
///
/// Fixes are taken in the order of their first edit; one is skipped if any of its edits
/// overlaps an edit already taken.
pub(crate) fn select(mut fixes: Vec<Candidate>) -> (Vec<Candidate>, Vec<Candidate>) {
    fixes.sort_by_key(|(_, fix)| fix.edits.first().map(|e| e.range.start));
    // Start to end of the edits taken; they never overlap, so a start is unique and a
    // range can only clash with its neighbours.
    let mut taken: BTreeMap<usize, usize> = BTreeMap::new();
    let clashes = |taken: &BTreeMap<usize, usize>, r: TextRange| {
        let before = taken.range(..=r.start).next_back();
        let after = taken.range(r.start.saturating_add(1)..).next();
        [before, after]
            .into_iter()
            .flatten()
            .any(|(&start, &end)| overlaps(TextRange { start, end }, r))
    };
    let (mut chosen, mut skipped) = (Vec::new(), Vec::new());
    for candidate in fixes {
        if candidate.1.edits.iter().any(|e| clashes(&taken, e.range)) {
            skipped.push(candidate);
        } else {
            taken.extend(
                candidate
                    .1
                    .edits
                    .iter()
                    .map(|e| (e.range.start, e.range.end)),
            );
            chosen.push(candidate);
        }
    }
    (chosen, skipped)
}

/// The data of `input` with every effect applied; `None` if it does not parse or a path is gone.
fn expected_data(input: &str, fm: &FrontMatter, effects: &[DataEffect]) -> Option<Data> {
    let mut data = build(&parse(input, fm).ok()?).ok()?;
    for effect in effects {
        if let DataEffect::ChangesScalar { path, new_plain } = effect {
            *lookup_mut(&mut data, path)? = resolve_plain(new_plain);
        }
    }
    Some(data)
}

/// Whether `output` is `input` with the front matter changed only as `effects` say.
///
/// Everything outside the front matter's lines must be byte-identical: the BOM, the opening
/// delimiter's line ending, and everything after the closing delimiter's text (its line
/// ending and the body). The data must be the same, apart from the effects.
pub(crate) fn verify(input: &str, output: &str, effects: &[DataEffect]) -> bool {
    let (Extraction::Found(before), Extraction::Found(after)) = (extract(input), extract(output))
    else {
        return false;
    };
    let same = |a: Option<&str>, b: Option<&str>| a.is_some() && a == b;
    if before.bom != after.bom
        || !same(
            input.get(before.open.end..before.content.start),
            output.get(after.open.end..after.content.start),
        )
        || !same(input.get(before.close.end..), output.get(after.close.end..))
    {
        return false;
    }
    let Some(expected) = expected_data(input, &before, effects) else {
        return false;
    };
    parse(output, &after)
        .ok()
        .and_then(|root| build(&root).ok())
        .is_some_and(|data| data_eq(&data, &expected))
}

/// The edits of the fixes, sorted for `apply_edits`.
fn sorted_edits<'a>(fixes: impl Iterator<Item = &'a Fix>) -> Vec<TextEdit> {
    let mut edits: Vec<TextEdit> = fixes.flat_map(|fix| fix.edits.iter().cloned()).collect();
    edits.sort_by_key(|e| (e.range.start, e.range.end));
    edits
}

/// What one pass did.
struct Pass {
    /// The text after the pass.
    output: String,
    /// The edits applied, sorted, in the coordinates of the text before the pass.
    edits: Vec<TextEdit>,
    /// How many fixes were applied.
    applied: usize,
    /// The fixes that were refused.
    refused: Vec<Diagnostic>,
}

/// Applies `chosen` to `text` if the result verifies; otherwise one fix at a time.
fn run_pass(text: &str, chosen: &[Candidate]) -> Pass {
    let effects: Vec<DataEffect> = chosen.iter().map(|(_, f)| f.data_effect.clone()).collect();
    let edits = sorted_edits(chosen.iter().map(|(_, fix)| fix));
    if let Some(output) = apply_edits(text, &edits).filter(|o| verify(text, o, &effects)) {
        return Pass {
            output,
            edits,
            applied: chosen.len(),
            refused: Vec::new(),
        };
    }
    // Some fix would change the data: find which, so the pass still makes progress. Edits
    // are applied to `text` together (same result as shifting each later one by the earlier
    // deltas), and each fix is checked against the effects accepted so far.
    let mut pass = Pass {
        output: text.to_string(),
        edits: Vec::new(),
        applied: 0,
        refused: Vec::new(),
    };
    let mut effects = Vec::new();
    for (diagnostic, fix) in chosen {
        let mut trial = pass.edits.clone();
        trial.extend(fix.edits.iter().cloned());
        trial.sort_by_key(|e| (e.range.start, e.range.end));
        effects.push(fix.data_effect.clone());
        match apply_edits(text, &trial).filter(|o| verify(text, o, &effects)) {
            Some(output) => {
                pass.output = output;
                pass.edits = trial;
                pass.applied += 1;
            }
            None => {
                effects.pop();
                pass.refused.push(diagnostic.clone());
            }
        }
    }
    pass
}

/// Carries ranges through `edits` (sorted, non-overlapping): a range an edit overlaps is
/// dropped, the others move by the edits before them.
fn shift(ranges: Vec<(RuleId, TextRange)>, edits: &[TextEdit]) -> Vec<(RuleId, TextRange)> {
    ranges
        .into_iter()
        .filter_map(|(rule, range)| {
            if edits.iter().any(|e| overlaps(e.range, range)) {
                return None;
            }
            let before = || edits.iter().filter(|e| e.range.end <= range.start);
            let added: usize = before().map(|e| e.new_text.len()).sum();
            let removed: usize = before().map(|e| e.range.end - e.range.start).sum();
            let moved = |offset: usize| offset.checked_add(added)?.checked_sub(removed);
            let range = TextRange {
                start: moved(range.start)?,
                end: moved(range.end)?,
            };
            Some((rule, range))
        })
        .collect()
}

impl Linter {
    /// Fixes what can be fixed in `source`.
    ///
    /// Only `Automatic` fixes are applied, in passes of non-overlapping fixes. A fix whose
    /// result would change the data (beyond what the fix declares) or touch anything outside
    /// the front matter is refused and reported in [`FixOutcome::refused`].
    pub fn fix(&self, source: &str) -> Result<FixOutcome, FixError> {
        let mut report = self.lint(source);
        if report
            .diagnostics
            .iter()
            .any(|d| matches!(d.rule, RuleId::YamlSyntax | RuleId::InputLimit))
        {
            return Err(FixError::Unparseable(report.diagnostics));
        }
        let mut text = source.to_string();
        let mut applied = 0;
        let mut refused = Vec::new();
        let mut refused_ranges: Vec<(RuleId, TextRange)> = Vec::new();
        for _ in 0..MAX_FIX_PASSES {
            let (chosen, _) = select(candidates(&report.diagnostics, &refused_ranges));
            if chosen.is_empty() {
                break;
            }
            let pass = run_pass(&text, &chosen);
            applied += pass.applied;
            refused_ranges.extend(pass.refused.iter().map(|d| (d.rule, d.range)));
            refused.extend(pass.refused);
            if !pass.edits.is_empty() {
                refused_ranges = shift(refused_ranges, &pass.edits);
                text = pass.output;
                report = self.lint(&text);
            }
        }
        let remaining = report
            .diagnostics
            .into_iter()
            .map(|mut d| {
                if refused_ranges.contains(&(d.rule, d.range)) {
                    d.message.push_str(REFUSED_NOTE);
                }
                d
            })
            .collect();
        Ok(FixOutcome {
            output: text,
            applied,
            refused,
            remaining,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Applicability, Config, Severity};

    fn linter() -> Linter {
        Linter::new(Config::default(), None).unwrap()
    }

    fn range(start: usize, end: usize) -> TextRange {
        TextRange { start, end }
    }

    fn edit(start: usize, end: usize) -> TextEdit {
        TextEdit {
            range: range(start, end),
            new_text: String::new(),
        }
    }

    fn fix(applicability: Applicability, edits: Vec<TextEdit>) -> Fix {
        Fix {
            applicability,
            data_effect: DataEffect::Preserving,
            edits,
        }
    }

    fn candidate(applicability: Applicability, edits: Vec<TextEdit>) -> Candidate {
        let diagnostic = Diagnostic {
            rule: RuleId::NoTrailingSpaces,
            severity: Severity::Warning,
            message: String::new(),
            range: edits.first().map_or(range(0, 0), |e| e.range),
            fix: None,
        };
        (diagnostic, fix(applicability, edits))
    }

    fn starts(fixes: &[Candidate]) -> Vec<(usize, usize)> {
        fixes
            .iter()
            .map(|(_, f)| (f.edits[0].range.start, f.edits[0].range.end))
            .collect()
    }

    #[test]
    fn fix_applies_and_reparses() {
        let outcome = linter().fix("---\na: 1 \nb: c,\n---\nx\n").unwrap();
        assert_eq!(outcome.output, "---\na: 1\nb: c\n---\nx\n");
        assert_eq!(outcome.applied, 2);
        assert!(outcome.refused.is_empty());
        assert_eq!(outcome.remaining, []);
    }

    #[test]
    fn fix_is_idempotent() {
        let outcome = linter().fix("---\na: 1\nb: c\n---\nx\n").unwrap();
        assert_eq!(outcome.output, "---\na: 1\nb: c\n---\nx\n");
        assert_eq!(outcome.applied, 0);
    }

    #[test]
    fn fix_never_touches_body() {
        let source = "---\na: 1 \n---\na: 1 \n---\n# fmlint-disable-line\n";
        let outcome = linter().fix(source).unwrap();
        assert_eq!(
            outcome.output,
            "---\na: 1\n---\na: 1 \n---\n# fmlint-disable-line\n"
        );
        assert_eq!(outcome.applied, 1);
    }

    #[test]
    fn fix_keeps_crlf_and_bom() {
        let outcome = linter()
            .fix("\u{FEFF}--- \r\na: 1 \r\n---\r\nb\r\n")
            .unwrap();
        assert_eq!(outcome.output, "\u{FEFF}---\r\na: 1\r\n---\r\nb\r\n");
        assert_eq!(outcome.applied, 2);
    }

    #[test]
    fn fix_refuses_alias_change() {
        let source = "---\na: &x foo,\nb: *x\n---\n";
        let outcome = linter().fix(source).unwrap();
        assert_eq!(outcome.output, source);
        assert_eq!(outcome.applied, 0);
        assert_eq!(outcome.refused.len(), 1);
        assert_eq!(outcome.refused[0].rule, RuleId::NoTrailingCommas);
        assert_eq!(
            outcome.remaining[0].message,
            "trailing comma (fix refused: would change data)"
        );
    }

    #[test]
    fn fix_refuses_only_the_failing_fix() {
        let source = "---\na: &x foo,\nb: *x\nc: 1 \n---\n";
        let outcome = linter().fix(source).unwrap();
        assert_eq!(outcome.output, "---\na: &x foo,\nb: *x\nc: 1\n---\n");
        assert_eq!(outcome.applied, 1);
        assert_eq!(outcome.refused.len(), 1);
        assert_eq!(outcome.remaining.len(), 1);
        assert!(
            outcome.remaining[0]
                .message
                .ends_with("(fix refused: would change data)")
        );
    }

    #[test]
    fn fix_without_front_matter_changes_nothing() {
        let outcome = linter().fix("hello \n").unwrap();
        assert_eq!(outcome.output, "hello \n");
        assert_eq!(outcome.applied, 0);
        assert_eq!(outcome.remaining.len(), 1);
    }

    #[test]
    fn fix_unparseable() {
        let Err(FixError::Unparseable(d)) = linter().fix("---\na: [1\n---\n") else {
            panic!("expected Unparseable");
        };
        assert_eq!(d[0].rule, RuleId::YamlSyntax);
    }

    #[test]
    fn suggestion_not_applied() {
        let mut automatic = candidate(Applicability::Automatic, vec![edit(0, 1)]);
        let mut suggestion = candidate(Applicability::Suggestion, vec![edit(2, 3)]);
        let mut refused = candidate(Applicability::Automatic, vec![edit(4, 5)]);
        let mut no_fix = candidate(Applicability::Automatic, vec![edit(6, 7)]);
        refused.0.rule = RuleId::NoTrailingCommas;
        automatic.0.fix = Some(automatic.1.clone());
        suggestion.0.fix = Some(suggestion.1.clone());
        refused.0.fix = Some(refused.1.clone());
        no_fix.0.fix = None;
        let diagnostics = [automatic.0, suggestion.0, refused.0, no_fix.0];
        let got = candidates(&diagnostics, &[(RuleId::NoTrailingCommas, range(4, 5))]);
        assert_eq!(starts(&got), [(0, 1)]);
    }

    #[test]
    fn select_skips_overlaps() {
        let fixes = [(0, 2), (1, 3), (3, 3), (3, 4)]
            .map(|(s, e)| candidate(Applicability::Automatic, vec![edit(s, e)]))
            .to_vec();
        let (chosen, skipped) = select(fixes);
        assert_eq!(starts(&chosen), [(0, 2), (3, 3)]);
        assert_eq!(starts(&skipped), [(1, 3), (3, 4)]);
    }

    #[test]
    fn select_checks_every_edit_of_a_fix() {
        let two = candidate(Applicability::Automatic, vec![edit(0, 1), edit(10, 12)]);
        let middle = candidate(Applicability::Automatic, vec![edit(5, 6)]);
        let clash = candidate(Applicability::Automatic, vec![edit(4, 5), edit(11, 13)]);
        let (chosen, skipped) = select(vec![two, middle, clash]);
        assert_eq!(starts(&chosen), [(0, 1), (5, 6)]);
        assert_eq!(starts(&skipped), [(4, 5)]);
    }

    #[test]
    fn shift_moves_or_drops_refused_ranges() {
        let rule = RuleId::NoTrailingCommas;
        let delete = |start, end| edit(start, end);
        let insert = |at| TextEdit {
            range: range(at, at),
            new_text: "ab".to_string(),
        };
        let edits = [delete(2, 5), insert(8), delete(12, 14)];
        let ranges = [
            (rule, range(0, 1)),
            (rule, range(6, 7)),
            (rule, range(9, 10)),
            (rule, range(4, 6)),
            (rule, range(15, 16)),
        ];
        // Kept; moved by the first edit; moved by the first two; overlapped by an edit; moved by all.
        let want = [(0, 1), (3, 4), (8, 9), (12, 13)].map(|(s, e)| (rule, range(s, e)));
        assert_eq!(shift(ranges.to_vec(), &edits), want);
    }

    #[test]
    fn verify_cases() {
        let input = "---\na: 1 \nb: c,\n---\nx \n";
        let comma = DataEffect::ChangesScalar {
            path: vec![crate::PathStep::Value(1)],
            new_plain: "c".to_string(),
        };
        let ok = "---\na: 1\nb: c\n---\nx \n";
        assert!(verify(input, ok, std::slice::from_ref(&comma)));
        // Data changed without an effect saying so.
        assert!(!verify(input, ok, &[]));
        // Body changed.
        assert!(!verify(
            input,
            "---\na: 1\nb: c\n---\nx\n",
            std::slice::from_ref(&comma)
        ));
        // Opening line ending changed.
        assert!(!verify(
            input,
            "---\r\na: 1\nb: c\n---\nx \n",
            std::slice::from_ref(&comma)
        ));
        // BOM changed.
        assert!(!verify(
            input,
            "\u{FEFF}---\na: 1\nb: c\n---\nx \n",
            std::slice::from_ref(&comma)
        ));
        // Output no longer parses.
        assert!(!verify(input, "---\na: [1\n---\nx \n", &[]));
        // An effect whose path does not exist.
        let missing = DataEffect::ChangesScalar {
            path: vec![crate::PathStep::Value(9)],
            new_plain: "c".to_string(),
        };
        assert!(!verify(input, ok, &[missing]));
    }
}
