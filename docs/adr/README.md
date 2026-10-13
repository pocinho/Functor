# Architecture Decision Records

## Purpose

ADRs record implementation decisions for the Functor CLI MVP, especially where the decision resolves an ambiguity or a conflict in the RFC series or the docs.

RFCs define what Functor is. ADRs record how this implementation chose to satisfy, subset or deviate from them. An ADR never edits an RFC. If an ADR deviates from an RFC, the deviation is also listed in [../rfc-conformance.md](../rfc-conformance.md).

## Process

1. Copy the template below into `ADR-NNNN-short-title.md` using the next free number.
2. Set the status to `Proposed`, open a pull request and discuss.
3. When merged, the status becomes `Accepted`.
4. An accepted ADR is never rewritten. To change a decision, add a new ADR and mark the old one `Superseded by ADR-NNNN`.

## Template

```text
# ADR-NNNN: Title

## Status
Proposed | Accepted | Superseded by ADR-NNNN

## Context
## Decision
## Consequences
## RFC Impact
```

## Index

| ADR | Title | Status |
|-----|-------|--------|
| [ADR-0001](ADR-0001-two-part-identity.md) | Two-part identity: opaque id plus slug | Accepted |
| [ADR-0002](ADR-0002-async-core-traits.md) | Async core traits with static dispatch | Accepted |
| [ADR-0003](ADR-0003-events-in-mvp.md) | Events are part of the MVP | Accepted |
| [ADR-0004](ADR-0004-workspace-location.md) | Workspace location and root manifest | Accepted |
| [ADR-0005](ADR-0005-serialization-boundary.md) | Serialization lives in functor-store | Accepted |
| [ADR-0006](ADR-0006-file-per-entity.md) | One file per artifact and per relationship | Accepted |
| [ADR-0007](ADR-0007-relationship-types.md) | MVP relationship type set | Accepted |
