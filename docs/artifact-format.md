# Artifact Format

## Status

Draft

## Purpose

This document defines the canonical artifact serialization format for the Functor CLI MVP.

The format is designed to be:

- Human-readable
- Version control friendly
- Deterministic
- Extensible
- API-first

TOML is the default artifact representation for the MVP.

---

## Design Principles

1. Artifact identity is stable.
2. Artifact content is versionable.
3. Metadata is explicit.
4. Relationships are external to artifacts.
5. Projections must not alter semantics.

---

## Minimum Artifact

```toml
id = "rfc-0000"
kind = "document"
title = "Functor Vision"
```

---

## Standard Fields

```toml
id = "rfc-0000"
kind = "document"
title = "Functor Vision"
description = "Platform vision and goals"
created = "2026-10-12T18:00:00Z"
updated = "2026-10-12T18:00:00Z"
```

### Required

- id
- kind
- title

### Optional

- description
- created
- updated
- tags
- metadata

---

## Metadata

```toml
[metadata]
author = "Paulo"
status = "draft"
```

Implementations may add metadata without breaking compatibility.

---

## Tags

```toml
tags = [
  "rfc",
  "architecture",
  "core"
]
```

---

## Content Section

```toml
[content]
format = "markdown"
path = "./content/rfc-0000.md"
```

Content may be embedded or referenced.

---

## Artifact Kinds

Suggested kinds:

- document
- rfc
- note
- specification
- dataset
- workflow
- validation
- execution

Implementations may define additional kinds.

---

## Identity Rules

Artifact identifiers must:

- Be unique within a repository
- Be stable over time
- Be human-readable where practical

Example:

```toml
id = "rfc-0017"
```

---

## Validation Rules

An artifact is valid when:

- Required fields exist
- Identifier is unique
- TOML is well-formed

---

## Future Work

Future RFCs may define:

- Embedded content formats
- Binary artifact references
- Schema validation
- Cross-repository references
- Artifact signing
