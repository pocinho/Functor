# ADR-0015: Persisted validation reports and execution events

## Status

Accepted

## Context

`ValidationEngine` returns a `ValidationReport`; `validate` is read-only and only publishes `validation.completed` ([validation-rules.md](../design/validation-rules.md), [event-model-mvp.md](../design/event-model-mvp.md)). C-0001-4 and C-0007-3 require validation results to be representable as artifacts. Event types and the `validate` command output are frozen contracts.

## Decision

### Validation reports

1. **Opt-in only.** `validate` stays read-only. `validate --save-report` (library: an explicit `ReportRecorder` collaborator, not a `ValidationEngine` change) persists the report after it is computed. The engine never writes.
2. **Report artifact.** Type `validation_report`, `metadata.evidence = "functor.report/1"`, immutable (ADR-0013). Content is JSON with the same shape as the `validate` JSON output (`schema_version` 1) stored at `content/evidence/reports/<artifact-id>.json`; metadata carries `outcome`, counts, `strict`, rule-set identity (`functor.rules/V0001-V00NN`), engine version and the subject. The subject is a capture of the validated state: for a whole-repository run, the repository id; for a scoped run, the artifact id **and** its revision and, when it has content, the content digest.
3. **Relationship.** `validation_report --validates--> subject artifact` ([ADR-0014](ADR-0014-evidence-relationships.md)). Whole-repository reports have no subject edge.
4. **No recursion.** Reports are computed from non-evidence data in a pass that **excludes** `validation_report` artifacts as subjects, and saving a report does not trigger another validation. A report describes the state it saw; later changes do not alter it.
5. **Save failure.** The computed `ValidationReport` is always returned, with an error describing the save failure (`Storage` or `Locked`); the CLI prints the report and the failure and exits `7`. Validation outcome and save outcome are separate fields.
6. **Output validation in runs.** When a run request asks for validation of its outputs, the orchestrator validates each output artifact as a scoped subject **after** `run_result` is committed, then saves one report per requested subject. A validation `Failed` outcome never changes the run outcome.

### Events

7. **New event types** (envelope `v` 1 unchanged; adding types is additive and this ADR is the deliberate edit): 

   | Type | Subject | Data |
   |------|---------|------|
   | `run.started` | run id | `workflow`, `provider`, `request_id` |
   | `run.completed` | run id | `outcome`, `result`, `outputs` (ids), `truncated` |
   | `validation.recorded` | report id | `outcome`, `subject`, `errors`, `warnings`, `infos` |

   `validation.completed` is unchanged and still published by `validate`, with or without saving.
8. **Single owner.** Only the orchestrator publishes execution events, after the durable transition: `run.started` after the intent and relationships are committed and before spawn; `run.completed` after `run_result` and relationships are committed. Evidence artifacts written through the orchestrator do **not** also publish `artifact.created`, so there are no duplicate notifications. A failed append is the usual warning; a failed mandatory write is never masked by an event, and no success event is published when a mandatory write failed.
9. **Readers.** Events are informational. Evidence inspection reads artifacts only, so lost or pruned logs do not hide evidence. `watch` filters accept the new type prefixes (`run.`, `validation.`) and never write.
10. **Recovery events.** `run recover` publishes `run.completed` with `outcome = interrupted`.

## Consequences

- `validate` output for the default path is byte-identical to Phase 0; the flag adds an optional `saved_report` object to the JSON (schema updated, `schema_version` stays 1 because the property is optional and absent by default).
- Old readers must ignore unknown event types. P1.1 adds a test that an unknown type does not break `watch` or log reading.
- Reports for a damaged repository are still produced; saving is refused when the repository is `Inconsistent` (writes are refused already), and the report is printed.

## RFC Impact

Implements C-0001-4 and C-0007-3 (report artifacts) and extends the event subset toward RFC-0017 with run and validation events. Removes the matching deviation once tests exist.
