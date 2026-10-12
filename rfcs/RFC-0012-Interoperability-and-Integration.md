# RFC-0012: Interoperability and Integration

## Status
Draft

## Summary

This RFC defines how Functor interoperates with external systems while preserving its API-first and semantic-model-first architecture.

Functor does not define interoperability through its core model. Instead, interoperability is achieved through adapters, integrations, and capability mappings built upon the Functor API.

## Motivation

A Functor implementation should be able to participate in broader ecosystems without requiring external systems to adopt Functor internals.

Examples include:

- MCP hosts and clients
- IDEs and editors
- CI/CD systems
- Version control platforms
- Data platforms
- Knowledge systems
- Agent runtimes

The goal is to enable participation without coupling the core model to any particular external protocol.

## Principles

### External Standards First

Where practical, Functor should adopt existing standards rather than invent new protocols.

### Semantic Independence

The Functor semantic model remains independent from transport, serialization, and integration concerns.

### Adapter-Based Architecture

Integrations are implemented as adapters over the API.

```text
Functor Semantic Model
          ↓
       Functor API
          ↓
    Integration Layer
      ├─ MCP
      ├─ CLI
      ├─ HTTP
      ├─ Git
      └─ Future Adapters
```

### Capability Mapping

External systems should interact with capabilities rather than implementation details.

Examples:

- query
- validate
- execute
- synchronize
- discover
- transform

## MCP Integration

Model Context Protocol (MCP) is considered a preferred interoperability mechanism for agent participation.

Functor implementations may expose API capabilities through MCP tools and resources.

MCP integration must:

- remain optional
- not alter the semantic model
- not introduce MCP-specific assumptions into core APIs
- be replaceable by other interoperability mechanisms

## Discovery

Implementations should support discovery of:

- capabilities
- supported projections
- repository features
- execution providers
- validation providers

Discovery should be capability-oriented rather than implementation-oriented.

## Non-Goals

This RFC does not define:

- wire protocols
- transport formats
- networking specifications
- message schemas

Those concerns may emerge from future implementation experience.

## Future Work

Future RFCs may define:

- Protocol specifications
- Agent collaboration models
- External federation mechanisms
- Capability negotiation
- Security and trust models

## Rationale

Interoperability should emerge from stable semantics and APIs.

Functor defines meaning.

Integrations define communication.
