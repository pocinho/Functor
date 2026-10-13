# Functor

> **What if understanding itself were a first-class computational resource?**

Functor is an API-first platform for creating, relating, validating, and evolving structured artifacts.

Rather than centering software around files, documents, notebooks, or isolated computations, Functor provides a semantic foundation for expressing relationships between artifacts, computations, validations, and knowledge.

Functor is not defined by a notebook, editor, CLI, protocol, or application.

These are projections over a common semantic core.

The long-term goal is to make understanding more durable, traceable, collaborative, and computationally accessible.

---

# Vision

Functor is built upon a simple belief:

> Understanding is one of the most important resources intelligent beings create, share, and evolve.

Modern tools allow us to:

- Create information
- Store information
- Search information
- Execute computations

Yet much of our understanding remains implicit.

Relationships are hidden.

Assumptions are undocumented.

Evidence becomes disconnected from conclusions.

Knowledge is fragmented across tools and systems.

Functor explores an alternative.

A world in which understanding itself becomes representable, queryable, executable, validatable, and shareable.

---

# Core Idea

Functor is built around a semantic model composed of:

```text
Repositories
       ↓

Artifacts
       ↓

Relationships
       ↓

Queries
       ↓

Validation
       ↓

Execution
```

These concepts form the foundation of the platform.

Applications, interfaces, and integrations build upon them.

---

# Architectural Principles

Functor follows a small set of enduring principles.

## Semantics Before Implementation

Meaning should be defined before storage, transport, or user interfaces.

```text
Meaning
    ↓
Contracts
    ↓
Implementation
```

---

## API First

Functor is defined by APIs and semantic contracts.

User interfaces are consumers of those contracts.

```text
Semantic Model
        ↓
API
        ↓
Projections
```

---

## Projection-Based Architecture

A CLI, notebook, desktop application, web application, or intelligent agent are all projections over the same underlying model.

```text
Functor Core

    ├── CLI
    ├── Desktop
    ├── Web
    ├── Notebook
    ├── MCP
    └── Future Interfaces
```

No projection is considered primary.

---

## Extensible by Design

Functor should evolve through extension rather than modification.

A small durable core is preferred over a large specialized platform.

---

## Validation Matters

Claims should be challengeable.

Results should be reproducible.

Relationships should be inspectable.

The platform should make reasoning easier to understand and evaluate.

---

# Repository Structure

The repository is organized into distinct layers.

```text
vision/
    Why Functor exists

rfcs/
    What Functor is

docs/
    How Functor is implemented

src/
    The implementation
```

---

# vision/

The vision documents explore the long-term aspirations of the project.

Current documents include:

```text
Semantic-Workspace-Protocol.md

Principles.md

Future-of-Understanding.md

Human-and-Intelligence-Collaboration.md
```

These documents provide direction and inspiration.

They are not normative specifications.

---

# rfcs/

The RFC series defines the platform.

The foundational RFC collection currently includes:

```text
RFC-0000  Vision

RFC-0001  Core Semantic Model
RFC-0002  API-First Architecture
RFC-0003  Artifact Model
RFC-0004  Relationship Model
RFC-0005  Identity and Addressing
RFC-0006  Query Model
RFC-0007  Validation and Execution
RFC-0008  Storage and Persistence
RFC-0009  Federation and Synchronization

RFC-0010  Capability Model
RFC-0011  Projection Model
RFC-0012  Interoperability and Integration
RFC-0013  Security and Trust Model

RFC-0014  Extension Model
RFC-0015  Repository Model
RFC-0016  Versioning Model
RFC-0017  Event Model
RFC-0018  Governance Model
```

Together these documents define the semantic and architectural foundation of Functor.

---

# docs/

The documentation directory bridges the gap between architecture and implementation.

Start with [docs/README.md](docs/README.md) for the full index and reading order. Examples include:

```text
mvp-scope.md
cli-mvp-roadmap.md
cli-spec.md

domain-model.md
identity-and-addressing.md
artifact-format.md
repository-layout.md
storage-design.md

query-model-mvp.md
validation-rules.md
event-model-mvp.md

workspace-layout.md
crate-dependencies.md
traits-and-interfaces.md
api-design-guidelines.md
error-handling.md

testing-strategy.md
rfc-conformance.md
adr/
```

These documents are intended to support implementation.

---

# Current Focus

The first objective, a CLI MVP, is implemented (Windows is the supported platform; see [docs/nonfunctional-requirements.md](docs/nonfunctional-requirements.md)).

The MVP exists to validate:

- Repository management
- Artifact management
- Relationships
- Queries
- Validation
- Persistence
- API-first architecture

The goal is architectural validation rather than feature completeness.

## Try it

```bash
cargo build --release
functor init --name "Functor Demo"
functor artifact create rfc-0000 --type rfc --title "Functor Vision"
functor artifact create rfc-0001 --type rfc --title "Core Semantic Model"
functor relation add rfc-0001 depends_on rfc-0000
functor query dependencies rfc-0001
functor validate
```

The full walkthrough, with the output of every step, is [docs/getting-started.md](docs/getting-started.md); the repository it produces is [examples/getting-started/](examples/getting-started/). Contributors run `pwsh scripts/ci.ps1` to execute the blocking checks locally ([docs/development-workflow.md](docs/development-workflow.md)).

---

# Why Repositories?

Functor treats repositories as bounded semantic workspaces.

A repository contains:

```text
Artifacts
Relationships
Metadata
Validations
Events
Configuration
```

Applications work against repositories rather than isolated files.

This provides:

- Context
- Traceability
- Ownership boundaries
- Shared understanding
- Future collaboration capabilities

---

# Beyond Notebooks

Functor originally explored concepts familiar from notebook systems.

Over time it evolved into something broader.

A notebook is now viewed as a projection rather than the center of the system.

```text
Repository
    └── Artifact Graph
            ├── Documents
            ├── Computations
            ├── Datasets
            ├── Visualizations
            └── Specifications
```

A notebook may become one way to view that graph.

It is no longer the graph itself.

---

# Human and Intelligence Collaboration

Functor is designed for collaboration between humans and intelligences.

Humans contribute:

- Intent
- Creativity
- Judgment
- Values
- Experience

Intelligences contribute:

- Analysis
- Synthesis
- Retrieval
- Transformation
- Exploration

Artifacts provide a common medium through which both can participate.

The platform seeks to improve collaboration rather than replace participants.

---

# Status

Functor is currently in the foundation-building stage.

The semantic model, RFCs and implementation guidance are established, and the CLI MVP validates them end to end: its exit criteria and the contracts it freezes are in [docs/mvp-scope.md](docs/mvp-scope.md), and how it conforms to each RFC is in [docs/rfc-conformance.md](docs/rfc-conformance.md). The crates are not published (`publish = false`).

Future work may include:

- Desktop applications
- Notebook projections
- Web applications
- MCP integration
- Federated repositories
- Extension ecosystems
- Collaborative semantic workspaces

Each of these will build upon the same semantic foundation.

---

# Guiding Question

Every major decision should be evaluated against a simple question:

> Does this improve the ability of intelligent beings to create, share, validate, evolve, or communicate understanding?

If the answer is yes, it is likely aligned with the purpose of Functor.

---

# License

MIT License

---

# Closing Thought

Functor began as an exploration of notebooks, artifacts, and computation.

It has evolved into a broader exploration:

> What happens when understanding itself becomes a first-class computational resource?

This repository is an attempt to answer that question.