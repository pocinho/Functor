//! Error type for model constructors.

/// A value violated a model invariant.
///
/// Mirrors `FunctorError::InvalidInput` in `functor-core`: the `field` names what was being
/// built and `reason` explains the violation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid {field}: {reason}")]
#[non_exhaustive]
pub struct ModelError {
    /// The name of the offending field or value kind, for example `"slug"`.
    pub field: &'static str,
    /// Why the value was rejected.
    pub reason: String,
}

impl ModelError {
    pub(crate) fn new(field: &'static str, reason: impl Into<String>) -> Self {
        Self {
            field,
            reason: reason.into(),
        }
    }
}

pub(crate) type Result<T> = std::result::Result<T, ModelError>;
