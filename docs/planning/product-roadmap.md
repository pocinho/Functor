# Product Roadmap

Status: Draft (implementation roadmap; future contracts and decisions remain subject to approval)
Replaces: `implementation-phases.md`
Related docs: [cli-mvp-roadmap.md](cli-mvp-roadmap.md), [mvp-scope.md](mvp-scope.md), [phase-1-execution-provenance-roadmap.md](phase-1-execution-provenance-roadmap.md), [rfc-conformance.md](rfc-conformance.md)

## Purpose and Product Lifecycle

Evolve the proven CLI baseline into a working API-first platform through Phases 1-7. Phase 0 validated the architecture; it was not a product release. The CLI is the first projection, not the product boundary.

Every remaining phase produces a usable prototype increment with explicit acceptance evidence. No phase is a release. The first product release is eligible only after Phases 0-7 are complete and the [product-readiness gate](#product-readiness-gate) passes; completion does not automatically authorize publication.

This roadmap specifies required outcomes and implementation work packages, not dates. Candidate crates follow [workspace-layout.md](../architecture/workspace-layout.md); technologies, exact API signatures, protocols and numerical limits require phase-specific decisions. RFC numbers follow [rfcs/README.md](../../rfcs/README.md).

## Baseline and Phase Map

| Phase | State | Required outcome | Principal RFCs |
|-------|-------|------------------|----------------|
| 0: CLI MVP | Complete, with recorded exit evidence | Local artifact graph, queries, validation and observable changes through core APIs | 0000-0008 subsets, 0011, 0015, 0017 |
| 1: Execution and Provenance | Complete; prototype exit evidence recorded | Executions and persisted validations as traceable evidence | 0001, 0004, 0007, 0017 |
| 2: Versioning and History | Planned | Immutable versions, historical inspection and safe lifecycle/migration | 0005, 0008, 0015, 0016, 0017 |
| 3: Capabilities and Extensions | Planned | Discoverable, replaceable providers and custom schemas/rules | 0010, 0014 |
| 4: Service Layer and Other Projections | Planned | Service API plus desktop/editor, web, notebook and graph explorer | 0002, 0006, 0010, 0011 |
| 5: Interoperability and Agents | Planned | Optional MCP integration and attributable, reviewable agent participation | 0010, 0012 |
| 6: Security, Trust and Governance | Planned | Version-bound trust, access policies and specification stewardship | 0013, 0016, 0018 |
| 7: Federation and Collective Memory | Planned | Autonomous repositories exchanging and querying shared knowledge | 0005, 0009, 0015, 0016 |

Phase 0 evidence is recorded in [cli-mvp-roadmap.md](cli-mvp-roadmap.md#m9-hardening-and-mvp-exit) and [mvp-scope.md](mvp-scope.md#exit-evidence), including the library-only workflow, CLI acceptance, store conformance, format contracts and performance baseline. These are historical results, not tests newly run by this roadmap.

Phase 1 evidence is recorded in the [phase-exit record](phase-1-execution-provenance-roadmap.md#phase-exit-evidence-2026-10-14): library/CLI acceptance, negative workflows, Windows owner-death cleanup, development/dependency checks and informational execution/repository benchmarks. It is not a product release.

Existing constraints matter:

- `revision` is a counter, not immutable history.
- Reports remain returned/printed by default; Phase 1 adds opt-in immutable report artifacts and a `validation.recorded` event ([ADR-0015](../adr/ADR-0015-validation-report-persistence-and-events.md)).
- `supports`, `contradicts` and `validates` are built-in directional types under [ADR-0014](../adr/ADR-0014-evidence-relationships.md), which amends ADR-0007; evidence assertions do not automatically establish truth.
- Event rotation and pruning already exist at 10 MiB / 10 archives ([ADR-0008](../adr/ADR-0008-event-log-rotation.md)); configurable retention and replay do not.
- Event append failure warns; events are not authoritative repository state.
- CLI schemas, file formats, validation codes and exit codes have [frozen contracts](mvp-scope.md#frozen-contracts).

## Sequencing and Entry Gates

```text
Phase 0 -> Phase 1 -> Phase 2 -> Phase 3 -> Phase 4 -> Phase 5 -> Phase 6 -> Phase 7
                                                                                |
                                                                     Product-readiness gate
```

This is delivery order, not permission to defer prerequisites. Work may overlap only when its contract dependencies are satisfied:

| Dependency | Why it is required |
|------------|--------------------|
| Phase 1 input capture -> Phase 2 version binding | Execution evidence must remain inspectable before canonical immutable versions exist |
| Phase 2 immutable versions -> Phase 6 signatures and Phase 7 sync | Trust and synchronization must bind to exact states, not mutable heads |
| Phase 3 capability contracts -> Phases 4-5 discovery | Projections and integrations advertise supported behavior, not implementation internals |
| Phase 4 service/parity contracts -> Phase 5 adapters | External adapters reuse tested semantics and access boundaries |
| Phase 5 attributable proposals -> Phase 6 policy enforcement | Actor and review records must be representable before trust policies evaluate them |
| Phase 6 trust/access policies -> Phase 7 peer exchange | Import and synchronization do not confer trust or bypass local policy |

Before implementation of each phase:

1. Accept the relevant RFCs or needed revisions and record the implementation approach in ADRs. The remaining-phase RFCs are currently Draft; this roadmap does not change their status. Planning/design review may precede acceptance, implementation may not.
2. Close the phase's decision gates and document in/out scope, errors, compatibility, recovery and measurable limits. If an RFC leaves an area undefined, specify it explicitly rather than claiming the RFC already mandates an implementation.
3. Define model values and API contracts before services and projections. Prefer additive changes; justify unavoidable changes to `functor-model` and `functor-core`. Do not introduce protocol-specific model behavior.
4. Update the affected [contracts](../architecture/traits-and-interfaces.md), [crate edges](../architecture/crate-dependencies.md), [workspace layout](../architecture/workspace-layout.md), dependency policy and checks in the implementing change. New crates are not approved solely by being named here.
5. Define test obligations and map affected requirements in [rfc-conformance.md](rfc-conformance.md). Never mark planned behavior Implemented or invent future test evidence.

### Safety Before Exposure

Full trust infrastructure arrives in Phase 6, but safety is required from the first relevant capability:

| Before enabling | Required earlier-phase gate |
|-----------------|-----------------------------|
| Phase 1 process execution | Explicit invocation permission, validated workflow/provider, path and working-directory boundary, environment policy, bounded output/runtime, cancellation and durable failure evidence; no arbitrary auto-execution on repository open |
| Phase 3 extension activation | Approved loading/trust model, compatibility checks, explicit activation and failure isolation; no implicit loading of untrusted repository code |
| Phase 4 network service | Explicit bind/exposure policy, authentication and authorization boundary, request/resource limits, safe content handling and consistent errors; no unauthenticated remote writes |
| Phase 5 agent writes/execution | Attributable actions, least-privilege capabilities and explicit approval/denial rules; untrusted artifact content cannot grant permissions |
| Phase 7 peer exchange | Verified state/integrity, local trust evaluation, scoped transfer permissions and bounded import/sync |

The current no-network dependency guard protects the offline CLI. Phase 4 must deliberately scope network dependencies to approved adapters and update the guard/tests without removing offline guarantees from model/core/local services. Phase 7 extends that decision for synchronization.

## Common Phase Exit

A phase is complete only when its required outcomes and negative scenarios pass through APIs, not just a demonstration UI. Each phase:

- Retains earlier acceptance workflows, or supplies an explicitly approved versioned migration and compatibility evidence for intentional changes.
- Tests success, invalid input, concurrency, interruption and persistence failure where relevant. No success-shaped result for failed mandatory work.
- Updates contracts, examples, schemas, RFC conformance and recorded validation commands/results with the implementation.
- Passes applicable [development checks](../quality/development-workflow.md) and phase-specific performance/resource budgets. Windows is blocking; Linux/macOS remain informational unless a separate decision changes that policy.
- Records remaining limitations and experimental surfaces without treating them as completed requirements.

## Phase 1: Execution and Provenance

**Outcome:** a workflow consumes captured inputs, produces output artifacts, and leaves a durable execution/validation trail that can be queried independently of the CLI.

**Prerequisites and decisions:** Phase 0; acceptance of execution/evidence semantics; ADRs for workflow/provider format, run lifecycle, input capture, persistence/recovery, evidence relationships and execution safety. Detailed milestones: [Phase 1 roadmap](phase-1-execution-provenance-roadmap.md).

**Required work packages:**

1. Add execution/evidence values and dyn-compatible API contracts with testkit doubles and conformance tests.
2. Persist execution records and validation reports as artifacts, retaining exact input/configuration evidence before Phase 2 versioning exists.
3. Implement a bounded local execution provider in candidate service `functor-runtime`, using repository/validation/event collaborators through core traits.
4. Define directional `supports` / `contradicts` semantics, provenance links and validation behavior. Link inputs, workflow, run, outputs and validations without interpreting an assertion as proof of truth.
5. Expose execution/result inspection and opt-in validation persistence through thin CLI commands; publish observable events without making the event log the evidence store.

**Affected surfaces:** model/core/testkit, store and repository persistence contracts, validation/query/events, CLI specification and schemas; a new runtime service.

**Exit evidence:** library and CLI workflows produce the same evidence graph; failed/non-zero runs, timeout, cancellation, interrupted writes and permission denial are explicit; evidence remains inspectable after inputs change and the repository reopens. Existing `validate` behavior remains compatible. Distributed execution, schedulers, automatic agents and general extension packaging are out of scope.

**Completed 2026-10-14:** [P1.6 evidence](phase-1-execution-provenance-roadmap.md#phase-exit-evidence-2026-10-14) records the actual tests, benchmark results and limitations. Phase 2 may begin only after its own RFC/contracts/ADR decision gate; Phase 1 captures must not be rewritten or treated as fabricated immutable version history.

## Phase 2: Versioning and History

**Outcome:** consumers can inspect, compare and reproduce exact historical entity/repository states without changing logical identity.

**Prerequisites and decisions:** Phase 1; accepted RFC-0016 and lifecycle/reference contracts. Decide version identifiers, repository snapshot consistency, concurrent updates, deletion/reference behavior, retention and migration rollback before changing formats.

**Required work packages:**

1. Extend `functor-repository` lifecycle and `functor-store` persistence with immutable artifact and relationship versions, lineage and consistent repository revisions covering metadata/configuration. Retain optimistic concurrency behavior.
2. Add version-specific resolution, history and comparison through core APIs and CLI. Preserve current logical references; document how they resolve to heads.
3. Add artifact deletion, relationship removal/update and tombstones. Keep historical endpoints inspectable; define query/validation treatment of live, deleted and version-pinned references.
4. Add an explicit `format_version` migration command with preflight, backup/recovery and interrupted-migration tests. Do not silently reinterpret a format-1 repository or fabricate history that was never retained.
5. Bind new execution/validation evidence to immutable versions. Keep Phase 1 captured evidence readable; any backfill must demonstrate equality, otherwise retain its capture rather than claim a canonical historical version.
6. Make event rotation/retention configurable, add replay positions/filtering and distinguish replayed events from live signals. Define compaction of derived data/events separately from authoritative immutable history; pruning cannot destroy promised lineage or change repository meaning.

**Affected surfaces:** repository/store/model/core, identity and addressing, query, validation, runtime, events and CLI. Versioning extends the existing repository crate, not a new lifecycle owner.

**Exit evidence:** a changed/deleted artifact and updated/removed relationship remain readable at old versions; comparisons and repository snapshots agree across library/CLI; old evidence still identifies its original inputs. Migration succeeds, refuses incompatible data and recovers from interruption without losing the source. Replayed notifications rebuild a derived view equal to one computed from authoritative state; missing/pruned events are reported rather than used to invent history.

**Out of scope:** distributed merging and synchronization protocols; those belong to Phase 7.

## Phase 3: Capabilities and Extensions

**Outcome:** consumers discover supported behavior and choose compatible providers without changing domain semantics.

**Prerequisites and decisions:** Phase 2; accepted RFC-0010/0014. Decide descriptors and independent capability versions, provider selection/conflicts, compatibility negotiation, lifecycle, loading/trust boundary and failure reporting. A Rust trait is not a promise of a stable dynamic-library ABI.

**Required work packages:**

1. Implement candidate `functor-capabilities`: list/inspect capabilities, requirements and versions, including execution, history and validation already delivered.
2. Implement candidate `functor-extensions`: extension registration, explicit enable/disable, lifecycle metadata, compatibility rejection and provider selection via core contracts.
3. Add artifact-type schemas and custom validation rules through providers; define rule identity, ordering, conflicts, outcomes and extension events without repurposing existing rule codes. Specify composition through explicit relationships/schemas and prove a composite artifact without inventing nested identity semantics.
4. Prove at least two interchangeable providers for one capability and independent behavior of unrelated capabilities when a provider fails.
5. Expose discovery/inspection through library and CLI. Make schemas/rule-provider identity discoverable in persisted evidence.

**Affected surfaces:** capability/extension services, core/model descriptors, validation, runtime, testkit, composition roots and dependency enforcement.

**Exit evidence:** compatible extensions can be discovered, activated, used and disabled; incompatible/untrusted activation fails explicitly; a custom schema/rule finds its fixture violation; a composite artifact resolves its components; replacing a provider preserves the capability contract. Failure of one extension does not silently disable or corrupt unrelated capabilities.

**Out of scope:** an extension marketplace, universal packaging or unrestricted native-code loading; choose the smallest approved loading mechanism that proves replaceability.

## Phase 4: Service Layer and Other Projections

**Outcome:** the same platform works through a service API and every required non-CLI projection: desktop/editor, web, notebook and graph explorer.

**Prerequisites and decisions:** Phase 3; accepted projection/service contracts; ADRs for transport, API/schema versioning, session/auth/access boundary, concurrency, streaming, UI frameworks, deployment/exposure and performance budgets. Complete the network safety gate before opening a listener.

**Required work packages:**

1. Implement candidate `functor-web` service adapter: REST and OpenAPI over capabilities/core APIs, structured errors, version-aware reads/writes and event subscriptions with reconnect/gap behavior.
2. Desktop/editor (`src/app/desktop`, candidate `functor-editor`): artifact authoring, relationship navigation and explicit validation/execution with visible results and stale-write handling.
3. Web projection: browse/author artifacts and relationships, inspect history/evidence and invoke permitted validation/execution through the service.
4. Notebook projection: resolve versioned artifact inputs, explicitly execute cells/workflows through runtime capabilities, persist outputs/validation and preserve input-to-result lineage.
5. Graph explorer: filter/traverse inbound and outbound relationships, distinguish edge semantics, and navigate history, execution and validation evidence back to source artifacts.
6. Define content-search matching and graph/notebook view queries over shared query APIs, with deterministic results and explicit unsupported-format behavior.
7. Add a projection capability/parity matrix. Implement common platform operations where supported and explicitly disclose unsupported interactions; no projection may redefine identity, query, execution or validation semantics. Shared shells/components are allowed, but every named projection needs its own usable interaction and acceptance evidence.

**Affected surfaces:** service/projection adapters and composition roots, discovery, query/content-search contracts, API schemas, dependency/network policy and quality tests. Do not put presentation state or transport DTOs in the model.

**Exit evidence:** independently exercise all four projections and the service, including the notebook and graph-specific workflows. CLI/library/service and each applicable projection agree on identities, version conflicts, content-search/view query results, validation outcomes and persisted execution evidence. Test access denial, malformed requests, unsafe content, disconnected clients and concurrent edits. The offline CLI still works without a running service.

**Out of scope:** production hosting scale and agent/MCP workflows. Completing only a service or one GUI does not complete Phase 4.

## Phase 5: Interoperability and Agents

**Outcome:** agents and external clients participate through discoverable APIs with attributable, reviewable evidence and bounded permissions.

**Prerequisites and decisions:** Phase 4; accepted RFC-0012 mappings and an ADR for MCP transport/version, actor representation, permission scope, proposal/review lifecycle and approval policy. The agent safety gate precedes any write or execution tool.

**Required work packages:**

1. Implement candidate `functor-mcp` as an optional adapter: tools/resources map to discovered capabilities, versioned references, validation/execution and structured errors. No MCP-specific assumptions enter core semantics.
2. Persist actor attribution and agent proposals, input evidence, review decisions and accepted/rejected outcomes as traceable artifacts/relationships.
3. Apply explicit permissions and approval requirements to mutating tools and executions; agents cannot elevate themselves through artifact text or tool arguments.
4. Provide review/inspection through APIs and existing human projections. Define retry/idempotency behavior so repeated integration calls do not duplicate accepted contributions.
5. Add adapter conformance covering library/service/MCP agreement and unsupported-capability discovery.

**Affected surfaces:** MCP adapter, capability discovery, actor/review contracts, evidence persistence, human projections and testkit. This is participation infrastructure, not a mandatory model vendor dependency.

**Exit evidence:** an agent discovers capabilities, proposes an evidence-backed change, a human inspects and accepts/rejects it, and the resulting state/history records both actors. Denied writes/executions change nothing; retries follow the documented policy; disabling MCP leaves all non-MCP workflows intact. Agent output is not automatically trusted.

**Out of scope:** unrestricted autonomous execution, a general agent orchestration framework and vendor-specific reasoning internals.

## Phase 6: Security, Trust and Governance

**Outcome:** users can verify the integrity/origin of exact versions, apply local access/trust policy and govern platform evolution.

**Prerequisites and decisions:** Phase 5, Phase 2 immutable states and earlier safety gates. Accept RFC-0013/0018 and additional contracts for authorization, key lifecycle and application approval policies where needed. RFC-0013 does not define authentication protocols; RFC-0018 governs specification evolution, not a ready-made artifact approval engine.

**Required work packages:**

1. Implement candidate `functor-security`: canonical version-bound integrity/signature/attestation records and verification, including actor/provider provenance.
2. Define key provisioning, rotation/revocation, unavailable-key behavior and verification outcomes. Bind approval/trust to specific versions; never inherit trust automatically after updates.
3. Consolidate and enforce least-privilege access and local trust policies consistently across CLI, service, extensions and agent adapters. Preserve attribution and inspectable reasons for decisions.
4. Exercise application review/approval policy with persisted evidence, separately from specification governance.
5. Document and exercise RFC proposal/review/acceptance, stewardship, compatibility classification, experimental features, deprecation and migration guidance. Reconcile RFC lifecycle terminology through approved governance decisions, not silent edits to statuses.

**Affected surfaces:** security service, version/evidence formats, access enforcement hooks, adapters, extension activation, ADR/RFC process and contributor documentation.

**Exit evidence:** tampering, wrong signer, revoked/unknown keys and stale-version approvals yield explicit outcomes; unauthorized operations are denied across adapters; verification never depends solely on event logs. A sample specification change follows the documented governance process with compatibility and migration review. A signed artifact may still be untrusted under local policy.

**Out of scope:** a central trust authority, mandatory cryptographic vendor or an assumption that synchronization/signing establishes truth.

## Phase 7: Federation and Collective Memory

**Outcome:** autonomous repositories share selected knowledge, preserve provenance and support organizational retrieval while remaining useful offline.

**Prerequisites and decisions:** Phase 6; immutable versions, trust/access evaluation and accepted RFC-0009/addressing contracts. Decide qualified reference syntax, peer discovery/transport, selection scopes, conflict handling, quotas, external-reference resolution and workspace membership/visibility.

**Required work packages:**

1. Implement candidate `functor-sync`: snapshot import/export and selective push/pull/bidirectional synchronization of immutable entities, lineage and evidence. Preserve identity; never rewrite history to force convergence.
2. Extend canonical addressing with repository-qualified and version-specific references; retain existing local references. Preserve unavailable external endpoints with explicit unresolved state and later resolution.
3. Detect missing versions, divergence, duplicate transfers and interrupted sessions. Expose conflicts for explicit resolution; do not choose an undocumented last writer or silently discard evidence.
4. Preserve origin/signature/trust metadata across transfer and evaluate local policy on receipt. Define handling for slug collisions without reassigning stable entity identities.
5. Implement shared-workspace membership/access and cross-repository discovery/query over artifacts, relationships, executions and validations. Search indexes remain derived and reveal neither inaccessible content nor misleading completeness when peers are offline.
6. Document transfer format/protocol compatibility, recovery, offline limitations and resource budgets.

**Affected surfaces:** synchronization service, repository lifecycle, addressing/query, versioned storage, security policies, discovery, service/CLI and shared-workspace projections.

**Exit evidence:** two repositories diverge offline, exchange a selected graph, report conflicts, resolve them and converge on the agreed immutable versions without losing lineage. Repeating or resuming a transfer is safe. Unavailable references resolve when a peer returns. Unauthorized/untrusted imports are handled according to explicit policy. A user can discover a shared conclusion and trace it across repositories to original inputs, execution and review; local work still functions with all peers disconnected.

**Out of scope:** global total ordering, mandatory centralized infrastructure, automatic resolution of every semantic disagreement or replication of every repository.

## Coverage of Remaining MVP Gaps

This table assigns delivery responsibility without changing the current conformance matrix. During each phase, expand requirement-level rows and add real implementation/test references; do not treat a phase title as proof of conformance.

| Current gap or limitation | Delivery / disposition |
|---------------------------|------------------------|
| C-0001-4/5, C-0007-3/4/5: report artifacts, execution and traceability | Phase 1; Phase 2 strengthens exact historical references |
| C-0004-2: remaining built-in evidence types | Phase 1 semantics for `supports` / `contradicts`; retain explicitly custom types unless separately specified |
| C-0003-1, C-0008-4, RFC-0016: real versions/history | Phase 2 |
| C-0003-6, C-0004-6: delete/remove; immutable MVP relationships | Phase 2 tombstones and relationship history/update |
| C-0017-3/6/8: event categories, bounded audit, replay/subscriptions/extensions | Phases 1-3 add categories/replay/provider events; Phase 4 service subscriptions; authoritative history is Phase 2, trust evidence Phase 6 |
| RFC-0010/0014: discovery, schemas and extensions | Phase 3 |
| C-0003-7: composition | Phase 3 decides an explicit relationship/schema contract and proves it with a composite artifact; no implicit nested-identity semantics |
| C-0003-8: execution by artifact type | Phase 1 explicit provider eligibility; Phase 3 discoverable provider/type mapping |
| C-0006-2/6: content search and projection queries | Phase 4 defines content matching and graph/notebook view contracts over shared query APIs |
| C-0006-7: query language, ranking, federation | Federated query in Phase 7; a new query language and relevance ranking are not release requirements unless separately approved |
| C-0011-3: other projections | All named Phase 4 projections are required |
| C-0002-4, RFC-0012: MCP and interoperability | Phase 5 |
| RFC-0013/0018: trust and governance | Phase 6, with earlier safety prerequisites |
| C-0005-7, C-0015-4, RFC-0009: external identities, exchange and sync | Phase 7 |
| C-0005-4: probabilistically unique ULIDs | Retain generation; Phase 7 adds collision/conflict tests and explicit rejection, not an impossible mathematical uniqueness guarantee |
| Slug extension of RFC-0005 (ADR-0001) | Retain compatible extension; Phase 7 defines collision behavior |
| Interactive prompts, completion, general CLI config, additional storage engines | Not required merely because the MVP excluded them; add only through separately approved scope. Phase-specific permission/configuration controls are still mandatory |

## Product-Readiness Gate

Completing all seven phases is necessary, not sufficient, for the first release. Before authorizing it:

1. Record acceptance evidence for every required phase outcome, including all Phase 4 projections and the complete cross-repository evidence/review workflow.
2. Review every conformance Subset/Deferred/Deviation against the accepted product scope. Resolve required gaps; explicitly document optional/out-of-scope behavior without claiming blanket implementation of all RFC examples.
3. Finalize supported APIs, capability/protocol versions, file/JSON/event schemas, compatibility/deprecation rules and migration/recovery guidance. Preserve access to Phase 0 repositories through the supported migration path.
4. Meet approved safety, integrity, authorization, privacy, resource/performance and supported-platform criteria. Record exact check commands/results and unresolved risks; do not silently waive a required gate.
5. Provide installation, backup/restore, operations and user/contributor documentation for the complete platform, with examples exercised by acceptance tests.
6. Obtain explicit release/publication decisions and any required ADRs, then define packaging, versioning, changelog and release automation. Crates remain `publish = false` until separately authorized.

The platform succeeds when artifacts remain the primary knowledge unit, APIs the primary interface, projections share one semantic core, humans and agents collaborate through evidence, and execution, validation, history and federation remain traceable.
