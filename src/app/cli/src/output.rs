//! Rendering of results and errors for the terminal and for scripts.
//!
//! `text` output is for humans; `json` output is the stable contract (`docs/cli-spec.md`).
//! Results go to stdout, diagnostics and errors to stderr.

use std::io::{self, Write};

use functor_core::{ErrorKind, FunctorError};
use serde::Serialize;

use crate::cli::OutputMode;
use crate::exit;

/// The version of the JSON output schemas.
pub const SCHEMA_VERSION: u32 = 1;

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
    if let FunctorError::NotFound { suggestions, .. } = error
        && !suggestions.is_empty()
    {
        text = format!("{text}\n  did you mean: {}", suggestions.join(", "));
    }
    if error.kind() == ErrorKind::NotARepository {
        text.push_str("\nhint: run `functor init` to create a repository here");
    }
    text
}

/// The JSON document printed for an error in `json` mode.
#[must_use]
pub fn error_json(error: &FunctorError, exit_code: u8) -> String {
    let document = ErrorDocument {
        error: error_body(error),
        exit_code,
    };
    serde_json::to_string(&document).unwrap_or_else(|_| {
        format!(r#"{{"error":{{"code":"internal","message":"could not render error"}},"exit_code":{}}}"#, exit::INTERNAL)
    })
}

/// Prints an error to stderr in the requested mode and returns its exit code.
#[must_use]
pub fn report_error(error: &FunctorError, mode: OutputMode) -> u8 {
    let code = exit::exit_code(error.kind());
    let rendered = match mode {
        OutputMode::Text => error_text(error),
        OutputMode::Json => error_json(error, code),
    };
    // Nothing useful can be done if stderr itself is closed.
    let _ = writeln!(io::stderr().lock(), "{rendered}");
    code
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
            "error: artifact not found: rfc-9\n  did you mean: rfc-0, rfc-1"
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
