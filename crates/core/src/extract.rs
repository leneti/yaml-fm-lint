//! Finding the front matter in a source file.

use crate::TextRange;

/// The most content bytes allowed between the opening and closing delimiter.
pub(crate) const MAX_FRONT_MATTER_BYTES: usize = 1_048_576;

/// A front-matter block; all ranges are byte ranges into the whole source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FrontMatter {
    /// The source starts with a UTF-8 BOM.
    pub bom: bool,
    /// The opening delimiter line, without its line ending.
    pub open: TextRange,
    /// From just after the opening line's ending to the start of the closing line.
    pub content: TextRange,
    /// The closing delimiter line, without its line ending.
    pub close: TextRange,
    /// Just after the closing line's ending, or the end of the source if it has none.
    pub end: usize,
}

/// What `extract` found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Extraction {
    /// The first line is not a delimiter.
    Missing,
    /// The opening delimiter has no closing delimiter.
    Unclosed { open: TextRange },
    /// No closing delimiter within `MAX_FRONT_MATTER_BYTES` of content.
    TooLarge { open: TextRange },
    /// A complete front-matter block.
    Found(FrontMatter),
}

const BOM: char = '\u{FEFF}';

/// Where the first line starts: after a BOM, if there is one.
pub(crate) fn start_offset(source: &str) -> usize {
    if source.starts_with(BOM) {
        BOM.len_utf8()
    } else {
        0
    }
}

/// One line of the source.
struct Line<'a> {
    /// The content without its line ending.
    text: &'a str,
    start: usize,
    /// The start of the next line, or the end of the source.
    next: usize,
}

impl Line<'_> {
    fn range(&self) -> TextRange {
        TextRange {
            start: self.start,
            end: self.start + self.text.len(),
        }
    }

    /// A line matching `^---[ \t]*$`.
    fn is_delimiter(&self) -> bool {
        self.text
            .strip_prefix("---")
            .is_some_and(|rest| rest.bytes().all(|b| b == b' ' || b == b'\t'))
    }
}

/// The line starting at `start`, or `None` at the end of the source.
fn line_at(source: &str, start: usize) -> Option<Line<'_>> {
    let rest = source.get(start..).filter(|rest| !rest.is_empty())?;
    Some(match rest.find('\n') {
        Some(i) => {
            let line = rest.get(..i)?;
            Line {
                text: line.strip_suffix('\r').unwrap_or(line),
                start,
                next: start + i + 1,
            }
        }
        None => Line {
            text: rest,
            start,
            next: source.len(),
        },
    })
}

/// Finds the front matter of `source` (§5.2).
pub(crate) fn extract(source: &str) -> Extraction {
    let Some(first) = line_at(source, start_offset(source)).filter(Line::is_delimiter) else {
        return Extraction::Missing;
    };
    let open = first.range();
    let content_start = first.next;
    let mut next = content_start;
    while let Some(line) = line_at(source, next) {
        if line.start - content_start > MAX_FRONT_MATTER_BYTES {
            break;
        }
        if line.is_delimiter() {
            return Extraction::Found(FrontMatter {
                bom: source.starts_with(BOM),
                open,
                content: TextRange {
                    start: content_start,
                    end: line.start,
                },
                close: line.range(),
                end: line.next,
            });
        }
        next = line.next;
    }
    // Either the loop broke on a line past the limit, or it ran off the end of the source.
    if source.len() - content_start > MAX_FRONT_MATTER_BYTES {
        Extraction::TooLarge { open }
    } else {
        Extraction::Unclosed { open }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::show;

    fn describe(source: &str) -> String {
        match extract(source) {
            Extraction::Missing => "missing".to_string(),
            Extraction::Unclosed { open } => format!("unclosed {}", show(source, open)),
            Extraction::TooLarge { open } => format!("too large {}", show(source, open)),
            Extraction::Found(fm) => format!(
                "found bom={} open={} content={:?} close={} end={}",
                fm.bom,
                show(source, fm.open),
                source.get(fm.content.start..fm.content.end).unwrap_or("?"),
                show(source, fm.close),
                fm.end,
            ),
        }
    }

    #[rstest]
    #[case::simple(
        "---\na: 1\n---\nbody",
        "found bom=false open=1:1-1:4 content=\"a: 1\\n\" close=3:1-3:4 end=13"
    )]
    #[case::bom(
        "\u{FEFF}---\n---\n",
        "found bom=true open=1:2-1:5 content=\"\" close=2:1-2:4 end=11"
    )]
    #[case::delimiter_trailing_blanks(
        "--- \t\na: 1\n---  \n",
        "found bom=false open=1:1-1:6 content=\"a: 1\\n\" close=3:1-3:6 end=17"
    )]
    #[case::crlf(
        "---\r\na: 1\r\n---\r\nb",
        "found bom=false open=1:1-1:4 content=\"a: 1\\r\\n\" close=3:1-3:4 end=16"
    )]
    #[case::no_final_newline(
        "---\n---",
        "found bom=false open=1:1-1:4 content=\"\" close=2:1-2:4 end=7"
    )]
    #[case::not_a_delimiter("---x\n", "missing")]
    #[case::blank_first_line("\n---\n---\n", "missing")]
    #[case::empty("", "missing")]
    #[case::unclosed("---\na: 1\n", "unclosed 1:1-1:4")]
    #[case::dots_do_not_close("---\na\n...\n", "unclosed 1:1-1:4")]
    fn extract_cases(#[case] source: &str, #[case] expected: &str) {
        assert_eq!(describe(source), expected);
    }

    #[test]
    fn too_large() {
        let source = format!("---\n{}", "a: 1\n".repeat(210_000));
        assert_eq!(describe(&source), "too large 1:1-1:4");
    }

    #[test]
    fn size_limit_is_inclusive() {
        let at_limit = format!("---\n{}\n---\n", "x".repeat(MAX_FRONT_MATTER_BYTES - 1));
        assert!(describe(&at_limit).starts_with("found"));
        let over_limit = format!("---\n{}\n---\n", "x".repeat(MAX_FRONT_MATTER_BYTES));
        assert_eq!(describe(&over_limit), "too large 1:1-1:4");
    }
}
