//! Parsing front matter into a [`Node`] tree with byte ranges.
//!
//! Saphyr reports character indexes, and some of its ends and empty-value positions are
//! unreliable (quoted scalars run on over trailing spaces and comments, block-scalar and
//! collection starts are imprecise, null values sit at the next token). The tree is built
//! from its events with an explicit stack and completed from the source text where needed.

use std::collections::HashSet;

use saphyr_parser::{Event, Parser, ScalarStyle, Span, Tag};

use crate::extract::FrontMatter;
use crate::rules::{input_limit, yaml_syntax};
use crate::tree::{Entry, Node, NodeKind, Style};
use crate::{RuleId, TextRange};

/// The deepest allowed nesting of collections; the root mapping is level 1.
pub(crate) const MAX_DEPTH: usize = 128;

/// A problem that stops linting: a YAML syntax error or an exceeded limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyntaxIssue {
    /// `YamlSyntax` or `InputLimit`.
    pub rule: RuleId,
    /// Where the problem is.
    pub range: TextRange,
    /// What is wrong.
    pub message: String,
}

/// Parses `source[fm.content]` into a tree.
pub(crate) fn parse(source: &str, fm: &FrontMatter) -> Result<Node, SyntaxIssue> {
    let (lo, hi) = (fm.content.start, fm.content.end);
    let content = source.get(lo..hi).unwrap_or("");
    // Saphyr counts characters; this maps a character index to a byte offset in `source`.
    let mut chars: Vec<usize> = content.char_indices().map(|(i, _)| lo + i).collect();
    chars.push(hi);
    let mut builder = Builder {
        src: source,
        lo,
        hi,
        chars,
        frames: Vec::new(),
        root: None,
    };
    let mut documents = 0;
    for event in Parser::new_from_str(content) {
        let (event, span) = event.map_err(|error| {
            let mut message = error.info().to_string();
            if let Some(first) = message.get_mut(..1) {
                first.make_ascii_lowercase();
            }
            builder.issue(RuleId::YamlSyntax, error.marker().index(), message)
        })?;
        match event {
            Event::DocumentStart(_) => {
                documents += 1;
                if documents > 1 {
                    let message = yaml_syntax::SECOND_DOCUMENT.to_string();
                    return Err(builder.issue(RuleId::YamlSyntax, span.start.index(), message));
                }
            }
            Event::Alias(anchor) => builder.alias(anchor, span)?,
            Event::Scalar(value, style, anchor, tag) => {
                builder.scalar(
                    value.into_owned(),
                    style,
                    props(anchor, tag.as_deref()),
                    span,
                )?;
            }
            Event::SequenceStart(anchor, tag) => {
                builder.open(false, props(anchor, tag.as_deref()), span)?;
            }
            Event::MappingStart(anchor, tag) => {
                builder.open(true, props(anchor, tag.as_deref()), span)?;
            }
            Event::SequenceEnd | Event::MappingEnd => builder.close(span)?,
            Event::Nothing | Event::StreamStart | Event::StreamEnd | Event::DocumentEnd => {}
        }
    }
    let Some(root) = builder.root else {
        // Empty or comment-only content.
        return Ok(Node {
            kind: NodeKind::Mapping {
                flow: false,
                entries: Vec::new(),
            },
            range: TextRange { start: lo, end: lo },
            anchor: None,
            tagged_str: false,
            has_tag: false,
        });
    };
    if matches!(root.kind, NodeKind::Mapping { .. }) {
        Ok(root)
    } else {
        Err(SyntaxIssue {
            rule: RuleId::YamlSyntax,
            range: root.range,
            message: yaml_syntax::NOT_A_MAPPING.to_string(),
        })
    }
}

/// The anchor and tag of a node.
#[derive(Clone, Copy)]
struct Props {
    anchor: Option<usize>,
    has_tag: bool,
    tagged_str: bool,
}

impl Props {
    fn any(self) -> bool {
        self.anchor.is_some() || self.has_tag
    }
}

fn props(anchor: usize, tag: Option<&Tag>) -> Props {
    Props {
        anchor: (anchor != 0).then_some(anchor),
        has_tag: tag.is_some(),
        tagged_str: tag.is_some_and(|t| t.handle == "tag:yaml.org,2002:" && t.suffix == "str"),
    }
}

/// A finished node, and where its text starts, including anchor and tag.
struct Built {
    node: Node,
    raw: usize,
}

/// A mapping key waiting for its value.
struct Pending {
    key: Node,
    colon: Option<usize>,
    explicit: bool,
}

enum Body {
    Sequence {
        items: Vec<Node>,
        /// The first `-`.
        first: Option<usize>,
    },
    Mapping {
        entries: Vec<Entry>,
        key: Option<Pending>,
        /// The first key, or its `?`.
        first: Option<usize>,
        /// Keys seen so far: (is a scalar, its value or source text).
        seen: HashSet<(bool, String)>,
    },
}

/// A collection that is still open.
struct Frame {
    props: Props,
    /// In flow context: `[...]`, `{...}`, or inside either.
    flow: bool,
    /// The `[` or `{`, if there is one.
    bracket: Option<usize>,
    /// Where the start event is (at the anchor or tag, if any).
    hint: usize,
    body: Body,
}

struct Builder<'a> {
    src: &'a str,
    /// The front matter content is `src[lo..hi]`.
    lo: usize,
    hi: usize,
    /// Byte offset of each character of the content, then `hi`.
    chars: Vec<usize>,
    frames: Vec<Frame>,
    root: Option<Node>,
}

impl Builder<'_> {
    fn byte(&self, char_index: usize) -> usize {
        self.chars.get(char_index).copied().unwrap_or(self.hi)
    }

    fn text(&self, start: usize, end: usize) -> &str {
        self.src.get(start..end).unwrap_or("")
    }

    fn char_at(&self, offset: usize) -> Option<char> {
        self.src.get(offset..self.hi)?.chars().next()
    }

    /// The start of the line that contains `offset` (or ends there).
    fn line_start(&self, offset: usize) -> usize {
        self.text(self.lo, offset)
            .rfind('\n')
            .map_or(self.lo, |i| self.lo + i + 1)
    }

    /// An issue at the empty range at character index `char_index`.
    fn issue(&self, rule: RuleId, char_index: usize, message: String) -> SyntaxIssue {
        let at = self.byte(char_index);
        SyntaxIssue {
            rule,
            range: TextRange { start: at, end: at },
            message,
        }
    }

    // Source-text helpers.

    /// YAML whitespace and line breaks; NBSP, U+2028 and U+0085 are content.
    fn is_blank(c: char) -> bool {
        matches!(c, ' ' | '\t' | '\r' | '\n')
    }

    /// After whitespace, line ends and comments.
    fn skip_blank(&self, mut pos: usize) -> usize {
        while let Some(c) = self.char_at(pos) {
            if c == '#' {
                pos = self
                    .text(pos, self.hi)
                    .find('\n')
                    .map_or(self.hi, |i| pos + i);
            } else if Self::is_blank(c) {
                pos += c.len_utf8();
            } else {
                break;
            }
        }
        pos
    }

    /// After the anchor and tag that start at `pos`, and what follows them.
    fn skip_props(&self, mut pos: usize) -> usize {
        while matches!(self.char_at(pos), Some('&' | '!')) {
            let token = self.text(pos, self.hi);
            pos += token.find(Self::is_blank).unwrap_or(token.len());
            pos = self.skip_blank(pos);
        }
        pos
    }

    /// The last character before `pos` that is not whitespace or inside a comment.
    ///
    /// Unless `after_comment`, `pos` is known to be the start of a node, so no comment can
    /// precede it on its own line, and a hit there costs no more than the blanks before it
    /// (this keeps very long lines linear).
    fn last_significant(&self, pos: usize, after_comment: bool) -> Option<usize> {
        let mut end = pos;
        if !after_comment {
            let head = self.text(self.lo, pos).trim_end_matches([' ', '\t', '\r']);
            let c = head.chars().next_back()?;
            if c != '\n' {
                return Some(self.lo + head.len() - c.len_utf8());
            }
            end = self.lo + head.len() - 1;
        }
        loop {
            let line_start = self.line_start(end);
            let line = self.text(line_start, end);
            let comment = line
                .char_indices()
                .find(|&(i, c)| {
                    c == '#'
                        && line
                            .get(..i)
                            .is_some_and(|b| b.is_empty() || b.ends_with([' ', '\t']))
                })
                .map_or(line.len(), |(i, _)| i);
            let code = line.get(..comment).unwrap_or("").trim_end();
            if let Some(c) = code.chars().next_back() {
                return Some(line_start + code.len() - c.len_utf8());
            }
            if line_start == self.lo {
                return None;
            }
            end = line_start - 1;
        }
    }

    /// The `-` indicator just before `pos`.
    fn dash_before(&self, pos: usize, after_comment: bool) -> Option<usize> {
        self.last_significant(pos, after_comment)
            .filter(|&i| self.char_at(i) == Some('-'))
    }

    /// The `?` of an explicit key that starts at `key_start`.
    fn explicit_at(&self, key_start: usize) -> Option<usize> {
        let q = self
            .last_significant(key_start, false)
            .filter(|&i| self.char_at(i) == Some('?'))?;
        if !Self::is_blank(self.char_at(q + 1)?) {
            return None;
        }
        // Not the end of a plain scalar such as `what?`: a `?` indicator starts its line or
        // follows `-`, `{`, `[` or `,`.
        let before = self.text(self.lo, q).trim_end_matches([' ', '\t']);
        (before.is_empty() || before.ends_with(['\n', '-', '{', '[', ','])).then_some(q)
    }

    /// The `:` that follows a key ending at `key_end`.
    fn find_colon(&self, key_end: usize) -> Option<usize> {
        let at = self.skip_blank(key_end);
        (self.char_at(at) == Some(':')).then_some(at)
    }

    /// The start of the `|` or `>` header that precedes the body starting at `from`.
    fn header_start(&self, from: usize) -> Option<usize> {
        let mut end = from;
        loop {
            let line_start = self.line_start(end);
            let line = self.text(line_start, end);
            if !line.trim().is_empty() {
                // The first header-shaped token that is followed by nothing but a comment.
                let mut offset = 0;
                for token in line.split_inclusive(char::is_whitespace) {
                    let word = token.trim_end();
                    let is_header = word.starts_with(['|', '>'])
                        && word
                            .chars()
                            .skip(1)
                            .all(|c| matches!(c, '+' | '-' | '0'..='9'));
                    let rest = line.get(offset + word.len()..).unwrap_or("").trim_start();
                    if is_header && (rest.is_empty() || rest.starts_with('#')) {
                        return Some(line_start + offset);
                    }
                    offset += token.len();
                }
                return None;
            }
            if line_start == self.lo {
                return None;
            }
            end = line_start - 1;
        }
    }

    /// The end of the quoted scalar that starts at `start`.
    fn quote_end(&self, start: usize) -> Option<usize> {
        let single = self.char_at(start)? == '\'';
        let rest = self.src.get(start + 1..self.hi)?;
        let mut chars = rest.char_indices().peekable();
        while let Some((i, c)) = chars.next() {
            match c {
                '\\' if !single => {
                    chars.next();
                }
                '\'' if single => {
                    if chars.next_if(|&(_, next)| next == '\'').is_none() {
                        return Some(start + 1 + i + 1);
                    }
                }
                '"' if !single => return Some(start + 1 + i + 1),
                _ => {}
            }
        }
        None
    }

    /// The end of the block scalar with its header at `header`. `end` is saphyr's end, the
    /// start of the line after the scalar and its trailing blank lines; those blank lines
    /// are only kept with `+` chomping.
    fn block_end(&self, header: usize, mut end: usize) -> usize {
        // Saphyr may end at the next token on an indented line; cut back to the line start.
        let line_start = self.line_start(end);
        if line_start > header && self.text(line_start, end).trim().is_empty() {
            end = line_start;
        }
        let header_line = self.text(header, self.hi).lines().next().unwrap_or("");
        if header_line
            .split_whitespace()
            .next()
            .unwrap_or("")
            .contains('+')
        {
            return end;
        }
        // shortcut: a whitespace-only line counts as blank even when it is more indented than
        // the body; the range then ends before it. Upgrade if a rule needs the exact end.
        while self.text(self.lo, end).ends_with('\n') {
            let line_start = self.line_start(end - 1);
            if line_start <= header || !self.text(line_start, end).trim().is_empty() {
                break;
            }
            end = line_start;
        }
        end
    }

    // Events.

    fn node(&self, kind: NodeKind, range: TextRange, props: Props) -> Node {
        Node {
            kind,
            range,
            anchor: props.anchor,
            tagged_str: props.tagged_str,
            has_tag: props.has_tag,
        }
    }

    fn alias(&mut self, anchor: usize, span: Span) -> Result<(), SyntaxIssue> {
        let start = self.byte(span.start.index());
        let range = TextRange {
            start,
            end: self.byte(span.end.index()),
        };
        let node = self.node(NodeKind::Alias { anchor }, range, props(0, None));
        self.attach(Built { node, raw: start })
    }

    fn scalar(
        &mut self,
        value: String,
        style: ScalarStyle,
        props: Props,
        span: Span,
    ) -> Result<(), SyntaxIssue> {
        let mut raw = self.byte(span.start.index());
        let end = self.byte(span.end.index());
        let start = if props.any() {
            self.skip_props(raw)
        } else {
            raw
        };
        let (style, range) = match style {
            ScalarStyle::Plain if value.is_empty() => {
                let at = self.null_position(props, raw, end);
                if !props.any() {
                    raw = at;
                }
                (Style::Plain, TextRange { start: at, end: at })
            }
            ScalarStyle::Plain => (Style::Plain, TextRange { start, end }),
            ScalarStyle::SingleQuoted | ScalarStyle::DoubleQuoted => {
                let style = if self.char_at(start) == Some('\'') {
                    Style::SingleQuoted
                } else {
                    Style::DoubleQuoted
                };
                let end = self.quote_end(start).unwrap_or(end);
                (style, TextRange { start, end })
            }
            ScalarStyle::Literal | ScalarStyle::Folded => {
                // Saphyr starts at the first body line, unless there are properties.
                let header = if props.any() {
                    start
                } else {
                    self.header_start(raw).unwrap_or(raw)
                };
                let style = if self.char_at(header) == Some('>') {
                    Style::Folded
                } else {
                    Style::Literal
                };
                let end = self.block_end(header, end);
                raw = header;
                (style, TextRange { start: header, end })
            }
        };
        let node = self.node(NodeKind::Scalar { value, style }, range, props);
        self.attach(Built { node, raw })
    }

    /// Where a null (empty plain) scalar sits: after its `:` or `-`, or after its properties.
    fn null_position(&self, props: Props, raw: usize, props_end: usize) -> usize {
        if props.any() {
            return props_end;
        }
        match self.frames.last() {
            Some(Frame {
                body: Body::Mapping {
                    key: Some(pending), ..
                },
                ..
            }) => pending
                .colon
                .map_or(pending.key.range.end, |colon| colon + 1),
            Some(Frame {
                flow: false,
                body: Body::Sequence { .. },
                ..
            }) => self.dash_before(raw, true).map_or(raw, |dash| dash + 1),
            _ => raw,
        }
    }

    fn open(&mut self, mapping: bool, props: Props, span: Span) -> Result<(), SyntaxIssue> {
        if self.frames.len() >= MAX_DEPTH {
            let message = input_limit::TOO_DEEP.to_string();
            return Err(self.issue(RuleId::InputLimit, span.start.index(), message));
        }
        let hint = self.byte(span.start.index());
        let open = if mapping { '{' } else { '[' };
        // The bracket is the first thing after the properties; the start span covers it (and
        // any spaces after it). A block collection's span ends before its first key or item.
        let first_char = if props.any() {
            self.skip_props(hint)
        } else {
            hint
        };
        let bracket = (self.byte(span.end.index()) > first_char
            && self.char_at(first_char) == Some(open))
        .then_some(first_char);
        let flow = bracket.is_some() || self.frames.last().is_some_and(|f| f.flow);
        let body = if mapping {
            Body::Mapping {
                entries: Vec::new(),
                key: None,
                first: None,
                seen: HashSet::new(),
            }
        } else {
            Body::Sequence {
                items: Vec::new(),
                first: None,
            }
        };
        self.frames.push(Frame {
            props,
            flow,
            bracket,
            hint,
            body,
        });
        Ok(())
    }

    fn close(&mut self, end_span: Span) -> Result<(), SyntaxIssue> {
        let Some(frame) = self.frames.pop() else {
            return Ok(());
        };
        let (kind, first, last_end) = match frame.body {
            Body::Sequence { items, first } => {
                let last = items.last().map(|n| n.range.end);
                let kind = NodeKind::Sequence {
                    flow: frame.flow,
                    items,
                };
                (kind, first, last)
            }
            Body::Mapping { entries, first, .. } => {
                let last = entries.last().map(|e| e.value.range.end);
                let kind = NodeKind::Mapping {
                    flow: frame.flow,
                    entries,
                };
                (kind, first, last)
            }
        };
        let range = match frame.bracket {
            Some(start) => {
                // The closer follows the last child, an optional `,` and blanks.
                let mut at = self.skip_blank(last_end.unwrap_or(start + 1));
                if self.char_at(at) == Some(',') {
                    at = self.skip_blank(at + 1);
                }
                let end = if matches!(self.char_at(at), Some(']' | '}')) {
                    at + 1
                } else {
                    self.byte(end_span.end.index())
                };
                TextRange { start, end }
            }
            None => {
                let start = first.unwrap_or(frame.hint);
                TextRange {
                    start,
                    end: last_end.unwrap_or(start),
                }
            }
        };
        let raw = if frame.props.any() {
            frame.hint
        } else {
            range.start
        };
        let node = Node {
            kind,
            range,
            anchor: frame.props.anchor,
            tagged_str: frame.props.tagged_str,
            has_tag: frame.props.has_tag,
        };
        self.attach(Built { node, raw })
    }

    /// Adds a finished node to the open collection, or makes it the root.
    fn attach(&mut self, built: Built) -> Result<(), SyntaxIssue> {
        enum Slot {
            Item,
            Key,
            Value,
        }
        let slot = match self.frames.last().map(|f| &f.body) {
            None => {
                self.root = Some(built.node);
                return Ok(());
            }
            Some(Body::Sequence { .. }) => Slot::Item,
            Some(Body::Mapping { key: None, .. }) => Slot::Key,
            Some(Body::Mapping { .. }) => Slot::Value,
        };
        match slot {
            Slot::Item => {
                let dash = self.dash_before(built.raw, false).unwrap_or(built.raw);
                if let Some(Frame {
                    body: Body::Sequence { items, first },
                    ..
                }) = self.frames.last_mut()
                {
                    first.get_or_insert(dash);
                    items.push(built.node);
                }
            }
            Slot::Key => {
                let question = self.explicit_at(built.raw);
                let colon = self.find_colon(built.node.range.end);
                let range = built.node.range;
                let name = match &built.node.kind {
                    NodeKind::Scalar { value, .. } => (true, value.clone()),
                    _ => (false, self.text(range.start, range.end).to_string()),
                };
                if let Some(Frame {
                    body:
                        Body::Mapping {
                            key, first, seen, ..
                        },
                    ..
                }) = self.frames.last_mut()
                {
                    if seen.contains(&name) {
                        return Err(SyntaxIssue {
                            rule: RuleId::YamlSyntax,
                            range,
                            message: format!("duplicate key `{}`", name.1),
                        });
                    }
                    seen.insert(name);
                    first.get_or_insert(question.unwrap_or(built.raw));
                    *key = Some(Pending {
                        key: built.node,
                        colon,
                        explicit: question.is_some(),
                    });
                }
            }
            Slot::Value => {
                if let Some(Frame {
                    body: Body::Mapping { entries, key, .. },
                    ..
                }) = self.frames.last_mut()
                    && let Some(pending) = key.take()
                {
                    entries.push(Entry {
                        key: pending.key,
                        value: built.node,
                        colon: pending.colon,
                        explicit: pending.explicit,
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Level;
    use crate::extract::{Extraction, extract};
    use crate::testing::{lint_all, show, wrap};
    use crate::tree::{Entry, NodeKind, Style, walk};
    use crate::{Config, PathStep, RuleId};

    fn parse_source(source: &str) -> Result<Node, SyntaxIssue> {
        let Extraction::Found(fm) = extract(source) else {
            panic!("no front matter in {source:?}");
        };
        let root = parse(source, &fm);
        if let Ok(root) = &root {
            // Every range is ordered and inside the content, on character boundaries.
            walk(root, &mut |node, _| {
                let TextRange { start, end } = node.range;
                assert!(fm.content.start <= start && start <= end && end <= fm.content.end);
                assert!(source.get(start..end).is_some(), "{:?}", node.range);
            });
        }
        root
    }

    /// The root of the front matter `fm` (wrapped as a whole file) and that file.
    fn root_of(fm: &str) -> (String, Node) {
        let source = wrap(fm);
        let root = parse_source(&source).expect("parses");
        (source, root)
    }

    fn entries(node: &Node) -> &[Entry] {
        match &node.kind {
            NodeKind::Mapping { entries, .. } => entries,
            other => panic!("not a mapping: {other:?}"),
        }
    }

    fn items(node: &Node) -> &[Node] {
        match &node.kind {
            NodeKind::Sequence { items, .. } => items,
            other => panic!("not a sequence: {other:?}"),
        }
    }

    /// The entry of the root mapping whose scalar key is `key`.
    fn entry<'a>(root: &'a Node, key: &str) -> &'a Entry {
        entries(root)
            .iter()
            .find(|e| matches!(&e.key.kind, NodeKind::Scalar { value, .. } if value == key))
            .unwrap_or_else(|| panic!("no key {key}"))
    }

    fn shown(source: &str, node: &Node) -> String {
        show(source, node.range)
    }

    #[test]
    fn scalar_ranges() {
        let (source, root) = root_of("title: \"hi\"  # c\n");
        let e = entry(&root, "title");
        assert_eq!(shown(&source, &e.key), "2:1-2:6");
        assert_eq!(shown(&source, &e.value), "2:8-2:12");
        let colon = e.colon.expect("has a colon");
        assert_eq!(
            show(
                &source,
                TextRange {
                    start: colon,
                    end: colon + 1
                }
            ),
            "2:6-2:7"
        );
        assert_eq!(shown(&source, &root), "2:1-2:12");
    }

    #[test]
    fn single_quoted_range_ends_at_the_closing_quote() {
        let (source, root) = root_of("a: 'it''s'   # c\nb: \"x\\\"y\"  \n");
        assert_eq!(shown(&source, &entry(&root, "a").value), "2:4-2:11");
        assert_eq!(shown(&source, &entry(&root, "b").value), "3:4-3:10");
    }

    #[test]
    fn spans_are_bytes_after_emoji() {
        let (source, root) = root_of("a: 😀 b\nc: d\n");
        let value = &entry(&root, "a").value;
        assert_eq!(shown(&source, value), "2:4-2:7");
        assert_eq!(value.range, TextRange { start: 7, end: 13 });
        assert_eq!(shown(&source, &entry(&root, "c").key), "3:1-3:2");
    }

    #[test]
    fn plain_multiline_range_excludes_trailing_spaces() {
        let (source, root) = root_of("a: foo\n  bar baz   \nb: 1\n");
        assert_eq!(shown(&source, &entry(&root, "a").value), "2:4-3:10");
    }

    #[test]
    fn block_scalar_range() {
        let source = include_str!("../../../tests/fixtures/edge/block-scalars.md");
        let root = parse_source(source).expect("parses");
        // Ruling P3: a block scalar ends after the line ending of its last kept line.
        assert_eq!(shown(source, &entry(&root, "literal").value), "2:10-7:1");
        assert_eq!(shown(source, &entry(&root, "folded").value), "7:9-12:1");
        assert_eq!(shown(source, &entry(&root, "strip").value), "12:8-15:1");
        // `>+` keeps the blank line 19.
        assert_eq!(shown(source, &entry(&root, "keep").value), "15:7-20:1");
    }

    #[test]
    fn block_scalar_trailing_blank_lines_need_keep_chomping() {
        let (source, root) = root_of("a: |\n  x\n\nb: |+\n  y\n\nc: 1\n");
        assert_eq!(shown(&source, &entry(&root, "a").value), "2:4-4:1");
        assert_eq!(shown(&source, &entry(&root, "b").value), "5:4-8:1");
    }

    #[test]
    fn block_scalar_header_with_properties_and_comment() {
        let (source, root) = root_of("a: &x !!str | # c\n  y\n\nb: 1\n");
        let value = &entry(&root, "a").value;
        assert_eq!(shown(&source, value), "2:13-4:1");
        assert_eq!(value.anchor, Some(1));
        assert!(value.has_tag && value.tagged_str);
        let (source, root) = root_of("a: |\nb: >2-\n    x\n");
        assert_eq!(shown(&source, &entry(&root, "a").value), "2:4-3:1");
        assert_eq!(shown(&source, &entry(&root, "b").value), "3:4-5:1");
    }

    #[test]
    fn flow_ranges() {
        let source = include_str!("../../../tests/fixtures/edge/nested-flow.md");
        let root = parse_source(source).expect("parses");
        assert_eq!(shown(source, &entry(&root, "data").value), "3:7-5:2");
        assert_eq!(shown(source, &entry(&root, "list").value), "6:7-9:2");
        let (source, root) = root_of("a: &q [1, 2]  \nb: {}\nc: []\n");
        assert_eq!(shown(&source, &entry(&root, "a").value), "2:7-2:13");
        assert_eq!(shown(&source, &entry(&root, "b").value), "3:4-3:6");
        assert_eq!(shown(&source, &entry(&root, "c").value), "4:4-4:6");
    }

    #[test]
    fn flow_pair_in_a_flow_sequence_is_a_flow_mapping() {
        let (source, root) = root_of("d: [a: 1, b]\n");
        let seq = &entry(&root, "d").value;
        let NodeKind::Mapping { flow, entries } = &items(seq)[0].kind else {
            panic!("not a mapping")
        };
        assert!(flow);
        assert_eq!(shown(&source, &items(seq)[0]), "2:5-2:9");
        assert_eq!(entries.len(), 1);
        assert!(matches!(items(seq)[1].kind, NodeKind::Scalar { .. }));
    }

    #[test]
    fn block_collection_ranges() {
        let (source, root) = root_of("a:\n  - x\n  - k: v\n    l: w\nb:\n- p\n");
        assert_eq!(shown(&source, &root), "2:1-7:4");
        let seq = &entry(&root, "a").value;
        assert_eq!(shown(&source, seq), "3:3-5:9");
        assert_eq!(shown(&source, &items(seq)[1]), "4:5-5:9");
        assert_eq!(shown(&source, &entry(&root, "b").value), "7:1-7:4");
    }

    #[test]
    fn nested_sequence_and_comment_before_item() {
        let (source, root) = root_of("a:\n  - - x\n  - # c\n    y\n");
        let seq = &entry(&root, "a").value;
        assert_eq!(shown(&source, seq), "3:3-5:6");
        assert_eq!(shown(&source, &items(seq)[0]), "3:5-3:8");
        assert_eq!(shown(&source, &items(seq)[1]), "5:5-5:6");
    }

    #[test]
    fn null_value_range() {
        let (source, root) = root_of("a:\nb: 1\n");
        assert_eq!(shown(&source, &entry(&root, "a").value), "2:3-2:3");
        let (source, root) = root_of("a:   # c\nb: &x\nc: !!str\nd: 1\n");
        assert_eq!(shown(&source, &entry(&root, "a").value), "2:3-2:3");
        assert_eq!(shown(&source, &entry(&root, "b").value), "3:6-3:6");
        assert_eq!(shown(&source, &entry(&root, "c").value), "4:9-4:9");
    }

    #[test]
    fn null_sequence_items() {
        let (source, root) = root_of("a:\n  - # c\n  - b\n  -  \n");
        let seq = &entry(&root, "a").value;
        assert_eq!(shown(&source, &items(seq)[0]), "3:4-3:4");
        assert_eq!(shown(&source, &items(seq)[2]), "5:4-5:4");
        assert_eq!(shown(&source, seq), "3:3-5:4");
    }

    #[test]
    fn explicit_key() {
        let (source, root) = root_of("? a\n: 1\n? b\nc: d\n");
        let list = entries(&root);
        assert!(list[0].explicit);
        assert!(list[1].explicit);
        assert!(!list[2].explicit);
        assert_eq!(shown(&source, &list[0].key), "2:3-2:4");
        assert_eq!(
            list[0].colon.map(|c| show(
                &source,
                TextRange {
                    start: c,
                    end: c + 1
                }
            )),
            Some("3:1-3:2".to_string())
        );
        assert_eq!(list[1].colon, None);
        // no `:`: the null value sits right after the key
        assert_eq!(shown(&source, &list[1].value), "4:4-4:4");
        assert_eq!(shown(&source, &root), "2:1-5:5");
    }

    #[test]
    fn question_mark_at_the_end_of_a_value_is_not_explicit() {
        let (_, root) = root_of("a: what?\nb: 1\nc: x?\n");
        assert!(entries(&root).iter().all(|e| !e.explicit));
    }

    #[test]
    fn flow_entries_without_colon() {
        let (_, root) = root_of("c: {x, y: 1, \"q\":2}\n");
        let NodeKind::Mapping { entries, .. } = &entry(&root, "c").value.kind else {
            panic!("not a mapping")
        };
        assert_eq!(
            entries
                .iter()
                .map(|e| e.colon.is_some())
                .collect::<Vec<_>>(),
            [false, true, true]
        );
    }

    #[test]
    fn paths() {
        let (_, root) = root_of("a:\n  - x\n  - y\n");
        let mut seen = Vec::new();
        walk(&root, &mut |node, path| {
            let text = match &node.kind {
                NodeKind::Scalar { value, .. } => value.clone(),
                _ => "*".to_string(),
            };
            seen.push((text, path.to_vec()));
        });
        let k = PathStep::Key;
        let v = PathStep::Value;
        let i = PathStep::Item;
        assert_eq!(
            seen,
            [
                ("*".to_string(), vec![]),
                ("a".to_string(), vec![k(0)]),
                ("*".to_string(), vec![v(0)]),
                ("x".to_string(), vec![v(0), i(0)]),
                ("y".to_string(), vec![v(0), i(1)]),
            ]
        );
    }

    #[test]
    fn anchors_aliases_and_tags() {
        let (source, root) = root_of("a: &d\n  k: v\nb: *d\nc: !!str 12\nd: !!int 3\ne: *d # x\n");
        let a = &entry(&root, "a").value;
        assert_eq!(a.anchor, Some(1));
        assert_eq!(shown(&source, a), "3:3-3:7");
        let b = &entry(&root, "b").value;
        assert_eq!(b.kind, NodeKind::Alias { anchor: 1 });
        assert_eq!(shown(&source, b), "4:4-4:6");
        let c = &entry(&root, "c").value;
        assert!(c.tagged_str && c.has_tag);
        assert_eq!(shown(&source, c), "5:10-5:12");
        let d = &entry(&root, "d").value;
        assert!(!d.tagged_str && d.has_tag);
        assert_eq!(d.anchor, None);
        assert!(
            matches!(&c.kind, NodeKind::Scalar { value, style: Style::Plain } if value == "12")
        );
    }

    #[test]
    fn crlf_ranges() {
        let source = "---\r\na: |\r\n  x\r\nb: \"y\"\r\n---\r\n";
        let root = parse_source(source).expect("parses");
        assert_eq!(shown(source, &entry(&root, "a").value), "2:4-4:1");
        assert_eq!(shown(source, &entry(&root, "b").value), "4:4-4:7");
    }

    #[test]
    fn empty_and_comment_only_content_is_an_empty_mapping() {
        for fm in ["", "# only\n", "\n\n"] {
            let source = wrap(fm);
            let Extraction::Found(found) = extract(&source) else {
                panic!("no front matter")
            };
            let root = parse(&source, &found).expect("parses");
            assert!(entries(&root).is_empty());
            assert_eq!(
                root.range,
                TextRange {
                    start: found.content.start,
                    end: found.content.start
                }
            );
        }
    }

    fn lint(source: &str) -> Vec<String> {
        // These tests are about parsing, not about style.
        let mut config = Config::default();
        config.rules.insert(RuleId::NoFlowSequence, Level::Off);
        lint_all(source, config)
    }

    #[test]
    fn yaml_syntax_cases() {
        assert_eq!(
            lint("---\na: [1\n---\n"),
            ["3:1-3:1 yaml-syntax while parsing a flow sequence, expected ',' or ']'"]
        );
        assert_eq!(
            lint("---\ntitle: a\ntitle: b\n---\n"),
            ["3:1-3:6 yaml-syntax duplicate key `title`"]
        );
        let not_mapping = "front matter must be a key/value mapping";
        assert_eq!(
            lint("---\n- a\n---\n"),
            [format!("2:1-2:4 yaml-syntax {not_mapping}")]
        );
        assert_eq!(
            lint("---\n\"str\"\n---\n"),
            [format!("2:1-2:6 yaml-syntax {not_mapping}")]
        );
        assert_eq!(lint("---\n---\n"), Vec::<String>::new());
        assert_eq!(lint("---\n# only\n---\n"), Vec::<String>::new());
    }

    #[test]
    fn duplicate_keys_compare_scalars_by_value_and_others_by_text() {
        assert_eq!(
            lint("---\na: 1\n'a': 2\n---\n"),
            ["3:1-3:4 yaml-syntax duplicate key `a`"]
        );
        assert_eq!(
            lint("---\n[a, b]: 1\n[a, b]: 2\n---\n"),
            ["3:1-3:7 yaml-syntax duplicate key `[a, b]`"]
        );
        assert_eq!(
            lint("---\n[a, b]: 1\n[a,b]: 2\n---\n"),
            Vec::<String>::new()
        );
        // the same key in different mappings is fine
        assert_eq!(
            lint("---\nx: {a: 1}\ny: {a: 2}\n---\n"),
            Vec::<String>::new()
        );
        // the first duplicate in document order wins
        assert_eq!(
            lint("---\nx: {a: 1, a: 2}\nb: 1\nb: 2\n---\n"),
            ["2:11-2:12 yaml-syntax duplicate key `a`"]
        );
    }

    #[test]
    fn second_document_is_an_error() {
        assert_eq!(
            lint("---\na: 1\n...\nb: 2\n---\n"),
            ["4:1-4:1 yaml-syntax front matter must contain a single YAML document"]
        );
    }

    #[test]
    fn tab_indentation_is_yaml_syntax() {
        let source = "---\na:\n\tb: 1\n---\n";
        let diagnostics = lint(source);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].starts_with("3:"), "{diagnostics:?}");
        assert!(
            diagnostics[0].contains("yaml-syntax tabs"),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn unknown_alias_is_yaml_syntax() {
        let diagnostics = lint("---\na: *nope\n---\n");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].contains("yaml-syntax"), "{diagnostics:?}");
    }

    fn nested(levels: usize) -> String {
        let mut fm = String::new();
        for i in 0..levels - 1 {
            fm.push_str(&format!("{}k{i}:\n", "  ".repeat(i)));
        }
        fm.push_str(&format!(
            "{}k{}: leaf\n",
            "  ".repeat(levels - 1),
            levels - 1
        ));
        format!("---\n{fm}---\n")
    }

    #[test]
    fn depth_limit() {
        let fixture = include_str!("../../../tests/fixtures/edge/deep-nesting.md");
        assert_eq!(
            lint(fixture),
            ["130:257-130:257 input-limit nesting is deeper than 128 levels"]
        );
        assert_eq!(
            lint(&nested(129)),
            ["130:257-130:257 input-limit nesting is deeper than 128 levels"]
        );
        assert_eq!(lint(&nested(128)), Vec::<String>::new());
    }

    #[test]
    fn depth_limit_counts_collections_only() {
        let flow = format!("---\na: {}x{}\n---\n", "[".repeat(127), "]".repeat(127));
        assert_eq!(lint(&flow), Vec::<String>::new());
        let flow = format!("---\na: {}x{}\n---\n", "[".repeat(128), "]".repeat(128));
        assert_eq!(
            lint(&flow),
            ["2:131-2:131 input-limit nesting is deeper than 128 levels"]
        );
    }

    #[test]
    fn long_flow_lines_parse_quickly() {
        let entries: Vec<String> = (0..60_000).map(|i| format!("k{i}: v")).collect();
        let start = std::time::Instant::now();
        let (_, root) = root_of(&format!("a: {{{}}}\n", entries.join(", ")));
        assert_eq!(entries.len(), 60_000);
        assert!(matches!(
            entry(&root, "a").value.kind,
            NodeKind::Mapping { .. }
        ));
        let explicit: Vec<String> = (0..60_000).map(|i| format!("? k{i}")).collect();
        let (_, root) = root_of(&format!("a: {{{}}}\n", explicit.join(", ")));
        let NodeKind::Mapping { entries, .. } = &entry(&root, "a").value.kind else {
            panic!("not a mapping")
        };
        assert!(entries.iter().all(|e| e.explicit));
        assert!(start.elapsed().as_secs() < 5, "{:?}", start.elapsed());
    }

    #[test]
    fn explicit_keys_in_flow_and_sequences() {
        let (_, root) = root_of("a: {? x, y: 1}\nb:\n  - ? p\n    : q\n");
        let flow = entries(&entry(&root, "a").value);
        assert_eq!(
            flow.iter().map(|e| e.explicit).collect::<Vec<_>>(),
            [true, false]
        );
        let item = &items(&entry(&root, "b").value)[0];
        assert!(entries(item)[0].explicit);
    }

    /// The source text covered by the value of `key` in `fm`.
    fn value_text(fm: &str, key: &str) -> String {
        let (source, root) = root_of(fm);
        let range = entry(&root, key).value.range;
        source[range.start..range.end].to_string()
    }

    #[test]
    fn flow_collections_with_inner_spaces() {
        for (fm, key, expected) in [
            ("a: [ x, y ]\nb: 1\n", "a", "[ x, y ]"),
            ("a: { k: v }\nb: 1\n", "a", "{ k: v }"),
            ("a: { }\nb: 1\n", "a", "{ }"),
            ("a: [ ]\nb: 1\n", "a", "[ ]"),
            ("a: [  ]\nb: 1\n", "a", "[  ]"),
            ("a: { k: [ 1, 2 ] }\nb: 1\n", "a", "{ k: [ 1, 2 ] }"),
            ("a: &q [ 1 ]\nb: 1\n", "a", "[ 1 ]"),
            ("a: !!seq [ 2 ]\nb: 1\n", "a", "[ 2 ]"),
            ("a:\n  - [ 1, 2 ]\n  - x\n", "a", "- [ 1, 2 ]\n  - x"),
            ("a: {k: v,}\nb: 1\n", "a", "{k: v,}"),
            ("a: {k: v, }\nb: 1\n", "a", "{k: v, }"),
            ("a: [x, y, ]\nb: 1\n", "a", "[x, y, ]"),
            ("a: [x,\n  y # c\n  ]\nb: 1\n", "a", "[x,\n  y # c\n  ]"),
            ("a: {k: |-,}\nb: 1\n", "a", "{k: |-,}"),
        ] {
            assert_eq!(value_text(fm, key), expected, "{fm:?}");
        }
        let (_, root) = root_of("a: [ x, y ]\n");
        assert!(matches!(
            entry(&root, "a").value.kind,
            NodeKind::Sequence { flow: true, .. }
        ));
        let (_, root) = root_of("a: { k: v }\n");
        assert!(matches!(
            entry(&root, "a").value.kind,
            NodeKind::Mapping { flow: true, .. }
        ));
    }

    #[test]
    fn block_scalar_end_when_saphyr_ends_on_an_indented_token() {
        for (fm, key, expected) in [
            ("a:\n  b: |\n    x\n  c: 1\n", "b", "|\n    x\n"),
            ("a:\n  b: |+\n    x\n\n  c: 1\n", "b", "|+\n    x\n\n"),
            ("a:\n  b: >\n    x\n  c: 1\n", "b", ">\n    x\n"),
            ("a:\n  b: |-\n    x\n\n  c: 1\n", "b", "|-\n    x\n"),
            ("a:\n  - k: |\n      x\n    l: 1\n", "k", "|\n      x\n"),
            (
                "a:\n  - k: |+\n      x\n\n    l: 1\n",
                "k",
                "|+\n      x\n\n",
            ),
            ("a:\n  b: |\n    x\n\n # c\n", "b", "|\n    x\n"),
            ("a:\n  b: |+\n    x\n\n # c\n", "b", "|+\n    x\n\n"),
            ("a:\n  b: >-\n    x\n\n # c\nz: 1\n", "b", ">-\n    x\n"),
        ] {
            let (source, root) = root_of(fm);
            let mut found = None;
            walk(&root, &mut |node, _| {
                if let NodeKind::Mapping { entries, .. } = &node.kind {
                    for e in entries {
                        if matches!(&e.key.kind, NodeKind::Scalar { value, .. } if value == key) {
                            found = Some(e.value.range);
                        }
                    }
                }
            });
            let range = found.expect("key found");
            assert_eq!(&source[range.start..range.end], expected, "{fm:?}");
        }
    }

    #[test]
    fn block_scalar_as_first_sequence_item_or_key() {
        assert_eq!(value_text("a:\n- |\n  x\n- b\n", "a"), "- |\n  x\n- b");
        let (source, root) = root_of("a:\n- |\n  x\n- b\n");
        let seq = &items(&entry(&root, "a").value);
        let r = seq[0].range;
        assert_eq!(&source[r.start..r.end], "|\n  x\n");
        // a root sequence of a block scalar: the error is at the dash
        assert_eq!(
            lint("---\n- |\n  x\n---\n"),
            ["2:1-4:1 yaml-syntax front matter must be a key/value mapping"]
        );
        let (source, root) = root_of("? |\n  k\n: v\n");
        let e = &entries(&root)[0];
        assert!(e.explicit);
        assert_eq!(&source[e.key.range.start..e.key.range.end], "|\n  k\n");
        assert_eq!(&source[root.range.start..root.range.end], "? |\n  k\n: v");
    }

    #[test]
    fn nbsp_is_content_not_whitespace() {
        let (source, root) = root_of("a: &x \u{a0}\nb: \u{85}\n");
        let r = entry(&root, "a").value.range;
        assert_eq!(&source[r.start..r.end], "\u{a0}");
        let r = entry(&root, "b").value.range;
        assert_eq!(&source[r.start..r.end], "\u{85}");
    }

    #[test]
    fn tab_after_colon_is_yaml_syntax() {
        // Ruling T4b: saphyr rejects a tab after `:`; that is a known parser limitation.
        let diagnostics = lint("---\na:\t1\n---\n");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(diagnostics[0].contains(" yaml-syntax "), "{diagnostics:?}");
    }

    #[test]
    fn deep_flow_shape_parses_without_per_node_paths() {
        // 127 open flow sequences (128 levels with the root), about 400 KB of scalars.
        let body = "a,".repeat(200_000);
        let fm = format!("a: {}{body}{}\n", "[".repeat(127), "]".repeat(127));
        let start = std::time::Instant::now();
        let (_, root) = root_of(&fm);
        let mut count = 0;
        walk(&root, &mut |_, path| {
            count += 1;
            assert!(path.len() <= 128);
        });
        assert!(count > 200_000);
        assert!(start.elapsed().as_secs() < 5, "{:?}", start.elapsed());
    }
}
