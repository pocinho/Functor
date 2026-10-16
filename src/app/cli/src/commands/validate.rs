//! `functor validate [ARTIFACT] [--strict] [--save-report]`.

use std::sync::Arc;

use functor_core::{
    FunctorError, ReportSubject, RepositoryReader, Result, SnapshotLoad, ValidationEngine,
    ValidationTarget,
};
use functor_model::{ArtifactRef, EntityKind, ValidationReport};
use functor_store::{FileEvidence, FileStore};
use functor_validation::RuleValidationEngine;
use sha2::{Digest, Sha256};

use crate::cli::ValidateArgs;
use crate::context::{self, Globals, Services};
use crate::events;
use crate::exit::validation_exit_code;
use crate::output::{print_result, report_error_code};
use crate::render::{SavedReportDto, ValidateDocument, validate_text};
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

async fn subject(
    store: &FileStore,
    load: &SnapshotLoad,
    services: &Services,
    reference: Option<&str>,
) -> Result<ReportSubject> {
    let Some(reference) = reference else {
        return Ok(ReportSubject::Repository);
    };
    let reference = ArtifactRef::parse(reference)?;
    let artifact = load
        .artifacts
        .iter()
        .find(|a| match &reference {
            ArtifactRef::Id(id) => a.id() == id,
            ArtifactRef::Slug(slug) => a.slug() == slug,
        })
        .ok_or_else(|| FunctorError::not_found(EntityKind::Artifact, reference.to_string()))?;
    let evidence = FileEvidence::open(
        store.clone(),
        Arc::clone(&services.clock),
        Arc::clone(&services.ids),
    );
    let digest = context::content(&evidence, artifact)
        .await?
        .map(|bytes| functor_model::ContentDigest::from_bytes(Sha256::digest(&bytes).into()));
    Ok(ReportSubject::Artifact {
        id: *artifact.id(),
        revision: artifact.revision(),
        digest,
    })
}

/// Runs `validate`, prints the report and returns the exit code its outcome maps to.
///
/// # Errors
/// See [`validate`].
pub async fn run(args: ValidateArgs, globals: &Globals, services: &Services) -> Result<u8> {
    let store = context::open_unchecked(globals, services)?;
    // Saving describes the same loaded state the rules evaluated, not a second artifact read.
    let (report, saved_subject) = if args.save_report {
        let load = store.load().await?;
        let target = match &args.artifact {
            Some(text) => ValidationTarget::Artifact(ArtifactRef::parse(text)?),
            None => ValidationTarget::Repository,
        };
        let report = match functor_validation::evaluate(&load, &target, services.clock.now()) {
            Ok(report) => report,
            Err(error) => return Err(with_suggestions(&store, error).await),
        };
        let subject = subject(&store, &load, services, args.artifact.as_deref()).await;
        (report, Some(subject))
    } else {
        (validate(&store, services, &args).await?, None)
    };

    // The run always publishes `validation.completed`, even when it found problems; it cannot
    // when the repository identity is unreadable.
    if let Ok(info) = store.info().await {
        let recorder = context::event_recorder(store.layout().root(), *info.id(), services)?;
        recorder
            .record(events::validation_completed(
                *info.id(),
                &report,
                args.strict,
            ))
            .await;
    }

    let mut document = ValidateDocument::new(&report, args.strict)?;
    let mut text = validate_text(&report, args.artifact.as_deref());
    let mut save_error = None;
    if let Some(subject) = saved_subject {
        let saved = async {
            let subject = subject?;
            let execution = context::execution(store.clone(), services).await?;
            execution
                .orchestrator
                .record_validation(&report, subject, args.strict, super::run::RULES)
                .await
        }
        .await;
        match saved {
            Ok(saved) => {
                super::run::warnings(&saved.warnings);
                text = format!("{text}\nsaved_report: {}", saved.id);
                document.saved_report = Some(SavedReportDto {
                    id: Some(saved.id.to_string()),
                    error: None,
                });
            }
            Err(error) => {
                document.saved_report = Some(SavedReportDto {
                    id: None,
                    error: Some(error.to_string()),
                });
                text.push_str("\nsaved_report: not saved");
                save_error = Some(error);
            }
        }
    }
    print_result(&text, &document, globals.output).map_err(output_error)?;
    if let Some(error) = save_error {
        report_error_code(
            &error,
            globals.output,
            globals.verbose,
            crate::exit::STORAGE,
        );
        return Ok(crate::exit::STORAGE);
    }
    Ok(validation_exit_code(report.outcome(), args.strict))
}
