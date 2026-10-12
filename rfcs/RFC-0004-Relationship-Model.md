# RFC-0004: Relationship Model

## Status

Draft

## Abstract

This RFC defines the relationship model for Functor.

Artifacts are the primary durable objects in the Functor ecosystem. Relationships provide the structure through which artifacts form larger systems of meaning, validation, execution, and collaboration.

The relationship model defines how artifacts reference, depend upon, extend, support, and derive from one another.

## Motivation

An isolated artifact contains information.

A connected set of artifacts contains context.

Functor aims to enable reasoning, traceability, validation, and evolution across collections of artifacts. This requires relationships to be treated as first-class elements of the semantic model.

## Goals

- Define a standard relationship model.
- Enable graph traversal and querying.
- Support traceability across artifacts.
- Preserve implementation independence.
- Provide a foundation for validation and execution workflows.

## Non-Goals

This RFC does not define:

- Storage implementations.
- Query languages.
- Execution semantics.
- Network protocols.

## Relationship Fundamentals

A relationship associates one artifact with another.

Conceptually:

```text
Artifact A ----relationship----> Artifact B
```

Relationships are directional.

Systems may choose to expose reverse relationships through indexing or query mechanisms.

## Core Relationship Types

### references

Indicates that an artifact references another artifact.

```text
RFC references Design Note
```

### depends_on

Indicates that an artifact requires another artifact.

```text
CLI depends_on API Specification
```

### derived_from

Indicates that an artifact originates from another artifact.

```text
Report derived_from Dataset
```

### extends

Indicates an extension or specialization.

```text
RFC-0004 extends RFC-0001
```

### supports

Indicates supporting evidence or rationale.

```text
Benchmark supports Proposal
```

### contradicts

Indicates disagreement or conflicting information.

```text
Experiment contradicts Hypothesis
```

## Relationship Identity

Relationships should possess stable identifiers.

Example:

```toml
id = "rel-001"
type = "depends_on"
source = "artifact-a"
target = "artifact-b"
```

## Relationship Metadata

Implementations may associate metadata with relationships.

Examples include:

- creation timestamp
- author
- confidence
- provenance
- annotations

Metadata must not alter semantic meaning.

## Graph Semantics

Artifacts and relationships together form a graph.

```text
Artifact
 ├─ references
 ├─ depends_on
 ├─ supports
 └─ derived_from
```

Implementations may store this graph using any suitable approach.

The semantic model must remain independent from storage choices.

## Validation Considerations

Relationships may participate in validation.

Examples:

- detect cycles
- verify referenced artifacts exist
- validate dependency integrity
- validate provenance chains

Validation behavior is specified in future RFCs.

## API Considerations

Future APIs should support operations such as:

```text
create_relationship
remove_relationship
list_relationships
find_related
traverse
```

The exact API surface is defined separately.

## Future Directions

Future RFCs may define:

- semantic relationship constraints
- relationship schemas
- graph queries
- inferred relationships
- agent-generated relationships

## Summary

Relationships provide the connective structure of the Functor semantic model.

Artifacts represent durable units of information.
Relationships express how those units interact.

Together they form the foundation for traceability, validation, execution, and collaborative evolution across the Functor ecosystem.
