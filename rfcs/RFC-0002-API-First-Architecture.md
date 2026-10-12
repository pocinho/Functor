# RFC-0002: API-First Architecture

## Status

Draft

## Abstract

Functor is an API-first platform.

The primary contract of the system is its semantic API rather than any specific user interface, transport protocol, storage engine, or runtime implementation.

All user experiences, including command-line tools, editors, notebooks, web applications, services, and intelligent agents, are projections over a common semantic model and API.

This RFC defines the architectural principles that guide the design and evolution of the Functor ecosystem.

---

## Motivation

Software systems often become tightly coupled to their initial user interface.

Examples include:

- CLI-first systems that later struggle to support graphical experiences
- Web-first systems that expose incomplete automation interfaces
- Notebook-centric systems that couple execution, storage, and presentation

Functor seeks to avoid this coupling.

The semantic model and API should remain stable even as user experiences evolve.

---

## Principles

### API Before Interface

Every capability exposed through a user interface should be expressible through the core API.

Interfaces are consumers of capabilities.

They do not define capabilities.

### Semantic Consistency

All projections must operate on the same underlying semantic model.

An artifact created through one projection must be accessible and manipulable through any other projection.

### Projection Independence

Projections should be replaceable without requiring changes to the core platform.

Examples of projections include:

- CLI
- Editor
- Notebook
- Web Application
- MCP Integration
- Automation Service

### Capability-Oriented Design

The platform should be organized around capabilities rather than interfaces.

Examples:

- Create Artifact
- Query Artifact
- Relate Artifacts
- Execute Validation
- Execute Workflow
- Observe Results

---

## Architectural Layers

```text
Semantic Model
      ↓
Core API
      ↓
Adapters and Integrations
      ↓
User Projections
```

### Semantic Model

Defines what exists.

Examples:

- Artifact
- Relation
- Validation
- Execution

### Core API

Defines what can be done.

Examples:

- create
- update
- query
- validate
- execute
- relate

### Adapters and Integrations

Translate the API into external systems.

Examples:

- MCP
- HTTP
- Local SDKs
- Future protocols

### User Projections

Provide human or machine-facing experiences.

Examples:

- CLI
- Editor
- Notebook
- Web UI
- Agent

---

## MCP and Agent Interoperability

Functor does not define a dedicated agent communication protocol at this stage.

Agent interoperability should be achieved through established interoperability standards where practical.

MCP is considered an integration layer rather than a core component of the semantic model.

Functor defines capabilities and meaning.

Interoperability layers expose those capabilities.

---

## Non-Goals

This RFC does not define:

- network protocols
- transport formats
- storage implementations
- wire representations
- user interface designs

Those concerns may be addressed by future RFCs.

---

## Future Work

Subsequent RFCs are expected to define:

1. Artifact Structure
2. Relationship Model
3. Validation Model
4. Execution Model
5. Query Model
6. Interoperability Protocols

These RFCs should remain consistent with the API-first principles established here.
