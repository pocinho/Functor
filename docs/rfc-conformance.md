# RFC Conformance Matrix

Status: Draft (to be kept current by every pull request)
Related docs: [mvp-scope.md](mvp-scope.md), [testing-strategy.md](testing-strategy.md), [adr/README.md](adr/README.md)

## Purpose

`docs/README.md` says that when implementation guidance conflicts with an RFC, the RFC wins. This matrix makes that rule operational: it lists the normative requirements the MVP touches, states for each whether it is implemented, implemented as a subset, deferred, or deliberately deviated from, and names where it lives and the test that proves it.

## Status Values

| Status | Meaning |
|--------|---------|
| Implemented | Satisfied fully by the MVP |
| Subset | Partially satisfied; the gap is stated |
| Deferred | Intentionally outside the MVP |
| Deviation | The MVP does something different from the RFC; an ADR or note explains why |

## Rules for Maintaining This Table

- Every pull request that changes behavior updates the affected rows.
- "Implemented" and "Subset" rows must name a test; the test name is added when the test is written (the column below gives the planned suite and scenario, which is the contract for what the test must prove).
- A "Deviation" requires an ADR. RFCs are never edited to match the implementation; a needed RFC change is proposed as a new RFC revision.
- Keep requirement text short and cite the RFC section.

## Matrix

Test column notation: `model:` unit and property tests in `functor-model`; `conf:` store conformance suite; `query:`, `val:`, `events:` crate tests; `cli:` end-to-end tests; `acc:` acceptance test.

### RFC-0001 Core Semantic Model

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0001-1 | Every artifact has identity, metadata, content and relationships | Implemented | [domain-model.md](domain-model.md) | `model:artifact_has_required_parts` |
| C-0001-2 | Identity is immutable | Implemented | `ArtifactPatch` has no id field; update never changes `id` | `conf:update_preserves_id` |
| C-0001-3 | Relationships are first-class entities | Implemented | Own type, own id, own file | `conf:relationship_has_identity` |
| C-0001-4 | Validation results are artifacts themselves | Deviation | MVP reports are printed and summarized in an event, not stored as artifacts. Deferred to the execution and provenance phase ([product-roadmap.md](product-roadmap.md)) | `val:report_not_persisted` |
| C-0001-5 | Execution is a reproducible operation producing artifacts | Deferred | | |
| C-0001-6 | Projections do not own the model | Implemented | CLI is a thin projection ([cli-spec.md](cli-spec.md)) | `acc:library_only_workflow` |

### RFC-0002 API-First Architecture

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0002-1 | Every UI capability is expressible through the core API | Implemented | `functor-core` traits; CLI calls only traits | `acc:library_only_workflow`, dependency-edge check |
| C-0002-2 | All projections operate on the same semantic model | Implemented | Single model crate | `cli:same_repo_library_and_cli` |
| C-0002-3 | Projections are replaceable without core changes | Implemented | Layering in [crate-dependencies.md](crate-dependencies.md) | dependency-edge check |
| C-0002-4 | MCP is an integration layer, not core | Deferred | | |

### RFC-0003 Artifact Model

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0003-1 | Artifact provides Identity, Type, Metadata, Version, Content | Subset | Version is the integer `revision`; no history. [artifact-format.md](artifact-format.md) | `conf:revision_increments` |
| C-0003-2 | Identity is stable and independent of storage location, filename, UI and protocol | Implemented | Opaque id inside the file; filename is derived from id, never the reverse | `conf:slug_rename_keeps_identity`, `store:filename_matches_id` |
| C-0003-3 | Types are extensible | Implemented | Open `ArtifactType` set | `model:unknown_type_accepted` |
| C-0003-4 | Metadata is extensible | Implemented | `[metadata]` table | `store:metadata_roundtrip` |
| C-0003-5 | Content format is not mandated | Implemented | `content.format` is free text | `store:content_roundtrip` |
| C-0003-6 | Artifact-centric API: create, get, update, delete, relate, validate, query | Subset | All except delete ([traits-and-interfaces.md](traits-and-interfaces.md)) | `conf:crud` |
| C-0003-7 | Composition (artifacts containing artifacts) | Deferred | Expressible with relationships only | |
| C-0003-8 | Execution capability by artifact type | Deferred | | |

### RFC-0004 Relationship Model

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0004-1 | Relationships are directional | Implemented | source and target | `model:relationship_direction` |
| C-0004-2 | Core types: references, depends_on, derived_from, extends, supports, contradicts | Subset | First four built-in; others as custom types ([ADR-0007](adr/ADR-0007-relationship-types.md)) | `model:builtin_vs_custom` |
| C-0004-3 | Relationships should have stable identifiers | Implemented | `rel_<ULID>` | `conf:relationship_has_identity` |
| C-0004-4 | Metadata must not alter semantic meaning | Implemented | Opaque metadata map | `store:relationship_metadata_roundtrip` |
| C-0004-5 | Validate existence of referenced artifacts and detect cycles | Implemented | `V0006`, `V0007`, `V0009`, `V0011` ([validation-rules.md](validation-rules.md)) | `val:V0006`, `val:V0007`, `val:V0009`, `val:V0011` |
| C-0004-6 | API: create, remove, list, find_related, traverse | Subset | No remove | `conf:relationships`, `query:traverse` |
| C-0004-7 | Reverse relationships via indexing or query | Implemented | Inbound traversal ([query-model-mvp.md](query-model-mvp.md)) | `query:inbound` |

### RFC-0005 Identity and Addressing

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0005-1 | Identity immutable through content, metadata and relationship changes | Implemented | | `conf:update_preserves_id` |
| C-0005-2 | Identifiers do not encode paths, keys or URLs | Implemented | `art_<ULID>` | `model:id_grammar` |
| C-0005-3 | Identifiers are opaque to consumers | Implemented | No timestamp accessor; slug is a separate alias ([ADR-0001](adr/ADR-0001-two-part-identity.md)) | `model:id_has_no_accessors`, review rule |
| C-0005-4 | Globally unique identifiers | Subset | ULID: probabilistically unique; uniqueness within a repository is additionally checked (`V0003`) | `val:V0003` |
| C-0005-5 | Canonical reference `functor:<entity-type>:<id>` (SHOULD) | Implemented | Accepted on input, emitted as `ref` in JSON ([identity-and-addressing.md](identity-and-addressing.md)) | `cli:accepts_canonical_ref` |
| C-0005-6 | Retrieval by identifier: get, resolve, exists | Implemented | `get`, `resolve`, `lookup` | `conf:resolve` |
| C-0005-7 | Cross-repository and federated identifiers | Deferred | | |

### RFC-0006 Query Model

See the coverage table in [query-model-mvp.md](query-model-mvp.md).

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0006-1 | Lookup by identity | Implemented | `lookup` | `query:lookup` |
| C-0006-2 | Search by metadata and content | Subset | Metadata, type, tag, text on slug, title and description; no content search | `query:search` |
| C-0006-3 | Traverse relationships, inbound, outbound, bidirectional, depth limit | Implemented | `traverse` | `query:traverse_table` |
| C-0006-4 | Storage-independent semantics | Implemented | Queries run over snapshots | `query:same_on_both_stores` |
| C-0006-5 | Results immutable during execution | Implemented | Single snapshot per query | `query:snapshot_consistency` |
| C-0006-6 | Filter and project | Subset | Filter fields in `SearchQuery`; projection is a rendering concern of the CLI (fixed columns, full JSON) | `query:search`, `cli:json_schema` |
| C-0006-7 | Query language, ranking, federation | Deferred | | |

### RFC-0007 Validation and Execution

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0007-1 | Validation outcomes explicit: Passed, Failed, Warning, Unknown | Implemented | `Outcome` | `val:outcome_table` |
| C-0007-2 | Validation of artifacts and relationships | Implemented | Rule catalog | `val:V0001` to `val:V0018` |
| C-0007-3 | Validation outputs representable as artifacts | Deviation | See C-0001-4 | |
| C-0007-4 | Execution as evidence; reproducible executions | Deferred | | |
| C-0007-5 | Traceability between inputs, executions, outputs, validations | Deferred | `validation.completed` event is the only record | |

### RFC-0008 Storage and Persistence

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0008-1 | Semantic model independent of any storage technology | Implemented | Model and core have no storage knowledge; two stores | `conf:*` on both stores |
| C-0008-2 | Durable identity across storage engines | Implemented | Id is in the entity, not the location | `store:move_repository_dir` |
| C-0008-3 | Local-first, offline | Implemented | Filesystem only | `cli:no_network` (review rule) |
| C-0008-4 | Preserve historical states when practical | Deferred | `revision` only | |
| C-0008-5 | Indexes must not affect semantics | Implemented | In-memory indexes rebuilt per snapshot | `query:same_on_both_stores` |
| C-0008-6 | Portability, no proprietary representation | Implemented | TOML and JSON Lines | `store:cross_platform_checkout` |

### RFC-0011 Projection Model

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0011-1 | CLI is a projection over the common API | Implemented | [cli-spec.md](cli-spec.md) | `acc:library_only_workflow` |
| C-0011-2 | Artifact identity unchanged across projections | Implemented | | `cli:same_repo_library_and_cli` |
| C-0011-3 | Other projections | Deferred | | |

### RFC-0015 Repository Model

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0015-1 | Repository MUST have a stable identifier | Implemented | `repo_<ULID>` in `repository.toml` | `store:init_creates_id`, `store:id_stable_across_open` |
| C-0015-2 | Repository metadata is informational and never alters semantics | Implemented | | review rule |
| C-0015-3 | Create, open, query, validate repository operations | Implemented | `init`, discovery, `query`, `validate` | `cli:init`, `cli:discovery` |
| C-0015-4 | Synchronize, export, import | Deferred | | |
| C-0015-5 | Repository not tied to a storage technology | Implemented | `RepositoryManager` trait | `conf:*` |

### RFC-0017 Event Model

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0017-1 | Every event contains identifier, type, timestamp, origin, subject, metadata | Implemented | Envelope in [event-model-mvp.md](event-model-mvp.md) | `events:envelope_fields` |
| C-0017-2 | Hierarchical event type names | Implemented | `artifact.created` and so on | `events:type_names` |
| C-0017-3 | Event types: artifact, relationship, validation (and others) | Subset | Four types only | `cli:each_mutation_emits_one_event` |
| C-0017-4 | Events are informational, not authoritative | Implemented | State never depends on the log | `events:append_failure_is_warning` |
| C-0017-5 | Do not mandate transport, serialization or synchronous processing | Implemented | Local JSON Lines is an implementation choice | |
| C-0017-6 | Auditability | Subset | Append-only log; no rotation, no signatures | `events:seq_gap_free` |
| C-0017-7 | Distributed ordering is not assumed | Implemented | `seq` is per repository only | |
| C-0017-8 | Subscriptions, replay, extension events | Deferred | `watch` is a simple tail | |

### Deferred RFCs

| RFC | Title | Status | Note |
|-----|-------|--------|------|
| 0009 | Federation and Synchronization | Deferred | Identity design leaves room for repository-qualified references |
| 0010 | Capability Model | Deferred | No registry; traits only |
| 0012 | Interoperability and Integration | Deferred | No MCP |
| 0013 | Security and Trust Model | Deferred | Local safety baseline in [nonfunctional-requirements.md](nonfunctional-requirements.md) |
| 0014 | Extension Model | Deferred | `metadata` and custom relationship and artifact types are the only extension points |
| 0016 | Versioning Model | Deferred | `revision` counter; versions published are immutable (RFC-0016 MUST) is **not** claimed |
| 0018 | Governance Model | Deferred | |

## Known Deviations (summary)

1. Validation reports are not artifacts (C-0001-4, C-0007-3).
2. Artifact identity is two-part (id plus slug), a compatible extension of RFC-0005 ([ADR-0001](adr/ADR-0001-two-part-identity.md)).
3. No deletion of artifacts or relationships (C-0003-6, C-0004-6).
4. Event log has no rotation (C-0017-6).
