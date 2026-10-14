//! Rendering of results and errors for the terminal and for scripts.
//!
//! `text` output is for humans; `json` output is the stable contract (`docs/cli/cli-spec.md`).
//! Results go to stdout, diagnostics and errors to stderr.

use std::io::{self, Write};

use functor_core::{ErrorKind, FunctorError};
use functor_model::EntityKind;
use serde::Serialize;

use crate::cli::OutputMode;
use crate::context::Globals;
use crate::exit;

/// The version of the JSON output schemas.
pub const SCHEMA_VERSION: u32 = 1;

/// Writes a confirmation of a mutating command (`created ...`). `--quiet` silences it in text
/// mode; JSON output is never suppressed.
///
/// # Errors
/// Returns I/O errors other than a broken pipe.
pub fn print_confirmation(text: &str, json: &impl Serialize, globals: &Globals) -> io::Result<()> {
    if globals.quiet && globals.output == OutputMode::Text {
        return Ok(());
    }
    print_result(text, json, globals.output)
}

/// Writes a command result to stdout.
///
/// A closed pipe (`| head`) is not an error: the reader has what it wanted.
///
/// # Errors
/// Returns I/O errors other than a broken pipe.
pub fn print_result(text: &str, json: &impl Serialize, mode: OutputMode) -> io::Result<()> {
    let rendered = match mode {
        OutputMode::Text => format!("{text}\n"),
        OutputMode::Json => {
            let mut rendered = serde_json::to_string(json).map_err(io::Error::other)?;
            rendered.push('\n');
            rendered
        }
    };
    let mut stdout = io::stdout().lock();
    match stdout
        .write_all(rendered.as_bytes())
        .and_then(|()| stdout.flush())
    {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        other => other,
    }
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    entity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reference: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    suggestions: Vec<String>,
}

#[derive(Serialize)]
struct ErrorDocument {
    error: ErrorBody,
    exit_code: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    run: Option<String>,
}

fn error_body(error: &FunctorError) -> ErrorBody {
    let (entity, reference, suggestions) = match error {
        FunctorError::NotFound {
            entity,
            reference,
            suggestions,
        } => (
            Some(entity.to_string()),
            Some(reference.clone()),
            suggestions.clone(),
        ),
        _ => (None, None, Vec::new()),
    };
    ErrorBody {
        code: error.kind().as_str(),
        message: error.to_string(),
        entity,
        reference,
        suggestions,
    }
}

/// The lines printed for an error in `text` mode: `error: <message>`, then optional hints.
#[must_use]
pub fn error_text(error: &FunctorError) -> String {
    let mut text = format!("error: {error}");
    if let FunctorError::NotFound {
        entity,
        suggestions,
        ..
    } = error
    {
        if !suggestions.is_empty() {
            text = format!("{text}\n  did you mean: {}", suggestions.join(", "));
        }
        if *entity == EntityKind::Artifact {
            text.push_str("\nhint: run `functor artifact list` to see available artifacts");
        }
        if *entity == EntityKind::Relationship {
            text.push_str("\nhint: run `functor relation list` to see available relationships");
        }
    }
    if error.kind() == ErrorKind::NotARepository {
        text.push_str("\nhint: run `functor init` to create a repository here");
    }
    text
}

/// The `caused by:` lines for an error's source chain, shown with `--verbose`.
#[must_use]
pub fn error_chain(error: &FunctorError) -> String {
    let mut lines = Vec::new();
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        lines.push(format!("  caused by: {cause}"));
        source = cause.source();
    }
    lines.join("\n")
}

/// The JSON document printed for an error in `json` mode.
#[must_use]
pub fn error_json(error: &FunctorError, exit_code: u8) -> String {
    let document = ErrorDocument {
        error: error_body(error),
        exit_code,
        run: None,
    };
    serde_json::to_string(&document).unwrap_or_else(|_| {
        format!(
            r#"{{"error":{{"code":"internal","message":"could not render error"}},"exit_code":{}}}"#,
            exit::INTERNAL
        )
    })
}

/// Reports an execution infrastructure failure, retaining its durable run identity for recovery.
pub fn report_run_failure(failure: &functor_runtime::RunFailure, globals: &Globals) -> u8 {
    for warning in &failure.warnings {
        eprintln!("warning: {}", crate::render::sanitize(warning));
    }
    let code = exit::exit_code(failure.kind());
    if globals.output == OutputMode::Json {
        let document = ErrorDocument {
            error: error_body(&failure.error),
            exit_code: code,
            run: failure.run.map(|id| id.to_string()),
        };
        match serde_json::to_string(&document) {
            Ok(json) => eprintln!("{json}"),
            Err(error) => {
                return report_error(
                    &FunctorError::Internal(format!("could not render run error: {error}")),
                    globals.output,
                    globals.verbose,
                );
            }
        }
    } else {
        let _ = report_error(&failure.error, globals.output, globals.verbose);
        if let Some(id) = failure.run {
            eprintln!(
                "run: {id}\nhint: inspect with `functor run show {id}`; repair with `functor run recover`"
            );
        }
    }
    code
}

/// Prints an error to stderr in the requested mode and returns its exit code.
///
/// With `verbose`, text mode also prints the cause chain.
#[must_use]
pub fn report_error(error: &FunctorError, mode: OutputMode, verbose: bool) -> u8 {
    let code = exit::exit_code(error.kind());
    report_error_code(error, mode, verbose, code);
    code
}

/// Reports an error with the command's policy code (a report-save failure is always exit 7).
pub fn report_error_code(error: &FunctorError, mode: OutputMode, verbose: bool, code: u8) {
    let rendered = match mode {
        OutputMode::Text => {
            let mut text = error_text(error);
            let chain = error_chain(error);
            if verbose && !chain.is_empty() {
                text = format!("{text}\n{chain}");
            }
            text
        }
        OutputMode::Json => error_json(error, code),
    };
    // Nothing useful can be done if stderr itself is closed.
    let _ = writeln!(io::stderr().lock(), "{rendered}");
}

#[cfg(test)]
mod tests {
    use functor_model::EntityKind;

    use super::*;

    #[test]
    fn text_errors_follow_the_documented_shape() {
        let error = FunctorError::NotFound {
            entity: EntityKind::Artifact,
            reference: "rfc-9".into(),
            suggestions: vec!["rfc-0".into(), "rfc-1".into()],
        };
        assert_eq!(
            error_text(&error),
            "error: artifact not found: rfc-9\n  did you mean: rfc-0, rfc-1\nhint: run `functor artifact list` to see available artifacts"
        );
        assert_eq!(
            error_text(&FunctorError::invalid_input("name", "must not be empty")),
            "error: invalid name: must not be empty"
        );
        assert!(
            error_text(&FunctorError::NotARepository { path: "x".into() })
                .ends_with("hint: run `functor init` to create a repository here")
        );
    }

    #[test]
    fn json_errors_follow_the_documented_shape() {
        let error = FunctorError::NotFound {
            entity: EntityKind::Artifact,
            reference: "rfc-9".into(),
            suggestions: vec!["rfc-0".into()],
        };
        assert_eq!(
            error_json(&error, 3),
            r#"{"error":{"code":"not_found","message":"artifact not found: rfc-9","entity":"artifact","reference":"rfc-9","suggestions":["rfc-0"]},"exit_code":3}"#
        );
        let conflict = FunctorError::conflict(EntityKind::Repository, "exists");
        assert_eq!(
            error_json(&conflict, 4),
            r#"{"error":{"code":"conflict","message":"exists"},"exit_code":4}"#
        );
    }
}
