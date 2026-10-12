# Functor Workspace Layout

## Purpose

This document proposes an initial Rust workspace structure for the Functor CLI MVP and the broader Functor ecosystem. The layout maps RFC concepts to concrete crates and establishes clear architectural boundaries.

## Design Principles

- API-first architecture
- Semantic model at the center
- Frontends as projections
- Extension-friendly composition
- Minimal coupling between crates
- Progressive evolution from MVP to ecosystem

## Workspace Structure

```text
functor/
├── Cargo.toml
├── crates/
│   ├── functor-model/
│   ├── functor-core/
│   ├── functor-store/
│   ├── functor-query/
│   ├── functor-validation/
│   ├── functor-runtime/
│   ├── functor-events/
│   ├── functor-repository/
│   ├── functor-capabilities/
│   ├── functor-cli/
│   └── functor-mcp/
├── rfcs/
├── examples/
├── roadmaps/
└── docs/
```

## Crate Responsibilities

### functor-model

Owns the core semantic types.

Relevant RFCs:

- RFC-0001 Core Semantic Model
- RFC-0003 Artifact Model
- RFC-0004 Relationship Model
- RFC-0005 Identity and Addressing

Example contents:

```rust
Artifact
ArtifactId
Relationship
RelationshipId
RepositoryId
VersionId
```

### functor-core

Defines the primary platform interfaces.

Relevant RFCs:

- RFC-0002 API-First Architecture

Example contents:

```rust
ArtifactStore
QueryEngine
ValidationEngine
Runtime
Repository
```

### functor-store

Persistence implementations.

Relevant RFCs:

- RFC-0008 Storage and Persistence

Initial MVP:

- File-based storage
- TOML serialization
- Local repositories

### functor-query

Query execution and graph traversal.

Relevant RFCs:

- RFC-0006 Query Model

Responsibilities:

- Filtering
- Traversal
- Selection
- Projection support

### functor-validation

Validation framework.

Relevant RFCs:

- RFC-0007 Validation and Execution

Examples:

- Missing references
- Invalid identities
- Repository consistency

### functor-runtime

Execution infrastructure.

Relevant RFCs:

- RFC-0007 Validation and Execution

MVP scope:

- Local execution hooks
- Future execution providers

### functor-events

Event publication and consumption.

Relevant RFCs:

- RFC-0017 Event Model

Examples:

```text
artifact.created
artifact.updated
repository.changed
```

### functor-repository

Repository lifecycle services.

Relevant RFCs:

- RFC-0009 Federation and Synchronization
- RFC-0015 Repository Model
- RFC-0016 Versioning Model

Responsibilities:

- Repository initialization
- Repository metadata
- Version tracking
- Synchronization support

### functor-capabilities

Capability discovery and registration.

Relevant RFCs:

- RFC-0010 Capability Model
- RFC-0014 Extension Model

Responsibilities:

- Capability registration
- Capability inspection
- Extension contribution

### functor-cli

CLI projection over the API.

Relevant RFCs:

- RFC-0011 Projection Model

Examples:

```bash
functor init
functor artifact create
functor query
functor validate
```

### functor-mcp

MCP integration layer.

Relevant RFCs:

- RFC-0012 Interoperability and Integration

Responsibilities:

- Tool exposure
- Resource exposure
- Agent interoperability

## Dependency Direction

```text
functor-model
      ↑
functor-core
      ↑
+--------------------------+
| store                    |
| query                    |
| validation               |
| runtime                  |
| repository               |
| events                   |
| capabilities             |
+--------------------------+
      ↑
+----------------+
| CLI            |
| MCP            |
+----------------+
```

Rules:

- Lower layers must not depend on upper layers.
- Model must remain dependency-light.
- Projections depend on services.
- Services depend on contracts.

## MVP Scope

Required crates:

```text
functor-model
functor-core
functor-store
functor-query
functor-validation
functor-cli
```

Deferred until later:

```text
functor-runtime
functor-events
functor-repository
functor-capabilities
functor-mcp
```

## Mapping RFCs to Implementation

| RFC | Primary Crates |
|------|----------------|
| 0001 | model |
| 0002 | core |
| 0003 | model |
| 0004 | model, query |
| 0005 | model |
| 0006 | query |
| 0007 | validation, runtime |
| 0008 | store |
| 0009 | repository |
| 0010 | capabilities |
| 0011 | cli, mcp |
| 0012 | mcp |
| 0013 | repository, validation |
| 0014 | capabilities |
| 0015 | repository |
| 0016 | repository |
| 0017 | events |
| 0018 | ecosystem governance |

## Long-Term Goal

The workspace should support multiple projections and integrations while preserving a stable semantic core. Implementations may evolve, but the semantic model and API contracts should remain the center of the architecture.
