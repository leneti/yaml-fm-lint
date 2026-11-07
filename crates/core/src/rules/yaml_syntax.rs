//! `yaml-syntax`: the front matter must be a single, valid YAML mapping.
//!
//! # Why
//!
//! Front matter that doesn't parse can't be read by any tool, and the other
//! rules can't say anything useful about it. Duplicate keys and a front matter
//! that isn't a key/value mapping are reported here too. The rule can't be
//! turned off.
//!
//! # Examples
//!
//! Bad:
//!
//! ```yaml
//! ---
//! title: [unclosed
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
