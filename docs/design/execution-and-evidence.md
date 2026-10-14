# Execution and Evidence Contract (Phase 1)

Status: Accepted (P1.0 decision contract with ADR-0012 to ADR-0015; not yet implemented)
Related ADRs: [ADR-0012](../adr/ADR-0012-execution-provider-and-safety.md), [ADR-0013](../adr/ADR-0013-run-lifecycle-and-evidence-persistence.md), [ADR-0014](../adr/ADR-0014-evidence-relationships.md), [ADR-0015](../adr/ADR-0015-validation-report-persistence-and-events.md)
Related RFCs: RFC-0001, RFC-0004, RFC-0007, RFC-0017
Roadmap: [phase-1-execution-provenance-roadmap.md](../planning/phase-1-execution-provenance-roadmap.md)

## Purpose

One place that states the concrete values and shapes P1.1 to P1.5 implement: workflow format, limits, evidence records, errors, events, rules, command spelling and test obligations. The ADRs hold the reasoning; this document holds the numbers. A value changes only through a new ADR.

## Workflow Artifact

`type = "workflow"`, content `format = "toml"`, inline or by path (1 MiB limit as for any content):

```toml
schema = "functor.workflow/1"
provider = "local-process"
program = "python"
args = ["scripts/summarize.py", "--in", "inputs/data", "--out", "outputs/summary"]

[inputs]
data = { required = true }

[outputs]
summary = { type = "report_data", format = "text" }

[env]
allow = []
set = { LANG = "C" }

[limits]
timeout_seconds = 30
```

Rules:

- `schema`, `provider`, `program` are required. `args` are literal strings; no expansion, no globbing, no shell.
- `[inputs]` names and `[outputs]` names follow the tag grammar. A run binds each input name to exactly one existing artifact; an unbound required input, an unknown binding, or a duplicate binding is `InvalidInput`.
- An output named `summary` is read from `outputs/summary` in the work directory. It becomes an artifact with the declared `type` and `format`. Missing required output means outcome `failed`. A declared output that is a directory, a symlink or over the size limit is `failed` with a diagnostic.
- `[limits]` may only **lower** the defaults below, never raise them past the hard maximum. The grant may lower them further.
- Unknown top-level keys are rejected for `functor.workflow/1` (an executable format is stricter than data files). A newer `schema` is `Unsupported`.

## Limits

| Limit | Default | Hard maximum |
|-------|---------|--------------|
| Wall-clock timeout | 60 s | 3600 s |
| Captured stdout and stderr | 1 MiB each (truncate, set `truncated`) | 8 MiB each |
| Arguments | 64 args, 1024 bytes each | same |
| Environment variables set or allowed | 32 | 32 |
| Inputs | 32, 16 MiB each, 64 MiB total | same |
| Outputs | 16, 16 MiB each, 64 MiB total | same |
| Workflow content | 1 MiB | 1 MiB |
| Lock wait (each short write) | 5 s (`FUNCTOR_LOCK_TIMEOUT`) | |
| Kill grace after cancel/timeout | none: the process tree is terminated at once (Job Object termination on Windows, `SIGKILL` to the process group elsewhere) | |
| Output drain after the tree is gone | 2 s, then the stream is cut and marked truncated | |

Each row gets a boundary test at the exact limit, one byte or unit over, and one under. Truncated output is stored in the `run_result` content (stdout and stderr as separate sections) with the truncation flags; truncation is not a failure.

## Evidence Records

All evidence artifacts carry `metadata.evidence = "<schema>"` and are immutable. Metadata values follow the existing typed rule (string, integer, boolean, array of strings); structured detail goes in the artifact content (JSON).

| Type | `evidence` | Metadata | Content |
|------|-----------|----------|---------|
| `capture` | `functor.capture/1` | `source` (artifact id), `source_revision` (int), `sha256`, `size`, `role` (`workflow` or `input`) | the captured bytes as a path to `content/evidence/captures/<hex>` |
| `run` | `functor.run/1` | `provider`, `workflow_capture`, `request_id`, `started` (RFC 3339), `network` (`not_enforced`) | JSON: bindings (name to capture id), arguments, limits, environment names, program path, program `sha256`, provider version, platform |
| `run_result` | `functor.run_result/1` | `run`, `outcome`, `finished`, `exit_code` (int, absent when none), `truncated` (bool), `outputs` (ids) | JSON: stdout, stderr, diagnostics, per-output digests |
| `validation_report` | `functor.report/1` | `outcome`, `errors`, `warnings`, `infos`, `strict`, `subject`, `rules` | JSON as `validate` output |

Slugs for evidence artifacts are generated: `run-<ulid-suffix>`, `run-result-...`, `capture-<first 12 hex>-...`, `report-...`; collisions retry with a new id.

## Outcomes and Errors

Run outcome (data in `run_result`): `succeeded`, `failed`, `timed_out`, `cancelled`, `start_failed`, `interrupted`. Derived, not stored: `running`, `interrupted_unknown`.

New `FunctorError` variants (the enum is `#[non_exhaustive]`; new `ErrorKind` entries and exit codes are additive):

| Variant | Kind code | Exit code | When |
|---------|-----------|-----------|------|
| `PermissionDenied { action, reason }` | `permission_denied` | 9 | No or insufficient grant; a disallowed environment name; a path outside the boundary |
| `Unsupported { what }` | `unsupported` | 2 | Unknown provider, newer workflow schema, `network = "none"` |

Existing mappings are unchanged. New exit code **8**: the command ran, but the run's outcome is not `succeeded` (outcome and run id are in the output). Exit 7 covers `Storage` and `Locked`, including a failed mandatory publication. Exit 6 stays validation-only. A cancelled CLI run (Ctrl-C) ends the child tree, records `cancelled` when storage allows, and exits 8.

| Situation | Child started | Evidence | Result |
|-----------|---------------|----------|--------|
| No grant | no | none | `PermissionDenied`, exit 9 |
| Pre-run write fails | no | possible orphan captures | `Storage`, exit 7 |
| Program missing | no | intent plus `start_failed` | outcome, exit 8 |
| Non-zero exit, missing output | yes | full | outcome `failed`, exit 8 |
| Timeout, cancel | yes (tree killed) | full | outcome, exit 8 |
| Post-run write fails | yes | intent only; staging kept | `Storage` with run id, exit 7 |
| Process killed mid-run | yes | intent only | `interrupted_unknown`; recover marks `interrupted` |
| Event append fails | yes | full | warning only; exit follows the outcome |

## New Validation Rules

Existing codes `V0001` to `V0019` are not renumbered. `V0012` stops covering `supports`, `contradicts` and `validates`; `V0009` also covers self-reference of those three.

| Code | Severity | Description |
|------|----------|-------------|
| `V0020` | Warning | A `run` has no `run_result` (running or interrupted) |
| `V0021` | Error | Evidence integrity: a capture file is missing or its digest differs, a `run_result` names a missing output, or an evidence record is malformed |
| `V0022` | Warning | Orphan evidence: a capture or output with no `run`, or a staging directory without a record |
| `V0023` | Warning | One source both `supports` and `contradicts` the same target |

These run in ordinary `validate` (read-only). Damaged or missing captured evidence is `V0021` and an `Error`, not a crash.

## Events

As in [ADR-0015](../adr/ADR-0015-validation-report-persistence-and-events.md): `run.started`, `run.completed`, `validation.recorded`. No `artifact.created` for evidence artifacts.

## Command Spelling

| Command | Purpose |
|---------|---------|
| `functor run start <workflow> [--input <name>=<artifact>]... [--grant-exec] [--timeout <s>] [--validate-outputs] [--request-id <id>]` | Execute. Without `--grant-exec` the command prints the plan (program, arguments, limits, network note) and exits 9. With it and a terminal on stdin, the plan is printed and an explicit `yes` is required, otherwise exit 9 |
| `functor run show <run>` | Intent, state, outcome, captures, outputs, truncation |
| `functor run list` | All runs with derived state |
| `functor run recover` | Mark interrupted runs; repair implied relationships; never reruns |
| `functor validate [artifact] [--strict] [--save-report]` | Opt-in persisted report |

Evidence navigation uses `query dependencies`, `query dependents` and `query relationships`. Assertions use `relation add <evidence> supports|contradicts <claim>`. `<run>` resolves by artifact slug or id. JSON schemas (`schema_version` 1): `run-start`, `run-show`, `run-list`, `run-recover`; `validate` gains an optional `saved_report`; `error.json` gains the two new kinds. All existing schemas and golden transcripts stay unchanged.

## Crates

`functor-model`: evidence ids/values, `ContentDigest`, relationship types. `functor-core`: `ExecutionProvider`, `EvidenceRecorder`/`ReportRecorder` traits, `RunRequest`, `ExecutionGrant`, error kinds. `functor-testkit`: fake provider, evidence fixtures, conformance suite. `functor-store` and `functor-repository`: evidence persistence, per-run lock, layout. New `functor-runtime`: `local-process` provider and orchestrator, depending on core traits only. `functor-cli`: handlers only. SHA-256 (`sha2`) is added to `functor-store`/`functor-runtime`, and a Job Object crate to `functor-runtime`, each under the dependency-policy checklist; neither enters model or core.

## Format and API Evolution

All additions are additive: `format_version` 1, event `v` 1, schema `schema_version` 1 stay. There is no migration. Old binaries read evidence as ordinary artifacts and relationships and warn `V0012` for the three promoted types. Phase 2 binds captures and reports to immutable versions without rewriting them.

## Test Obligations

| Conformance | Obligation |
|-------------|------------|
| C-0001-4, C-0007-3 | Report artifact round trip; saved report equals the printed report; save failure returns the report; no recursive reports |
| C-0001-5, C-0007-4 | Reproduce the deterministic fixture from captures after the live input changes; declared outputs equal |
| C-0007-5 | Complete traceability both directions; interrupted and orphan states visible |
| C-0004-2 | Built-in parsing of the three names, direction, self-reference, duplicates, `V0023`, stored custom compatibility |
| C-0001-1 | Library-only and CLI paths over the same records (existing row, extended) |

Test families from the roadmap apply: lifecycle invariants; capture under concurrent edit; failure injection at every publication step in [ADR-0013](../adr/ADR-0013-run-lifecycle-and-evidence-persistence.md); every limit boundary; path escape; disallowed environment; cancellation with child cleanup on Windows; schema and golden tests for each CLI outcome; unknown event type tolerated by readers; `validate` default output unchanged.

## Implementation Notes (P1.1)

Choices made while writing the model, core traits and testkit; none changes an accepted ADR.

- **Where things live.** Values (`Workflow`, `ExecutionLimits`, `RunIntent`, `RunResult`, `RunRecord`, `Capture`, `ContentDigest`, `EvidenceKind`) are in `functor-model`; every numeric bound of the limits table is a constant in `functor_model::bounds`. `ExecutionProvider`, `EvidenceRecorder`, `ReportRecorder`, the plan, grant and cancel types are in `functor-core`. The model hashes nothing: `ContentDigest` validates and formats `sha256:<hex>`, and the SHA-256 dependency stays in the store and runtime crates. The testkit uses a clearly non-cryptographic `fake_digest`.
- **Failed runs publish no outputs.** The ADRs left open whether a failed run retains partial outputs. P1.1 fixes it: only a `succeeded` run lists outputs; stdout, stderr and diagnostics of every run are kept in the `run_result`. `RunResult::new` and `ProviderRun::check` enforce it, so partial output cannot be mistaken for a result.
- **Exit status shape.** Only `succeeded` and `failed` runs carry an exit status; `timed_out`, `cancelled`, `start_failed` and `interrupted` have none, because the process was killed or never ran. A `failed` run that exited 0 (a missing required output) must carry a diagnostic.
- **Error mapping.** `PermissionDenied` maps to exit 9 and `Unsupported` to exit 2 now. Exit 8 and the `run` commands arrive in P1.5. The `error.json` schema accepts both new codes and exit codes up to 9.
- **Rules and events deferred.** `V0020` to `V0023` and the event types are implemented in P1.4; this step changes only the `V0009` description (evidence types) and the scope of `V0012`.
- **Relationship promotion.** The only existing tests that used `supports` as the custom example now use `related_to`; the transcript for the custom-type warning changed accordingly. Self-reference of `supports`, `contradicts` and `validates` is rejected by the model constructor and reported as `V0009` when loading a file.
- **Provider pre-flight.** `ExecutionPlan::check` is the shared refusal logic (provider name, limits against the grant, granted environment names, input bindings and sizes). The path, program-resolution and environment-value rules of ADR-0012 are enforced by the `local-process` provider in P1.3, which must also run `run_provider_conformance`.
- **Lease.** `RunLease` is `Debug` so it can appear in test assertions; dropping it releases the run.

## Implementation Notes (P1.2)

Durable evidence is implemented by `EvidenceStore` in `functor-store`, generic over the entity store and a blob/lock backend (`FileEvidence` over `FileStore`, `InMemoryEvidence` over `MemoryStore`). decisions that refine the ADRs are in [ADR-0016](../adr/ADR-0016-evidence-files-and-repair.md) (Accepted). In short:

- **Files are content-addressed.** Reports are `content/evidence/reports/<sha256 hex>`, not `<artifact-id>.json` as this contract first said, because the store assigns the artifact id. Streams, outputs and results are files too; the run intent stays inline JSON.
- **Stored report document.** It is the `functor validate --output json` document (schema version 1); the validation time and subject (id, revision, content digest) are artifact metadata. Findings round trip exactly, including the incomplete flag, which is recovered from the stored outcome.
- **Idempotent intent.** One request id is serialized by a short lock file; a repeat returns the existing run with `created = false` and repairs its edges.
- **Edges.** `capture --derived_from--> source`, `run --depends_on--> capture`, `run_result --derived_from--> run`, `output --derived_from--> run_result` and `report --validates--> subject` are created in publication order and recreated by `recover` from the records. A failure after the commit point is returned as an error while the run reads as complete.
- **Immutability.** The shared store rules refuse to update an artifact whose type and `evidence` metadata agree (`Conflict`), so both stores and the CLI inherit it; the store conformance suite pins it.
- **Not yet done.** Staging directories for provider work (`.functor/runs/<run-id>/`) are created by the orchestrator in P1.4. Validation rules `V0020` to `V0023`, the events and the CLI are P1.4 and P1.5. The persisted rule-set identity of ADR-0015 is not recorded: the recorder does not know which engine produced a report.
- **Dependencies.** `sha2` and `serde_json` were added to `functor-store` only (licenses, bans and sources checked with `cargo deny check licenses bans sources`).

## Implementation Notes (P1.3)

`LocalProcessProvider` in `functor-runtime` implements ADR-0012. How the controls are enforced and what was decided while building it:

- **Not a sandbox.** The child has the caller's privileges and network. What is enforced: no shell (structured program and arguments), the granted program, a cleared environment, bounded streams and outputs, a timeout, cancellation and tree termination.
- **Program and grant.** A bare name is looked up on the caller's `PATH` (with `PATHEXT` on Windows); a path with a separator is repository-relative and may not leave the repository, including through a link (`PermissionDenied`). The resolved absolute path must equal the granted program, otherwise `PermissionDenied` before anything is staged. A program that does not exist, or exists but cannot start, is a `start_failed` outcome, not an error. The program file is hashed (streamed) and recorded.
- **Environment.** Cleared, then `SystemRoot`, `PATH`, `TEMP`, `TMP` (Windows) or `PATH`, `TMPDIR`, then the workflow's allowed names that the grant lists, then the workflow's literal values. `env_names` returns names only, for provenance.
- **Staging.** `<work root>/<run id>/work` holds `inputs/<name>` and an empty `outputs/`, and is the child's working directory. It is deleted and recreated for each execution of a run id, and left in place afterwards: the orchestrator (P1.4) keeps it after a failure and removes it after publication. Inputs are verified against their captured digests before anything is staged (`InvalidInput`).
- **Outputs.** Read only after exit status 0, from `outputs/<name>`: a missing required output, a non-regular file (directory or link), a file over 16 MiB or more than 64 MiB in total makes the run `failed` with a diagnostic and no outputs.
- **Termination.** There is no grace period: on timeout or cancellation the whole job is terminated at once. The same termination runs after a normal exit, so a child left behind by a finished run is cleaned up. A token that is already cancelled starts nothing.
- **Output drain.** Reader threads keep draining past the limit so the child never blocks on a full pipe. After the tree is gone the supervisor waits at most 2 s for end-of-file; because a Windows process started at the same moment can inherit a pipe's write end, a stream still open after that is cut and marked truncated.
- **Wrapper caveat.** `process-wrap`'s `wait` and `try_wait` both consume the job's completion port, which made a first version hang under parallel load. The supervisor polls and reaps the leader process and uses the job only to terminate ([vetting record](../architecture/dependency-policy.md#vetting-record-process-wrap-adr-0012)).
- **Test program.** `runtime-fixture` (`functor-runtime/fixture/main.rs`) is a deterministic helper built with the crate for the real-process tests; it is not part of the product.
- **Not covered.** Network is not enforced (accepted in ADR-0012), so there is no network test. A repository-relative program that is a symbolic link out of the repository is covered by the same canonical-path check as `..`, but only `..` is tested because creating links on Windows needs a privilege. Non-Windows results are informational.

## Review Decisions

Reviewed with the maintainer during P1.0 and accepted together with ADR-0012 to ADR-0015.

1. Names of outcomes, evidence types and slug prefixes are final as written above.
2. The numeric limits above are final for Phase 1.
3. The Windows process-tree kill uses a third-party Job Object crate, chosen and recorded under the dependency policy before P1.3 starts. No `unsafe` exception is made.
4. Network access is not enforced. Provenance records `network = "not_enforced"`; a workflow declaring `network = "none"` is refused with `Unsupported`.
5. Execution permission is `--grant-exec` **plus** an interactive confirmation when stdin is a terminal: the CLI prints the plan, and anything other than an explicit `yes` is exit 9 with no capture or write. When stdin is not a terminal the flag alone is sufficient, so scripts work. The library never prompts; callers pass an `ExecutionGrant`.

### Decisions taken before P1.4

Agreed with the maintainer after P1.3:

6. **Workflow format ownership.** `functor-runtime` owns the `functor.workflow/1` TOML format and its parser, because it is the crate that consumes it (ADR-0005). Parsing rejects unknown top-level keys, a newer schema (`Unsupported`) and `network = "none"` (`Unsupported`).
7. **Output validation is structural.** The optional validation of outputs (ADR-0015, point 6) runs the existing structural rules scoped to each output artifact. Content-level checks of an output wait for the extension model (Phase 3).
8. **Orchestrator location.** The orchestrator lives in `functor-runtime` next to the provider and depends only on core traits (`EvidenceRecorder`, `ReportRecorder`, the repository, validation and event traits), never on a concrete store, validator or publisher; composition roots wire them.
9. **Request ids.** If `--request-id` is not given, the CLI generates a unique one, so every invocation is a new run. Passing one makes repeating the command idempotent.
10. **Staging cleanup.** After the `run_result` and its edges are committed, the orchestrator removes the run's work folder; on any failure it keeps it for diagnosis.
11. **Rule-set identity.** The caller of `validate --save-report` supplies the rule-set identity (for example `V0001-V0023`), which the recorder stores as report metadata, because the recorder does not know which engine produced a report.
12. **Report document bytes.** A saved report is semantically the same as `validate --output json` but its keys are sorted, so it is not required to be byte-identical.
13. **Benchmark budgets.** Phase-specific execution and evidence-storage budgets are set in P1.6 from measured results and stay informational, like the existing benchmark policy.
