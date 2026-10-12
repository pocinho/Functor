# RFC-0005: Identity and Addressing

## Status

Draft

## Summary

This RFC defines how Artifacts, Relationships, and future Functor resources are identified and addressed.

Identity must be stable, implementation-independent, and durable across storage backends, user interfaces, and interoperability layers.

Addressing provides a canonical mechanism for locating and referencing entities within the Functor ecosystem.

---

## Motivation

As the Functor graph grows, references between artifacts become as important as the artifacts themselves.

Without a consistent identity model:

- references become fragile
- relationships cannot be reliably persisted
- distributed systems cannot collaborate
- queries become ambiguous
- interoperability becomes difficult

Identity must therefore be treated as a first-class concern.

---

## Goals

- Provide globally unique identifiers.
- Decouple identity from storage location.
- Support future federation and synchronization.
- Enable stable references across projections.
- Allow artifacts to be relocated without breaking references.

---

## Non-Goals

This RFC does not define:

- storage engines
- synchronization protocols
- transport protocols
- authorization models

Those concerns may depend upon identity but are defined separately.

---

## Identity Principles

### Identity Is Immutable

An entity identifier MUST remain stable throughout its lifetime.

Changes to:

- content
- metadata
- relationships

MUST NOT alter the entity identifier.

### Identity Is Independent of Storage

Identifiers MUST NOT encode:

- filesystem paths
- database primary keys
- URLs
- user interface locations

Identity and location are separate concerns.

### Identity Is Opaque

Consumers MUST NOT infer semantic meaning from an identifier.

Identifiers exist only to establish uniqueness and references.

---

## Addressable Entities

The following entities are expected to possess identities:

- Artifact
- Relationship
- Validation
- Execution

Future RFCs MAY introduce additional addressable entities.

---

## Canonical References

All addressable entities SHOULD expose a canonical reference.

Conceptually:

```text
functor:<entity-type>:<identifier>
```

Examples:

```text
functor:artifact:3d8af7
functor:relationship:91cd2e
```

The exact syntax remains implementation-defined at this stage.

---

## Versioning

Identity and version are distinct.

```text
Artifact Identity
        +
   Version State
```

An artifact may evolve through multiple revisions while preserving a stable identity.

---

## Reference Integrity

Relationships SHOULD reference entity identities rather than storage locations.

Example:

```text
Artifact A
    references
Artifact B
```

The relationship remains valid regardless of where either artifact is stored.

---

## Query Integration

All query mechanisms SHOULD support retrieval by canonical identifier.

Examples:

```text
get(id)
resolve(id)
exists(id)
```

The exact API surface is defined separately.

---

## Future Considerations

Future RFCs may extend this model with:

- content-addressable identities
- federated identifiers
- cross-repository references
- provenance chains
- cryptographic verification

These capabilities should build upon, rather than replace, the identity model established by this RFC.

---

## Rationale

Artifacts provide the nouns of the ecosystem.

Relationships provide the grammar.

Identity provides the names.

Without stable identity, neither relationships nor distributed collaboration can be reliably established.
