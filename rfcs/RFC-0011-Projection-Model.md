# RFC-0011: Projection Model

## Status

Draft

## Abstract

Functor is an API-first platform whose capabilities are exposed through one or more projections.

A projection is a representation of the Functor semantic model optimized for a particular interaction mode, audience, or workflow.

Projections do not define domain behavior. They expose domain behavior already defined by the semantic model, API contracts, and capability model.

This RFC defines the role, requirements, and constraints of projections within the Functor ecosystem.

---

## Motivation

Different users and systems interact with information differently.

Examples include:

- command-line interfaces
- graphical editors
- notebooks
- web applications
- intelligent agents
- automation pipelines
- MCP integrations

Traditionally, applications embed their own data models and workflows.

Functor adopts a different approach.

The semantic model remains authoritative.

User experiences are projections over the same platform.

This enables multiple interaction styles without fragmenting the underlying representation.

---

## Goals

- Define projections as first-class architectural concepts.
- Separate presentation concerns from semantic concerns.
- Enable multiple user experiences over a common API.
- Allow new projections to emerge without modifying core platform RFCs.
- Promote interoperability between implementations.

---

## Non-Goals

This RFC does not define:

- visual design systems
- specific user interfaces
- editor behaviors
- notebook behaviors
- transport protocols

These may be specified by future RFCs.

---

## Definition

A projection is a consumer-facing interpretation of the Functor platform.

A projection may:

- present artifacts
- navigate relationships
- execute queries
- invoke validations
- initiate execution
- expose capabilities

A projection must not redefine platform semantics.

---

## Architectural Position

```text
Semantic Model
        ↓
Core API
        ↓
Capability Model
        ↓
Projection
```

The projection layer exists above the API and capability layers.

All projections interact with the platform through defined APIs.

---

## Projection Types

### CLI Projection

Optimized for:

- automation
- scripting
- developer workflows
- batch operations

### Editor Projection

Optimized for:

- authoring
- reviewing
- navigation
- relationship exploration

### Notebook Projection

Optimized for:

- exploratory workflows
- iterative execution
- analysis and experimentation

### Web Projection

Optimized for:

- accessibility
- collaboration
- publishing

### Agent Projection

Optimized for:

- autonomous operation
- orchestration
- reasoning workflows

### MCP Projection

Optimized for:

- capability exposure
- tool discovery
- interoperability with intelligent systems

---

## Shared Semantics

Regardless of projection type:

- artifact identity remains unchanged
- relationships remain unchanged
- query behavior remains unchanged
- validation semantics remain unchanged
- execution semantics remain unchanged

A projection changes representation, not meaning.

---

## Projection Independence

Implementations may provide:

- one projection
- multiple projections
- custom projections

Compliance with the Functor platform does not require any specific projection.

A headless implementation remains valid.

---

## Extensibility

New projections may be introduced without modification to:

- artifact definitions
- relationship definitions
- identity rules
- query rules
- storage semantics

This allows the platform to evolve while preserving compatibility.

---

## Future Directions

Future RFCs may define:

- notebook projection guidelines
- editor integration standards
- web application recommendations
- agent interaction patterns
- projection packaging and discovery

---

## Summary

Functor is defined by its semantic model, APIs, and capabilities.

Projections are replaceable interfaces built upon those foundations.

This architecture enables diverse experiences while preserving a single source of truth for meaning and behavior.
