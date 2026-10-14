# Error Handling

Status: Accepted
Related RFCs: RFC-0002, RFC-0007
Related docs: [api-design-guidelines.md](api-design-guidelines.md), [cli-spec.md](../cli/cli-spec.md), [validation-rules.md](../design/validation-rules.md)

## Purpose

Defines the error taxonomy, how errors map to CLI exit codes and messages, and the boundary between **errors** and **validation findings**.

## Errors vs Findings

- A **`FunctorError`** means an operation could not be performed: bad input, missing entity, I/O failure.
- A **validation `Finding`** means the operation worked and discovered a problem in repository data. `functor validate` finding problems is a successful run with outcome `Failed` and exit code 6, not a `FunctorError`.
- `validate` returns a `FunctorError` only if it cannot do its job at all (not a repository, unreadable root).

## Taxonomy

```rust
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum FunctorError {
    #[error("{entity} not found: {reference}")]
    NotFound { entity: EntityKind, reference: String, suggestions: Vec<String> },

    #[error("{detail}")]
    Conflict { entity: EntityKind, detail: String },        // duplicate slug, stale revision, duplicate relationship

    #[error("invalid {field}: {reason}")]
    InvalidInput { field: &'static str, reason: String },   // grammar and constraint violations

    #[error("not a functor repository: {path}")]
    NotARepository { path: PathBuf },

    #[error("unsupported repository format version {found}; supported: {supported}")]
    UnsupportedFormat { found: u32, supported: u32 },

    #[error("repository data is inconsistent: {detail}; run `functor validate`")]
    Inconsistent { detail: String },                       // strict snapshot hit a LoadIssue

    #[error("repository is locked by another process: {path}")]
    Locked { path: PathBuf },

    #[error("storage error during {operation} on {path}")]
    Storage { operation: &'static str, path: PathBuf, #[source] source: std::io::Error },

    #[error("internal error: {0}")]
    Internal(String),                                       // bug; should never be user-caused

    #[error("permission denied for {action}: {reason}")]
    PermissionDenied { action: String, reason: String },    // no execution grant, policy refusal (ADR-0012)

    #[error("unsupported: {what}")]
    Unsupported { what: String },                           // unknown provider, newer workflow schema
}
```

`EntityKind` is `Artifact | Relationship | Repository | Event` and is defined in `functor-model`.

`FunctorError::kind()` returns a data-less `ErrorKind` (one per variant, `ErrorKind::ALL` lists them, `ErrorKind::as_str()` is the stable wire code). Projections map **kinds**, not errors, so adding a variant forces every mapping to be updated at compile time. `From<ModelError>` produces `InvalidInput`; `From<SnapshotError>` produces `Inconsistent`.

## Exit Codes

Stable across releases. New codes are only added, never reassigned.

| Code | Meaning | Source |
|------|---------|--------|
| 0 | Success. `validate`: outcome `Passed` or `Warning` (see `--strict` in [cli-spec.md](../cli/cli-spec.md)) | |
| 1 | Unexpected internal error | `Internal`, panics caught at the top level |
| 2 | Usage or input error | clap parse errors, `InvalidInput`, `Unsupported` |
| 3 | Not found | `NotFound` |
| 4 | Conflict | `Conflict` |
| 5 | Not a repository or unsupported format | `NotARepository`, `UnsupportedFormat`, `Inconsistent` |
| 6 | Validation failed | `validate` outcome `Failed` (or `Warning` with `--strict`) |
| 7 | Storage or locking failure | `Storage`, `Locked` |
| 8 | A run did not succeed | `run start`: outcome other than `succeeded`, or an idempotent request returning unfinished work |
| 9 | Permission denied | `PermissionDenied` (no execution grant or a policy refusal) |

Codes 8 and 9 are the Phase 1 additions ([execution-and-evidence.md](../design/execution-and-evidence.md)); codes 0 to 7 are unchanged.

`watch` exits 0 on Ctrl-C (SIGINT) or when its output pipe closes, and 7 if the event log becomes unreadable.

`run start` handles Ctrl-C by cancelling and awaiting the child tree and terminal evidence, then exits 8. A failed mandatory write instead exits with its infrastructure error code. `validate --save-report` always prints its computed report; any save failure exits 7 and is separate from the validation outcome.

## Message Rules

User-facing error text (stderr, `text` mode):

```text
error: artifact not found: rfc-0009
  did you mean: rfc-0000, rfc-0001
hint: run `functor artifact list` to see available artifacts
```

- First line `error: <message>` with lowercase start and no trailing period.
- Optional indented detail lines and `hint:` lines.
- The error source chain is printed only with `-v` or `--verbose`.
- Never print stack traces. Internal errors print `error: internal error: <message>` and ask the user to report it; the backtrace appears with `RUST_BACKTRACE=1`.
- Messages must name the user-visible reference (slug or id), never an internal path unless the error is about that path.

In `--output json` mode errors go to **stderr** as one JSON object so scripts can parse them. Stdout stays empty except when a read-only execution plan or a computed validation/run report is already available:

```json
{"error":{"code":"not_found","message":"artifact not found: rfc-0009","entity":"artifact","reference":"rfc-0009","suggestions":["rfc-0000"]},"exit_code":3}
```

`code` is a stable snake_case value per variant: `not_found`, `conflict`, `invalid_input`, `not_a_repository`, `unsupported_format`, `inconsistent`, `locked`, `storage`, `internal`, `permission_denied`, `unsupported`. Execution infrastructure errors add an optional top-level `run` id when a durable intent exists, so inspection and recovery remain possible.

## Propagation Rules

- Library crates convert foreign errors at their boundary (`std::io::Error` to `Storage` with the operation and path; TOML parse errors during a strict load to `Inconsistent`). Foreign error types never appear in public signatures.
- `functor-cli` has no error glue crate: every error is a `FunctorError` or a usage error before the top-level handler maps it to an exit code. The mapping lives in **one** function, `exit_code(ErrorKind)` in `src/app/cli/src/exit.rs`, written as a `match` without a wildcard arm and covered by a test that lists every kind. The exit-code constants (`SUCCESS`, `USAGE`, `VALIDATION_FAILED`, ...) live in the same module.
- A warning (for example, "event could not be appended") is not an error. It is printed to stderr as `warning: ...` and does not change the exit code, per [event-model-mvp.md](../design/event-model-mvp.md).

## Logging vs Errors

Diagnostics are plain `error:` and `warning:` lines on stderr and, with `-v`, the cause chain of an error. Errors are values; nothing replaces them and nothing goes to stdout. There is no logging framework in the MVP. See [nonfunctional-requirements.md](../quality/nonfunctional-requirements.md).

## Test Obligations

- Exhaustive mapping test from every `FunctorError` variant to exit code and JSON `code`.
- Golden tests for the text form of each variant.
- A test asserting `--output json` errors are valid JSON on stderr and stdout is empty.
