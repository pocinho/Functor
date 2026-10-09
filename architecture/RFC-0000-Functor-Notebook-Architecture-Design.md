# RFC-0000: Functor Notebook Architecture & Design

- **RFC:** 0000
- **Title:** Functor Notebook Architecture & Design
- **Status:** Draft
- **Version:** 2.0
- **Authors:** Functor Project
- **Supersedes:** Early FNL Draft Specification

---

# Executive Summary

Functor is a graph-native notebook platform built around a strict separation of concerns:

| Layer | Responsibility |
|---------|---------|
| TOML | Notebook persistence and metadata |
| FNL | Typed orchestration and graph definition |
| Runtime Engines | Rust, F#, Python, SQL, JavaScript, etc. |
| Functor Editor | Semantic editing, visualization, and execution experience |

This architecture treats notebooks as **typed execution graphs** rather than ordered collections of text cells.

The notebook file is persisted as TOML.

The execution graph is described using FNL.

Computation is performed by runtime engines.

---

# Vision

Functor aims to become:

> A semantic, graph-native, multi-runtime notebook platform where notebooks behave more like distributed software systems than text documents.

Unlike traditional notebooks, Functor treats:

- code
- data
- visualizations
- AI interactions
- agents
- queries

as first-class graph nodes.

---

# Core Design Philosophy

## Storage Is Not Execution

Notebook persistence should not dictate execution semantics.

Execution semantics should not dictate runtime implementation.

These concerns evolve independently.

```text
Notebook Storage
       │
       ▼
     FNL
       │
       ▼
 Runtime Engines
```

---

## Notebooks Are Graphs

Traditional notebooks:

```text
Cell 1
Cell 2
Cell 3
Cell 4
```

Functor notebooks:

```text
Load Data
     │
     ▼
Transform
  ┌──┴──┐
  ▼     ▼
Plot  Report
```

Execution order emerges from dependencies.

It is not determined solely by position in the document.

---

## Cells Are Typed Runtime Nodes

Functor treats each cell as a semantic node.

Examples:

```text
Markdown
Python
Rust
F#
SQL
Plot
Table
LLM
Agent
Query
API
```

Every node type has explicitly defined behavior.

---

## Data Flow Is Explicit

Dependencies should be visible and inspectable.

Consumers declare inputs.

Producers declare outputs.

No hidden state.

---

# System Architecture

```text
┌─────────────────────────┐
│      Functor Editor     │
└─────────────┬───────────┘
              │
              ▼
┌─────────────────────────┐
│       Notebook TOML     │
└─────────────┬───────────┘
              │
              ▼
┌─────────────────────────┐
│           FNL           │
│  Graph & Orchestration  │
└─────────────┬───────────┘
              │
   ┌──────────┼──────────┐
   ▼          ▼          ▼
 Rust       Python      SQL
 Engine     Engine     Engine
```

---

# Layer Definitions

## Layer 1: Notebook Container (TOML)

TOML is the persistent notebook format.

The container is responsible for:

- Cell identity
- Metadata
- Ordering
- Workspace settings
- Presentation settings
- Persistence

The container is **not responsible** for execution semantics.

---

### Example Notebook

```toml
version = "1"

title = "Revenue Analysis"

[workspace]
theme = "dark"

[[cell]]
id = "load-data"
kind = "fnl"

content = """
python:
    ...
"""

[[cell]]
id = "analyze"
kind = "fnl"

content = """
rust:
    ...
"""

[[cell]]
id = "report"
kind = "markdown"

content = """
# Results
"""
```

---

## Layer 2: FNL

Functor Notebook Language (FNL) is the orchestration layer.

FNL describes:

- Graph edges
- Runtime selection
- Inputs
- Outputs
- Contracts
- Execution boundaries

FNL does not perform computation.

---

### Example

```fnl
rust:

input:
    sales <- load-data.sales

code:
    let totals =
        sales
            .group_by("region")
            .sum("revenue");

output:
    totals
```

---

## Layer 3: Runtime Engines

Runtime engines perform actual execution.

A runtime may be:

- Rust
- F#
- Python
- SQL
- JavaScript
- Bash
- PowerShell

Future implementations may add runtimes without modifying the FNL specification.

---

# Notebook Model

A notebook is a directed acyclic graph (DAG).

```text
Notebook
    │
    ▼
  Graph
    │
 ┌──┴───┐
 ▼      ▼
Node  Node
```

---

## Graph Node

Every notebook cell becomes a graph node.

```text
Node
 ├─ Metadata
 ├─ Runtime
 ├─ Inputs
 ├─ Outputs
 └─ Body
```

---

# Node Taxonomy

## Runtime Nodes

Nodes that execute code.

### Python Node

```fnl
python:

code:
    import pandas as pd

    sales = pd.read_csv("sales.csv")

output:
    sales
```

---

### Rust Node

```fnl
rust:

input:
    sales <- load.sales

code:
    let totals =
        sales
            .group_by("region")
            .sum("revenue");

output:
    totals
```

---

### F# Node

```fnl
fsharp:

input:
    sales <- load.sales

code:
    let totals =
        sales
        |> aggregate

output:
    totals
```

---

### SQL Node

```fnl
sql:

input:
    sales <- load.sales

query:
    SELECT *
    FROM sales

output:
    result
```

---

# Semantic Nodes

Semantic nodes describe intent rather than implementation.

---

## Plot Node

```fnl
plot:

input:
    data <- aggregate.totals

x:
    region

y:
    revenue
```

---

## Table Node

```fnl
table:

input:
    data <- aggregate.totals
```

---

## Markdown Node

```markdown
# Revenue Report
```

---

## Query Node

```fnl
query:

source:
    postgres

sql:
    SELECT *
    FROM sales
```

---

# Data Model

## Inputs

Inputs create graph edges.

```fnl
input:
    customers <- customers.results
    orders <- orders.results
```

Graph:

```text
customers ──┐
            ▼
         analyze
            ▲
orders ─────┘
```

---

## Outputs

Outputs publish values.

```fnl
output:
    totals
    customers
```

Consumers reference outputs.

```fnl
input:
    totals <- aggregate.totals
```

---

# Type System

Functor supports optional type contracts.

## Primitive Contracts

```fnl
output:
    count : Integer
```

```fnl
output:
    revenue : Decimal
```

---

## Structured Contracts

```fnl
output:
    customer : Customer
```

```fnl
output:
    sales : Table
```

---

## Future Type Validation

Future engines may provide:

- Schema validation
- Type inference
- Compatibility checks
- Static diagnostics

---

# Execution Model

Functor executes the notebook graph.

Not the notebook text.

---

## Dependency Resolution

The engine constructs a DAG.

```text
load
 │
 ▼
aggregate
 │
 ▼
plot
```

---

## Scheduling

Execution order is derived automatically.

Nodes execute when dependencies are satisfied.

---

## Parallel Execution

Independent branches may run concurrently.

```text
        load
          │
     ┌────┴────┐
     ▼         ▼
 aggregate   analyze
     ▼         ▼
   plot     summary
```

---

# Incremental Recomputation

Functor tracks dependency invalidation.

When a node changes:

```text
data source
     │
     ▼
 aggregate
     │
     ▼
   report
```

Only affected downstream nodes are recomputed.

Unaffected branches remain valid.

---

# Semantic Editor Model

Functor Editor operates on a graph AST rather than raw text.

---

## Example

FNL:

```fnl
input:
    sales <- load-data.sales
```

AST:

```fsharp
InputBinding(
    Name = "sales",
    Source = OutputRef(
        Node = "load-data",
        Output = "sales"
    )
)
```

---

## Editor Features

Because the graph is explicit, the editor can provide:

- Dependency visualization
- Rename refactoring
- Graph navigation
- Static validation
- Topology inspection
- Intelligent execution

without parsing runtime-specific source code.

---

# AI & Agent Integration

Functor treats AI interactions as graph nodes.

---

## LLM Node

```fnl
llm:

model:
    phi-4

prompt:
    Summarize revenue trends.

input:
    data <- aggregate.totals

output:
    summary
```

---

## Agent Node

```fnl
agent:

endpoint:
    analytics-agent

input:
    totals <- aggregate.totals

output:
    recommendation
```

---

## MCP Node

```fnl
mcp:

server:
    github

tool:
    search_repositories
```

---

## A2A Node

```fnl
agent:

protocol:
    a2a

target:
    reporting-agent
```

---

# Future Roadmap

Potential future capabilities include:

- Distributed execution
- Remote executors
- Cluster scheduling
- Live collaboration
- Graph versioning
- Visual graph editing
- Asset registries
- Runtime plug-ins
- MCP-native workflows
- Agentic notebook automation

---

# Security Principles

Implementations should provide:

- Node sandboxing
- Runtime isolation
- Permission boundaries
- Secret management
- Execution auditing
- Dependency validation

---

# Key Architectural Decision

Functor formally adopts the following separation:

```text
TOML
    ↓
Notebook Persistence

FNL
    ↓
Execution Graph

Runtime Engines
    ↓
Computation

Functor Editor
    ↓
Semantic User Experience
```

This separation allows the notebook format, orchestration language, editor experience, and runtime technologies to evolve independently while remaining interoperable.

---

# Conclusion

Functor is not designed as another notebook file format.

Functor is a graph-native computing platform.

Its architecture is based on four foundational principles:

1. Notebooks are graphs.
2. FNL orchestrates computation.
3. TOML persists notebook structure.
4. Runtime engines perform execution.

Together these principles enable a semantic, typed, multi-runtime environment that scales from simple exploratory notebooks to agent-driven and distributed computational workflows.
