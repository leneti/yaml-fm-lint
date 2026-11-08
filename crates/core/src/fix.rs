//! The fix engine: applies automatic fixes, and keeps only those that provably leave the
//! front-matter data unchanged (spec §5.6).

use std::collections::BTreeSet;
use std::ops::Bound;

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

/// The most verifications (each parses the output) one `fix` call may run.
pub(crate) const MAX_FIX_VERIFIES: usize = 64;

/// Appended to the message of a finding whose fix was refused.
const REFUSED_NOTE: &str = " (fix refused: would change data)";

/// Appended to the message of a finding whose fix was never verified: the budget ran out.
const SKIPPED_NOTE: &str = " (fix skipped: too many refused fixes)";

/// A diagnostic and its automatic fix.
type Candidate = (Diagnostic, Fix);

/// The diagnostics with an `Automatic` fix, minus those whose `(rule, range)` was refused.
pub(crate) fn candidates(
    diagnostics: &[Diagnostic],
    refused: &BTreeSet<(RuleId, TextRange)>,
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
/// overlaps an edit already taken, or if its own edits overlap each other.
pub(crate) fn select(mut fixes: Vec<Candidate>) -> (Vec<Candidate>, Vec<Candidate>) {
    fixes.sort_by_key(|(_, fix)| fix.edits.first().map(|e| e.range.start));
    // The `(start, end)` of the edits taken. They don't overlap each other (a zero-length
    // insert may share its start with the edit after it, within one fix), so a range can
    // only clash with its neighbours in this order.
    let mut taken: BTreeSet<(usize, usize)> = BTreeSet::new();
    let clashes = |taken: &BTreeSet<(usize, usize)>, r: TextRange| {
        let same_start = taken.range((r.start, 0)..=(r.start, usize::MAX)).next();
        let before = taken.range(..(r.start, 0)).next_back();
        let after_start = (Bound::Excluded((r.start, usize::MAX)), Bound::Unbounded);
        let after = taken.range(after_start).next();
        [same_start, before, after]
            .into_iter()
            .flatten()
            .any(|&(start, end)| overlaps(TextRange { start, end }, r))
    };
    let (mut chosen, mut skipped) = (Vec::new(), Vec::new());
    for candidate in fixes {
        let edits = &candidate.1.edits;
        let own_edits_overlap = edits
            .windows(2)
            .any(|w| matches!(w, [a, b] if b.range.start < a.range.end));
        if own_edits_overlap || edits.iter().any(|e| clashes(&taken, e.range)) {
            skipped.push(candidate);
        } else {
            taken.extend(edits.iter().map(|e| (e.range.start, e.range.end)));
            chosen.push(candidate);
        }
    }
    (chosen, skipped)
}

/// An input text, parsed once, to check outputs against.
struct Baseline<'a> {
    text: &'a str,
    fm: FrontMatter,
    data: Data,
}

impl<'a> Baseline<'a> {
    /// `None` if `text` has no complete front matter or it does not parse.
    fn new(text: &'a str) -> Option<Baseline<'a>> {
        let Extraction::Found(fm) = extract(text) else {
            return None;
        };
        let data = build(&parse(text, &fm).ok()?).ok()?;
        Some(Baseline { text, fm, data })
    }

    /// Whether `output` is the text with the front matter changed only as `effects` say.
    ///
    /// Everything outside the front matter's lines must be byte-identical: the BOM, the
    /// opening delimiter's line ending, and everything after the closing delimiter's text
    /// (its line ending and the body). The data must be the same, apart from the effects.
    fn check<'e>(&self, output: &str, effects: impl Iterator<Item = &'e DataEffect>) -> bool {
        let Extraction::Found(after) = extract(output) else {
            return false;
        };
        let before = &self.fm;
        let same = |a: Option<&str>, b: Option<&str>| a.is_some() && a == b;
        if before.bom != after.bom
            || !same(
                self.text.get(before.open.end..before.content.start),
                output.get(after.open.end..after.content.start),
            )
            || !same(
                self.text.get(before.close.end..),
                output.get(after.close.end..),
            )
        {
            return false;
        }
        let mut expected = self.data.clone();
        for effect in effects {
            if let DataEffect::ChangesScalar { path, new_plain } = effect {
                let Some(slot) = lookup_mut(&mut expected, path) else {
                    return false;
                };
                *slot = resolve_plain(new_plain);
            }
        }
        parse(output, &after)
            .ok()
            .and_then(|root| build(&root).ok())
            .is_some_and(|data| data_eq(&data, &expected))
    }
}

/// Whether `output` is `input` with the front matter changed only as `effects` say.
#[cfg_attr(not(test), allow(dead_code))] // the engine uses `Baseline` to parse `input` once
pub(crate) fn verify(input: &str, output: &str, effects: &[DataEffect]) -> bool {
    Baseline::new(input).is_some_and(|base| base.check(output, effects.iter()))
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
    /// The verification budget ran out before every fix was decided.
    exhausted: bool,
}

/// The result of checking some fixes on top of those already accepted.
enum Attempt {
    Verified {
        output: String,
        edits: Vec<TextEdit>,
    },
    Failed,
    OutOfBudget,
}

/// Verifies the accepted edits and effects plus the fixes `next`; costs one verification.
fn attempt(
    base: &Baseline<'_>,
    accepted: (&[TextEdit], &[&DataEffect]),
    next: &[Candidate],
    budget: &mut usize,
) -> Attempt {
    let Some(left) = budget.checked_sub(1) else {
        return Attempt::OutOfBudget;
    };
    *budget = left;
    let mut edits = accepted.0.to_vec();
    edits.extend(next.iter().flat_map(|(_, fix)| fix.edits.iter().cloned()));
    edits.sort_by_key(|e| (e.range.start, e.range.end));
    let effects = (accepted.1.iter().copied()).chain(next.iter().map(|(_, fix)| &fix.data_effect));
    match apply_edits(base.text, &edits).filter(|output| base.check(output, effects)) {
        Some(output) => Attempt::Verified { output, edits },
        None => Attempt::Failed,
    }
}

/// Applies `chosen` to `text` if the result verifies. Otherwise it accepts the longest
/// prefix that verifies (found by bisection), refuses the fix after it, and goes on with
/// the rest, so the pass always makes progress without verifying every fix on its own.
fn run_pass(text: &str, chosen: &[Candidate], budget: &mut usize) -> Pass {
    let mut pass = Pass {
        output: text.to_string(),
        edits: Vec::new(),
        applied: 0,
        refused: Vec::new(),
        exhausted: false,
    };
    // `text` is the input of the previous pass or the original, which both parsed.
    let Some(base) = Baseline::new(text) else {
        pass.exhausted = true;
        return pass;
    };
    let mut effects: Vec<&DataEffect> = Vec::new();
    let mut start = 0;
    while let Some(rest) = chosen.get(start..).filter(|rest| !rest.is_empty()) {
        // Invariant: the first `lo` fixes of `rest` verify, the first `hi` don't (or hi = len).
        let (mut lo, mut hi) = (0, rest.len());
        let mut best = None;
        let mut k = hi;
        loop {
            let next = rest.get(..k).unwrap_or_default();
            match attempt(&base, (&pass.edits, &effects), next, budget) {
                Attempt::OutOfBudget => {
                    pass.exhausted = true;
                    break;
                }
                Attempt::Verified { output, edits } => {
                    lo = k;
                    best = Some((output, edits));
                }
                Attempt::Failed => hi = k,
            }
            if hi - lo <= 1 {
                break;
            }
            k = lo + (hi - lo) / 2;
        }
        if let Some((output, edits)) = best {
            pass.output = output;
            pass.edits = edits;
            pass.applied += lo;
            effects.extend(
                rest.get(..lo)
                    .unwrap_or_default()
                    .iter()
                    .map(|(_, f)| &f.data_effect),
            );
        }
        if pass.exhausted {
            break;
        }
        if let Some((diagnostic, _)) = rest.get(lo).filter(|_| lo < hi) {
            pass.refused.push(diagnostic.clone());
        }
        start += lo + 1;
    }
    pass
}

/// Carries ranges through `edits` (sorted, non-overlapping): a range an edit overlaps is
/// dropped, the others move by the edits before them.
fn shift(
    ranges: &BTreeSet<(RuleId, TextRange)>,
    edits: &[TextEdit],
) -> BTreeSet<(RuleId, TextRange)> {
    let mut by_start: Vec<_> = ranges.iter().collect();
    by_start.sort_by_key(|(_, range)| *range);
    let mut kept = BTreeSet::new();
    // Both lists go by start: `done` edits end at or before the range, the rest start later.
    let (mut done, mut added, mut removed) = (0, 0, 0);
    for &(rule, range) in by_start {
        while let Some(edit) = edits.get(done).filter(|e| e.range.end <= range.start) {
            added += edit.new_text.len();
            removed += edit.range.end - edit.range.start;
            done += 1;
        }
        let touches_before = done
            .checked_sub(1)
            .and_then(|i| edits.get(i))
            .is_some_and(|e| e.range.start == range.start);
        let touches_after = edits.get(done).is_some_and(|e| overlaps(e.range, range));
        if touches_before || touches_after {
            continue;
        }
        let moved = |offset: usize| offset.checked_add(added)?.checked_sub(removed);
        if let (Some(start), Some(end)) = (moved(range.start), moved(range.end)) {
            kept.insert((rule, TextRange { start, end }));
        }
    }
    kept
}

impl Linter {
    /// Fixes what can be fixed in `source`.
    ///
    /// Only `Automatic` fixes are applied, in passes of non-overlapping fixes. A fix whose
    /// result would change the data (beyond what the fix declares) or touch anything outside
    /// the front matter is refused and reported in [`FixOutcome::refused`]. At most
    /// [`MAX_FIX_VERIFIES`] verifications run; if many fixes are refused the budget runs
    /// out, and the fixes not yet verified are left in `remaining` with a note.
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
        let mut refused_ranges = BTreeSet::new();
        let mut budget = MAX_FIX_VERIFIES;
        let mut exhausted = false;
        for _ in 0..MAX_FIX_PASSES {
            let (chosen, _) = select(candidates(&report.diagnostics, &refused_ranges));
            if chosen.is_empty() {
                break;
            }
            let pass = run_pass(&text, &chosen, &mut budget);
            applied += pass.applied;
            refused_ranges.extend(pass.refused.iter().map(|d| (d.rule, d.range)));
            if !pass.edits.is_empty() {
                refused_ranges = shift(&refused_ranges, &pass.edits);
                text = pass.output;
                report = self.lint(&text);
            }
            if pass.exhausted {
                exhausted = true;
                break;
            }
        }
        let unverified: BTreeSet<_> = if exhausted {
            let fixable = candidates(&report.diagnostics, &refused_ranges);
            fixable.iter().map(|(d, _)| (d.rule, d.range)).collect()
        } else {
            BTreeSet::new()
        };
        let remaining: Vec<Diagnostic> = report
            .diagnostics
            .into_iter()
            .map(|mut d| {
                let key = (d.rule, d.range);
                if refused_ranges.contains(&key) {
                    d.message.push_str(REFUSED_NOTE);
                } else if unverified.contains(&key) {
                    d.message.push_str(SKIPPED_NOTE);
                }
                d
            })
            .collect();
        // Only refusals that still stand in the final output; one a later pass resolved is gone.
        let refused = (remaining.iter())
            .filter(|d| d.message.ends_with(REFUSED_NOTE))
            .cloned()
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
    fn fix_does_not_report_a_refusal_a_later_pass_resolved() {
        // The flow-mapping fix conflicts with the indentation fix in the first pass and is
        // refused there, then applied in the second pass.
        let source = "---\nreferences:\n             - { title : \"Branching Strategies\", url: \"/\" }\n---\nBody\n";
        let outcome = linter().fix(source).unwrap();
        assert_eq!(
            outcome.output,
            "---\nreferences:\n  - title: Branching Strategies\n    url: /\n---\nBody\n"
        );
        assert_eq!(outcome.applied, 5);
        assert!(outcome.refused.is_empty());
        assert!(outcome.remaining.is_empty());
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

    /// `lines` trailing-space lines after `bad` aliased trailing-comma pairs.
    fn many_fixes(bad: usize, lines: usize) -> String {
        let pairs = (0..bad).map(|i| format!("a{i}: &a{i} x,\nb{i}: *a{i}\n"));
        let spaces = (0..lines).map(|i| format!("k{i}: 1 \n"));
        format!(
            "---\n{}{}---\nBody \n",
            pairs.collect::<String>(),
            spaces.collect::<String>()
        )
    }

    #[test]
    fn one_refusal_among_many_fixes_is_fast() {
        let source = many_fixes(1, 2000);
        let started = std::time::Instant::now();
        let outcome = linter().fix(&source).unwrap();
        assert!(
            started.elapsed().as_secs() < 2,
            "took {:?}",
            started.elapsed()
        );
        assert_eq!(outcome.applied, 2000);
        assert_eq!(outcome.refused.len(), 1);
        assert_eq!(outcome.remaining.len(), 1);
        assert!(outcome.remaining[0].message.ends_with(REFUSED_NOTE));
        assert!(!outcome.output.contains("1 \n"));
        assert!(
            outcome
                .output
                .starts_with("---\na0: &a0 x,\nb0: *a0\nk0: 1\n")
        );
    }

    #[test]
    fn exhausted_budget_leaves_a_note_and_safe_output() {
        let source = many_fixes(100, 50);
        let started = std::time::Instant::now();
        let outcome = linter().fix(&source).unwrap();
        assert!(
            started.elapsed().as_secs() < 2,
            "took {:?}",
            started.elapsed()
        );
        // The data is unchanged, and so is everything outside the front matter.
        assert!(verify(&source, &outcome.output, &[]));
        let refused = outcome
            .remaining
            .iter()
            .filter(|d| d.message.ends_with(REFUSED_NOTE))
            .count();
        let skipped = outcome
            .remaining
            .iter()
            .filter(|d| d.message.ends_with(SKIPPED_NOTE))
            .count();
        assert_eq!(refused, outcome.refused.len());
        assert!(refused > 0 && skipped > 0);
        // Every fixable problem is accounted for: applied, refused or skipped.
        assert_eq!(refused + skipped + outcome.applied, 150);
        assert_eq!(outcome.remaining.len(), refused + skipped);
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
        let refused = BTreeSet::from([(RuleId::NoTrailingCommas, range(4, 5))]);
        let got = candidates(&diagnostics, &refused);
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
    fn select_handles_edits_sharing_a_start() {
        // A fix may insert at a position and replace from it; later fixes still see both.
        let insert = TextEdit {
            range: range(5, 5),
            new_text: "x".to_string(),
        };
        let both = candidate(Applicability::Automatic, vec![insert, edit(5, 7)]);
        let inside = candidate(Applicability::Automatic, vec![edit(6, 8)]);
        let after = candidate(Applicability::Automatic, vec![edit(7, 9)]);
        // Its own edits overlap, so it can't be applied at all.
        let malformed = candidate(Applicability::Automatic, vec![edit(20, 24), edit(22, 26)]);
        let (chosen, skipped) = select(vec![both, inside, after, malformed]);
        assert_eq!(starts(&chosen), [(5, 5), (7, 9)]);
        assert_eq!(starts(&skipped), [(6, 8), (20, 24)]);
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
        assert_eq!(shift(&BTreeSet::from(ranges), &edits), BTreeSet::from(want));
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
