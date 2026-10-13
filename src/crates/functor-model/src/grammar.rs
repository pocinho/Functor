//! Shared grammar checks for validated names.

use crate::error::{ModelError, Result};

/// Checks `[first][rest]{0,max-1}` style names: one leading character class, then a second class.
pub(crate) fn check_name(
    field: &'static str,
    input: &str,
    max_len: usize,
    first: fn(char) -> bool,
    rest: fn(char) -> bool,
    shape: &str,
) -> Result<()> {
    let mut chars = input.chars();
    let Some(head) = chars.next() else {
        return Err(ModelError::new(field, "must not be empty"));
    };
    if input.len() > max_len {
        return Err(ModelError::new(
            field,
            format!("must be at most {max_len} characters"),
        ));
    }
    if !first(head) || !chars.all(rest) {
        return Err(ModelError::new(field, format!("must match {shape}")));
    }
    Ok(())
}

pub(crate) fn is_lower(c: char) -> bool {
    c.is_ascii_lowercase()
}

pub(crate) fn is_lower_digit(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit()
}

pub(crate) fn is_lower_digit_underscore(c: char) -> bool {
    is_lower_digit(c) || c == '_'
}

/// Trims `input` and checks it is non-empty, within `max_chars` and free of control characters.
pub(crate) fn check_text_line(
    field: &'static str,
    input: &str,
    max_chars: usize,
) -> Result<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ModelError::new(field, "must not be empty"));
    }
    if trimmed.chars().count() > max_chars {
        return Err(ModelError::new(
            field,
            format!("must be at most {max_chars} characters"),
        ));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(ModelError::new(
            field,
            "must not contain control characters",
        ));
    }
    Ok(trimmed.to_owned())
}
