//! Process exit codes and the single mapping from errors to them.
//!
//! The codes are part of the stable CLI contract; see `docs/error-handling.md`. New codes are
//! only added, never reassigned.

use functor_core::ErrorKind;

/// The command succeeded.
pub const SUCCESS: u8 = 0;
/// An unexpected internal error.
pub const INTERNAL: u8 = 1;
/// A usage or input error, including argument parsing failures.
pub const USAGE: u8 = 2;
/// A referenced entity was not found.
pub const NOT_FOUND: u8 = 3;
/// The operation conflicts with existing state.
pub const CONFLICT: u8 = 4;
/// Not a repository, an unsupported format, or inconsistent repository data.
pub const REPOSITORY: u8 = 5;
/// Validation failed; this is an outcome, not an error kind.
pub const VALIDATION_FAILED: u8 = 6;
/// A storage or locking failure.
pub const STORAGE: u8 = 7;

/// Maps an error kind to its exit code.
///
/// The match is deliberately free of a wildcard arm: adding an [`ErrorKind`] fails compilation
/// here until its exit code is decided.
#[must_use]
pub fn exit_code(kind: ErrorKind) -> u8 {
    match kind {
        ErrorKind::Internal => INTERNAL,
        ErrorKind::InvalidInput => USAGE,
        ErrorKind::NotFound => NOT_FOUND,
        ErrorKind::Conflict => CONFLICT,
        ErrorKind::NotARepository | ErrorKind::UnsupportedFormat | ErrorKind::Inconsistent => {
            REPOSITORY
        }
        ErrorKind::Storage | ErrorKind::Locked => STORAGE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_kind_has_the_documented_exit_code() {
        let expected = [
            (ErrorKind::NotFound, 3),
            (ErrorKind::Conflict, 4),
            (ErrorKind::InvalidInput, 2),
            (ErrorKind::NotARepository, 5),
            (ErrorKind::UnsupportedFormat, 5),
            (ErrorKind::Inconsistent, 5),
            (ErrorKind::Locked, 7),
            (ErrorKind::Storage, 7),
            (ErrorKind::Internal, 1),
        ];
        // The table must list every kind, in the same order as `ErrorKind::ALL`.
        let kinds: Vec<ErrorKind> = expected.iter().map(|(kind, _)| *kind).collect();
        assert_eq!(kinds, ErrorKind::ALL);
        for (kind, code) in expected {
            assert_eq!(exit_code(kind), code, "{kind:?}");
        }
    }

    #[test]
    fn named_constants_match_the_contract() {
        assert_eq!(
            [
                SUCCESS,
                INTERNAL,
                USAGE,
                NOT_FOUND,
                CONFLICT,
                REPOSITORY,
                VALIDATION_FAILED,
                STORAGE
            ],
            [0, 1, 2, 3, 4, 5, 6, 7]
        );
    }
}
