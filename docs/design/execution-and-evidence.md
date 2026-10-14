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
| Kill grace after cancel/timeout | 2 s, then force | |

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

## Review Decisions

Reviewed with the maintainer during P1.0 and accepted together with ADR-0012 to ADR-0015.

1. Names of outcomes, evidence types and slug prefixes are final as written above.
2. The numeric limits above are final for Phase 1.
3. The Windows process-tree kill uses a third-party Job Object crate, chosen and recorded under the dependency policy before P1.3 starts. No `unsafe` exception is made.
4. Network access is not enforced. Provenance records `network = "not_enforced"`; a workflow declaring `network = "none"` is refused with `Unsupported`.
5. Execution permission is `--grant-exec` **plus** an interactive confirmation when stdin is a terminal: the CLI prints the plan, and anything other than an explicit `yes` is exit 9 with no capture or write. When stdin is not a terminal the flag alone is sufficient, so scripts work. The library never prompts; callers pass an `ExecutionGrant`.
