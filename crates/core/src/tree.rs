//! The parsed front matter as a tree of nodes with byte ranges.

use crate::{PathStep, TextRange};

/// One node of the parsed front matter.
///
/// `range` is a byte range into the whole source:
/// - a quoted scalar includes its quotes;
/// - a block scalar runs from its `|`/`>` header to just after the line ending of its last
///   kept line (trailing blank lines count only with `+` chomping);
/// - a flow collection runs from `[`/`{` to just after `]`/`}`;
/// - a block collection runs from its first key or `-` to the end of its last value;
/// - a null value is an empty range just after the `:` (or `-`);
/// - anchors and tags are not part of the range.
///
/// The path from the root is not stored; [`walk`] hands it out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Node {
    /// What the node is.
    pub kind: NodeKind,
    /// Where the node is.
    pub range: TextRange,
    /// The anchor this node defines, if any.
    pub anchor: Option<usize>,
    /// The node is tagged `!!str`.
    pub tagged_str: bool,
    /// The node has any tag.
    pub has_tag: bool,
}

/// What a [`Node`] is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NodeKind {
    /// A scalar with its decoded text.
    Scalar { value: String, style: Style },
    /// A sequence; `flow` is true for `[...]` and for sequences inside flow collections.
    Sequence { flow: bool, items: Vec<Node> },
    /// A mapping; `flow` is true for `{...}` and for the single pairs inside `[...]`.
    Mapping { flow: bool, entries: Vec<Entry> },
    /// An alias (`*name`) of the anchor with this id.
    Alias { anchor: usize },
}

/// How a scalar is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Style {
    /// Unquoted.
    Plain,
    /// `'...'`.
    SingleQuoted,
    /// `"..."`.
    DoubleQuoted,
    /// `|`.
    Literal,
    /// `>`.
    Folded,
}

/// One key/value pair of a mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    /// The key.
    pub key: Node,
    /// The value.
    pub value: Node,
    /// The byte offset of the `:` indicator; `None` when the entry has no `:`.
    pub colon: Option<usize>,
    /// The entry starts with `?` followed by a space.
    pub explicit: bool,
}

/// One open collection of [`walk`] and the next child to visit.
struct Cursor<'a> {
    node: &'a Node,
    next: usize,
}

/// Visits `root` and everything below it in pre-order (keys before values), handing `f` each
/// node with its path from the root. A key's path ends in `Key(i)`, its value's in `Value(i)`.
///
/// One shared path stack and an explicit cursor stack: memory is O(depth), not O(nodes * depth).
#[allow(dead_code)] // first caller lands with the rules
pub(crate) fn walk<'a>(root: &'a Node, f: &mut impl FnMut(&'a Node, &[PathStep])) {
    let mut path = Vec::new();
    f(root, &path);
    let mut stack = vec![Cursor {
        node: root,
        next: 0,
    }];
    while let Some(top) = stack.last_mut() {
        let i = top.next;
        top.next += 1;
        let child = match &top.node.kind {
            NodeKind::Sequence { items, .. } => items.get(i).map(|n| (n, PathStep::Item(i))),
            NodeKind::Mapping { entries, .. } => entries.get(i / 2).map(|e| {
                if i % 2 == 0 {
                    (&e.key, PathStep::Key(i / 2))
                } else {
                    (&e.value, PathStep::Value(i / 2))
                }
            }),
            NodeKind::Scalar { .. } | NodeKind::Alias { .. } => None,
        };
        match child {
            None => {
                stack.pop();
                path.pop();
            }
            Some((node, step)) => {
                path.push(step);
                f(node, &path);
                if matches!(
                    node.kind,
                    NodeKind::Sequence { .. } | NodeKind::Mapping { .. }
                ) {
                    stack.push(Cursor { node, next: 0 });
                } else {
                    path.pop();
                }
            }
        }
    }
}
