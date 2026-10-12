# RFC-0016: Versioning Model

## Status

Draft

## Abstract

This RFC defines how Functor artifacts, relationships, repositories, and associated metadata evolve over time.

Functor separates identity from version. Identities remain stable across revisions, while versions represent specific states of an entity at a point in time.

The versioning model enables reproducibility, traceability, synchronization, validation, federation, and historical inspection.

---

## Motivation

Artifacts evolve.

Relationships evolve.

Repositories evolve.

Consumers must be able to:

- inspect history
- reproduce prior results
- compare revisions
- synchronize changes
- validate historical states

without ambiguity.

Functor therefore requires a first-class versioning model.

---

## Design Principles

### Stable Identity

Identity answers:

> What is this?

An identity remains constant throughout the lifetime of an entity.

Example:

```text
artifact:functor:rfc-0003
```

The artifact remains the same logical artifact even as its content changes.

### Immutable Versions

Version answers:

> Which state of this entity is being referenced?

Versions are immutable.

Once published, a version MUST NOT be modified.

Example:

```text
artifact:functor:rfc-0003@1
artifact:functor:rfc-0003@2
artifact:functor:rfc-0003@3
```

### Versioned References

References MAY target:

- a logical identity
- a specific version

Examples:

```text
artifact:functor:rfc-0003
artifact:functor:rfc-0003@3
```

Consumers SHOULD use specific versions whenever reproducibility is required.

---

## Artifact Versioning

Every modification to an artifact creates a new version.

A version SHOULD capture:

- content state
- metadata state
- author attribution
- creation timestamp
- provenance information

Each version represents a complete and reproducible artifact state.

---

## Relationship Versioning

Relationships MAY evolve independently of artifacts.

Changes to a relationship produce a new relationship version.

Version history SHOULD preserve:

- source identity
- target identity
- relationship type
- metadata

Historical relationship states MUST remain inspectable.

---

## Repository Versioning

Repositories evolve through collections of changes.

A repository revision represents a consistent snapshot of:

- artifacts
- relationships
- metadata
- capabilities
- configuration

Repository revisions provide reproducible repository states.

---

## Lineage

Version history forms lineage.

Example:

```text
v1 -> v2 -> v3 -> v4
```

Consumers SHOULD be able to:

- inspect lineage
- navigate lineage
- compare lineage states

Lineage records MUST be preserved.

---

## Diffs and Comparison

Implementations SHOULD support comparison between versions.

Comparison MAY be performed on:

- artifacts
- relationships
- repositories

The representation of diffs is implementation-defined.

---

## Reproducibility

Validation and execution SHOULD reference specific versions.

Preferred:

```text
validate artifact:functor:rfc-0003@4
```

Less precise:

```text
validate artifact:functor:rfc-0003
```

Version-specific references improve reproducibility and traceability.

---

## Federation Considerations

Synchronization MUST operate on immutable versions.

Federated repositories SHOULD be able to determine:

- missing versions
- lineage divergence
- synchronization requirements

without rewriting historical records.

---

## Security and Trust Considerations

Trust assertions SHOULD bind to immutable versions.

Examples include:

- signatures
- attestations
- reviews
- approvals
- validation results

Trust associated with one version MUST NOT automatically apply to later versions.

---

## Non-Goals

This RFC does not define:

- semantic version numbering
- merge strategies
- conflict resolution
- repository workflows
- transport protocols

These concerns are expected to be addressed in future RFCs.

---

## Future Work

Future RFCs may define:

- repository events
- merge semantics
- change notifications
- governance workflows
- collaborative review processes

---

## Summary

The Functor Versioning Model is built on four principles:

1. Stable identities.
2. Immutable versions.
3. Preserved lineage.
4. Reproducible historical states.

Together these principles provide the foundation for synchronization, trust, validation, governance, and long-term evolution across the Functor ecosystem.
