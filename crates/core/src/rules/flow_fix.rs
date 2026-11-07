//! The fix shared by the flow-collection rules: `[...]` and `{...}` to block style.

use crate::document::{NodeRef, Role};
use crate::lint::Context;
use crate::tree::{Node, NodeKind};
use crate::{Applicability, DataEffect, Fix, TextEdit, TextRange};

/// The column (in characters) of `offset` on its line.
fn column(ctx: &Context<'_>, offset: usize) -> Option<usize> {
    let line = ctx.doc.lines.get(ctx.doc.line_index(offset)?)?;
    Some(ctx.source.get(line.range.start..offset)?.chars().count())
}

/// The block lines of a flow collection, one per item; `None` if one cannot be written as a
/// single line from its source text.
fn block_lines(source: &str, node: &Node) -> Option<Vec<String>> {
    let NodeKind::Sequence { items, .. } = &node.kind else {
        return None;
    };
    items
        .iter()
        .map(|item| {
            if !matches!(item.kind, NodeKind::Scalar { .. }) {
                return None;
            }
            let text = source.get(item.range.start..item.range.end)?;
            (!text.contains('\n')).then(|| format!("- {text}"))
        })
        .collect()
}

/// A fix that rewrites the flow collection `r` in block style, if that is safe.
///
/// The collection must be the value of a block mapping entry or an item of a block sequence,
/// outside any flow collection, with no anchor, tag or comment on it. A value moves to
/// the lines below its key, indented by the `indentation` size; an item uses the compact form
/// (its first line stays after the parent's `- `). Lines use the file's line ending.
pub(crate) fn flow_to_block(ctx: &Context<'_>, r: &NodeRef<'_>) -> Option<Fix> {
    let node = r.node;
    if r.in_flow || node.anchor.is_some() || node.has_tag {
        return None;
    }
    let lines = block_lines(ctx.source, node)?;
    // Where the replaced text starts, the indent of the lines, and whether the first line
    // goes on a new line (a value) or stays where the collection was (an item).
    let (start, indent, new_line) = match (r.role, r.entry, r.parent) {
        (Role::Value, Some(entry), Some(parent))
            if matches!(parent.kind, NodeKind::Mapping { flow: false, .. }) =>
        {
            let key_column = column(ctx, entry.key.range.start)?;
            (
                entry.colon? + 1,
                key_column + usize::from(ctx.config.indent_size),
                true,
            )
        }
        (Role::Item, _, Some(parent))
            if matches!(parent.kind, NodeKind::Sequence { flow: false, .. }) =>
        {
            (
                node.range.start,
                column(ctx, parent.range.start)? + 2,
                false,
            )
        }
        _ => return None,
    };
    // A comment in the replaced text would be lost; one after it on the last line would end
    // up after the last item only.
    let rest = ctx.source.get(node.range.end..)?;
    let line_end = node.range.end + rest.find('\n').unwrap_or(rest.len());
    if ctx
        .doc
        .comments
        .iter()
        .any(|c| (start..=line_end).contains(&c.range.start))
    {
        return None;
    }
    let separator = format!("{}{}", ctx.line_ending, " ".repeat(indent));
    let block = lines.join(&separator);
    Some(Fix {
        applicability: Applicability::Automatic,
        data_effect: DataEffect::Preserving,
        edits: vec![TextEdit {
            range: TextRange {
                start,
                end: node.range.end,
            },
            new_text: if new_line {
                format!("{separator}{block}")
            } else {
                block
            },
        }],
    })
}
