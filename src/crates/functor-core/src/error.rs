//! The error taxonomy of the Functor API.
//!
//! See `docs/architecture/error-handling.md`. Validation findings are **not** errors: a validation run that
//! finds problems succeeds and reports them.

use std::path::PathBuf;

use functor_model::{EntityKind, ModelError, SnapshotError};

/// The result type of fallible core operations.
pub type Result<T> = std::result::Result<T, FunctorError>;

/// An operation could not be performed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FunctorError {
    /// A referenced entity does not exist.
    #[error("{entity} not found: {reference}")]
    NotFound {
        /// The kind of entity that was looked up.
        entity: EntityKind,
        /// The reference the user supplied (slug, id or similar).
        reference: String,
        /// Close alternatives to offer, for example similar slugs.
        suggestions: Vec<String>,
    },

    /// The operation conflicts with existing state: duplicate slug, stale revision, duplicate
    /// relationship, an existing repository. The message is the detail alone; `entity` is data
    /// for projections.
    #[error("{detail}")]
    Conflict {
        /// The kind of entity involved.
        entity: EntityKind,
        /// What conflicts.
        detail: String,
    },

    /// An input violated a grammar or constraint.
    #[error("invalid {field}: {reason}")]
    InvalidInput {
        /// The field or value kind.
        field: &'static str,
        /// Why it was rejected.
        reason: String,
    },

    /// No repository was found at or above the path.
    #[error("not a functor repository: {}", path.display())]
    NotARepository {
        /// Where the search started.
        path: PathBuf,
    },

    /// The repository was written with a format this release cannot read.
    #[error("unsupported repository format version {found}; supported: {supported}")]
    UnsupportedFormat {
        /// The version found on disk.
        found: u32,
        /// The version this release supports.
        supported: u32,
    },

    /// Repository data is inconsistent, so a strict load failed.
    #[error("repository data is inconsistent: {detail}; run `functor validate`")]
    Inconsistent {
        /// What is wrong, as far as the loader found out.
        detail: String,
    },

    /// Another process holds the repository write lock.
    #[error("repository is locked by another process: {}", path.display())]
    Locked {
        /// The lock file.
        path: PathBuf,
    },

    /// A filesystem operation failed.
    #[error("storage error during {operation} on {}", path.display())]
    Storage {
        /// What was being attempted, for example `write`.
        operation: &'static str,
        /// The file or directory involved.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },

    /// A bug: this should never be caused by user input.
    #[error("internal error: {0}")]
    Internal(String),

    /// The caller has not been granted permission for an action, or a policy forbids it: no
    /// execution grant, a disallowed environment variable, a path outside the boundary
    /// (ADR-0012). Nothing was done.
    #[error("permission denied for {action}: {reason}")]
    PermissionDenied {
        /// What was attempted, for example `execute`.
        action: String,
        /// Why it was refused.
        reason: String,
    },

    /// The request names something this implementation does not support: an unknown provider, a
    /// newer workflow schema, a network policy it cannot enforce (ADR-0012).
    #[error("unsupported: {what}")]
    Unsupported {
        /// What is not supported.
        what: String,
    },
}

/// The variant of a [`FunctorError`] without its data.
///
/// Projections map kinds to exit codes and wire codes; because the enum is exhaustive, adding a
/// kind forces every mapping to be updated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// [`FunctorError::NotFound`]
    NotFound,
    /// [`FunctorError::Conflict`]
    Conflict,
    /// [`FunctorError::InvalidInput`]
    InvalidInput,
    /// [`FunctorError::NotARepository`]
    NotARepository,
    /// [`FunctorError::UnsupportedFormat`]
    UnsupportedFormat,
    /// [`FunctorError::Inconsistent`]
    Inconsistent,
    /// [`FunctorError::Locked`]
    Locked,
    /// [`FunctorError::Storage`]
    Storage,
    /// [`FunctorError::Internal`]
    Internal,
    /// [`FunctorError::PermissionDenied`]
    PermissionDenied,
    /// [`FunctorError::Unsupported`]
    Unsupported,
}

impl ErrorKind {
    /// Every kind, in declaration order.
    pub const ALL: [Self; 11] = [
        Self::NotFound,
        Self::Conflict,
        Self::InvalidInput,
        Self::NotARepository,
        Self::UnsupportedFormat,
        Self::Inconsistent,
        Self::Locked,
        Self::Storage,
        Self::Internal,
        Self::PermissionDenied,
        Self::Unsupported,
    ];

    /// The stable `snake_case` code used in JSON error output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::InvalidInput => "invalid_input",
            Self::NotARepository => "not_a_repository",
            Self::UnsupportedFormat => "unsupported_format",
            Self::Inconsistent => "inconsistent",
            Self::Locked => "locked",
            Self::Storage => "storage",
            Self::Internal => "internal",
            Self::PermissionDenied => "permission_denied",
            Self::Unsupported => "unsupported",
        }
    }
}

impl FunctorError {
    /// The kind of this error.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::NotFound { .. } => ErrorKind::NotFound,
            Self::Conflict { .. } => ErrorKind::Conflict,
            Self::InvalidInput { .. } => ErrorKind::InvalidInput,
            Self::NotARepository { .. } => ErrorKind::NotARepository,
            Self::UnsupportedFormat { .. } => ErrorKind::UnsupportedFormat,
            Self::Inconsistent { .. } => ErrorKind::Inconsistent,
            Self::Locked { .. } => ErrorKind::Locked,
            Self::Storage { .. } => ErrorKind::Storage,
            Self::Internal(_) => ErrorKind::Internal,
            Self::PermissionDenied { .. } => ErrorKind::PermissionDenied,
            Self::Unsupported { .. } => ErrorKind::Unsupported,
        }
    }

    /// A permission-denied error.
    #[must_use]
    pub fn permission_denied(action: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::PermissionDenied {
            action: action.into(),
            reason: reason.into(),
        }
    }

    /// An unsupported error.
    #[must_use]
    pub fn unsupported(what: impl Into<String>) -> Self {
        Self::Unsupported { what: what.into() }
    }

    /// A not-found error without suggestions.
    #[must_use]
    pub fn not_found(entity: EntityKind, reference: impl Into<String>) -> Self {
        Self::NotFound {
            entity,
            reference: reference.into(),
            suggestions: Vec::new(),
        }
    }

    /// A conflict error.
    #[must_use]
    pub fn conflict(entity: EntityKind, detail: impl Into<String>) -> Self {
        Self::Conflict {
            entity,
            detail: detail.into(),
        }
    }

    /// An invalid-input error.
    #[must_use]
    pub fn invalid_input(field: &'static str, reason: impl Into<String>) -> Self {
        Self::InvalidInput {
            field,
            reason: reason.into(),
        }
    }

    /// A storage error for a failed filesystem operation.
    #[must_use]
    pub fn storage(
        operation: &'static str,
        path: impl Into<PathBuf>,
        source: std::io::Error,
    ) -> Self {
        Self::Storage {
            operation,
            path: path.into(),
            source,
        }
    }
}

impl From<ModelError> for FunctorError {
    fn from(error: ModelError) -> Self {
        Self::InvalidInput {
            field: error.field,
            reason: error.reason,
        }
    }
}

impl From<SnapshotError> for FunctorError {
    fn from(error: SnapshotError) -> Self {
        Self::Inconsistent {
            detail: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::io;

    use functor_model::{ArtifactId, Slug};

    use super::*;

    fn one_of_each() -> Vec<FunctorError> {
        vec![
            FunctorError::not_found(EntityKind::Artifact, "rfc-9"),
            FunctorError::conflict(EntityKind::Artifact, "slug `a` is taken"),
            FunctorError::invalid_input("slug", "must not be empty"),
            FunctorError::NotARepository {
                path: PathBuf::from("/tmp/x"),
            },
            FunctorError::UnsupportedFormat {
                found: 2,
                supported: 1,
            },
            FunctorError::Inconsistent {
                detail: "two artifacts share an id".into(),
            },
            FunctorError::Locked {
                path: PathBuf::from("/tmp/x/.functor/lock"),
            },
            FunctorError::storage("write", "/tmp/x/a.toml", io::Error::other("disk full")),
            FunctorError::Internal("oops".into()),
            FunctorError::permission_denied("execute", "no execution grant"),
            FunctorError::unsupported("provider `remote`"),
        ]
    }

    #[test]
    fn every_variant_maps_to_a_distinct_kind() {
        let kinds: Vec<ErrorKind> = one_of_each().iter().map(FunctorError::kind).collect();
        assert_eq!(kinds, ErrorKind::ALL);
        let unique: HashSet<_> = kinds.iter().collect();
        assert_eq!(unique.len(), ErrorKind::ALL.len());
    }

    #[test]
    fn wire_codes_are_stable_and_unique() {
        let codes: Vec<&str> = ErrorKind::ALL.iter().map(|k| k.as_str()).collect();
        assert_eq!(
            codes,
            [
                "not_found",
                "conflict",
                "invalid_input",
                "not_a_repository",
                "unsupported_format",
                "inconsistent",
                "locked",
                "storage",
                "internal",
                "permission_denied",
                "unsupported"
            ]
        );
    }

    #[test]
    fn messages_follow_the_documented_wording() {
        let messages: Vec<String> = one_of_each().iter().map(ToString::to_string).collect();
        assert_eq!(messages[0], "artifact not found: rfc-9");
        assert_eq!(messages[2], "invalid slug: must not be empty");
        assert!(messages[3].starts_with("not a functor repository: "));
        assert_eq!(
            messages[4],
            "unsupported repository format version 2; supported: 1"
        );
        assert_eq!(
            messages[5],
            "repository data is inconsistent: two artifacts share an id; run `functor validate`"
        );
        assert!(messages[6].starts_with("repository is locked by another process: "));
        assert!(messages[7].starts_with("storage error during write on "));
        assert_eq!(messages[8], "internal error: oops");
        assert_eq!(
            messages[9],
            "permission denied for execute: no execution grant"
        );
        assert_eq!(messages[10], "unsupported: provider `remote`");
    }

    #[test]
    fn storage_errors_expose_their_source() {
        let error = FunctorError::storage("write", "x", io::Error::other("boom"));
        let source = std::error::Error::source(&error).expect("source");
        assert_eq!(source.to_string(), "boom");
    }

    #[test]
    fn model_errors_become_invalid_input() {
        let error: FunctorError = Slug::parse("Bad").unwrap_err().into();
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
        assert!(error.to_string().starts_with("invalid slug:"));
    }

    #[test]
    fn snapshot_errors_become_inconsistent() {
        let error: FunctorError =
            SnapshotError::DuplicateArtifactId(ArtifactId::from_bits(1)).into();
        assert_eq!(error.kind(), ErrorKind::Inconsistent);
    }
}
