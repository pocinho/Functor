# Command Design

This document has been superseded by [cli-spec.md](cli-spec.md), which defines the exact command surface of the CLI MVP.

The earlier draft described commands outside the MVP (`graph`, `runtime`, `protocol`, `agent`, `doctor`, `status`, `config`, `artifact delete`). They are deferred; see "Compatibility With Earlier Drafts" in [cli-spec.md](cli-spec.md). The design principles carried over unchanged:

- API first and projection independent
- Human friendly and agent friendly
- Scriptable with deterministic output
- The CLI stays a thin client over the Functor API: new capabilities are added to the core API first, then projected into commands
