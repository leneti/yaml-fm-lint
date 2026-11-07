//! `require-front-matter`: the file must start with a front-matter block.
//!
//! # Why
//!
//! Tools that read the front matter see nothing when the file has none, or when
//! the block is never closed, and the cause is easy to miss by eye.
//!
//! # Examples
//!
//! Bad:
//!
//! ```yaml
//! title: Hello
//! ```
//!
//! Bad, because the opening `---` is never closed:
//!
//! ```yaml
//! ---
//! title: Hello
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

/// The file does not start with `---`.
pub(crate) const MISSING: &str = "no front matter (the first line must be ---)";

/// The opening `---` has no closing `---`.
pub(crate) const UNCLOSED: &str = "opening --- is never closed";
