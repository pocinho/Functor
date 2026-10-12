# RFC-0001: Core Semantic Model

## Status

Draft

## Abstract

This RFC defines the core semantic model of Functor.

Functor is built around a small set of durable concepts that can be represented, queried, validated, versioned, and exchanged independently of any user interface or transport protocol.

The semantic model is the foundation upon which APIs, runtimes, storage systems, user interfaces, and future interoperability mechanisms are built.

---

# Motivation

Functor requires a stable conceptual foundation before defining APIs, runtimes, storage implementations, or protocols.

The semantic model answers a single question:

> What are the primary entities that exist within a Functor system?

These entities should remain useful regardless of whether Functor is accessed through a CLI, editor, notebook, web application, agent, or future projection.

---

# Design Principles

## Durable Concepts

Concepts should represent long-lived ideas rather than implementation details.

Examples of durable concepts include:

- Artifact
- Relationship
- Validation
- Execution
- Identity

Examples of non-durable concepts include:

- Window
- Button
- Notebook Cell
- Terminal Command
- HTTP Endpoint

---

## Interface Independence

The semantic model must not depend on any particular user experience.

The same artifact should be usable from:

- CLI
- Editor
- Notebook
- Web application
- Agent
- MCP integration

---

## Protocol Independence

The semantic model must not depend on a specific transport mechanism.

Future protocols must adapt to the model rather than define it.

---

# Core Entities

## Artifact

Artifacts are the primary unit of information.

An artifact is a versioned structure containing meaningful content.

Examples:

- RFC
- Specification
- Dataset
- Workflow
- Analysis
- Research note
- Source code asset

Every artifact possesses:

- Identity
- Metadata
- Content
- Relationships

---

## Identity

Every artifact must possess a stable identifier.

Identifiers allow artifacts to be:

- referenced
- queried
- versioned
- related

Identity is considered immutable.

---

## Relationship

Relationships define how artifacts connect.

Examples:

- depends_on
- derived_from
- references
- validates
- supersedes
- related_to

Relationships are first-class entities.

The graph emerges from relationships.

---

## Validation

A validation verifies a property of one or more artifacts.

Examples:

- Review
- Test
- Benchmark
- Analysis
- Simulation

Validations provide confidence in artifacts.

Validation results are artifacts themselves.

---

## Execution

Execution represents a reproducible operation.

Examples:

- Running code
- Evaluating a workflow
- Performing analysis
- Producing derived artifacts

Executions may generate outputs that become new artifacts.

---

# Semantic Graph

Functor represents knowledge as a graph of entities and relationships.

Example:

```text
RFC
  ├─ depends_on ─→ Specification
  ├─ validated_by ─→ Benchmark
  └─ supersedes ─→ Previous RFC
```

The graph is an emergent structure rather than a primary storage format.

---

# Projection Model

All user experiences are projections over the semantic model.

Examples:

| Projection | Representation |
|------------|---------------|
| CLI | Commands over artifacts |
| Editor | Structured authoring |
| Notebook | Exploratory workflow |
| Agent | Automated interaction |
| Graph View | Relationship visualization |

No projection owns the model.

---

# Non-Goals

This RFC does not define:

- Storage formats
- Wire protocols
- Transport mechanisms
- Runtime implementations
- Query languages
- User interfaces

These concerns are addressed by future RFCs.

---

# Future Work

Subsequent RFCs are expected to define:

1. API-First Architecture
2. Artifact Structure
3. Relationship Semantics
4. Validation Framework
5. Execution Model
6. Query Model
7. Interoperability Layers

---

# Guiding Question

When introducing a new concept, ask:

> Can this concept be represented, queried, validated, versioned, and used independently of a specific user interface or protocol?

If not, it likely belongs in a projection or implementation layer rather than the core semantic model.
