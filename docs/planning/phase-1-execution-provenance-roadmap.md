# Phase 1: Execution and Provenance Roadmap

Status: Complete (working-prototype increment; phase-exit evidence recorded 2026-10-14; not a release)
Related RFCs: [RFC-0007](../../rfcs/RFC-0007-Validation-and-Execution.md), [RFC-0001](../../rfcs/RFC-0001-Core-Semantic-Model.md), [RFC-0004](../../rfcs/RFC-0004-Relationship-Model.md), [RFC-0017](../../rfcs/RFC-0017-Event-Model.md)
Related docs: [product-roadmap.md](product-roadmap.md), [cli-mvp-roadmap.md](cli-mvp-roadmap.md), [mvp-scope.md](mvp-scope.md), [rfc-conformance.md](rfc-conformance.md)

## Purpose

Turn execution and validation into durable, queryable evidence over the proven Phase 0 API-first baseline. A user must be able to explain which workflow and inputs produced an output, what happened during execution, and which validation supports or challenges a conclusion.

This is the implementation roadmap and completion record; the accepted [execution contract](../design/execution-and-evidence.md) and ADRs define behavior. P1.0 closed the decisions before code. Phase 1 completion is a prototype milestone; the first product release remains subject to all phases and the [product-readiness gate](product-roadmap.md#product-readiness-gate).

## Scope

Required:

- Explicit, bounded local workflow execution through a core API and at least one approved provider.
- Durable run records and persisted validation reports represented as artifacts.
- Captured workflow/input/configuration evidence, provider/environment description and output provenance.
- Traceable relationships between inputs, execution, outputs and validation, including defined directional `supports` / `contradicts` semantics.
- Library-only and CLI execution/result inspection; text/JSON contracts and observable events.
- Explicit failure, timeout, cancellation, recovery and permission behavior.

Not included:

- Distributed execution, scheduling, general orchestration or automatic execution on repository open.
- General extension packaging or discovery (Phase 3), network services (Phase 4), or agent-initiated workflows (Phase 5).
- Full immutable repository/entity versioning (Phase 2), signatures/trust infrastructure (Phase 6), or federation (Phase 7).
- A claim that local subprocess execution is a security sandbox or that capturing inputs guarantees every external dependency is reproducible.

## Baseline to Preserve

- Model values have no storage/runtime/serialization dependency; service contracts are dyn-compatible ([ADR-0010](../adr/ADR-0010-async-trait-and-dyn.md)).
- Services are siblings and receive collaborators through core traits. A runtime must not depend on concrete store/validation/event services; composition roots wire them ([crate-dependencies.md](../architecture/crate-dependencies.md)).
- `ValidationEngine` returns a `ValidationReport`; persistence/publication is the caller's responsibility. Existing `validate` does not create report artifacts. Preserve this behavior with an explicit opt-in persistence path unless an approved versioned change replaces it.
- `revision` cannot retrieve an earlier state. Capture exact workflow/input evidence before execution; do not claim the counter provides reproducibility.
- At the Phase 0 baseline, `supports` / `contradicts` used custom-type behavior under [ADR-0007](../adr/ADR-0007-relationship-types.md), including warning `V0012`. Phase 1 deliberately promotes them under [ADR-0014](../adr/ADR-0014-evidence-relationships.md); other custom types retain that warning.
- File/schema/event versions, rule codes and exit codes are [frozen contracts](mvp-scope.md#frozen-contracts). New behavior must not silently alter them.
- Events are informational. An event append warning is not proof of lost run evidence, nor can a successful event append substitute for required evidence persistence.

## Milestone Map and Dependencies

```text
P1.0 Decisions -> P1.1 Contracts + Testkit -> P1.2 Durable Evidence
                                         -> P1.3 Local Provider
                         P1.2 + P1.3 -> P1.4 Evidence Integration
                                             -> P1.5 CLI Projection
                                                   -> P1.6 Phase Exit
```

P1.2 and P1.3 can proceed independently against P1.1 contracts/test doubles. CLI schemas/rendering may be prepared after P1.1, but P1.5 cannot complete before integration. P1.6 requires every earlier milestone. Safety decisions in P1.0 and enforcement in P1.3 precede real execution; completion of Phase 6 is not a substitute.

## P1.0: Close Semantic, Persistence and Safety Decisions

**Deliverables:** accepted RFCs/revisions needed for the execution/evidence subset, ADRs for implementation choices, and contract documents with scope, errors and test obligations. Do not change RFC status merely to match a prototype.

| Decision | Must specify before implementation |
|----------|------------------------------------|
| Workflow/provider contract | Executable artifact eligibility, input/output binding, configuration schema, provider selection, supported local operation and unsupported-provider error |
| Run lifecycle | Proposed states/transitions, start and terminal timestamps, outcome versus infrastructure error, cancellation/timeout, interrupted/unknown state and recovery ownership; final names require approval |
| Input evidence | Exact captured bytes/state and digests for workflow and inputs, capture consistency under concurrent edits, configuration, provider/version, environment allowlist and declared external dependencies |
| Execution safety | Explicit user permission, approved executable/argument handling, working-directory and path boundaries, inherited environment/secrets policy, network policy, numeric runtime/output/resource limits and process-tree cancellation |
| Output policy | Output binding/validation, storage limits, stdout/stderr handling, truncation indication, sensitive data handling and whether failed runs retain diagnostic/partial outputs |
| Persistence/recovery | Durable run intent before side effects, multi-entity write boundary, idempotency, lock ownership, publication order and recovery after each partial failure; never hold a write lock for the entire child process |
| Validation persistence | Opt-in report-artifact path, subject/capture identity, rule/provider identity, report schema, output compatibility, and failure behavior when a report cannot be saved |
| Evidence relationships | Source/target direction, admissible subjects, cycle/self-reference/duplicate rules, whether new types become built-ins, treatment of existing stored custom types and any additional provenance relationship vocabulary |
| Observation | Execution/validation event types and payloads, relation to durable state, event failure warnings, watcher compatibility and no duplicate notifications from multiple projections |
| Format/API evolution | Additive versus versioned model/core, file and CLI schema changes, error/exit mapping, migration needs and Phase 2 transition |

**Status:** decisions recorded in Accepted [ADR-0012](../adr/ADR-0012-execution-provider-and-safety.md) to [ADR-0015](../adr/ADR-0015-validation-report-persistence-and-events.md) and the [execution and evidence contract](../design/execution-and-evidence.md). The provider gate remains closed until P1.3 records the Job Object crate under the dependency policy; P1.1 and P1.2 may proceed.

**Done when:** decisions are approved with concrete limits and negative scenarios, contracts can express the complete sample workflow, and conformance gaps C-0001-4/5, C-0007-3/4/5 and C-0004-2 have an implementation/test obligation. RFC-0007 leaves provider protocols and sandboxing undefined; the ADR must make the chosen local boundary explicit.

**Gate:** no provider process is enabled while permission, limits, captured-input semantics or durable failure handling remain unspecified. Approval of this roadmap alone does not close these decisions.

## P1.1: Model, Core Contracts and Testkit

**Status:** implemented (see the [P1.1 implementation notes](../design/execution-and-evidence.md#implementation-notes-p11)). Evidence for the done-when criteria: contracts compile with public documentation under `cargo clippy -D warnings`; `MemoryEvidence` and `FakeProvider` pass the shared suites and each deliberately broken fake is rejected (`evidence::tests`, `execution::tests` in `functor-testkit`); invalid transitions cannot produce a success record (`model:a_success_must_be_a_real_success`, `model:a_run_completes_exactly_once_and_only_with_its_own_result`, `core:broken_provider_reports_are_rejected`); `script:check-deps.ps1` shows no storage or protocol dependency in model or core.

**Components:** `functor-model`, `functor-core`, `functor-testkit`; [domain model](../design/domain-model.md), [traits](../architecture/traits-and-interfaces.md), [error handling](../architecture/error-handling.md).

**Deliverables:**

- Execution request, run/result, captured-input/output and provenance values following P1.0. Keep artifact identity canonical rather than creating a separate unqueryable result store.
- Dyn-compatible execution/provider and evidence-persistence contracts covering start/run, result retrieval and inspection. Exact signatures follow design review, not illustrative RFC function names.
- Explicit separation of execution outcome, validation outcome and infrastructure/persistence error.
- Testkit fake provider, deterministic clock/ids, cancellation controls, evidence fixtures and a shared execution/evidence conformance suite.
- Additive evidence relationship semantics and parsing rules, including compatibility for stored custom names if promoted to built-ins.

**Tests:** lifecycle transitions/invariants; invalid bindings; evidence relationship direction and validation; deterministic capture/result values; dyn-compatibility; fake provider scenarios and deliberately broken implementations rejected by conformance tests.

**Done when:** contracts compile with public documentation, fake implementations pass shared tests, invalid transitions cannot produce success records, and no storage/protocol dependency enters model/core.

## P1.2: Durable Execution and Validation Evidence

**Status:** implemented for the library path (see the [P1.2 implementation notes](../design/execution-and-evidence.md#implementation-notes-p12) and [ADR-0016](../adr/ADR-0016-evidence-files-and-repair.md), Accepted). Evidence for the done-when criteria: evidence survives reopening and moving a repository (`evidence_survives_a_restart_and_a_moved_repository`); no failed mandatory write reports success, tested at every write boundary (`a_failure_at_every_write_never_reports_a_false_success_and_always_recovers`); existing file formats are unchanged (`format_version` 1, no entity file layout change); reopening exposes interrupted work without erasing or rerunning it (`reopening_exposes_interrupted_work_without_erasing_or_rerunning_it`). Not yet covered: a test that reopens evidence from a separate OS process (a lease and a reopened repository are tested in-process) and the CLI projection (P1.5).

**Components:** persistence through core traits, implementations in `functor-store` and lifecycle/layout helpers in `functor-repository`; evidence schema/codec owned by the representation layer.

**Deliverables:**

- Persist run identity and validated captured-input intent before execution begins. A provider must not start if mandatory pre-run persistence fails.
- Retain workflow/input state, configuration and provider/environment provenance sufficient to inspect and, for the bounded supported fixture, reproduce a run after mutable inputs change.
- Persist terminal evidence and output artifacts with the approved publication/recovery protocol. Represent interrupted or ambiguous completion honestly; do not automatically rerun an operation whose side effects may already have occurred.
- Persist opted-in validation reports as artifacts with outcome, findings, subject capture, rule/provider identity and provenance; preserve the returned report even when explaining a save failure.
- Define consistency when writes fail between run, output, relationship and report updates. Recovery must expose pending/orphaned evidence and repair only what the approved protocol proves safe.
- Keep captured evidence inspectable through normal artifact/query APIs, protected from accidental mutation by the approved evidence policy; do not pretend this implements general RFC-0016 version history.

**Tests:** memory/file conformance; codec round trips; reopen/moved-repository identity; input edits during/after capture; concurrent writers; stale revisions; insufficient permissions; failure injection at every publication boundary; interrupted/unknown recovery; invalid/missing captured evidence and explicit diagnostics.

**Done when:** evidence survives process restart, no failed mandatory write reports a successful publication, existing file-format behavior is preserved or explicitly versioned, and repository reopening exposes interrupted work without silently erasing or rerunning it.

## P1.3: Bounded Local Execution Provider

**Status:** implemented (see the [P1.3 implementation notes](../design/execution-and-evidence.md#implementation-notes-p13) and the `process-wrap` [vetting record](../architecture/dependency-policy.md#vetting-record-process-wrap-adr-0012)). Evidence for the done-when criteria: `functor-runtime/tests/provider.rs` runs the shared provider conformance suite against real child processes, then each safety control at its boundary: stream limits at the limit, one under and one over; item and total output sizes at and one byte over; timeout at its threshold; grant program, environment and limits (allowed and denied); path escape; tampered inputs; cancellation, timeout and a finished run's leftovers each cleaning up a grandchild (its heartbeat stops); pre-cancelled tokens starting nothing; fresh staging per run. Fifteen consecutive full runs passed. Unsupported or unreproducible dependencies are not hidden: network is stated as not enforced.

**Components:** candidate `functor-runtime` service; provider contract from P1.1; testkit and dependency-policy/check updates.

**Deliverables:**

- Execute the approved local workflow shape with captured inputs and explicit permission, without requiring network access or a running service.
- Enforce the approved executable/argument, working-directory/path, environment, timeout, cancellation and output policies. Prefer structured arguments over implicit shell interpretation; any shell support needs an explicit decision and safety tests.
- Capture bounded stdout/stderr and declared outputs; expose truncation and resource-limit outcomes. Do not leak secrets through provenance or logs.
- Classify provider/executable absence, invalid workflow, child exit failure, timeout and cancellation separately from persistence faults.
- Accept injected collaborators rather than constructing a store, validator or publisher. Provider execution alone cannot claim durable publication; P1.4 owns the integrated guarantee.

**Tests:** success; non-zero exit; missing executable/provider; malformed workflow; permission denial; path escape; disallowed environment; large/binary output; timeout; cancellation and child cleanup; missing/malformed outputs; deterministic fixture replay. Run real process tests on Windows; keep other platform results informational under existing policy.

**Done when:** every safety control has a passing allowed/denied or boundary test at the approved numerical threshold, cancellation cleans up spawned work, and unsupported/unreproducible dependencies are explicitly identified rather than hidden.

## P1.4: Integrate the Evidence Graph, Validation and Events

**Status:** implemented (see the [P1.4 implementation notes](../design/execution-and-evidence.md#implementation-notes-p14)). Evidence for the done-when criteria: `functor-runtime/tests/orchestrator.rs` (fast, fake provider and in-memory evidence) covers traceability both ways, every outcome, refusals before any run, duplicate requests, event and storage failures, output validation pass/warning/failed/unknown, no recursive reports, read-only `validate`, `supports`/`contradicts` direction and `V0020` to `V0023`; `tests/end_to_end.rs` runs the real file store, event log and child process: durable run surviving a reopen, replay from captured inputs, failing, timed-out and cancelled children, a refused grant, an unwritable event log, a deleted event log, one child for a repeated request id, and clean validation. `V0021` is structural only (deviation recorded in the notes).

**Components:** runtime orchestration, core repository/validation/event traits, query/validation services and composition wiring.

**Deliverables:**

- Wire capture -> durable intent -> execution -> terminal/output publication -> provenance relationships -> optional output validation. Define reporting for every failure after the provider starts.
- Make evidence navigable in both directions using approved relationships; distinguish derivation, validation and support/contradiction assertions. An execution failure is not automatically evidence that a claim is false.
- Persist validation against the actual captured subject before storing its report; do not repeatedly validate generated reports as new work or let a report artifact retroactively change what its findings describe.
- Define read-only behavior for ordinary validation and explicit persistence for the new path. Query and `watch` never create evidence.
- Publish approved execution/evidence events from one orchestration owner after the relevant durable transition. Preserve best-effort event behavior without masking mandatory storage failures.
- Expose inconsistent/incomplete runs to validation/recovery inspection; add rule codes deliberately without renumbering `V0001` to `V0019`.

**Tests:** complete traceability; supports/contradicts direction; validation pass/fail/warning/unknown; no accidental recursive reports; live query consistency; duplicate request handling; event failure after durable success; storage failure with no misleading success notification; existing custom-type and read-only validation compatibility.

**Done when:** the full workflow works through library APIs alone, query/traversal can identify every input/output/report, and lost/pruned event logs do not prevent evidence inspection.

## P1.5: CLI Projection and User Workflow

**Status:** implemented. `run start/show/list/recover`, `validate --save-report`, exit 8, permission preview/terminal confirmation and cancellation now project the library APIs. [CLI contracts and walkthrough](../cli/cli-spec.md) and [implementation notes](../design/execution-and-evidence.md#implementation-notes-p15) describe the shapes and limitations. Evidence: CLI `tests/run.rs` (real subprocesses and library cross-reading), renderer golden tests for every state, explicit confirmation/cancellation adapter tests, JSON schema acceptance/rejection, and the unchanged existing validation goldens. OS console Ctrl-C delivery remains a manual check; the adapter and runtime cancellation are tested separately.

**Components:** `functor-cli`, [CLI specification](../cli/cli-spec.md), [JSON schemas](../cli/schemas/), examples and walkthrough documentation.

**Deliverables:**

- Document and implement explicit execution, result/evidence inspection and opt-in persisted-validation commands/options over the same core APIs. Command spelling is a P1.0 contract decision.
- Define text/JSON success, failed-run, cancelled/timeout and infrastructure-error output, schema versions and exit mapping. Execution failure must not masquerade as a successful validation or ordinary artifact creation.
- Show run identity and evidence references; distinguish an incomplete publication from a completed child process. Surface permissions/limits and persistence errors with actionable diagnostics.
- Extend `watch` rendering/filter examples for the approved event vocabulary while preserving old events and existing commands.
- Keep domain orchestration out of handlers; the composition root selects concrete providers.

**Tests:** golden/schema tests for each outcome; argument/ref resolution; permission-denied and storage-error paths; cross-process result retrieval; compatibility for existing CLI commands; library-written evidence read by CLI and CLI-created evidence read by libraries; watch observation without mutation.

**Done when:** a documented walkthrough runs from a clean Phase 0-compatible repository, every new output shape has schema/golden coverage, and ordinary `validate` still follows its frozen behavior.

## P1.6: Hardening and Phase Exit

**Status:** implemented and verified on Windows; [actual commands, results and limitations](#phase-exit-evidence-2026-10-14) below.

**Required acceptance workflow:**

1. Create a workflow artifact, input artifacts and a claim to evaluate through the API.
2. Explicitly execute the approved bounded local fixture.
3. Retrieve its durable run, captured inputs and output artifacts after reopening the repository.
4. Persist a validation report against the captured output; demonstrate supporting and contradicting evidence assertions about the claim with their defined semantics.
5. Traverse from a conclusion/report to outputs, run, workflow and original inputs, and in reverse from an input to dependent evidence.
6. Change the live input artifact and demonstrate that the original captured input remains inspectable. Re-execute against that capture for the deterministic fixture and compare declared outputs, not necessarily timestamps/run ids.
7. Repeat through CLI commands and cross-read the results through the library without a CLI-specific model.

**Required negative workflows:** invalid workflow/provider; denied execution; non-zero child exit; missing output; timeout; cancellation; pre-run storage failure (no child starts); post-run publication failure (no false success); process interruption; concurrent input change; failed event append; damaged/missing captured evidence. Persist terminal failure evidence when storage is available; otherwise expose the mandatory write failure and recoverable prior run state.

**Validation evidence to record with implementation:**

- Unit/property and conformance tests for values, contracts, stores, provider and orchestration, with actual test references added to [rfc-conformance.md](rfc-conformance.md).
- Library-only, CLI/schema/golden and real-process acceptance results, including all negative workflows and safe child cleanup.
- Exact commands and results for the relevant targeted tests, then the existing full development checks and informational benchmark:

```powershell
.\scripts\ci.ps1
cargo test --workspace --doc
cargo deny check
$env:RUSTDOCFLAGS = '-D warnings'
cargo doc --workspace --no-deps
.\scripts\bench.ps1
```

The benchmark measures existing repository budgets; add a phase-specific fixture/check for execution overhead, captured-evidence storage and the approved resource limits. Record environment/results and compare Phase 0 budgets; child workload duration is not platform overhead. Preserve the existing informational benchmark policy unless changed explicitly.

**Done when:** every preceding milestone meets its gate; the complete positive/negative workflows pass on Windows; earlier acceptance and frozen contracts remain valid or have an approved migration; documentation and conformance evidence agree; remaining limitations are explicit. This document's proposed tests/commands are obligations, not evidence they have already passed.

### Phase-exit evidence (2026-10-14)

P1.0-P1.5 were completed in preceding milestones; P1.6 adds the final integrated acceptance, owner-death hardening and measurements. The tested platform was Windows x64 (`Microsoft Windows 10.0.26300`), PowerShell 7.6.6 and rustc 1.99.0. The release benchmark records the full environment and sampling policy in [phase-1.json](../quality/benchmarks/phase-1.json).

**Positive workflows (passed):**

- `rt:end_to_end::phase_one_acceptance_preserves_claims_reports_and_replay_after_live_edits`: library-only creation of workflow/input/claim, explicit grant, real execution, reopened durable evidence, exact captured bytes, saved scoped validation report, explicit supporting and contradicting assertions, `V0023` conflict semantics, upstream and downstream provenance, live-input edit, then equal declared output bytes from captured workflow and input.
- `cli:run::phase_one_cli_acceptance_traces_claims_and_replays_captured_inputs`: the corresponding separate-process CLI workflow, version-1 schema checks, then library cross-reading of the same evidence/report/relationships. Ordinary validation and all Phase 0 goldens/reference examples remain unchanged.

**Required negative workflows (passed):**

| Workflow | Evidence |
|----------|----------|
| Invalid workflow/provider/schema | `rt:orchestrator::refusals_happen_before_any_run_exists`, `cli:run::invalid_bindings_provider_schema_and_timeouts_are_refused_before_a_run` |
| Denied execution / explicit terminal permission | `cli:run::no_grant_prints_a_plan_and_writes_no_evidence_even_when_quiet`, `cli:run::terminal_permission_requires_an_explicit_yes_and_always_shows_the_plan`, `rt:provider::the_resolved_program_must_be_the_granted_one` |
| Non-zero child / missing output / start failure | `rt:orchestrator::every_child_outcome_is_recorded_not_raised`, `cli:run::each_child_outcome_has_a_result_and_the_documented_exit_code`, `cli:run::a_missing_program_is_a_start_failed_outcome_not_an_infrastructure_error` |
| Timeout / cancellation / descendants | `rt:provider::a_timeout_kills_the_whole_process_tree`, `rt:provider::cancelling_a_running_child_ends_it_promptly_and_cleans_up_its_children`, `rt:end_to_end::cancelling_a_running_child_records_a_cancelled_run` |
| Pre-run storage failure, no child | `rt:orchestrator::a_failed_intent_write_means_no_child_starts_and_no_started_event`, `cli:run::mandatory_intent_storage_failure_never_reports_success_or_starts_the_child` |
| Post-child publication failure, no false success | `cli:run::failed_result_publication_keeps_the_run_id_and_never_emits_completion`, `store:a_failure_at_every_write_never_reports_a_false_success_and_always_recovers`, `store:a_failure_after_the_commit_point_is_reported_and_repaired_by_recovery` |
| Process interruption and recovery without rerun | `cli:run::killing_the_cli_releases_the_lease_kills_descendants_and_recovers_without_rerunning` (Windows), `store:reopening_exposes_interrupted_work_without_erasing_or_rerunning_it` |
| Concurrent input change / workflow edit after review | `store:an_input_that_keeps_changing_during_capture_is_refused`, `store:one_change_during_capture_is_retried_and_the_stable_state_is_captured`, `rt:orchestrator::a_workflow_changed_after_permission_is_refused_before_recording_a_run` |
| Failed event append | `rt:orchestrator::a_failed_event_append_is_a_warning_and_the_evidence_is_complete`, `cli:run::failed_events_warn_but_do_not_change_the_run_outcome` |
| Damaged/missing captured evidence | `store:damaged_and_missing_evidence_is_reported_with_explicit_diagnostics`, `rt:provider::tampered_inputs_are_refused_before_anything_is_staged` |
| Failed report saving preserves computed report | `cli:run::saved_validation_is_opt_in_and_failure_preserves_the_computed_report`, `rt:orchestrator::a_report_that_cannot_be_saved_is_still_returned_and_the_run_stands` |
| Stream read fault / bounded drain | `rt:supervise::stream_read_errors_are_not_success_shaped_captures`, `rt:supervise::a_stream_past_the_shared_drain_deadline_is_cut_and_marked_truncated` |
| Resource bounds at/below/over | `model:limits_stay_within_the_hard_maximums`, `model:workflow_bounds_hold_exactly_at_the_limit`, `model:workflows_enforce_the_contract_bounds`, `core:input_size_limits_hold_at_the_boundary`, `core:the_total_input_size_holds_at_the_boundary`, `rt:provider::stream_limits_hold_at_the_boundary`, `rt:provider::output_size_limits_hold_at_the_boundary` |

The new interruption test initially failed: the std Job Object wrapper did not set kill-on-close. The correction uses `process-wrap`'s Tokio `JobObject` with `KillOnDrop`; the test now kills a real CLI owner and observes both lease release and stopped grandchild heartbeat. Existing timeout, cancellation and normal-exit tree cleanup tests still pass. No `unsafe` or new workspace dependency was introduced; the vetted dependency's changed feature/transitive footprint is recorded in [dependency-policy.md](../architecture/dependency-policy.md#vetting-record-process-wrap-adr-0012).

**Commands and results:**

| Exact command | Result |
|---------------|--------|
| `cargo test -p functor-runtime --lib --test provider --test end_to_end` | Passed: 16 unit, 21 provider and 11 real end-to-end tests |
| `cargo test -p functor-cli --all-features --test run` | Passed: 18 execution/report integration tests, including complete CLI acceptance and Windows owner-kill recovery |
| `.\scripts\ci.ps1` | Passed: fmt, all-target/all-feature Clippy with warnings denied, dependency/conformance checks, full workspace/all-feature tests, default-feature CLI tests, Markdown links |
| `cargo test --workspace --doc` | Passed (workspace currently has no runnable doctest examples) |
| `cargo deny check` | Passed advisories, bans, licenses and sources; non-fatal existing license/duplicate-version warnings only. The earlier RustSec database-fetch limitation did not recur |
| `$env:RUSTDOCFLAGS = '-D warnings'; cargo doc --workspace --no-deps` | Passed |
| `$env:RUSTDOCFLAGS = '-D warnings'; cargo doc --workspace --all-features --no-deps` | Passed |
| `.\scripts\bench.ps1 -Baseline docs\quality\benchmarks\baseline.json` | Passed; 10,000 artifacts / 19,995 generated DAG relationships; all medians within budgets and below 3x unchanged Phase 0 baseline |
| `.\scripts\bench.ps1 -Artifacts 1000 -Baseline docs\quality\benchmarks\baseline.json` | Passed; 1,000 artifacts / 1,995 generated DAG relationships; all medians within budgets |
| `.\scripts\bench-execution.ps1 -Report docs\quality\benchmarks\phase-1.json` | Passed; release overhead/storage/replay and stream/output/timeout checks; timing remains informational |

Existing repository benchmark medians (ms), five samples after warm-up:

| Scenario | Phase 0 baseline, 1k / 10k | P1.6, 1k / 10k | Budget, 1k / 10k |
|----------|---------------------------|----------------|-----------------|
| artifact show | 92.5 / 562.3 | 64 / 405 | 300 / 750 |
| query dependencies | 92.2 / 590.9 | 63 / 437 | 200 / 2000 |
| validate | 92.8 / 561.8 | 63 / 407 | 3000 / 3000 |
| relation add | 151.8 / 1089.8 | 124 / 810 | 300 / 1500 |

Observed repository-command peak memory was at most 43 MB at 10k, below the 200 MB budget. The execution benchmark's no-op direct-child/end-to-end medians were 29.94/135.53 ms (estimated overhead 105.59 ms); the 1 MiB echo was 30.39/342.60 ms (312.21 ms overhead). Peak sampled CLI working set for no-op/echo was about 15.2 MiB. Repeated 1 MiB inputs shared one 1,048,576-byte payload file; all capture files totaled 1,049,357 bytes, and all evidence content files totaled 3,156,189 bytes (26 files, excluding entity/relationship TOML, event log, tools and retained failed-run staging). Declared replay output remained 1,048,580 bytes and hash-equal after the live file changed. Stream caps, oversized-output refusal and 1-second timeout were checked, not inferred from latency. Measurements are machine-local observations, not new performance guarantees.

**Remaining limitations:** actual Windows console Ctrl-C delivery and privileged symbolic-link creation are manual checks; signal-to-token cancellation and real process-tree cancellation are automated. Non-Windows runs remain informational, and abrupt owner-death cleanup there is not guaranteed. Execution is not a filesystem/network/memory/CPU sandbox. Validation remains structural (`V0021`); evidence readers check digests. Captures provide exact bytes, not immutable source history or a guarantee about external dependencies. Completion is not release authorization.

## Phase 2 Handoff

- Record the capture schema, subject identities, provider versions and limitations needed to bind new runs/reports to immutable versions later.
- Keep original evidence immutable under the chosen policy; do not rewrite past run inputs or trust metadata when versioning arrives.
- Do not invent pre-Phase 2 history from revision counters. Retain captures when a canonical historical version cannot be proven equivalent.
- Hand off interrupted-run recovery semantics, publication boundaries and new event types so history/migration does not turn informational events into the source of truth.
- Update [product-roadmap.md](product-roadmap.md) with actual Phase 1 completion evidence; begin Phase 2 only after its own contracts/ADRs are approved.

Recorded handoff:

| Surface | Phase 1 contract to preserve |
|---------|------------------------------|
| File/envelope versions | Repository `format_version = 1`, event `v = 1`, CLI `schema_version = 1`; no migration was introduced |
| Evidence schemas | `functor.capture/1`, `functor.run/1`, `functor.run_result/1`, `functor.report/1`; content-addressed payloads and immutable evidence artifacts ([ADR-0016](../adr/ADR-0016-evidence-files-and-repair.md)) |
| Source identity | Capture source artifact id, source revision, digest and exact bytes; revision counters do not prove historical version identity |
| Report subject | Repository, or artifact id + evaluated revision + optional content digest; historical source metadata is not a repository snapshot |
| Provider provenance | `local-process`, `functor-runtime 0.1.0`, captured workflow/configuration, resolved program path/digest, platform and environment names; `network = "not_enforced"` |
| Publication/recovery | `run_result` is the commit point; no write lock across execution; per-run lease distinguishes running from abandoned; mandatory-write failures are explicit; recover repairs implied edges and records `interrupted`, never reruns |
| Events | `run.started`, `run.completed`, `validation.recorded` follow durable transitions, remain best-effort and never replace evidence authority |
| Versioning transition | Bind new evidence to approved immutable versions later; retain old captures unless equivalence is proven, never backfill invented history or rewrite prior evidence |
