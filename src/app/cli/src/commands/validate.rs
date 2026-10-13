//! `functor validate [ARTIFACT] [--strict]`.

use functor_core::{FunctorError, RepositoryReader, Result, ValidationEngine, ValidationTarget};
use functor_model::{ArtifactRef, EntityKind, ValidationReport};
use functor_store::FileStore;
use functor_validation::RuleValidationEngine;

use crate::cli::ValidateArgs;
use crate::context::{self, Globals, Services};
use crate::events::{self, Recorder};
use crate::exit::validation_exit_code;
use crate::output::print_result;
use crate::render::{ValidateDocument, validate_text};
use crate::resolve;

/// Adds "did you mean" suggestions to an unknown scoped artifact. Best effort: if the repository
/// cannot be loaded the plain error is kept.
async fn with_suggestions(store: &FileStore, error: FunctorError) -> FunctorError {
    if let FunctorError::NotFound {
        entity: EntityKind::Artifact,
        reference,
        ..
    } = &error
        && let Ok(load) = store.load().await
    {
        let suggestions = resolve::suggest(
            reference,
            load.artifacts
                .iter()
                .map(|artifact| artifact.slug().as_str()),
        );
        return FunctorError::NotFound {
            entity: EntityKind::Artifact,
            reference: reference.clone(),
            suggestions,
        };
    }
    error
}

/// Validates the repository, or the findings about one artifact.
///
/// # Errors
/// `NotARepository` outside a repository, `InvalidInput` for a bad reference, `NotFound` for an
/// artifact that did not load cleanly, or a storage error. Problems *in* the repository are not
/// errors: they are findings of the returned report.
pub async fn validate(
    store: &FileStore,
    services: &Services,
    args: &ValidateArgs,
) -> Result<ValidationReport> {
    let target = match &args.artifact {
        Some(text) => ValidationTarget::Artifact(ArtifactRef::parse(text)?),
        None => ValidationTarget::Repository,
    };
    let engine = RuleValidationEngine::new(store, std::sync::Arc::clone(&services.clock));
    match engine.validate(target).await {
        Ok(report) => Ok(report),
        Err(error) => Err(with_suggestions(store, error).await),
    }
}

fn output_error(error: std::io::Error) -> FunctorError {
    FunctorError::storage("write output", "stdout", error)
}

/// Runs `validate`, prints the report and returns the exit code its outcome maps to.
///
/// # Errors
/// See [`validate`].
pub async fn run(args: ValidateArgs, globals: &Globals, services: &Services) -> Result<u8> {
    let store = context::open_unchecked(globals, services)?;
    let report = validate(&store, services, &args).await?;

    // The run always publishes `validation.completed`, even when it found problems; it cannot
    // when the repository identity is unreadable.
    if let Ok(info) = store.info().await {
        Recorder.record(events::validation_completed(
            *info.id(),
            &report,
            args.strict,
        ));
    }

    let document = ValidateDocument::new(&report, args.strict)?;
    let text = validate_text(&report, args.artifact.as_deref());
    print_result(&text, &document, globals.output).map_err(output_error)?;
    Ok(validation_exit_code(report.outcome(), args.strict))
}
