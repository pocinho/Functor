# RFC-0008: Storage and Persistence

## Status

Draft

## Summary

This RFC defines the storage and persistence principles of the Functor ecosystem.

Storage systems are responsible for preserving artifacts, relationships, identities,
metadata, validation results, and execution records.

The storage layer is not the source of meaning within Functor.
Meaning is defined by the semantic model. Storage is responsible for durability,
retrieval, consistency, and evolution.

## Motivation

Functor defines:

- Artifacts
- Relationships
- Identity
- Queries
- Validation
- Execution

For these concepts to be useful across sessions, systems, and collaborators,
they must be persisted.

The storage model must support:

- Local-first workflows
- Offline operation
- Versioning
- Reproducibility
- Future federation
- Multiple implementation strategies

## Principles

### Storage-Agnostic Semantics

The semantic model must not depend on a specific database,
repository, filesystem, or cloud service.

A Functor artifact should be representable in:

- Filesystems
- Git repositories
- Embedded databases
- Document stores
- Graph databases
- Remote services

### Durable Identity

An artifact identifier must remain stable regardless of where the artifact is stored.

Moving content between repositories or storage engines must not change identity.

### Local-First

Functor implementations should prefer local persistence as the default mode.

Network connectivity should enhance functionality but should not be required for
core operations.

### Version Preservation

Storage systems should preserve historical states when practical.

Implementations may support:

- Snapshots
- Revision histories
- Event logs
- Source-control integration

## Storage Responsibilities

A storage implementation should provide:

- Persistence
- Retrieval
- Indexing
- Version access
- Metadata management
- Relationship storage

A storage implementation should not define:

- Semantic meaning
- Validation behavior
- Execution behavior
- Query language semantics

## Repository Concept

A repository is a logical collection of artifacts and relationships.

Repositories may be:

- Local
- Shared
- Embedded
- Remote
- Federated

Repositories expose capabilities through the Functor API.

## Persistence Units

The minimum persistence unit is an Artifact.

Associated information may include:

- Identity
- Metadata
- Relationships
- Validation records
- Execution records

## Indexing

Implementations may create indexes to support efficient querying.

Indexes are implementation details and must not affect semantic behavior.

## Portability

Data should be portable between implementations.

Storage providers should avoid proprietary representations that prevent migration.

## Future Considerations

Future RFCs may define:

- Federation
- Synchronization
- Replication
- Change streams
- Distributed repositories

These concerns are intentionally excluded from this RFC.

## Non-Goals

This RFC does not define:

- Database technologies
- File formats
- Synchronization protocols
- Network protocols
- Replication strategies

These concerns belong to future RFCs.

## Conclusion

Storage provides durability for the Functor semantic model.

The architecture remains API-first and storage-agnostic, enabling multiple
implementations while preserving a consistent understanding of artifacts,
relationships, identity, and history.
