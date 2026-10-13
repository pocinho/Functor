//! `functor artifact create|show|list|update`.

use std::fs;
use std::path::{Component, Path};

use functor_core::{ArtifactRepository, FunctorError, Result};
use functor_model::{
    Artifact, ArtifactPatch, ArtifactType, Content, ContentFormat, ContentPath, Metadata,
    MetadataKey, MetadataValue, NewArtifact, Patched, Revision, Slug, Tag, Tags, Title,
};
use functor_repository::{Layout, run_blocking};

use crate::cli::{
    ArtifactCommand, ArtifactCreateArgs, ArtifactListArgs, ArtifactShowArgs, ArtifactUpdateArgs,
};
use crate::context::{Context, Globals};
use crate::events::{self, Recorder};
use crate::output::{print_confirmation, print_result};
use crate::render::{
    ArtifactDocument, ArtifactDto, ArtifactListDocument, created_text, show_text, table,
    updated_text,
};
use crate::resolve;

/// The largest content file `show --content` will read.
const MAX_CONTENT_BYTES: u64 = 1024 * 1024;

// ---- argument conversion ----

pub(crate) fn parse_pair(text: &str) -> Result<(MetadataKey, MetadataValue)> {
    let Some((key, value)) = text.split_once('=') else {
        return Err(FunctorError::invalid_input(
            "metadata",
            format!("`{text}` must have the form KEY=VALUE"),
        ));
    };
    Ok((
        MetadataKey::new(key)?,
        MetadataValue::String(value.to_owned()),
    ))
}

fn parse_tags<'a>(items: impl IntoIterator<Item = &'a str>) -> Result<Tags> {
    let tags = items
        .into_iter()
        .map(|tag| Tag::new(tag.trim()))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(Tags::new(tags))
}

fn guess_format(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("md" | "markdown") => "markdown",
        Some("toml") => "toml",
        Some("json") => "json",
        _ => "text",
    }
}

/// Turns a file the user pointed at into a repository-relative content path.
///
/// The path is resolved against the current directory, must exist, must be a file and must lie
/// inside the repository (links are followed, so a link cannot lead outside).
fn content_path(root: &Path, given: &Path) -> Result<ContentPath> {
    let invalid = |reason: String| FunctorError::invalid_input("content path", reason);
    let full = if given.is_absolute() {
        given.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| FunctorError::storage("resolve path", given, error))?
            .join(given)
    };
    let real = fs::canonicalize(&full)
        .map_err(|_| invalid(format!("`{}` does not exist", given.display())))?;
    let root_real = fs::canonicalize(root)
        .map_err(|error| FunctorError::storage("resolve path", root, error))?;
    let relative = real
        .strip_prefix(&root_real)
        .map_err(|_| invalid(format!("`{}` is outside the repository", given.display())))?;
    if !real.is_file() {
        return Err(invalid(format!("`{}` is not a file", given.display())));
    }
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => parts.push(
                part.to_str()
                    .ok_or_else(|| invalid(format!("`{}` is not valid UTF-8", given.display())))?,
            ),
            _ => {
                return Err(invalid(format!(
                    "`{}` is not a plain path",
                    given.display()
                )));
            }
        }
    }
    Ok(ContentPath::new(&parts.join("/"))?)
}

/// Builds new content from `--content-text` / `--content-path` / `--content-format`.
fn content_from(
    root: &Path,
    text: Option<String>,
    path: Option<&Path>,
    format: Option<&str>,
) -> Result<Option<Content>> {
    match (text, path) {
        (Some(text), None) => {
            let format = ContentFormat::new(format.unwrap_or("text"))?;
            Ok(Some(Content::Inline { format, text }))
        }
        (None, Some(path)) => {
            let content_path = content_path(root, path)?;
            let format = ContentFormat::new(format.unwrap_or_else(|| guess_format(path)))?;
            Ok(Some(Content::Path {
                format,
                path: content_path,
            }))
        }
        (None, None) if format.is_some() => Err(FunctorError::invalid_input(
            "content format",
            "requires --content-text or --content-path",
        )),
        (None, None) => Ok(None),
        // The argument parser makes the two content sources mutually exclusive.
        (Some(_), Some(_)) => Err(FunctorError::invalid_input(
            "content",
            "use either --content-text or --content-path, not both",
        )),
    }
}

// ---- handlers ----

/// Creates an artifact.
///
/// # Errors
/// `InvalidInput` for bad arguments, `Conflict` for a taken slug, or a store error.
pub async fn create(
    store: &dyn ArtifactRepository,
    root: &Path,
    args: ArtifactCreateArgs,
    recorder: &Recorder,
) -> Result<Artifact> {
    let slug = Slug::parse(&args.slug)?;
    let title = Title::new(args.title.as_deref().unwrap_or(slug.as_str()))?;
    let mut new = NewArtifact::new(slug, ArtifactType::new(&args.artifact_type)?, title)
        .with_tags(parse_tags(args.tags.iter().map(String::as_str))?);
    if let Some(description) = args.description {
        new = new.with_description(description);
    }
    let mut metadata = Metadata::new();
    for pair in &args.metadata {
        let (key, value) = parse_pair(pair)?;
        metadata.insert(key, value);
    }
    new = new.with_metadata(metadata);
    if let Some(content) = content_from(
        root,
        args.content_text,
        args.content_path.as_deref(),
        args.content_format.as_deref(),
    )? {
        new = new.with_content(content);
    }

    let artifact = store.create(new).await?;
    recorder.record(events::artifact_created(&artifact));
    Ok(artifact)
}

/// Loads an artifact and, if asked, its content text.
///
/// # Errors
/// `NotFound`, `InvalidInput` for a bad reference, or `Storage` if a content file cannot be read.
pub async fn show(
    store: &dyn ArtifactRepository,
    root: &Path,
    args: &ArtifactShowArgs,
) -> Result<(Artifact, Option<String>)> {
    let artifact = resolve::artifact(store, &args.artifact).await?;
    if !args.content {
        return Ok((artifact, None));
    }
    let text = match artifact.content() {
        Content::Inline { text, .. } => Some(text.clone()),
        Content::Path { path, .. } => {
            let file = Layout::new(root).content_file(path);
            Some(run_blocking(move || read_content(&file)).await?)
        }
        _ => None,
    };
    Ok((artifact, text))
}

fn read_content(file: &Path) -> Result<String> {
    let length = fs::metadata(file)
        .map_err(|error| FunctorError::storage("read", file, error))?
        .len();
    if length > MAX_CONTENT_BYTES {
        return Err(FunctorError::invalid_input(
            "content",
            format!("`{}` is larger than 1 MiB", file.display()),
        ));
    }
    let bytes = fs::read(file).map_err(|error| FunctorError::storage("read", file, error))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Lists artifacts ordered by slug (then id), optionally filtered.
///
/// # Errors
/// `InvalidInput` for a bad filter, or a store error.
pub async fn list(
    store: &dyn ArtifactRepository,
    args: &ArtifactListArgs,
) -> Result<Vec<Artifact>> {
    let artifact_type = args
        .artifact_type
        .as_deref()
        .map(ArtifactType::new)
        .transpose()?;
    let tag = args.tag.as_deref().map(Tag::new).transpose()?;
    let mut artifacts: Vec<Artifact> = store
        .list()
        .await?
        .into_iter()
        .filter(|a| {
            artifact_type
                .as_ref()
                .is_none_or(|wanted| a.artifact_type() == wanted)
        })
        .filter(|a| tag.as_ref().is_none_or(|wanted| a.tags().contains(wanted)))
        .collect();
    artifacts.sort_by(|a, b| (a.slug(), a.id()).cmp(&(b.slug(), b.id())));
    Ok(artifacts)
}

fn has_changes(args: &ArtifactUpdateArgs) -> bool {
    args.slug.is_some()
        || args.title.is_some()
        || args.description.is_some()
        || args.clear_description
        || args.tags.is_some()
        || !args.metadata.is_empty()
        || !args.unset_metadata.is_empty()
        || args.content_text.is_some()
        || args.content_path.is_some()
        || args.clear_content
}

/// Applies the change options to an artifact.
///
/// # Errors
/// `InvalidInput` when no change option is given or a value is invalid, `NotFound`, `Conflict`
/// for a taken slug or a stale `--expect-revision`, or a store error.
pub async fn update(
    store: &dyn ArtifactRepository,
    root: &Path,
    args: ArtifactUpdateArgs,
    recorder: &Recorder,
) -> Result<Patched> {
    if !has_changes(&args) {
        return Err(FunctorError::invalid_input(
            "update",
            "at least one change option is required (for example --title)",
        ));
    }
    let expected = args.expect_revision.map(Revision::new).transpose()?;
    let current = resolve::artifact(store, &args.artifact).await?;

    let mut patch = ArtifactPatch::new();
    if let Some(slug) = &args.slug {
        patch = patch.with_slug(Slug::parse(slug)?);
    }
    if let Some(title) = &args.title {
        patch = patch.with_title(Title::new(title)?);
    }
    if let Some(description) = args.description {
        patch = patch.with_description(Some(description));
    } else if args.clear_description {
        patch = patch.with_description(None);
    }
    if let Some(tags) = &args.tags {
        patch = patch.with_tags(parse_tags(
            tags.split(',').filter(|t| !t.trim().is_empty()),
        )?);
    }
    if !args.metadata.is_empty() || !args.unset_metadata.is_empty() {
        let mut metadata = current.metadata().clone();
        for pair in &args.metadata {
            let (key, value) = parse_pair(pair)?;
            metadata.insert(key, value);
        }
        for key in &args.unset_metadata {
            metadata.remove(&MetadataKey::new(key)?);
        }
        patch = patch.with_metadata(metadata);
    }
    if args.clear_content {
        patch = patch.with_content(Content::None);
    } else if let Some(content) = content_from(
        root,
        args.content_text,
        args.content_path.as_deref(),
        args.content_format.as_deref(),
    )? {
        patch = patch.with_content(content);
    }

    let patched = store.update(current.id(), patch, expected).await?;
    if !patched.changed.is_empty() {
        recorder.record(events::artifact_updated(
            &patched.artifact,
            &patched.changed,
        ));
    }
    Ok(patched)
}

// ---- command entry point: handlers + rendering ----

fn output_error(error: std::io::Error) -> FunctorError {
    FunctorError::storage("write output", "stdout", error)
}

/// Runs an `artifact` subcommand and prints its result.
///
/// # Errors
/// Whatever the handler returns.
pub async fn run(command: ArtifactCommand, context: &Context, globals: &Globals) -> Result<()> {
    let root = context.handle.root.as_path();
    let store: &dyn ArtifactRepository = &context.store;
    let recorder = Recorder;

    match command {
        ArtifactCommand::Create(args) => {
            let artifact = create(store, root, args, &recorder).await?;
            let document = ArtifactDocument::new(ArtifactDto::new(&artifact, None));
            print_confirmation(&created_text(&artifact), &document, globals).map_err(output_error)
        }
        ArtifactCommand::Show(args) => {
            let (artifact, text) = show(store, root, &args).await?;
            let document = ArtifactDocument::new(ArtifactDto::new(&artifact, text.clone()));
            print_result(
                &show_text(&artifact, text.as_deref()),
                &document,
                globals.output,
            )
            .map_err(output_error)
        }
        ArtifactCommand::List(args) => {
            let artifacts = list(store, &args).await?;
            let text = list_text(&artifacts, args.long);
            let document = ArtifactListDocument::new(
                artifacts
                    .iter()
                    .map(|a| ArtifactDto::new(a, None))
                    .collect(),
            );
            print_result(&text, &document, globals.output).map_err(output_error)
        }
        ArtifactCommand::Update(args) => {
            let patched = update(store, root, args, &recorder).await?;
            let document = ArtifactDocument::updated(
                ArtifactDto::new(&patched.artifact, None),
                &patched.changed,
            );
            let text = updated_text(&patched.artifact, &patched.changed);
            print_confirmation(&text, &document, globals).map_err(output_error)
        }
    }
}

pub(crate) fn list_text(artifacts: &[Artifact], long: bool) -> String {
    use crate::render::sanitize;

    if artifacts.is_empty() {
        return "no artifacts found".to_owned();
    }
    if long {
        let rows: Vec<Vec<String>> = artifacts
            .iter()
            .map(|a| {
                vec![
                    a.slug().to_string(),
                    a.id().to_string(),
                    a.artifact_type().to_string(),
                    a.revision().to_string(),
                    a.updated().to_string(),
                    sanitize(a.title().as_str()),
                ]
            })
            .collect();
        table(&["SLUG", "ID", "TYPE", "REV", "UPDATED", "TITLE"], &rows)
    } else {
        let rows: Vec<Vec<String>> = artifacts
            .iter()
            .map(|a| {
                vec![
                    a.slug().to_string(),
                    a.artifact_type().to_string(),
                    a.revision().to_string(),
                    sanitize(a.title().as_str()),
                ]
            })
            .collect();
        table(&["SLUG", "TYPE", "REV", "TITLE"], &rows)
    }
}
