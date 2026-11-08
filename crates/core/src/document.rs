//! The front matter as lines, comments and nodes: what the rules look at.

use crate::data::Data;
use crate::extract::FrontMatter;
use crate::tree::{Entry, Node, NodeKind, Style};
use crate::{PathStep, TextRange};

/// What a front-matter line is, for the whitespace rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineKind {
    /// Anything else, including the `---` delimiter lines.
    Structural,
    /// The line starts strictly after a block scalar's header line and inside its range.
    BlockText,
    /// The line starts strictly inside a multi-line plain or quoted scalar.
    ScalarContinuation,
}

/// One line of the front matter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Line {
    /// The line without its line ending.
    pub range: TextRange,
    /// What the line is.
    pub kind: LineKind,
    /// The line is covered by an `ignoreKeys` entry: from the key's line to the end of its value.
    pub ignored: bool,
}

/// A `# ...` comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Comment {
    /// From the `#` to the end of the line's content.
    pub range: TextRange,
    /// Only whitespace comes before the `#` on its line.
    pub own_line: bool,
}

/// Where a node sits in its parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    /// The top-level mapping.
    Root,
    /// A mapping key.
    Key,
    /// A mapping value.
    Value,
    /// A sequence item.
    Item,
}

/// A node with its surroundings.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NodeRef<'a> {
    /// The node.
    pub node: &'a Node,
    /// Where it sits in its parent.
    pub role: Role,
    /// It is inside a flow collection (the collection itself is not).
    pub in_flow: bool,
    /// The mapping entry of a key or value.
    pub entry: Option<&'a Entry>,
    /// The collection that holds it.
    pub parent: Option<&'a Node>,
}

/// The text a scalar occupies, for the line and comment scanners.
#[derive(Debug, Clone, Copy)]
struct Span {
    /// For a block scalar this is the end of its header line, so a header comment is outside.
    start: usize,
    end: usize,
    kind: LineKind,
}

/// The parsed front matter with its lines and comments.
#[derive(Debug)]
pub(crate) struct Document {
    /// The top-level mapping.
    pub root: Node,
    /// The values of the whole tree.
    pub data: Data,
    /// Every front-matter line, delimiters included.
    pub lines: Vec<Line>,
    /// Every comment, in source order.
    pub comments: Vec<Comment>,
    /// Multi-line-capable scalars, sorted by start and disjoint.
    scalars: Vec<Span>,
}

/// The scalar that has `offset` strictly inside it.
fn scalar_at(scalars: &[Span], offset: usize) -> Option<&Span> {
    let before = scalars.partition_point(|s| s.start < offset);
    scalars
        .get(before.checked_sub(1)?)
        .filter(|s| offset < s.end)
}

/// Visits `node` and everything below it in pre-order (keys before values), handing `f` each
/// node with its path from the root. Like [`walk`], one shared path stack: O(depth) memory.
fn visit<'a>(
    node: &'a Node,
    role: Role,
    in_flow: bool,
    link: (Option<&'a Entry>, Option<&'a Node>),
    path: &mut Vec<PathStep>,
    f: &mut impl FnMut(NodeRef<'a>, &[PathStep]),
) {
    let (entry, parent) = link;
    f(
        NodeRef {
            node,
            role,
            in_flow,
            entry,
            parent,
        },
        path,
    );
    // Recursion depth is the nesting depth, which `parse` caps at MAX_DEPTH.
    match &node.kind {
        NodeKind::Sequence { flow, items } => {
            for (i, item) in items.iter().enumerate() {
                path.push(PathStep::Item(i));
                visit(
                    item,
                    Role::Item,
                    in_flow || *flow,
                    (None, Some(node)),
                    path,
                    f,
                );
                path.pop();
            }
        }
        NodeKind::Mapping { flow, entries } => {
            for (i, e) in entries.iter().enumerate() {
                let in_flow = in_flow || *flow;
                let link = (Some(e), Some(node));
                path.push(PathStep::Key(i));
                visit(&e.key, Role::Key, in_flow, link, path, f);
                path.pop();
                path.push(PathStep::Value(i));
                visit(&e.value, Role::Value, in_flow, link, path, f);
                path.pop();
            }
        }
        NodeKind::Scalar { .. } | NodeKind::Alias { .. } => {}
    }
}

fn node_refs(root: &Node) -> Vec<NodeRef<'_>> {
    let mut out = Vec::new();
    visit(
        root,
        Role::Root,
        false,
        (None, None),
        &mut Vec::new(),
        &mut |r, _| out.push(r),
    );
    out
}

/// The scalars that can span lines, sorted by start.
fn scalar_spans(source: &str, root: &Node) -> Vec<Span> {
    let mut spans: Vec<Span> = node_refs(root)
        .into_iter()
        .filter_map(|r| {
            let NodeKind::Scalar { style, .. } = &r.node.kind else {
                return None;
            };
            let range = r.node.range;
            if range.start >= range.end {
                return None;
            }
            if matches!(style, Style::Literal | Style::Folded) {
                let header_end = source
                    .get(range.start..range.end)
                    .and_then(|text| text.find('\n'))
                    .map_or(range.end, |i| range.start + i);
                Some(Span {
                    start: header_end,
                    end: range.end,
                    kind: LineKind::BlockText,
                })
            } else {
                Some(Span {
                    start: range.start,
                    end: range.end,
                    kind: LineKind::ScalarContinuation,
                })
            }
        })
        .collect();
    spans.sort_by_key(|s| s.start);
    spans
}

impl Document {
    /// Classifies the lines of `fm`, finds its comments and marks the `ignore_keys` lines.
    pub(crate) fn new(
        source: &str,
        fm: &FrontMatter,
        root: Node,
        data: Data,
        ignore_keys: &[String],
    ) -> Document {
        let scalars = scalar_spans(source, &root);
        let mut lines = Vec::new();
        let text = source.get(fm.open.start..fm.close.end).unwrap_or("");
        let mut start = fm.open.start;
        for piece in text.split('\n') {
            let content = piece.strip_suffix('\r').unwrap_or(piece);
            lines.push(Line {
                range: TextRange {
                    start,
                    end: start + content.len(),
                },
                kind: scalar_at(&scalars, start).map_or(LineKind::Structural, |s| s.kind),
                ignored: false,
            });
            start += piece.len() + 1;
        }
        let mut doc = Document {
            root,
            data,
            lines,
            comments: Vec::new(),
            scalars,
        };
        doc.comments = doc.find_comments(source);
        doc.mark_ignored(ignore_keys);
        doc
    }

    /// A `#` is a comment when it starts the line (after indentation) or follows a space or
    /// tab, and is not inside a scalar.
    fn find_comments(&self, source: &str) -> Vec<Comment> {
        let mut comments = Vec::new();
        for line in &self.lines {
            let text = source.get(line.range.start..line.range.end).unwrap_or("");
            for (i, byte) in text.bytes().enumerate() {
                let before = text.as_bytes().get(..i).unwrap_or_default();
                let after_blank = before.last().is_none_or(|b| matches!(b, b' ' | b'\t'));
                if byte == b'#' && after_blank && !self.in_scalar(line.range.start + i) {
                    comments.push(Comment {
                        range: TextRange {
                            start: line.range.start + i,
                            end: line.range.end,
                        },
                        own_line: before.iter().all(|b| matches!(b, b' ' | b'\t')),
                    });
                }
            }
        }
        comments
    }

    /// Marks the lines of every top-level entry whose key is in `ignore_keys`.
    fn mark_ignored(&mut self, ignore_keys: &[String]) {
        let NodeKind::Mapping { entries, .. } = &self.root.kind else {
            return;
        };
        let mut spans = Vec::new();
        for entry in entries {
            let NodeKind::Scalar { value, .. } = &entry.key.kind else {
                continue;
            };
            if ignore_keys.contains(value) {
                let first = entry.key.range.start;
                // The last character the entry occupies; a block value's range ends after its
                // line ending, a null value's is empty.
                let last = entry.value.range.end.saturating_sub(1).max(first);
                spans.push((first, last));
            }
        }
        for (first, last) in spans {
            let (Some(first), Some(last)) = (self.line_index(first), self.line_index(last)) else {
                continue;
            };
            for line in self.lines.get_mut(first..=last).unwrap_or_default() {
                line.ignored = true;
            }
        }
    }

    /// The index in `lines` of the line holding `offset`; a line's ending belongs to it.
    pub(crate) fn line_index(&self, offset: usize) -> Option<usize> {
        let after = self.lines.partition_point(|l| l.range.start <= offset);
        after.checked_sub(1)
    }

    /// Every node, in pre-order with keys before values.
    pub(crate) fn nodes(&self) -> Vec<NodeRef<'_>> {
        node_refs(&self.root)
    }

    /// Hands `f` every node as [`nodes`](Self::nodes) lists them, with its path from the root, in
    /// one traversal (the path stack is shared: O(depth) memory).
    pub(crate) fn walk_nodes<'a>(&'a self, f: &mut impl FnMut(NodeRef<'a>, &[PathStep])) {
        visit(
            &self.root,
            Role::Root,
            false,
            (None, None),
            &mut Vec::new(),
            f,
        );
    }

    /// Whether `offset` is strictly inside a scalar's range. For a block scalar that is the body
    /// after the header line.
    pub(crate) fn in_scalar(&self, offset: usize) -> bool {
        scalar_at(&self.scalars, offset).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{document, wrap};

    const BLOCK_SCALARS: &str = include_str!("../../../tests/fixtures/edge/block-scalars.md");
    const DISABLE_COMMENTS: &str = include_str!("../../../tests/fixtures/edge/disable-comments.md");

    /// The kind of each line, 1-based line numbers.
    fn kinds(doc: &Document, lines: &[usize]) -> Vec<LineKind> {
        lines
            .iter()
            .filter_map(|n| doc.lines.get(n - 1).map(|l| l.kind))
            .collect()
    }

    /// The comments as (line number, text, own_line).
    fn comments(source: &str, doc: &Document) -> Vec<(usize, String, bool)> {
        doc.comments
            .iter()
            .map(|c| {
                let line = doc.line_index(c.range.start).map_or(0, |i| i + 1);
                let text = source.get(c.range.start..c.range.end).unwrap_or("?");
                (line, text.to_string(), c.own_line)
            })
            .collect()
    }

    #[test]
    fn line_kinds_block_scalars() {
        let (_, doc) = document(BLOCK_SCALARS, &[]);
        assert_eq!(doc.lines.len(), 21);
        assert_eq!(kinds(&doc, &[3, 4, 5, 6]), [LineKind::BlockText; 4]);
        assert_eq!(kinds(&doc, &[8, 9, 10, 11]), [LineKind::BlockText; 4]);
        assert_eq!(kinds(&doc, &[1, 2, 7, 12, 21]), [LineKind::Structural; 5]);
        // `keep: >+` keeps its trailing blank line 19; `after:` is structural again.
        assert_eq!(kinds(&doc, &[16, 17, 18, 19]), [LineKind::BlockText; 4]);
        assert_eq!(kinds(&doc, &[20]), [LineKind::Structural]);
    }

    #[test]
    fn line_kinds_multiline_quoted() {
        let (_, doc) = document(&wrap("a: \"x\n\n  y\"\nb: 1"), &[]);
        assert_eq!(kinds(&doc, &[2]), [LineKind::Structural]);
        assert_eq!(kinds(&doc, &[3, 4]), [LineKind::ScalarContinuation; 2]);
        assert_eq!(kinds(&doc, &[5]), [LineKind::Structural]);
    }

    #[test]
    fn line_kinds_multiline_plain_and_crlf() {
        let (_, doc) = document("---\r\na: x\r\n  y\r\nb: |\r\n  t\r\n---\r\n", &[]);
        assert_eq!(kinds(&doc, &[3]), [LineKind::ScalarContinuation]);
        assert_eq!(kinds(&doc, &[5]), [LineKind::BlockText]);
        assert_eq!(kinds(&doc, &[2, 4, 6]), [LineKind::Structural; 3]);
        // Ranges exclude the carriage return.
        assert_eq!(
            doc.lines.get(1).map(|l| l.range),
            Some(TextRange { start: 5, end: 9 })
        );
    }

    #[test]
    fn comments_found() {
        let (_, doc) = document(DISABLE_COMMENTS, &[]);
        let found = comments(DISABLE_COMMENTS, &doc);
        assert_eq!(
            found,
            [
                (
                    2,
                    "# fmlint-disable-next-line no-trailing-spaces".to_string(),
                    true
                ),
                (5, "# fmlint-disable-line".to_string(), false),
                (6, "# fmlint-disable-next-line not-a-rule".to_string(), true),
            ]
        );
    }

    #[test]
    fn comments_respect_scalars() {
        let source = wrap(
            "a: x#y\nb: \"p # q\"\nc: 'r #s'\nd: | # header\n  # body\ne: [1, # in flow\n  2]\n",
        );
        let (_, doc) = document(&source, &[]);
        let found: Vec<_> = comments(&source, &doc)
            .into_iter()
            .map(|(line, text, _)| (line, text))
            .collect();
        assert_eq!(
            found,
            [(5, "# header".to_string()), (7, "# in flow".to_string())]
        );
    }

    #[test]
    fn comments_after_multiline_quoted_scalar() {
        let source = wrap("a: \"x #\n  y\" # real\n");
        let (_, doc) = document(&source, &[]);
        let found = comments(&source, &doc);
        assert_eq!(found, [(3, "# real".to_string(), false)]);
    }

    #[test]
    fn ignore_keys_marks_lines() {
        let (_, doc) = document(&wrap("raw:\n  - a  b\ntitle: x\n"), &["raw".to_string()]);
        let ignored: Vec<bool> = doc.lines.iter().map(|l| l.ignored).collect();
        assert_eq!(ignored, [false, true, true, false, false]);
    }

    #[test]
    fn ignore_keys_covers_block_scalar_and_null() {
        let source = wrap("a: |\n  x\n\nb:\nc: 1\n");
        let (_, doc) = document(&source, &["a".to_string(), "b".to_string()]);
        let ignored: Vec<bool> = doc.lines.iter().map(|l| l.ignored).collect();
        // The blank line after a clip scalar is not part of the value.
        assert_eq!(ignored, [false, true, true, false, true, false, false]);
    }

    #[test]
    fn nodes_pre_order() {
        let (_, doc) = document(&wrap("a: {b: c}\n"), &[]);
        let nodes = doc.nodes();
        let roles: Vec<_> = nodes.iter().map(|n| n.role).collect();
        assert_eq!(
            roles,
            [Role::Root, Role::Key, Role::Value, Role::Key, Role::Value]
        );
        let flow: Vec<_> = nodes.iter().map(|n| n.in_flow).collect();
        assert_eq!(flow, [false, false, false, true, true]);
        assert!(matches!(
            nodes.get(2).map(|n| &n.node.kind),
            Some(NodeKind::Mapping { flow: true, .. })
        ));
        assert!(
            nodes
                .first()
                .is_some_and(|n| n.parent.is_none() && n.entry.is_none())
        );
        assert!(
            nodes
                .get(3)
                .is_some_and(|n| n.parent.is_some() && n.entry.is_some())
        );
    }

    #[test]
    fn walk_nodes_matches_nodes_and_tree_walk() {
        let (_, doc) = document(&wrap("a: {b: [c, d]}\ne:\n  - f: g\n"), &[]);
        let mut seen = Vec::new();
        doc.walk_nodes(&mut |r, path| seen.push((r, path.to_vec())));
        let nodes = doc.nodes();
        assert_eq!(seen.len(), nodes.len());
        for ((r, _), n) in seen.iter().zip(&nodes) {
            assert!(std::ptr::eq(r.node, n.node));
            assert_eq!((r.role, r.in_flow), (n.role, n.in_flow));
        }
        let mut paths = Vec::new();
        crate::tree::walk(&doc.root, &mut |_, path| paths.push(path.to_vec()));
        let got: Vec<_> = seen.into_iter().map(|(_, p)| p).collect();
        assert_eq!(got, paths);
    }

    #[test]
    fn in_scalar_is_strict() {
        let source = wrap("a: \"xy\"\n");
        let (_, doc) = document(&source, &[]);
        let quote = source.find('"').unwrap_or(0);
        assert!(!doc.in_scalar(quote));
        assert!(doc.in_scalar(quote + 1));
        assert!(!doc.in_scalar(quote + 5));
    }
}
