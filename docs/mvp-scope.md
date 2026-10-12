# MVP Scope

## Purpose

This document defines the scope of the initial Functor CLI MVP.

The MVP exists to validate the core architectural foundations defined by the RFC series while minimizing implementation complexity.

## Included Scope

### Repository Management

Commands:

```bash
functor init
```

Capabilities:

- Create a repository
- Create repository metadata
- Create repository directory structure

### Artifact Management

Commands:

```bash
functor artifact create
functor artifact show
functor artifact list
```

Capabilities:

- Create artifacts
- Load artifacts
- List artifacts
- Display artifact metadata

### Relationship Management

Commands:

```bash
functor relation add
functor relation list
functor relation show
```

Supported relationship types:

- depends_on
- references
- derived_from

### Query Operations

Commands:

```bash
functor query
```

Minimum queries:

- List artifacts
- Show dependencies
- Show dependents
- Show relationships

### Validation

Commands:

```bash
functor validate
```

Validation rules:

- Missing identifiers
- Duplicate identifiers
- Missing relationship targets
- Invalid references
- Repository structure violations

### Storage

The MVP uses:

- Local filesystem
- TOML serialization

## Excluded Scope

The following are intentionally excluded from the MVP:

- Federation (RFC-0009)
- Dynamic capability discovery (RFC-0010)
- Non-CLI projections (RFC-0011)
- MCP integration (RFC-0012)
- Security and trust infrastructure (RFC-0013)
- Extensions and plugins (RFC-0014)
- Versioning (RFC-0016)
- Events and subscriptions (RFC-0017)
- Governance concerns (RFC-0018)

## MVP Success Criteria

A user should be able to execute:

```bash
functor init
functor artifact create rfc-0000
functor artifact create rfc-0001
functor relation add rfc-0001 depends_on rfc-0000
functor query dependencies rfc-0001
functor validate
```

and receive correct, reproducible results.

## Exit Criteria

The MVP phase is complete when:

- Repository operations work
- Artifact management works
- Relationship management works
- Queries work
- Validation works
- The CLI operates exclusively through the API layer
- Future projections can be built without modifying the core model
