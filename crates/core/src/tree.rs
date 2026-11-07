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
    /// The path from the root.
    pub path: Vec<PathStep>,
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
