//! `input-limit`: the front matter must stay within the size and complexity limits.
//!
//! # Why
//!
//! Huge or deeply nested input would make the linter slow or run it out of
//! memory, so the linter refuses it instead. The rule can't be turned off.
//!
//! # Examples
//!
//! Bad, because the front matter is larger than 1 MiB:
//!
//! ```yaml
//! ---
//! # more than 1,048,576 bytes before the closing line
//! ---
//! ```
//!
//! Good:
//!
//! ```yaml
//! ---
//! title: Hello
//! ---
//! ```
//!
//! # Fix
//!
//! No fix.

/// No closing `---` within `MAX_FRONT_MATTER_BYTES` of content.
pub(crate) const TOO_LARGE: &str = "front matter is larger than 1 MiB (1,048,576 bytes)";

/// Collections nest deeper than `parse::MAX_DEPTH`.
pub(crate) const TOO_DEEP: &str = "nesting is deeper than 128 levels";

/// Aliases expand to more than `data::MAX_EXPANDED_NODES` nodes.
pub(crate) const TOO_MANY_NODES: &str = "aliases expand to more than 100,000 nodes";
