//! `functor query artifacts|dependencies|dependents|relationships`. All are read-only.

use functor_core::{ArtifactRepository, FunctorError, QueryEngine, RepositoryReader, Result};
use functor_model::{
    Artifact, ArtifactRef, ArtifactType, Direction, Relationship, RelationshipType, SearchQuery,
    Tag, Traversal, TraversalResult,
};
use functor_query::SnapshotQueryEngine;

use super::artifact::{list_text, parse_pair};
use crate::cli::{
    Depth, QueryArtifactsArgs, QueryCommand, QueryRelationshipsArgs, QueryTraverseArgs,
    RelationshipDirection,
};
use crate::context::{Context, Globals};
use crate::output::print_result;
use crate::render::{
    ArtifactDto, ArtifactListDocument, RelationshipsDocument, SlugIndex, TraversalDocument,
    relationship_list_text, traversal_text,
};
use crate::resolve;

/// Finds artifacts matching every given criterion, ordered by slug then id.
///
/// # Errors
/// `InvalidInput` for a malformed criterion, or the engine's error.
pub async fn artifacts(
    engine: &dyn QueryEngine,
    args: &QueryArtifactsArgs,
) -> Result<Vec<Artifact>> {
    let mut query = SearchQuery::new();
    if let Some(artifact_type) = &args.artifact_type {
        query = query.with_type(ArtifactType::new(artifact_type)?);
    }
    if let Some(tag) = &args.tag {
        query = query.with_tag(Tag::new(tag)?);
    }
    if let Some(text) = &args.text {
        query = query.with_text(text.as_str());
    }
    for pair in &args.metadata {
        let (key, value) = parse_pair(pair)?;
        query = query.with_metadata(key, value);
    }
    engine.search(&query).await
}

/// What a traversal followed, for rendering.
#[derive(Debug)]
pub struct Traversed {
    /// The result.
    pub result: TraversalResult,
    /// The relationship types that were followed.
    pub types: Vec<RelationshipType>,
    /// The depth limit, `None` for the whole closure.
    pub max_depth: Option<u32>,
}

fn parse_types(types: &[String]) -> Result<Vec<RelationshipType>> {
    types
        .iter()
        .map(|text| RelationshipType::parse(text))
        .collect::<std::result::Result<_, _>>()
        .map_err(FunctorError::from)
}

/// Follows relationships from an artifact: outbound for `dependencies`, inbound for `dependents`.
///
/// # Errors
/// `InvalidInput` for a bad reference or type, `NotFound` for an unknown artifact, or the
/// engine's error.
pub async fn traverse(
    repository: &dyn ArtifactRepository,
    engine: &dyn QueryEngine,
    args: &QueryTraverseArgs,
    direction: Direction,
) -> Result<Traversed> {
    let mut types = parse_types(&args.types)?;
    if types.is_empty() {
        types.push(RelationshipType::DependsOn);
    }
    let max_depth = match args.depth {
        Depth::Limited(depth) => Some(depth),
        Depth::All => None,
    };
    // Resolving first gives the friendly `NotFound` with suggestions.
    let start = resolve::artifact(repository, &args.artifact).await?;

    let mut traversal = Traversal::new(ArtifactRef::Id(*start.id()))
        .with_types(types.iter().cloned())
        .with_direction(direction);
    traversal.max_depth = max_depth;
    let result = engine.traverse(&traversal).await?;
    Ok(Traversed {
        result,
        types,
        max_depth,
    })
}

/// Every relationship touching an artifact, ordered by relationship id.
///
/// # Errors
/// `InvalidInput` for a bad reference or type, `NotFound` for an unknown artifact, or the
/// engine's error.
pub async fn relationships<R: RepositoryReader + ?Sized>(
    repository: &dyn ArtifactRepository,
    engine: &SnapshotQueryEngine<'_, R>,
    args: &QueryRelationshipsArgs,
) -> Result<(Artifact, Vec<Relationship>)> {
    let types = parse_types(&args.types)?;
    let direction = match args.direction {
        RelationshipDirection::In => Direction::Inbound,
        RelationshipDirection::Out => Direction::Outbound,
        RelationshipDirection::Both => Direction::Both,
    };
    let artifact = resolve::artifact(repository, &args.artifact).await?;
    engine
        .relationships(&ArtifactRef::Id(*artifact.id()), &types, direction)
        .await
}

fn output_error(error: std::io::Error) -> FunctorError {
    FunctorError::storage("write output", "stdout", error)
}

/// Runs a `query` subcommand and prints its result.
///
/// # Errors
/// Whatever the handler returns.
pub async fn run(command: QueryCommand, context: &Context, globals: &Globals) -> Result<()> {
    let repository: &dyn ArtifactRepository = &context.store;
    let engine = SnapshotQueryEngine::new(&context.store);

    match command {
        QueryCommand::Artifacts(args) => {
            let found = artifacts(&engine, &args).await?;
            let document = ArtifactListDocument::new(
                found.iter().map(|a| ArtifactDto::new(a, None)).collect(),
            );
            print_result(&list_text(&found, false), &document, globals.output).map_err(output_error)
        }
        QueryCommand::Dependencies(args) => {
            print_traversal(
                "dependencies",
                Direction::Outbound,
                repository,
                &engine,
                &args,
                globals,
            )
            .await
        }
        QueryCommand::Dependents(args) => {
            print_traversal(
                "dependents",
                Direction::Inbound,
                repository,
                &engine,
                &args,
                globals,
            )
            .await
        }
        QueryCommand::Relationships(args) => {
            let (artifact, found) = relationships(repository, &engine, &args).await?;
            let slugs = SlugIndex::new(&repository.list().await?);
            let document = RelationshipsDocument::new(&artifact, &found, &slugs)?;
            print_result(
                &relationship_list_text(&found, &slugs)?,
                &document,
                globals.output,
            )
            .map_err(output_error)
        }
    }
}

async fn print_traversal(
    things: &str,
    direction: Direction,
    repository: &dyn ArtifactRepository,
    engine: &dyn QueryEngine,
    args: &QueryTraverseArgs,
    globals: &Globals,
) -> Result<()> {
    let traversed = traverse(repository, engine, args, direction).await?;
    let document = TraversalDocument::new(
        &traversed.result,
        direction,
        &traversed.types,
        traversed.max_depth,
    )?;
    let text = traversal_text(
        things,
        &traversed.result,
        &traversed.types,
        traversed.max_depth,
    );
    print_result(&text, &document, globals.output).map_err(output_error)
}
