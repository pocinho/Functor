Certainly. Here's the content I would put into crate-dependencies.md based on the RFC series and the API-first architecture we've established.

# Crate Dependencies

This document defines the dependency structure of the Functor workspace.

The primary goals are:

- Maintain strict architectural boundaries
- Preserve API-first design principles
- Minimize coupling
- Enable independent evolution of projections and integrations
- Keep the semantic model isolated from implementation details

---

# Architectural Layers

Functor is organized into layers.

Dependencies flow downward only.

```text
Projections
        ↓
Capabilities
        ↓
Core APIs
        ↓
Semantic Model
```

Storage and runtime implementations attach to the Core APIs.

---

# Workspace Structure

```text
functor/

├── crates/
│
├── functor-model
├── functor-core
├── functor-store
├── functor-runtime
├── functor-query
├── functor-validation
│
├── functor-cli
├── functor-mcp
├── functor-web
├── functor-editor
│
├── functor-events
├── functor-sync
├── functor-security
│
└── functor-extensions
```

---

# Layer 1: Semantic Model

## functor-model

Defines the core platform types.

Examples:

```rust
ArtifactId
RelationshipId
RepositoryId
VersionId

Artifact
Relationship
Repository

Event
Capability
```

Responsibilities:

- Domain entities
- Core value objects
- Semantic contracts
- Shared type definitions

Must not depend on:

```text
Storage
Runtime
CLI
Networking
Serialization Frameworks
```

This crate should remain as pure as possible.

---

# Layer 2: Core APIs

## functor-core

Defines platform interfaces.

Examples:

```rust
ArtifactRepository
RelationshipRepository

QueryEngine

ValidationEngine

ExecutionEngine
```

Responsibilities:

- Traits
- Service contracts
- Core abstractions

Depends on:

```text
functor-model
```

---

# Layer 3: Infrastructure

Infrastructure crates provide implementations.

---

## functor-store

Implements persistence.

Examples:

```rust
FileStore
GitStore
MemoryStore
```

Responsibilities:

- Repository persistence
- Serialization
- Data access

Depends on:

```text
functor-core
functor-model
```

---

## functor-query

Implements query execution.

Responsibilities:

```text
Filtering
Traversal
Selection
Graph exploration
```

Depends on:

```text
functor-core
functor-model
```

---

## functor-runtime

Implements execution.

Responsibilities:

```text
Executions
Runners
Execution results
Execution pipelines
```

Depends on:

```text
functor-core
functor-model
```

---

## functor-validation

Implements validation.

Examples:

```text
Reference validation
Schema validation
Relationship validation
Repository validation
```

Depends on:

```text
functor-core
functor-model
```

---

## functor-events

Implements RFC-0017.

Responsibilities:

```text
Event publication
Event subscription
Event replay
```

Depends on:

```text
functor-core
functor-model
```

---

## functor-sync

Implements RFC-0009.

Responsibilities:

```text
Repository synchronization
Federation support
Replication
```

Depends on:

```text
functor-core
functor-model
functor-store
```

---

## functor-security

Implements RFC-0013.

Responsibilities:

```text
Signatures
Trust assertions
Verification
```

Depends on:

```text
functor-core
functor-model
```

---

# Layer 4: Projections

Projections provide user-facing experiences.

They never define semantics.

They consume them.

---

## functor-cli

CLI projection.

Responsibilities:

```text
Command handling
Output formatting
Interactive workflows
```

Depends on:

```text
functor-core
functor-store
functor-query
functor-runtime
functor-validation
```

---

## functor-mcp

MCP projection.

Responsibilities:

```text
Tool exposure
Resource exposure
Agent interoperability
```

Depends on:

```text
functor-core
functor-query
functor-runtime
functor-validation
```

---

## functor-web

Web projection.

Responsibilities:

```text
REST APIs
Web interfaces
Dashboards
```

Depends on:

```text
functor-core
functor-query
functor-runtime
```

---

## functor-editor

Desktop editor projection.

Responsibilities:

```text
Artifact authoring
Relationship visualization
Repository navigation
```

Depends on:

```text
functor-core
functor-query
functor-runtime
```

---

# Extension System

## functor-extensions

Provides extension loading.

Responsibilities:

```text
Discovery
Registration
Capability contribution
Lifecycle management
```

Depends on:

```text
functor-core
functor-model
```

Extensions interact through public traits rather than direct crate coupling.

---

# MVP Dependency Graph

Initial MVP should focus on:

```text
functor-model
        ↑
functor-core
        ↑
functor-store
        ↑
functor-query
        ↑
functor-validation
        ↑
functor-cli
```

Everything else can be introduced later.

---

# Allowed Dependencies

Allowed:

```text
CLI
  -> Core

CLI
  -> Query

CLI
  -> Validation

Store
  -> Core

Runtime
  -> Core
```

---

# Forbidden Dependencies

Forbidden:

```text
Model
  -> Store

Model
  -> Runtime

Model
  -> CLI

Core
  -> CLI

Store
  -> CLI

Runtime
  -> CLI

Projection
  -> Projection
```

Examples:

```text
functor-cli
    must not depend on
functor-editor

functor-model
    must not depend on
functor-store
```

---

# Architectural Principle

The semantic model is the center of gravity.

Everything else is replaceable.

```text
Semantic Model
        ↓
Core APIs
        ↓
Infrastructure
        ↓
Projections
```

If a projection is removed:

```text
CLI
Editor
Notebook
Web
MCP
```

the platform should remain intact.

If a storage engine is replaced:

```text
File Store
Git Store
Database Store
```

the platform should remain intact.

If a protocol evolves:

```text
MCP
HTTP
Future Protocol
```

the platform should remain intact.

The semantic model and core APIs are the durable foundation of Functor.
