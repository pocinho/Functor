# RFC-0006: Query Model

## Status

Draft

## Abstract

This RFC defines the Query Model for the Functor ecosystem.

The Query Model provides a consistent mechanism for discovering, traversing, filtering, projecting, and retrieving artifacts and their relationships. It is independent of storage backends, transport mechanisms, and user interfaces.

Queries are a first-class capability of the Functor API and represent the primary mechanism through which humans, applications, and intelligent agents explore the semantic graph.

---

## Motivation

Previous RFCs define:

- Vision (RFC-0000)
- Core Semantic Model (RFC-0001)
- API-First Architecture (RFC-0002)
- Artifact Model (RFC-0003)
- Relationship Model (RFC-0004)
- Identity and Addressing (RFC-0005)

With artifacts, relationships, and identities established, consumers require a standard way to locate and navigate information.

The Query Model provides this capability.

---

## Goals

The Query Model SHALL:

- Locate artifacts by identity.
- Traverse relationships.
- Filter by metadata and attributes.
- Support graph exploration.
- Support multiple projections.
- Remain independent of storage implementation.
- Support both human and agent consumers.

---

## Core Principles

### Query as Navigation

Queries are not limited to retrieval.

A query may:

- locate a specific artifact
- discover related artifacts
- traverse relationships
- construct views
- support reasoning workflows

### Storage Independence

Consumers SHALL interact with a common query abstraction.

Implementations MAY use:

- files
- databases
- graph stores
- distributed repositories

without changing query semantics.

### Projection Independence

The same query should be usable from:

- CLI projections
- editor projections
- web projections
- notebook projections
- MCP integrations

---

## Query Operations

### Lookup

Retrieve artifacts by identity.

Example:

```text
artifact("rfc-0006")
```

### Search

Locate artifacts using metadata and content.

Example:

```text
search(type = "rfc")
```

### Traverse

Follow one or more relationships.

Example:

```text
traverse(rfc-0006, relation = references)
```

### Filter

Restrict results based on attributes.

Example:

```text
filter(status = draft)
```

### Project

Transform results into a specific representation.

Example:

```text
project(title, author, status)
```

---

## Graph Traversal

The Functor graph is navigated through relationships.

Traversal MAY be:

- inbound
- outbound
- bidirectional

Traversal depth MAY be limited.

Example:

```text
artifact
  -> references
  -> references
```

---

## Query Results

A query result SHALL contain:

- identifier
- artifact reference
- relationship information when applicable
- metadata required by the projection

Results SHOULD remain immutable for the duration of a query execution.

---

## API Considerations

Implementations SHOULD expose query capabilities through the Functor API.

Illustrative interface:

```rust
trait QueryEngine {
    fn lookup(&self, id: ArtifactId);
    fn search(&self, query: Query);
    fn traverse(&self, traversal: Traversal);
}
```

The exact interface is implementation-defined.

---

## Future Work

Future RFCs may define:

- query language syntax
- graph analytics
- ranking
- reasoning support
- federated querying
- distributed graph traversal

These capabilities build upon the model defined in this RFC.

---

## Summary

The Query Model provides a storage-independent mechanism for discovering and navigating artifacts and relationships.

It establishes querying as a first-class capability of the Functor ecosystem and enables future projections, APIs, and intelligent consumers to explore the semantic graph through a consistent abstraction.
