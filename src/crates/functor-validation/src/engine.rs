//! The asynchronous engine: one lenient load per call, then the pure evaluation.

use std::sync::Arc;

use async_trait::async_trait;
use functor_core::{Clock, RepositoryReader, Result, ValidationEngine, ValidationTarget};
use functor_model::ValidationReport;

use crate::evaluate::evaluate;

/// A [`ValidationEngine`] that validates a fresh lenient load of any [`RepositoryReader`].
///
/// Validation never modifies the repository. The report is stamped with the injected clock.
pub struct RuleValidationEngine<'a, R: ?Sized> {
    reader: &'a R,
    clock: Arc<dyn Clock>,
}

impl<R: ?Sized> std::fmt::Debug for RuleValidationEngine<'_, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuleValidationEngine")
            .finish_non_exhaustive()
    }
}

impl<'a, R: RepositoryReader + ?Sized> RuleValidationEngine<'a, R> {
    /// Creates an engine over `reader` that stamps reports with `clock`.
    #[must_use]
    pub fn new(reader: &'a R, clock: Arc<dyn Clock>) -> Self {
        Self { reader, clock }
    }
}

#[async_trait]
impl<R: RepositoryReader + ?Sized> ValidationEngine for RuleValidationEngine<'_, R> {
    async fn validate(&self, target: ValidationTarget) -> Result<ValidationReport> {
        let load = self.reader.load().await?;
        evaluate(&load, &target, self.clock.now())
    }
}
