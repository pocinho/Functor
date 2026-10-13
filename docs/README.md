# Functor Documentation

Welcome to the Functor architecture and implementation documentation.

This documentation set accompanies the Functor RFC series and provides practical guidance for building the first CLI MVP implementation in Rust.

The RFCs define what Functor is. The documents in this directory define how to build it.

---

# Purpose

Functor is an API-first platform for creating, relating, validating, and evolving structured artifacts.

This documentation bridges the gap between:

```text
RFC Specifications -> Architecture -> Contracts -> Implementation
```

The documents are implementation-oriented and written for contributors, implementers and intelligent agents participating in development.

---

# Document Conventions

Every document begins with a status line and, where useful, links to its related RFCs, ADRs and sibling documents.

| Status | Meaning |
|--------|---------|
| Draft | Under development; may change |
| Accepted | Agreed and implemented; changes need a pull request that also updates affected tests |

ADRs have their own statuses ([adr/README.md](adr/README.md)).

**Precedence:** an RFC always wins over a document here. Where the MVP intentionally subsets or deviates from an RFC, it is recorded in an ADR and in [rfc-conformance.md](rfc-conformance.md); RFCs are never edited to match the implementation.

**Contracts are documents first.** Formats, rule codes, exit codes, JSON schemas and trait signatures are defined here before they are coded. A pull request that changes behavior updates the contract document in the same change.

---

# Documentation Structure

## Planning

| Document | Purpose |
|----------|---------|
| [mvp-scope.md](mvp-scope.md) | What is in and out of the MVP, exit criteria with their evidence, the frozen contracts |
| [cli-mvp-roadmap.md](cli-mvp-roadmap.md) | Ordered milestones M0 to M9 with acceptance evidence |
| [product-roadmap.md](product-roadmap.md) | Long-term phases after the MVP (replaces `implementation-phases.md`) |

## Decisions and Traceability

| Document | Purpose |
|----------|---------|
| [adr/](adr/README.md) | Architecture Decision Records (ADR-0001 to ADR-0011) |
| [rfc-conformance.md](rfc-conformance.md) | RFC requirement to MVP status, implementation and test (every named test is checked by `scripts/check-conformance.ps1`) |
| [glossary.md](glossary.md) | Canonical terminology |

## Architecture

| Document | Purpose |
|----------|---------|
| [workspace-layout.md](workspace-layout.md) | Rust workspace structure and crate responsibilities |
| [crate-dependencies.md](crate-dependencies.md) | Allowed and forbidden dependency edges, composition root |
| [dependency-policy.md](dependency-policy.md) | Approved third-party crates and rules for adding more |
| [traits-and-interfaces.md](traits-and-interfaces.md) | Core service traits |
| [api-design-guidelines.md](api-design-guidelines.md) | API conventions |
| [error-handling.md](error-handling.md) | Error taxonomy, exit codes, messages |

## Domain and Data

| Document | Purpose |
|----------|---------|
| [domain-model.md](domain-model.md) | Rust types and invariants of the semantic model |
| [identity-and-addressing.md](identity-and-addressing.md) | Ids, slugs, canonical references, CLI resolution |
| [artifact-format.md](artifact-format.md) | Artifact TOML format |
| [repository-layout.md](repository-layout.md) | On-disk repository structure, relationship and repository files |
| [storage-design.md](storage-design.md) | Atomic writes, locking, determinism, format versioning |

## Behavior

| Document | Purpose |
|----------|---------|
| [query-model-mvp.md](query-model-mvp.md) | Query operations and semantics |
| [validation-rules.md](validation-rules.md) | Rule catalog, outcomes, exit-code policy |
| [event-model-mvp.md](event-model-mvp.md) | Events, log format, `watch` |
| [repository-lifecycle.md](repository-lifecycle.md) | Init, discovery, concurrency, evolution |

## CLI

| Document | Purpose |
|----------|---------|
| [cli-spec.md](cli-spec.md) | Commands, options, output, JSON schemas, exit codes |
| [command-design.md](command-design.md) | Pointer to `cli-spec.md` (superseded) |
| [getting-started.md](getting-started.md) | MVP walkthrough with expected output; the acceptance script. Its result is the reference repository [`examples/getting-started/`](../examples/getting-started/) |

## Quality and Process

| Document | Purpose |
|----------|---------|
| [testing-strategy.md](testing-strategy.md) | Test levels, conformance suite, goldens, acceptance test |
| [nonfunctional-requirements.md](nonfunctional-requirements.md) | Determinism, portability, performance budgets (measured by `scripts/bench.ps1`, baseline in `benchmarks/`), local safety and its evidence |
| [development-workflow.md](development-workflow.md) | Toolchain, lints, CI, Definition of Done |

---

# Relationship to RFCs

RFCs define what Functor is. Documentation defines how Functor is implemented. See "Precedence" above.

---

# Recommended Reading Order

1. [RFC-0000 Vision](../rfcs/RFC-0000-Functor-Vision.md)
2. [mvp-scope.md](mvp-scope.md)
3. [glossary.md](glossary.md)
4. [adr/](adr/README.md)
5. [domain-model.md](domain-model.md) and [identity-and-addressing.md](identity-and-addressing.md)
6. [workspace-layout.md](workspace-layout.md), [crate-dependencies.md](crate-dependencies.md)
7. [traits-and-interfaces.md](traits-and-interfaces.md), [error-handling.md](error-handling.md), [api-design-guidelines.md](api-design-guidelines.md)
8. [artifact-format.md](artifact-format.md), [repository-layout.md](repository-layout.md), [storage-design.md](storage-design.md)
9. [query-model-mvp.md](query-model-mvp.md), [validation-rules.md](validation-rules.md), [event-model-mvp.md](event-model-mvp.md)
10. [cli-spec.md](cli-spec.md), [getting-started.md](getting-started.md)
11. [cli-mvp-roadmap.md](cli-mvp-roadmap.md)
12. [testing-strategy.md](testing-strategy.md), [development-workflow.md](development-workflow.md), [nonfunctional-requirements.md](nonfunctional-requirements.md)
13. [rfc-conformance.md](rfc-conformance.md)

---

# MVP Focus

The initial implementation focuses on:

- Artifacts
- Relationships
- Identity
- Repositories
- Queries
- Validation
- Storage
- Events (a minimal subset and `functor watch`, ADR-0003)

Excluded from the MVP:

- Federation
- MCP integration
- Extensions
- Versioning and history
- Full event streams (replay, subscriptions)
- Trust infrastructure
- Governance workflows
- Execution

The full list is in [mvp-scope.md](mvp-scope.md).

---

# Guiding Principles

## API First
All functionality is accessible through APIs; the CLI is a projection.

## Semantic Integrity
Implementation choices preserve the semantic model defined by the RFCs.

## Simplicity
Prefer simple implementations that validate the architecture.

## Traceability
Implementation decisions remain traceable to RFCs and ADRs.

## Determinism
The same input yields the same bytes, in files and in output.

## Incremental Evolution
Build the smallest useful implementation first.

---

# Long-Term Goal

The CLI MVP is not the final product. It is the first validation of the Functor platform. Future projections may include desktop applications, notebooks, web applications, MCP integrations, agent participation, and distributed repositories ([product-roadmap.md](product-roadmap.md)).
