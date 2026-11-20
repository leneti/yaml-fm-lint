//! Plain words for I/O errors: no operating system error numbers, no paths.

use std::io::{Error, ErrorKind};

/// Why `e` happened, in a few words.
pub fn reason(e: &Error) -> &'static str {
    match e.kind() {
        ErrorKind::NotFound => "no such file or directory",
        ErrorKind::PermissionDenied => "permission denied",
        ErrorKind::IsADirectory => "is a directory",
        ErrorKind::NotADirectory => "not a directory",
        ErrorKind::ReadOnlyFilesystem => "read-only file system",
        ErrorKind::StorageFull => "no space left on device",
        _ => "input/output error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_become_plain_words() {
        for (kind, text) in [
            (ErrorKind::NotFound, "no such file or directory"),
            (ErrorKind::PermissionDenied, "permission denied"),
            (ErrorKind::IsADirectory, "is a directory"),
            (ErrorKind::NotADirectory, "not a directory"),
            (ErrorKind::ReadOnlyFilesystem, "read-only file system"),
            (ErrorKind::StorageFull, "no space left on device"),
            (ErrorKind::BrokenPipe, "input/output error"),
        ] {
            assert_eq!(reason(&Error::from(kind)), text);
        }
    }

    /// A message with a path and an error number in it never gets through.
    #[test]
    fn os_details_are_dropped() {
        let e = Error::new(
            ErrorKind::PermissionDenied,
            "/tmp/.tmpAb12: denied (os error 13)",
        );
        assert_eq!(reason(&e), "permission denied");
    }
}
