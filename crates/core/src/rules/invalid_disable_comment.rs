//! `invalid-disable-comment`: a disable comment must name rules that exist and can be disabled.
//!
//! # Why
//!
//! A misspelled rule name silently disables nothing, so the problem it was meant
//! to hide stays visible and the comment looks like it works.
//!
//! # Examples
//!
//! Bad, because there is no such rule:
//!
//! ```yaml
//! # fmlint-disable-next-line not-a-rule
//! title: Hello
//! ```
//!
//! Bad, because `yaml-syntax` can't be disabled:
//!
//! ```yaml
//! # fmlint-disable-line yaml-syntax
//! ```
//!
//! Good:
//!
//! ```yaml
//! # fmlint-disable-next-line no-trailing-spaces
//! title: Hello
//! ```
//!
//! # Fix
//!
//! No fix.

/// The message for a rule name that does not exist.
pub(crate) fn unknown(id: &str) -> String {
    format!("unknown rule `{id}` in disable comment")
}

/// The message for a rule that exists but can't be disabled.
pub(crate) fn cant_disable(id: &str) -> String {
    format!("rule `{id}` can't be disabled")
}
