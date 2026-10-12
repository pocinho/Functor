# Traits and Interfaces

## Purpose

This document defines the primary Rust traits and interfaces that form the Functor API surface.

The goal is to establish stable contracts between the semantic model, storage implementations, projections, extensions, and integrations.

## Design Principles

- Trait-first architecture
- API-first design
- Dependency inversion
- Projection independence
- Implementation agnostic contracts

## Core Traits

### ArtifactRepository

```rust
pub trait ArtifactRepository {
    fn create(&self, artifact: Artifact) -> Result<ArtifactId>;
    fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>>;
    fn update(&self, artifact: Artifact) -> Result<()>;
    fn delete(&self, id: &ArtifactId) -> Result<()>;
    fn list(&self) -> Result<Vec<Artifact>>;
}
```

### RelationshipRepository

```rust
pub trait RelationshipRepository {
    fn create(&self, relationship: Relationship) -> Result<RelationshipId>;
    fn get(&self, id: &RelationshipId) -> Result<Option<Relationship>>;
    fn list(&self) -> Result<Vec<Relationship>>;
}
```

### QueryEngine

```rust
pub trait QueryEngine {
    fn execute(&self, query: Query) -> Result<QueryResult>;
}
```

### ValidationEngine

```rust
pub trait ValidationEngine {
    fn validate(&self, target: ValidationTarget)
        -> Result<ValidationReport>;
}
```

### ExecutionEngine

```rust
pub trait ExecutionEngine {
    fn execute(&self, target: ExecutionTarget)
        -> Result<ExecutionResult>;
}
```

### EventPublisher

```rust
pub trait EventPublisher {
    fn publish(&self, event: Event) -> Result<()>;
}
```

### EventSubscriber

```rust
pub trait EventSubscriber {
    fn handle(&self, event: &Event) -> Result<()>;
}
```

### CapabilityProvider

```rust
pub trait CapabilityProvider {
    fn capabilities(&self) -> Vec<Capability>;
}
```

### Extension

```rust
pub trait Extension {
    fn metadata(&self) -> ExtensionMetadata;
    fn capabilities(&self) -> Vec<Capability>;
}
```

## MVP Scope

The initial CLI MVP should implement:

- ArtifactRepository
- RelationshipRepository
- QueryEngine
- ValidationEngine

The remaining interfaces can be introduced incrementally.

## Dependency Rules

- `functor-model` contains types only.
- `functor-core` contains traits and contracts.
- `functor-store` implements persistence interfaces.
- `functor-cli` consumes APIs and provides a projection.
- Extensions depend on core contracts, not concrete implementations.

## Future Evolution

Future RFCs may introduce:

- Async variants
- Streaming query APIs
- Distributed repositories
- Capability negotiation
- Federated execution
- Protocol adapters

Traits should evolve conservatively to preserve compatibility.
