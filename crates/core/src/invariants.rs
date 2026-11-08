//! Checks of the guarantees `fix` makes (spec §12), for the property tests and the fuzz target.

use crate::data::{Data, build, data_eq, resolve_plain};
use crate::extract::{Extraction, FrontMatter, extract};
use crate::parse::parse;
use crate::{Applicability, FixError, Linter, RuleId};

/// The refusal notes `fix` appends to the message of a finding it left alone.
const NOTES: [&str; 2] = [
    " (fix refused: would change data)",
    " (fix skipped: too many refused fixes)",
];

/// The front matter of `text` and its data, if it has a complete block that parses.
fn parsed(text: &str) -> Option<(FrontMatter, Data)> {
    let Extraction::Found(fm) = extract(text) else {
        return None;
    };
    let data = build(&parse(text, &fm).ok()?).ok()?;
    Some((fm, data))
}

/// Whether `output` is `input` with, at most, trailing commas dropped from string leaves.
///
/// This is an oracle independent of the effects `fix` declares: a string that ends with a
/// comma may come out as the plain scalar of its text without the trailing commas and spaces.
fn same_data(input: &Data, output: &Data) -> bool {
    match (input, output) {
        (Data::Seq(a), Data::Seq(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same_data(a, b))
        }
        (Data::Map(a), Data::Map(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|((ak, av), (bk, bv))| data_eq(ak, bk) && same_data(av, bv))
        }
        (Data::Str(text), _) if text.ends_with(',') => {
            let kept = text.trim_end_matches(|c: char| c == ',' || c.is_whitespace());
            data_eq(input, output) || data_eq(output, &resolve_plain(kept))
        }
        _ => data_eq(input, output),
    }
}

/// Runs `linter.fix(input)` and checks what it promises (spec §12). It never panics.
///
/// `fix` either fails with `Unparseable`, or:
/// 1. the output has no `yaml-syntax` or `input-limit` finding, and its data equals the
///    input's, except that a string leaf ending in a comma may have lost it;
/// 2. fixing the output applies nothing;
/// 3. no `Automatic` fix is left in `remaining` unless it carries a refusal note;
/// 4. the BOM, the delimiter line endings and everything after the front matter are
///    byte-identical (an input without complete front matter comes back unchanged).
///
/// # Errors
///
/// Describes the first guarantee that does not hold.
pub fn check_fix(linter: &Linter, input: &str) -> Result<(), String> {
    let outcome = match linter.fix(input) {
        Ok(outcome) => outcome,
        Err(FixError::Unparseable(_)) => return Ok(()),
    };
    let output = outcome.output.as_str();

    if let Some(d) = (linter.lint(output).diagnostics.iter())
        .find(|d| matches!(d.rule, RuleId::YamlSyntax | RuleId::InputLimit))
    {
        return Err(format!(
            "the output has a {} error: {}\noutput: {output:?}",
            d.rule, d.message
        ));
    }

    match parsed(input) {
        None if output != input => {
            return Err(format!(
                "the input has no parseable front matter but changed\noutput: {output:?}"
            ));
        }
        None => {}
        Some((before, data)) => {
            let Some((after, fixed)) = parsed(output) else {
                return Err(format!("the output does not parse\noutput: {output:?}"));
            };
            if !same_data(&data, &fixed) {
                return Err(format!(
                    "the data changed\nbefore: {data:?}\nafter: {fixed:?}\noutput: {output:?}"
                ));
            }
            let same = |a: Option<&str>, b: Option<&str>| a.is_some() && a == b;
            if before.bom != after.bom
                || !same(
                    input.get(before.open.end..before.content.start),
                    output.get(after.open.end..after.content.start),
                )
                || !same(input.get(before.close.end..), output.get(after.close.end..))
            {
                return Err(format!(
                    "the BOM, a delimiter line ending or the body changed\noutput: {output:?}"
                ));
            }
        }
    }

    match linter.fix(output) {
        Ok(again) if again.applied == 0 => {}
        Ok(again) => {
            return Err(format!(
                "not idempotent: fixing the output applied {} more\noutput: {output:?}\nthen: {:?}",
                again.applied, again.output
            ));
        }
        Err(_) => {
            return Err(format!(
                "the output is unparseable by fix\noutput: {output:?}"
            ));
        }
    }

    if let Some(d) = outcome.remaining.iter().find(|d| {
        let automatic = d
            .fix
            .as_ref()
            .is_some_and(|f| f.applicability == Applicability::Automatic);
        automatic && !NOTES.iter().any(|note| d.message.ends_with(note))
    }) {
        return Err(format!(
            "an automatic fix remains: {} {}\noutput: {output:?}",
            d.rule, d.message
        ));
    }
    Ok(())
}
