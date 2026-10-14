//! The `functor.workflow/1` format: the TOML content of a `workflow` artifact (ADR-0012).
//!
//! An executable format is stricter than the data files: unknown keys are rejected at every level,
//! a newer schema is `Unsupported`, and a network policy the local provider cannot enforce is
//! refused rather than silently ignored.
//!
//! ```toml
//! schema = "functor.workflow/1"
//! provider = "local-process"
//! program = "tools/summarize.exe"        # bare name on PATH, or a repository-relative path
//! args = ["inputs/data", "outputs/summary"]
//!
//! [inputs]
//! data = { required = true }
//!
//! [outputs]
//! summary = { type = "dataset", format = "text", required = true }
//!
//! [env]
//! allow = ["LANG"]
//! set = { MODE = "fast" }
//!
//! [limits]
//! timeout_seconds = 30
//! stream_bytes = 65536
//! ```
//!
//! Inputs and outputs are `required` unless they say otherwise. Limits may only lower the
//! defaults; a larger value is accepted up to the hard maximum and then clamped.

use std::collections::BTreeMap;

use functor_core::{FunctorError, Result};
use functor_model::{
    ArtifactType, ExecutionLimits, InputSpec, OutputSpec, WORKFLOW_SCHEMA, Workflow, WorkflowSpec,
    bounds,
};
use toml_edit::{DocumentMut, Item, Table, TableLike};

fn invalid(reason: impl Into<String>) -> FunctorError {
    FunctorError::invalid_input("workflow", reason)
}

/// Rejects keys that are not in `allowed`.
fn only_keys(table: &dyn TableLike, allowed: &[&str], place: &str) -> Result<()> {
    match table
        .iter()
        .map(|(key, _)| key)
        .find(|key| !allowed.contains(key))
    {
        Some(key) => Err(invalid(format!("unknown key `{key}` in {place}"))),
        None => Ok(()),
    }
}

fn string(item: &Item, key: &str, place: &str) -> Result<String> {
    item.as_str()
        .map(str::to_owned)
        .ok_or_else(|| invalid(format!("`{key}` in {place} must be a string")))
}

fn required_string(table: &dyn TableLike, key: &str, place: &str) -> Result<String> {
    let item = table
        .get(key)
        .ok_or_else(|| invalid(format!("missing `{key}` in {place}")))?;
    string(item, key, place)
}

fn string_array(item: &Item, key: &str, place: &str) -> Result<Vec<String>> {
    let array = item
        .as_array()
        .ok_or_else(|| invalid(format!("`{key}` in {place} must be an array of strings")))?;
    array
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| invalid(format!("`{key}` in {place} must contain only strings")))
        })
        .collect()
}

fn flag(table: &dyn TableLike, place: &str) -> Result<bool> {
    match table.get("required") {
        None => Ok(true),
        Some(item) => item
            .as_bool()
            .ok_or_else(|| invalid(format!("`required` in {place} must be true or false"))),
    }
}

fn table_of<'a>(item: &'a Item, key: &str, place: &str) -> Result<&'a dyn TableLike> {
    item.as_table_like()
        .ok_or_else(|| invalid(format!("`{key}` in {place} must be a table")))
}

fn inputs(table: &dyn TableLike) -> Result<Vec<InputSpec>> {
    table
        .iter()
        .map(|(name, item)| {
            let place = format!("input `{name}`");
            let spec = table_of(item, name, "[inputs]")?;
            only_keys(spec, &["required"], &place)?;
            Ok(InputSpec {
                name: name.to_owned(),
                required: flag(spec, &place)?,
            })
        })
        .collect()
}

fn outputs(table: &dyn TableLike) -> Result<Vec<OutputSpec>> {
    table
        .iter()
        .map(|(name, item)| {
            let place = format!("output `{name}`");
            let spec = table_of(item, name, "[outputs]")?;
            only_keys(spec, &["type", "format", "required"], &place)?;
            Ok(OutputSpec {
                name: name.to_owned(),
                artifact_type: ArtifactType::new(&required_string(spec, "type", &place)?)?,
                format: required_string(spec, "format", &place)?,
                required: flag(spec, &place)?,
            })
        })
        .collect()
}

fn limits(table: &dyn TableLike) -> Result<ExecutionLimits> {
    only_keys(table, &["timeout_seconds", "stream_bytes"], "[limits]")?;
    let number = |key: &str, default: u64| -> Result<u64> {
        match table.get(key) {
            None => Ok(default),
            Some(item) => item
                .as_integer()
                .and_then(|n| u64::try_from(n).ok())
                .ok_or_else(|| invalid(format!("`{key}` in [limits] must be a positive integer"))),
        }
    };
    let timeout = number(
        "timeout_seconds",
        u64::from(bounds::DEFAULT_TIMEOUT_SECONDS),
    )?;
    let timeout = u32::try_from(timeout)
        .map_err(|_| invalid("`timeout_seconds` in [limits] is out of range"))?;
    Ok(ExecutionLimits::new(
        timeout,
        number("stream_bytes", bounds::DEFAULT_STREAM_BYTES)?,
    )?)
}

fn env(table: &dyn TableLike) -> Result<(Vec<String>, BTreeMap<String, String>)> {
    only_keys(table, &["allow", "set"], "[env]")?;
    let allow = match table.get("allow") {
        Some(item) => string_array(item, "allow", "[env]")?,
        None => Vec::new(),
    };
    let mut set = BTreeMap::new();
    if let Some(item) = table.get("set") {
        for (name, value) in table_of(item, "set", "[env]")?.iter() {
            set.insert(name.to_owned(), string(value, name, "[env.set]")?);
        }
    }
    Ok((allow, set))
}

fn schema(root: &Table) -> Result<()> {
    let found = required_string(root, "schema", "the workflow")?;
    if found == WORKFLOW_SCHEMA {
        Ok(())
    } else if found.starts_with("functor.workflow/") {
        Err(FunctorError::unsupported(format!(
            "workflow schema `{found}` (this version reads `{WORKFLOW_SCHEMA}`)"
        )))
    } else {
        Err(invalid(format!(
            "`schema` must be `{WORKFLOW_SCHEMA}`, not `{found}`"
        )))
    }
}

/// Parses the content of a workflow artifact into a validated [`Workflow`].
///
/// # Errors
/// `InvalidInput` for content that is too large, not UTF-8 or TOML, has an unknown or missing key,
/// a wrong type or a value the model refuses (names, limits, program, environment);
/// `Unsupported` for a newer schema or for `network = "none"`.
pub fn parse_workflow(bytes: &[u8]) -> Result<Workflow> {
    if bytes.len() as u64 > bounds::MAX_WORKFLOW_BYTES {
        return Err(invalid(format!(
            "the workflow is larger than {} bytes",
            bounds::MAX_WORKFLOW_BYTES
        )));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("the workflow is not UTF-8"))?;
    let document: DocumentMut = text
        .parse()
        .map_err(|error| invalid(format!("invalid TOML: {error}")))?;
    let root = document.as_table();
    only_keys(
        root,
        &[
            "schema", "provider", "program", "args", "network", "inputs", "outputs", "env",
            "limits",
        ],
        "the workflow",
    )?;
    schema(root)?;
    match root.get("network").map(|item| item.as_str()) {
        None | Some(Some("not_enforced")) => {}
        Some(Some("none")) => {
            return Err(FunctorError::unsupported(
                "`network = \"none\"`: the local provider cannot enforce a network policy",
            ));
        }
        Some(_) => {
            return Err(invalid("`network` must be `\"not_enforced\"` or omitted"));
        }
    }

    let mut spec = WorkflowSpec {
        provider: required_string(root, "provider", "the workflow")?,
        program: required_string(root, "program", "the workflow")?,
        ..WorkflowSpec::default()
    };
    if let Some(item) = root.get("args") {
        spec.args = string_array(item, "args", "the workflow")?;
    }
    if let Some(item) = root.get("inputs") {
        spec.inputs = inputs(table_of(item, "inputs", "the workflow")?)?;
    }
    if let Some(item) = root.get("outputs") {
        spec.outputs = outputs(table_of(item, "outputs", "the workflow")?)?;
    }
    if let Some(item) = root.get("env") {
        (spec.env_allow, spec.env_set) = env(table_of(item, "env", "the workflow")?)?;
    }
    if let Some(item) = root.get("limits") {
        spec.limits = Some(limits(table_of(item, "limits", "the workflow")?)?);
    }
    Ok(Workflow::new(spec)?)
}

#[cfg(test)]
mod tests {
    use functor_core::ErrorKind;

    use super::*;

    const FULL: &str = r#"
schema = "functor.workflow/1"
provider = "local-process"
program = "tools/summarize.exe"
args = ["inputs/data", "outputs/summary"]

[inputs]
data = { required = true }
extra = { required = false }

[outputs]
summary = { type = "dataset", format = "text", required = true }
notes = { type = "note", format = "markdown", required = false }

[env]
allow = ["LANG"]
set = { MODE = "fast" }

[limits]
timeout_seconds = 30
stream_bytes = 4096
"#;

    fn kind(text: &str) -> ErrorKind {
        parse_workflow(text.as_bytes()).unwrap_err().kind()
    }

    #[test]
    fn the_documented_example_parses_with_everything_in_order() {
        let workflow = parse_workflow(FULL.as_bytes()).unwrap();
        assert_eq!(workflow.provider(), "local-process");
        assert_eq!(workflow.program(), "tools/summarize.exe");
        assert_eq!(workflow.args(), ["inputs/data", "outputs/summary"]);
        let inputs: Vec<(&str, bool)> = workflow
            .inputs()
            .iter()
            .map(|i| (i.name.as_str(), i.required))
            .collect();
        assert_eq!(inputs, [("data", true), ("extra", false)]);
        let outputs: Vec<(&str, &str, &str, bool)> = workflow
            .outputs()
            .iter()
            .map(|o| {
                (
                    o.name.as_str(),
                    o.artifact_type.as_str(),
                    o.format.as_str(),
                    o.required,
                )
            })
            .collect();
        assert_eq!(
            outputs,
            [
                ("summary", "dataset", "text", true),
                ("notes", "note", "markdown", false)
            ]
        );
        assert_eq!(workflow.env_allow(), ["LANG"]);
        assert_eq!(workflow.env_set()["MODE"], "fast");
        assert_eq!(workflow.limits(), ExecutionLimits::new(30, 4096).unwrap());
    }

    #[test]
    fn the_smallest_workflow_uses_the_defaults() {
        let workflow = parse_workflow(
            b"schema = \"functor.workflow/1\"\nprovider = \"local-process\"\nprogram = \"tool\"\n",
        )
        .unwrap();
        assert!(workflow.args().is_empty() && workflow.inputs().is_empty());
        assert_eq!(workflow.limits(), ExecutionLimits::DEFAULT);
    }

    #[test]
    fn tables_and_inline_tables_are_both_accepted_and_required_defaults_to_true() {
        let workflow = parse_workflow(
            br#"
schema = "functor.workflow/1"
provider = "local-process"
program = "tool"

[inputs.data]

[outputs.out]
type = "dataset"
format = "text"
"#,
        )
        .unwrap();
        assert!(workflow.inputs()[0].required);
        assert!(workflow.outputs()[0].required);
    }

    #[test]
    fn a_newer_schema_and_an_unenforceable_network_policy_are_unsupported() {
        assert_eq!(
            kind(&FULL.replace("workflow/1", "workflow/2")),
            ErrorKind::Unsupported
        );
        assert_eq!(
            kind(&format!("network = \"none\"\n{FULL}")),
            ErrorKind::Unsupported
        );
        // The documented value is accepted; anything else is a plain input error.
        assert!(parse_workflow(format!("network = \"not_enforced\"\n{FULL}").as_bytes()).is_ok());
        assert_eq!(
            kind(&format!("network = \"anything\"\n{FULL}")),
            ErrorKind::InvalidInput
        );
        assert_eq!(
            kind(&FULL.replace("functor.workflow/1", "something.else/1")),
            ErrorKind::InvalidInput
        );
    }

    #[test]
    fn unknown_keys_are_refused_at_every_level() {
        for (name, text) in [
            ("top level", FULL.replace("[inputs]", "extra = 1\n[inputs]")),
            (
                "input",
                FULL.replace("{ required = true }", "{ required = true, x = 1 }"),
            ),
            (
                "output",
                FULL.replace(
                    "required = true }\nnotes",
                    "required = true, x = 1 }\nnotes",
                ),
            ),
            (
                "env",
                FULL.replace("allow = [\"LANG\"]", "allow = [\"LANG\"]\nx = 1"),
            ),
            (
                "limits",
                FULL.replace("stream_bytes = 4096", "stream_bytes = 4096\nx = 1"),
            ),
        ] {
            assert_eq!(kind(&text), ErrorKind::InvalidInput, "{name}");
        }
    }

    #[test]
    fn wrong_types_and_missing_keys_are_described() {
        for (name, text) in [
            (
                "program missing",
                FULL.replace("program = \"tools/summarize.exe\"\n", ""),
            ),
            (
                "provider missing",
                FULL.replace("provider = \"local-process\"\n", ""),
            ),
            (
                "schema missing",
                FULL.replace("schema = \"functor.workflow/1\"\n", ""),
            ),
            ("program type", FULL.replace("\"tools/summarize.exe\"", "5")),
            (
                "args type",
                FULL.replace("[\"inputs/data\", \"outputs/summary\"]", "\"x\""),
            ),
            ("args element", FULL.replace("\"inputs/data\"", "1")),
            (
                "required type",
                FULL.replace(
                    "{ required = true }\nextra",
                    "{ required = \"yes\" }\nextra",
                ),
            ),
            (
                "output type missing",
                FULL.replace("type = \"dataset\", ", ""),
            ),
            (
                "output format missing",
                FULL.replace("format = \"text\", ", ""),
            ),
            ("env set value", FULL.replace("MODE = \"fast\"", "MODE = 1")),
            (
                "limits type",
                FULL.replace("timeout_seconds = 30", "timeout_seconds = \"30\""),
            ),
            (
                "limits negative",
                FULL.replace("timeout_seconds = 30", "timeout_seconds = -1"),
            ),
            (
                "inputs not a table",
                FULL.replace(
                    "[inputs]\ndata = { required = true }\nextra = { required = false }\n",
                    "inputs = 3\n",
                ),
            ),
        ] {
            assert_eq!(kind(&text), ErrorKind::InvalidInput, "{name}");
        }
    }

    #[test]
    fn values_the_model_refuses_are_refused() {
        for (name, text) in [
            (
                "absolute program",
                FULL.replace("tools/summarize.exe", "/usr/bin/tool"),
            ),
            ("secret env name", FULL.replace("\"LANG\"", "\"API_TOKEN\"")),
            ("bad input name", FULL.replace("extra", "Bad Name")),
            (
                "timeout over the maximum",
                FULL.replace("timeout_seconds = 30", "timeout_seconds = 3601"),
            ),
            (
                "zero stream limit",
                FULL.replace("stream_bytes = 4096", "stream_bytes = 0"),
            ),
            (
                "bad artifact type",
                FULL.replace("\"dataset\"", "\"Data Set\""),
            ),
        ] {
            assert_eq!(kind(&text), ErrorKind::InvalidInput, "{name}");
        }
    }

    #[test]
    fn limits_above_the_defaults_are_clamped_not_raised() {
        let workflow = parse_workflow(
            FULL.replace("timeout_seconds = 30", "timeout_seconds = 3600")
                .replace("stream_bytes = 4096", "stream_bytes = 8388608")
                .as_bytes(),
        )
        .unwrap();
        assert_eq!(workflow.limits(), ExecutionLimits::DEFAULT);
    }

    #[test]
    fn content_that_is_not_text_or_toml_or_is_too_large_is_refused() {
        assert_eq!(
            parse_workflow(&[0xff, 0xfe, 0x00]).unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
        assert_eq!(kind("this is = = not toml"), ErrorKind::InvalidInput);
        let padding = "#".repeat(usize::try_from(bounds::MAX_WORKFLOW_BYTES).unwrap());
        assert_eq!(kind(&format!("{FULL}{padding}")), ErrorKind::InvalidInput);
        // Exactly at the limit is fine.
        let room = usize::try_from(bounds::MAX_WORKFLOW_BYTES).unwrap() - FULL.len();
        let at_limit = format!("{FULL}{}", "#".repeat(room));
        assert_eq!(at_limit.len() as u64, bounds::MAX_WORKFLOW_BYTES);
        assert!(parse_workflow(at_limit.as_bytes()).is_ok());
    }
}
