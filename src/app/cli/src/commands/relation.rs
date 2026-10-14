//! `functor relation add|list|show`.

use functor_core::{ArtifactRepository, FunctorError, RelationshipRepository, Result};
use functor_model::{Metadata, NewRelationship, Relationship, RelationshipType};

use super::artifact::parse_pair;
use crate::cli::{RelationAddArgs, RelationCommand, RelationListArgs, RelationShowArgs};
use crate::context::{Context, Globals};
use crate::events::{self, Recorder};
use crate::output::{print_confirmation, print_result};
use crate::render::{
    RelationshipDocument, RelationshipDto, RelationshipListDocument, SlugIndex, added_text,
    relationship_list_text, relationship_show_text,
};
use crate::resolve;

const BUILT_IN_TYPES: &str =
    "depends_on, references, derived_from, extends, supports, contradicts, validates";

/// Parses a type name; a type that is not built in needs `--allow-custom-type`.
fn parse_type(text: &str, allow_custom: bool) -> Result<RelationshipType> {
    let relationship_type = RelationshipType::parse(text)?;
    if !relationship_type.is_built_in() && !allow_custom {
        return Err(FunctorError::invalid_input(
            "relationship type",
            format!(
                "`{relationship_type}` is not a built-in type ({BUILT_IN_TYPES}); \
                 use --allow-custom-type to create it anyway"
            ),
        ));
    }
    Ok(relationship_type)
}

/// Creates `source --type--> target`.
///
/// # Errors
/// `InvalidInput` for a bad type, metadata or self-reference of an acyclic type, `NotFound` for
/// an unknown endpoint, `Conflict` for a duplicate, or a store error.
pub async fn add(
    artifacts: &dyn ArtifactRepository,
    relationships: &dyn RelationshipRepository,
    args: RelationAddArgs,
    recorder: &Recorder,
) -> Result<(Relationship, SlugIndex)> {
    let relationship_type = parse_type(&args.relation_type, args.allow_custom_type)?;
    let mut metadata = Metadata::new();
    for pair in &args.metadata {
        let (key, value) = parse_pair(pair)?;
        metadata.insert(key, value);
    }
    let source = resolve::artifact(artifacts, &args.source).await?;
    let target = resolve::artifact(artifacts, &args.target).await?;

    let new =
        NewRelationship::new(relationship_type, *source.id(), *target.id()).with_metadata(metadata);
    let relationship = relationships.create(new).await?;
    recorder
        .record(events::relationship_created(&relationship))
        .await;
    Ok((relationship, SlugIndex::new(&[source, target])))
}

/// Lists relationships ordered by id, optionally filtered.
///
/// # Errors
/// `InvalidInput` for a bad type filter, `NotFound` for an unknown `--source` or `--target`, or
/// a store error.
pub async fn list(
    artifacts: &dyn ArtifactRepository,
    relationships: &dyn RelationshipRepository,
    args: &RelationListArgs,
) -> Result<(Vec<Relationship>, SlugIndex)> {
    let types = args
        .types
        .iter()
        .map(|text| RelationshipType::parse(text))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let source = match &args.source {
        Some(text) => Some(*resolve::artifact(artifacts, text).await?.id()),
        None => None,
    };
    let target = match &args.target {
        Some(text) => Some(*resolve::artifact(artifacts, text).await?.id()),
        None => None,
    };

    let mut found: Vec<Relationship> = relationships
        .list()
        .await?
        .into_iter()
        .filter(|r| types.is_empty() || types.contains(r.relationship_type()))
        .filter(|r| source.is_none_or(|wanted| *r.source() == wanted))
        .filter(|r| target.is_none_or(|wanted| *r.target() == wanted))
        .collect();
    found.sort_by_key(|r| *r.id());
    let slugs = SlugIndex::new(&artifacts.list().await?);
    Ok((found, slugs))
}

/// Loads one relationship.
///
/// # Errors
/// `InvalidInput` for a bad reference, `NotFound`, or a store error.
pub async fn show(
    artifacts: &dyn ArtifactRepository,
    relationships: &dyn RelationshipRepository,
    args: &RelationShowArgs,
) -> Result<(Relationship, SlugIndex)> {
    let relationship = resolve::relationship(relationships, &args.relation).await?;
    let slugs = SlugIndex::new(&artifacts.list().await?);
    Ok((relationship, slugs))
}

fn output_error(error: std::io::Error) -> FunctorError {
    FunctorError::storage("write output", "stdout", error)
}

/// Runs a `relation` subcommand and prints its result.
///
/// # Errors
/// Whatever the handler returns.
pub async fn run(command: RelationCommand, context: &Context, globals: &Globals) -> Result<()> {
    let artifacts: &dyn ArtifactRepository = &context.store;
    let relationships: &dyn RelationshipRepository = &context.store;
    let recorder = &context.recorder;

    match command {
        RelationCommand::Add(args) => {
            let (relationship, slugs) = add(artifacts, relationships, args, recorder).await?;
            let document = RelationshipDocument::new(RelationshipDto::new(&relationship, &slugs)?);
            print_confirmation(&added_text(&relationship, &slugs)?, &document, globals)
                .map_err(output_error)
        }
        RelationCommand::List(args) => {
            let (found, slugs) = list(artifacts, relationships, &args).await?;
            let document = RelationshipListDocument::new(
                found
                    .iter()
                    .map(|r| RelationshipDto::new(r, &slugs))
                    .collect::<Result<Vec<_>>>()?,
            );
            print_result(
                &relationship_list_text(&found, &slugs)?,
                &document,
                globals.output,
            )
            .map_err(output_error)
        }
        RelationCommand::Show(args) => {
            let (relationship, slugs) = show(artifacts, relationships, &args).await?;
            let document = RelationshipDocument::new(RelationshipDto::new(&relationship, &slugs)?);
            print_result(
                &relationship_show_text(&relationship, &slugs)?,
                &document,
                globals.output,
            )
            .map_err(output_error)
        }
    }
}
