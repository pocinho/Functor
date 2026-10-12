# API Design Guidelines

## Purpose

This document defines API design conventions for the Functor ecosystem.

## Core Principles

### API First

APIs are the primary contract of the platform.

### Semantic Consistency

Expose RFC-defined domain concepts:

- Artifact
- Relationship
- Repository
- Validation
- Execution
- Capability

### Projection Independence

APIs must not assume a CLI, GUI, web, or agent consumer.

## Trait Design

### Focused Traits

```rust
pub trait ArtifactRepository {
    fn get(&self, id: &ArtifactId) -> Result<Artifact>;
}
```

### Composition Over Inheritance

Prefer many small composable traits.

### Async Boundaries

I/O-facing traits should be asynchronous.

## Naming Conventions

### Types

```text
Artifact
Relationship
Repository
Query
Validation
```

### Traits

```text
ArtifactRepository
RelationshipRepository
QueryEngine
ValidationEngine
ExecutionEngine
```

### Methods

```text
create
get
list
query
validate
execute
```

## Error Handling

```rust
pub enum FunctorError {
    NotFound,
    ValidationFailed,
    RepositoryError,
}
```

## Dependency Rules

```text
functor-model
    ↑
functor-core
    ↑
functor-store
    ↑
functor-cli
```

## Capability Exposure

Capabilities should be discoverable and implementation-independent.

## Versioning

Favor additive API evolution.

## Testing

Support:

- Unit tests
- Integration tests
- Mock implementations

## Documentation

Every public API should document:

- Purpose
- Inputs
- Outputs
- Error conditions
- Guarantees

## MVP Scope

Prioritize:

- ArtifactRepository
- RelationshipRepository
- QueryEngine
- ValidationEngine
