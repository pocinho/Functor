# Repository Layout

## Purpose

This document defines the canonical on-disk layout of a Functor repository.

The repository layout provides a durable storage structure for artifacts,
relationships, validations, events, and repository metadata while remaining
independent from any specific projection such as a CLI, editor, web UI, or MCP.

Related RFCs:

- RFC-0008 Storage and Persistence
- RFC-0009 Federation and Synchronization
- RFC-0015 Repository Model
- RFC-0017 Event Model

---

## Design Principles

- Human-readable where practical
- Stable repository structure
- Tool-friendly organization
- Projection-independent storage
- Portable across platforms
- Version-control friendly

---

## Repository Root

A Functor repository is identified by the presence of a `.functor` directory.

Example:

```text
my-project/
├── .functor/
├── artifacts/
├── relationships/
├── validations/
├── events/
└── README.md
```

---

## .functor Directory

The `.functor` directory contains repository metadata.

```text
.functor/
├── repository.toml
├── capabilities.toml
├── extensions.toml
└── settings.toml
```

### repository.toml

Repository identity and metadata.

Example:

```toml
id = "repo-functor-core"
name = "Functor Core"
version = "1.0"
```

---

## Artifacts

Artifacts are stored beneath the `artifacts/` directory.

```text
artifacts/
├── rfc-0000.toml
├── rfc-0001.toml
└── example.toml
```

Artifact format is defined in `artifact-format.md`.

---

## Relationships

Relationships are stored independently from artifacts.

```text
relationships/
├── relationships.toml
└── indexes/
```

Example:

```toml
[[relationship]]
source = "rfc-0001"
type = "depends_on"
target = "rfc-0000"
```

---

## Validations

Validation results MAY be persisted.

```text
validations/
├── latest/
└── history/
```

Example:

```text
validations/
└── history/
    └── validation-2026-10-12.toml
```

---

## Events

Repositories MAY persist event streams.

```text
events/
├── current.log
└── archive/
```

Events are informational records and are not authoritative state.

---

## Extensions

Extensions MAY store data within a dedicated namespace.

```text
.functor/
extensions/
    my-extension/
```

Extension data MUST NOT modify core repository structures.

---

## Future Directories

Future RFCs may introduce:

```text
indexes/
cache/
executions/
snapshots/
exports/
```

Repositories SHOULD tolerate the existence of additional directories.

---

## Recommended MVP Layout

```text
my-repository/
├── .functor/
│   └── repository.toml
├── artifacts/
├── relationships/
├── validations/
└── events/
```

This structure is sufficient for the CLI MVP while remaining compatible with future RFCs.
