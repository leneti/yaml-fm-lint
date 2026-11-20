//! Reading files: for linting, only as much as the front matter needs.

use std::io::{BufRead, BufReader, Read};

use yaml_fm_lint_core::MAX_FRONT_MATTER_BYTES;

/// Why a file couldn't be read.
#[derive(Debug)]
pub enum ReadError {
    Io(std::io::Error),
    NotUtf8,
}

impl From<std::io::Error> for ReadError {
    fn from(e: std::io::Error) -> ReadError {
        ReadError::Io(e)
    }
}

const BOM: &[u8] = b"\xEF\xBB\xBF";

/// How much of a file is read at most: the longest front matter, and room for its delimiters.
const READ_LIMIT: u64 = MAX_FRONT_MATTER_BYTES as u64 + 65_536;

/// A line matching `^---[ \t]*\r?$`, with or without its `\n`.
fn is_delimiter(line: &[u8]) -> bool {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    line.strip_prefix(b"---")
        .is_some_and(|rest| rest.iter().all(|b| matches!(b, b' ' | b'\t')))
}

/// The start of the file that gives the same lint result as the whole file: the first line if
/// it doesn't open front matter, else everything up to the closing delimiter, or up to the
/// point where the front matter is known to be too large. What follows the front matter isn't
/// read, so it isn't checked for UTF-8.
// shortcut: a file whose opening delimiter line has over 64 KiB of trailing blanks is cut
// short of the size limit; fix by counting the limit from the end of that line.
pub fn read_for_lint(reader: impl Read) -> Result<String, ReadError> {
    let mut reader = BufReader::new(reader.take(READ_LIMIT));
    let mut buf = Vec::new();
    reader.read_until(b'\n', &mut buf)?;
    let first = buf.strip_prefix(BOM).unwrap_or(&buf);
    if is_delimiter(first) {
        let content_start = buf.len();
        loop {
            let line_start = buf.len();
            if reader.read_until(b'\n', &mut buf)? == 0 {
                break;
            }
            let line = buf.get(line_start..).unwrap_or_default();
            // A line that starts past the limit can't be a delimiter that counts.
            if is_delimiter(line) || buf.len() - content_start > MAX_FRONT_MATTER_BYTES {
                break;
            }
        }
    }
    let at_cap = reader.get_ref().limit() == 0;
    into_text(buf, at_cap)
}

/// The whole file.
pub fn read_whole(mut reader: impl Read) -> Result<String, ReadError> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    into_text(buf, false)
}

/// `buf` as text. When the read stopped at the cap, a character cut in two at the end is
/// dropped; any other invalid UTF-8 is an error.
fn into_text(buf: Vec<u8>, at_cap: bool) -> Result<String, ReadError> {
    match String::from_utf8(buf) {
        Ok(text) => Ok(text),
        Err(e) if at_cap && e.utf8_error().error_len().is_none() => {
            let valid = e.utf8_error().valid_up_to();
            let mut bytes = e.into_bytes();
            bytes.truncate(valid);
            String::from_utf8(bytes).map_err(|_| ReadError::NotUtf8)
        }
        Err(_) => Err(ReadError::NotUtf8),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use std::path::Path;

    use yaml_fm_lint_core::{Config, Linter};

    use super::*;

    fn linter() -> Linter {
        Linter::new(Config::default(), None).unwrap()
    }

    fn read(bytes: &[u8]) -> String {
        read_for_lint(bytes).unwrap()
    }

    /// The prefix lints like the whole text.
    fn assert_same(full: &str) {
        let linter = linter();
        let prefix = read(full.as_bytes());
        assert!(full.starts_with(&prefix));
        assert_eq!(linter.lint(&prefix), linter.lint(full));
    }

    #[test]
    fn prefix_lints_the_same_on_fixtures() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
        let mut count = 0;
        for folder in ["1x", "edge"] {
            for entry in std::fs::read_dir(root.join(folder)).unwrap() {
                let path = entry.unwrap().path();
                if path.extension().is_some_and(|e| e == "md") {
                    assert_same(&std::fs::read_to_string(path).unwrap());
                    count += 1;
                }
            }
        }
        assert!(count >= 30, "found only {count} fixtures");
    }

    #[test]
    fn prefix_lints_the_same_on_generated_cases() {
        let ten_mb = "body line\n".repeat(1_000_000);
        let max = MAX_FRONT_MATTER_BYTES;
        for full in [
            format!("hello\n{ten_mb}"),
            format!("---\na: 1\n{}", "b: 2\n".repeat(600_000)),
            format!("---\n{}", "a: 1\n".repeat(210_000)),
            // Exactly the limit in content bytes, then the delimiter.
            format!("---\n{}---\nbody", "a".repeat(max - 1) + "\n"),
            // One byte over.
            format!("---\n{}---\nbody", "a".repeat(max) + "\n"),
            format!("\u{FEFF}---\na: 1\n---\n{ten_mb}"),
            "---\r\na: \"x\"\r\n---\r\nbody\r\n".to_string(),
            "---   \t\r\na: 1\r\n---\t\r\n".to_string(),
            // A 4-byte character across the read cap.
            format!("---\n{}😀\n", "a".repeat(max + 65_530)),
        ] {
            assert_same(&full);
        }
    }

    #[test]
    fn stops_early() {
        let ten_mb = "x".repeat(10_000_000);
        assert_eq!(read(format!("hello\n{ten_mb}").as_bytes()), "hello\n");
        assert_eq!(
            read(format!("---\na: 1\n---\n{ten_mb}").as_bytes()),
            "---\na: 1\n---\n"
        );
    }

    #[test]
    fn body_utf8_not_checked() {
        let bytes = b"---\na: 1\n---\n\xff";
        assert_eq!(read(bytes), "---\na: 1\n---\n");
        assert!(matches!(read_whole(&bytes[..]), Err(ReadError::NotUtf8)));
    }

    #[test]
    fn invalid_utf8_inside_front_matter_is_an_error() {
        assert!(matches!(
            read_for_lint(&b"---\na: \xff\n---\n"[..]),
            Err(ReadError::NotUtf8)
        ));
    }

    #[test]
    fn a_cut_character_is_only_dropped_at_the_cap() {
        // The file itself ends in half a character: not valid.
        assert!(matches!(
            read_for_lint(&b"hello\xF0\x9F"[..]),
            Err(ReadError::NotUtf8)
        ));
    }
}
