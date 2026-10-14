# Phase 1: Execution and Provenance Roadmap

Status: Draft (planned working-prototype increment; not implemented and not a release)
Related RFCs: [RFC-0007](../../rfcs/RFC-0007-Validation-and-Execution.md), [RFC-0001](../../rfcs/RFC-0001-Core-Semantic-Model.md), [RFC-0004](../../rfcs/RFC-0004-Relationship-Model.md), [RFC-0017](../../rfcs/RFC-0017-Event-Model.md)
Related docs: [product-roadmap.md](product-roadmap.md), [cli-mvp-roadmap.md](cli-mvp-roadmap.md), [mvp-scope.md](mvp-scope.md), [rfc-conformance.md](rfc-conformance.md)

## Purpose

Turn execution and validation into durable, queryable evidence over the proven Phase 0 API-first baseline. A user must be able to explain which workflow and inputs produced an output, what happened during execution, and which validation supports or challenges a conclusion.

This is an implementation roadmap, not an accepted execution contract. P1.0 closes the decisions before code. Phase 1 completion is a prototype milestone; the first product release remains subject to all phases and the [product-readiness gate](product-roadmap.md#product-readiness-gate).

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
- `supports` / `contradicts` currently use custom-type behavior under [ADR-0007](../adr/ADR-0007-relationship-types.md), including warning `V0012`.
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

## Phase 2 Handoff

- Record the capture schema, subject identities, provider versions and limitations needed to bind new runs/reports to immutable versions later.
- Keep original evidence immutable under the chosen policy; do not rewrite past run inputs or trust metadata when versioning arrives.
- Do not invent pre-Phase 2 history from revision counters. Retain captures when a canonical historical version cannot be proven equivalent.
- Hand off interrupted-run recovery semantics, publication boundaries and new event types so history/migration does not turn informational events into the source of truth.
- Update [product-roadmap.md](product-roadmap.md) with actual Phase 1 completion evidence; begin Phase 2 only after its own contracts/ADRs are approved.
