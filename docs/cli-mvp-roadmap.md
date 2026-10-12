# CLI MVP Roadmap

## Purpose

This document describes a practical implementation roadmap for the first Functor CLI MVP.

The goal of the MVP is not to implement every RFC. The goal is to validate the core architectural assumptions defined in RFC-0000 through RFC-0018 through a working API-first implementation.

---

# MVP Goal

Demonstrate:

- Artifact creation
- Relationship management
- Repository persistence
- Query capabilities
- Validation workflows
- API-first architecture
- CLI as a projection over the API

Success is measured by proving the platform architecture, not by feature completeness.

---

# Guiding Principles

## API First

All functionality must be implemented in reusable APIs before being exposed through the CLI.

```text
Core API
    ↓
CLI Commands
```

The CLI must never become the system of record.

## Semantic First

Implementation must follow the semantic model defined by the RFC series.

## Incremental Delivery

Each milestone must produce a usable result.

---

# Workspace Structure

Suggested Rust workspace:

```text
functor/
├── crates/
│   ├── functor-model
│   ├── functor-core
│   ├── functor-store
│   ├── functor-query
│   ├── functor-validation
│   └── functor-cli
├── examples/
├── rfcs/
└── roadmaps/
```

---

# Phase 1 - Core Domain Model

## Objective

Implement RFC-0001 through RFC-0005.

## Deliverables

### functor-model

Core types:

```rust
Artifact
ArtifactId
Relationship
RelationshipId
Repository
Query
```

### Identity Support

```rust
ArtifactId
RelationshipId
RepositoryId
```

### Relationship Types

Examples:

```text
depends_on
references
extends
derived_from
```

## Completion Criteria

Artifacts and relationships can be created entirely in memory.

---

# Phase 2 - Core APIs

## Objective

Implement RFC-0002.

### functor-core

Traits only.

```rust
ArtifactRepository
RelationshipRepository
QueryEngine
ValidationEngine
```

No storage implementation.

## Completion Criteria

Domain operations can be performed through API contracts.

---

# Phase 3 - Repository Storage

## Objective

Implement foundational RFC-0008 support.

### functor-store

Local file-backed implementation.

Suggested layout:

```text
.functor/
    repository.toml

artifacts/
relationships/
```

## Commands

```bash
functor init
```

## Completion Criteria

Repositories persist to disk.

---

# Phase 4 - Artifact Management

## Objective

Implement RFC-0003.

## Commands

```bash
functor artifact create
functor artifact list
functor artifact show
```

Example:

```bash
functor artifact create rfc-0000
```

## Completion Criteria

Artifacts can be stored and retrieved.

---

# Phase 5 - Relationship Management

## Objective

Implement RFC-0004.

## Commands

```bash
functor relation add
functor relation list
```

Example:

```bash
functor relation add \
  rfc-0001 \
  depends-on \
  rfc-0000
```

## Completion Criteria

Artifacts form a navigable graph.

---

# Phase 6 - Query Engine

## Objective

Implement RFC-0006.

### functor-query

Support:

```text
Lookup
Traversal
Dependency inspection
Reverse dependency inspection
```

## Commands

```bash
functor query artifacts
functor query dependencies rfc-0001
functor query dependents rfc-0000
```

## Completion Criteria

Users can discover and navigate repository content.

---

# Phase 7 - Validation

## Objective

Implement an initial RFC-0007 subset.

### functor-validation

Checks:

```text
Missing references
Invalid identities
Duplicate identifiers
Broken relationships
```

## Commands

```bash
functor validate
```

## Completion Criteria

Repository integrity can be verified.

---

# Phase 8 - Events

## Objective

Implement an RFC-0017 subset.

### Event Types

```text
artifact.created
artifact.updated
relationship.created
validation.completed
```

## Commands

```bash
functor watch
```

## Completion Criteria

Consumers can observe repository activity.

---

# Future Phases

The following RFCs are intentionally deferred:

```text
RFC-0009 Federation and Synchronization
RFC-0010 Capability Model
RFC-0012 Interoperability and Integration
RFC-0013 Security and Trust Model
RFC-0014 Extension Model
RFC-0016 Versioning Model
RFC-0018 Governance Model
```

These should be implemented only after the core architecture proves itself.

---

# MVP Success Criteria

The MVP is considered successful when the following workflow exists:

```bash
functor init

functor artifact create rfc-0000
functor artifact create rfc-0001

functor relation add \
  rfc-0001 \
  depends-on \
  rfc-0000

functor query dependencies rfc-0001

functor validate
```

This demonstrates:

- Semantic Model
- API-First Architecture
- Artifact Model
- Relationship Model
- Identity and Addressing
- Query Model
- Validation
- Persistence

and provides a strong foundation for future evolution.
