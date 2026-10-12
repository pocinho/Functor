# RFC-0017: Event Model

## Status

Draft

## Abstract

This RFC defines the Functor Event Model.

Events represent observable occurrences within a Functor system. They provide a mechanism through which repositories, projections, extensions, runtimes, and integrations may react to changes without requiring direct coupling.

The Event Model enables automation, synchronization, notifications, orchestration, auditing, and reactive workflows while preserving the API-first architecture defined by previous RFCs.

This RFC defines event semantics and event contracts but does not define transport protocols or message formats.

## Motivation

Functor artifacts, relationships, repositories, validations, and executions change over time.

Consumers frequently need to respond to these changes.

Examples include:

- Updating a projection when an artifact changes
- Triggering validation after modification
- Updating search indexes
- Synchronizing repositories
- Recording audit trails
- Initiating automated workflows

Without a common event model, components become tightly coupled and difficult to evolve independently.

Functor therefore defines events as first-class architectural primitives.

## Goals

The Event Model SHALL:

- Enable observation of platform activity
- Decouple event producers and consumers
- Support local and distributed implementations
- Support extension participation
- Enable automation and orchestration
- Preserve auditability

The Event Model SHALL NOT:

- Define transport protocols
- Define message serialization
- Require specific messaging technologies
- Mandate synchronous processing

## Event Definition

An event represents the occurrence of a meaningful platform activity.

Examples:

```text
Artifact Created
Artifact Updated
Relationship Added
Validation Completed
Execution Failed
Repository Synchronized
```

Events describe what happened.

Events do not prescribe what actions must occur.

## Event Structure

Every event SHALL contain:

```text
Event Identifier
Event Type
Timestamp
Origin
Subject
Metadata
```

Example:

```text
event_id: evt:12345
event_type: artifact.created
subject: artifact:rfc-0017
origin: repository:core
timestamp: 2026-10-12T19:00:00Z
```

Implementations MAY attach additional metadata.

## Event Types

Implementations SHOULD use a hierarchical naming convention.

Examples:

```text
artifact.created
artifact.updated
artifact.deleted

relationship.created
relationship.updated
relationship.deleted

validation.started
validation.completed
validation.failed

execution.started
execution.completed
execution.failed

repository.synchronized
repository.updated
```

Additional event categories MAY be defined by extensions.

## Event Producers

Any component MAY emit events.

Examples:

```text
Repository
Runtime
Validation Engine
Extension
Projection
Integration Adapter
```

## Event Consumers

Any component MAY subscribe to or process events.

Examples:

```text
CLI
Editor
Web UI
Search Indexer
Synchronization Service
Validation Engine
Agent Integration
```

Consumers SHOULD remain independent from the producer implementation.

## Event Ordering

Implementations SHOULD preserve ordering where practical.

However, event ordering is not globally guaranteed.

Consumers MUST NOT assume total ordering across distributed systems.

Where ordering is required, implementations MAY provide repository-local guarantees.

## Event Persistence

Implementations MAY persist events.

Persisted events MAY be used for:

- Auditing
- Diagnostics
- Synchronization
- Historical Analysis
- Replay

Persistence strategy is implementation-defined.

## Event Replay

Implementations MAY support replaying previously persisted events.

Replay enables:

- Recovery
- Rebuilding projections
- Reconstructing indexes
- System migration

Consumers SHOULD be capable of distinguishing replayed events from newly generated events.

## Event Filtering

Consumers SHOULD be able to filter events based on:

```text
Event Type
Repository
Artifact
Origin
Timestamp
Metadata
```

Filtering behavior is implementation-defined.

## Extension Participation

Extensions MAY:

```text
Publish Events
Subscribe to Events
Transform Events
Observe Events
```

Extensions MUST NOT require proprietary event behavior that violates the semantics defined by this RFC.

## Federation Considerations

Events MAY be shared between repositories.

However:

> Events are not authoritative.
>
> Artifacts and versions remain authoritative.

Events communicate change.

Repositories communicate state.

This distinction is important for synchronization and trust.

## Security Considerations

Events SHOULD carry sufficient provenance information to establish origin.

Consumers MUST treat events as informational signals rather than authoritative truth.

Trust decisions MUST rely on repository state, artifact versions, validations, and signatures rather than events alone.

## Relationship to Previous RFCs

This RFC builds upon:

- RFC-0002 API-First Architecture
- RFC-0007 Validation and Execution
- RFC-0008 Storage and Persistence
- RFC-0009 Federation and Synchronization
- RFC-0015 Repository Model
- RFC-0016 Versioning Model

## Future Work

Future RFCs may define:

- Event transports
- Event subscriptions
- Reactive automation
- Workflow orchestration
- Event streaming APIs
- Distributed event propagation
