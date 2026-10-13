//! `functor init`.

use std::path::PathBuf;

use functor_core::{FunctorError, InitOptions, RepositoryManager, Result};
use serde::Serialize;

use crate::cli::InitArgs;
use crate::context::{Globals, Services};
use crate::output::{SCHEMA_VERSION, print_confirmation};

#[derive(Serialize)]
struct RepositoryDto {
    id: String,
    name: String,
    root: String,
    format_version: u32,
}

#[derive(Serialize)]
struct InitDocument {
    schema_version: u32,
    repository: RepositoryDto,
}

/// Creates a repository and reports it.
///
/// The target is `args.path`, else `--repo`, else the current directory.
///
/// # Errors
/// See `RepositoryManager::init`.
pub async fn run(args: InitArgs, globals: &Globals, services: &Services) -> Result<()> {
    let target = args
        .path
        .or_else(|| globals.repo.clone())
        .unwrap_or_else(|| PathBuf::from("."));

    let mut options = InitOptions::new();
    options.name = args.name;
    options.description = args.description;
    options.nested = args.nested;

    let handle = services.manager().init(&target, options).await?;
    let root = handle.root.display().to_string();
    let text = format!(
        "initialized repository \"{}\" ({}) at {root}",
        handle.info.name(),
        handle.info.id()
    );
    let document = InitDocument {
        schema_version: SCHEMA_VERSION,
        repository: RepositoryDto {
            id: handle.info.id().to_string(),
            name: handle.info.name().to_string(),
            root,
            format_version: handle.info.format_version().get(),
        },
    };
    print_confirmation(&text, &document, globals)
        .map_err(|error| FunctorError::storage("write output", "stdout", error))
}
