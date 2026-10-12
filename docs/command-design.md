# Command Design

## Purpose

The Functor CLI is the primary reference client for the Functor protocol. It exposes artifact creation, discovery, validation, execution, and evolution through a consistent command model.

## Design Principles

- API-first
- Projection-independent
- Human-friendly
- Agent-friendly
- Scriptable by default
- Deterministic output

## Command Structure

```text
functor <resource> <action> [options]
```

Examples:

```bash
functor artifact create
functor artifact validate
functor graph inspect
functor runtime run
functor protocol export
```

## Core Resources

### Artifact

Represents a persistent unit of knowledge, reasoning, computation, or documentation.

```bash
functor artifact create
functor artifact show <id>
functor artifact list
functor artifact update <id>
functor artifact delete <id>
```

### Graph

Manages relationships between artifacts.

```bash
functor graph inspect
functor graph visualize
functor graph validate
```

### Runtime

Executes computations and validations.

```bash
functor runtime run
functor runtime validate
functor runtime explain
```

### Protocol

Imports and exports protocol representations.

```bash
functor protocol export
functor protocol import
functor protocol schema
```

## Workspace Commands

```bash
functor init
functor doctor
functor status
functor config show
```

## Agent Operations

```bash
functor agent list
functor agent inspect <id>
functor agent invoke <id>
```

## Output Modes

```bash
--output text
--output json
--output toml
```

## Long-Term Direction

The CLI should remain a thin client over the Functor API. New capabilities should be added to the protocol and runtime first, then projected through CLI commands.
