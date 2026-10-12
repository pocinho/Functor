# RFC-0014: Extension Model

## Status
Draft

## Summary

This RFC defines the Functor Extension Model.

Extensions are the primary mechanism through which Functor implementations add capabilities without modifying Functor Core. The extension model enables evolution, customization, and ecosystem growth while preserving a stable semantic foundation.

## Motivation

Functor is designed around a durable semantic model and API-first architecture.

As implementations grow, new functionality will emerge, including:

- Storage providers
- Query providers
- Validation engines
- Execution runtimes
- Renderers
- Projections
- Importers and exporters
- Integration adapters

Embedding all functionality inside the core platform would increase complexity and reduce adaptability.

Functor therefore adopts an extension-oriented architecture.

## Goals

- Allow independent evolution of functionality.
- Preserve a stable core platform.
- Support capability discovery.
- Enable composition of multiple implementations.
- Encourage ecosystem growth and experimentation.

## Non-Goals

This RFC does not define:

- Packaging formats
- Distribution mechanisms
- Security models for extension execution
- Language-specific implementation techniques

These concerns may be defined by future RFCs.

## Core Principles

### Core Remains Small

Functor Core should define semantics, APIs, and behavioral contracts.

Optional functionality should be implemented through extensions whenever practical.

### Capability-Oriented Design

Extensions contribute capabilities.

Consumers interact with capabilities rather than implementation details.

### Discoverability

A Functor implementation must expose which extensions are available and which capabilities they provide.

### Replaceability

Multiple extensions may satisfy the same capability.

Implementations should avoid coupling consumers to specific extension providers.

## Extension Definition

An extension is a deployable unit that contributes one or more capabilities to a Functor implementation.

Examples:

- Git-backed storage extension
- SQLite storage extension
- Markdown renderer extension
- TOML importer extension
- Python execution extension
- MCP integration extension

## Extension Categories

### Storage Extensions

Provide persistence and retrieval behavior.

### Query Extensions

Provide specialized indexing, traversal, or search functionality.

### Execution Extensions

Provide runtime execution environments.

### Validation Extensions

Provide validation strategies and verification mechanisms.

### Projection Extensions

Provide user-facing representations and experiences.

### Integration Extensions

Enable interoperability with external systems.

## Capability Contribution

Extensions declare the capabilities they contribute.

Illustrative example:

```text
Extension: sqlite-storage

Capabilities:
- storage
- transactions
- indexing
```

Capability declarations enable discovery and composition.

## Composition

Multiple extensions may coexist within a deployment.

Examples:

```text
SQLite Storage
PostgreSQL Storage
Git Repository Provider
```

or:

```text
Markdown Renderer
HTML Renderer
Terminal Renderer
```

Functor does not require a single implementation strategy.

## Lifecycle

An extension may progress through the following states:

```text
Discovered
Loaded
Active
Suspended
Unloaded
```

Implementations may define additional lifecycle behavior.

## Failure Isolation

Extensions should fail independently whenever possible.

A failure within one extension should not require failure of unrelated capabilities.

## API Considerations

Implementations should expose mechanisms for:

- Listing extensions
- Listing capabilities
- Enabling extensions
- Disabling extensions
- Inspecting extension metadata

Specific API contracts are outside the scope of this RFC.

## Future Work

Future RFCs may define:

- Extension packaging
- Extension distribution
- Extension repositories
- Security boundaries
- Sandboxing
- Capability negotiation

## Conclusion

The Extension Model provides a controlled mechanism for ecosystem evolution.

Functor Core defines semantics and contracts.

Extensions provide specialization, implementation diversity, and long-term adaptability.
