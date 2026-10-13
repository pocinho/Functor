# Product Roadmap

Status: Draft (long-term direction; not a commitment)
Replaces: `implementation-phases.md`
Related docs: [cli-mvp-roadmap.md](cli-mvp-roadmap.md) (the concrete MVP plan), [mvp-scope.md](mvp-scope.md)

## Purpose

Describes the long-term evolution of Functor from the CLI MVP to a collaborative, API-first platform. The MVP itself is planned in [cli-mvp-roadmap.md](cli-mvp-roadmap.md). This document is deliberately coarse: phases are ordered by dependency, not by schedule, and each is gated by RFC acceptance and an ADR.

The earlier version of this document (`implementation-phases.md`) used incorrect RFC numbers (for example "RFC-0001 API-First Architecture") and crate names that do not exist (`functor-artifact`, `functor-graph`, `functor-api`). The RFC numbers below follow [rfcs/README.md](../../rfcs/README.md); crate names follow [workspace-layout.md](../architecture/workspace-layout.md).

## Phase 0: CLI MVP

See [cli-mvp-roadmap.md](cli-mvp-roadmap.md).

RFCs: 0000 to 0008 (subsets), 0011 (CLI projection), 0015 (subset), 0017 (subset).
Outcome: a local-first repository of artifacts and relationships that can be queried, validated and observed, entirely through a core API.

## Phase 1: Execution and Provenance

Goals:

- Execution as evidence (RFC-0007 execution half): run a workflow artifact and record the result as an artifact.
- Traceability between inputs, executions, outputs and validations.

Candidate deliverables: `functor-runtime`, execution records, `supports` and `contradicts` relationships with defined semantics, persisted validation reports.

## Phase 2: Versioning and History

Goals:

- Real artifact versions and history (RFC-0016) replacing the `revision` counter.
- Deletion with tombstones, event replay, configurable rotation and compaction.

Candidate deliverables: versioning support in `functor-repository` (which already owns the repository lifecycle, [ADR-0009](../adr/ADR-0009-functor-repository-crate.md)), a migration command for `format_version`.

## Phase 3: Capabilities and Extensions

Goals:

- Capability discovery (RFC-0010) and extension model (RFC-0014).
- Custom validation rules and artifact-type schemas.

Candidate deliverables: `functor-capabilities`, `functor-extensions`, a stable extension trait set.

## Phase 4: Service Layer and Other Projections

Goals:

- Expose the same API to other projections (RFC-0011): desktop (`src/app/desktop`), web, notebook, graph explorer.
- Dynamic dispatch is already allowed for the core traits ([ADR-0010](../adr/ADR-0010-async-trait-and-dyn.md)), so a second projection or a plugin host can hold trait objects.

Candidate deliverables: `functor-web`, `functor-editor`, REST and OpenAPI over the core traits.

## Phase 5: Interoperability and Agents

Goals:

- MCP and agent interoperability (RFC-0012), agent participation, evidence and review workflows (vision: [Human-And-Intelligence-Collaboration](../../vision/Human-And-Intelligence-Collaboration.md)).

Candidate deliverables: `functor-mcp`, agent contribution model.

## Phase 6: Security, Trust and Governance

Goals:

- Signing, provenance verification and access control (RFC-0013), governance workflows (RFC-0018).

Candidate deliverables: `functor-security`.

## Phase 7: Federation and Collective Memory

Goals:

- Federation, synchronization and cross-repository references (RFC-0009), shared workspaces and organizational memory.

Candidate deliverables: `functor-sync`, federated identifiers extending the canonical reference syntax from [identity-and-addressing.md](../design/identity-and-addressing.md).

## Success Criteria for the Platform

1. Artifacts are the primary unit of knowledge.
2. APIs are the primary interface.
3. Applications are projections of the same semantic core.
4. Humans and agents collaborate through shared artifacts.
5. Execution, validation and reasoning remain traceable.

## Sequencing Principles

- A phase starts only after its RFC is accepted and an ADR records the implementation approach.
- The semantic model and core API stay stable; later phases add crates rather than reshaping `functor-model` and `functor-core`.
- Every phase keeps the previous phases' acceptance tests green.
