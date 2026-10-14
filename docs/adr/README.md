# Architecture Decision Records

## Purpose

ADRs record implementation decisions for the Functor CLI MVP, especially where the decision resolves an ambiguity or a conflict in the RFC series or the docs.

RFCs define what Functor is. ADRs record how this implementation chose to satisfy, subset or deviate from them. An ADR never edits an RFC. If an ADR deviates from an RFC, the deviation is also listed in [../rfc-conformance.md](../planning/rfc-conformance.md).

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
| [ADR-0002](ADR-0002-async-core-traits.md) | Async core traits with static dispatch | Superseded by ADR-0010 |
| [ADR-0003](ADR-0003-events-in-mvp.md) | Events are part of the MVP | Accepted (amended by ADR-0008) |
| [ADR-0004](ADR-0004-workspace-location.md) | Workspace location and root manifest | Accepted |
| [ADR-0005](ADR-0005-serialization-boundary.md) | Serialization lives in functor-store | Accepted |
| [ADR-0006](ADR-0006-file-per-entity.md) | One file per artifact and per relationship | Accepted |
| [ADR-0007](ADR-0007-relationship-types.md) | MVP relationship type set | Accepted |
| [ADR-0008](ADR-0008-event-log-rotation.md) | Event log rotation (10 MiB, 10 archives) | Accepted |
| [ADR-0009](ADR-0009-functor-repository-crate.md) | functor-repository is part of the MVP | Accepted |
| [ADR-0010](ADR-0010-async-trait-and-dyn.md) | async-trait; dynamic dispatch allowed | Accepted |
| [ADR-0011](ADR-0011-toml-edit-preserving-updates.md) | toml_edit; updates preserve comments and formatting | Accepted |
| [ADR-0012](ADR-0012-execution-provider-and-safety.md) | Workflow, local execution provider and execution safety | Proposed |
| [ADR-0013](ADR-0013-run-lifecycle-and-evidence-persistence.md) | Run lifecycle, input capture and evidence persistence | Proposed |
| [ADR-0014](ADR-0014-evidence-relationships.md) | Evidence relationship vocabulary (amends ADR-0007) | Proposed |
| [ADR-0015](ADR-0015-validation-report-persistence-and-events.md) | Persisted validation reports and execution events | Proposed |
