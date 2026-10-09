# RFC 0001: Functor Notebook Language (FNL)

- **RFC:** 0001
- **Title:** Functor Notebook Language (FNL)
- **Status:** Draft
- **Version:** 0.1
- **Authors:** Functor Project

---

# Abstract

Functor Notebook Language (FNL) is a notebook-oriented orchestration language designed for the Functor ecosystem.

FNL separates:

- Notebook persistence
- Execution semantics
- Runtime implementations

into independent layers.

| Layer | Responsibility |
|---------|---------|
| TOML | Notebook graph container |
| FNL | Execution and orchestration DSL |
| Runtime | Rust, F#, Python, SQL, JavaScript, Bash, etc. |

This separation enables:

- Semantic editing
- Typed data flow
- Runtime independence
- Graph-based execution
- Incremental recomputation
- Long-term format stability

---

# Motivation

Most notebook systems couple storage, execution, and presentation into a single format.

Examples include:

- Jupyter notebooks stored as JSON
- Markdown-based notebooks
- Runtime-specific notebook systems

As notebooks grow larger and more collaborative, these approaches create challenges around:

- Version control
- Refactoring
- Graph analysis
- Multi-language orchestration
- Semantic tooling

Functor takes a different approach.

Instead of treating notebooks as ordered text documents, Functor models notebooks as a **typed execution graph**.

---

# Goals

The primary goals of FNL are:

- Explicit data flow
- Multi-runtime execution
- Typed notebook nodes
- Semantic editor tooling
- Reproducible execution graphs
- Independent evolution of storage and runtime systems

---

# Non-Goals

FNL is not:

- A replacement for Rust
- A replacement for Python
- A replacement for F#
- A replacement for SQL
- A general-purpose programming language

FNL exists solely to describe notebook structure, data flow, and execution orchestration.

---

# Architectural Overview

```text
┌─────────────────┐
│   Functor UI    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Notebook TOML   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│      FNL        │
│ Orchestration   │
└────────┬────────┘
         │
 ┌───────┼─────────┐
 ▼       ▼         ▼
Rust    Python     SQL
Node    Node      Node
```

---

# Design Principles

## 1. Storage Is Not Execution

Notebook persistence and execution semantics must remain separate concerns.

TOML stores notebook structure.

FNL describes orchestration and data flow.

Runtimes perform computation.

---

## 2. Cells Are Typed Nodes

A notebook is a graph of nodes.

It is not merely a collection of text blocks.

```text
Notebook
 ├─ Markdown Node
 ├─ Data Node
 ├─ Python Node
 ├─ Rust Node
 ├─ SQL Node
 ├─ Plot Node
 └─ Table Node
```

Each node type has explicit semantics.

---

## 3. Explicit Data Flow

Dependencies must be declared visibly.

Every consumer should identify its providers.

Example:

```fnl
input:
    sales <- load-data.sales
```

Rather than being hidden inside runtime code.

---

## 4. Runtime Independence

FNL does not execute code.

FNL orchestrates runtimes.

Supported runtimes may include:

- Rust
- F#
- Python
- SQL
- JavaScript
- Bash
- PowerShell

without modifying the core language specification.

---

# Notebook Container

The notebook file format SHALL be TOML.

Example:

```toml
version = "1"

title = "Customer Analysis"

[[cell]]
id = "load-data"
kind = "fnl"

content = """
python:
    ...
"""

[[cell]]
id = "aggregate"
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

The TOML layer is responsible for:

- Identity
- Metadata
- Persistence
- Ordering
- Workspace configuration

The TOML layer MUST NOT define execution semantics.

---

# Cell Model

Every notebook cell is represented as a graph node.

Minimum form:

```toml
[[cell]]
id = "aggregate"
kind = "fnl"
```

Extended form:

```toml
[[cell]]
id = "aggregate"

name = "Regional Totals"

kind = "fnl"

tags = [
    "sales",
    "finance"
]

depends_on = [
    "load-data"
]
```

---

# FNL Syntax

Inside an FNL node, orchestration is expressed declaratively.

Example:

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

The FNL layer does not describe how Rust executes.

It only defines:

- Inputs
- Outputs
- Execution boundaries
- Runtime selection

---

# Runtime Nodes

## Python Node

```fnl
python:

code:
    import pandas as pd

    sales = pd.read_csv("sales.csv")

output:
    sales
```

---

## Rust Node

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

## F# Node

```fnl
fsharp:

input:
    sales <- load-data.sales

code:
    let totals =
        sales
        |> DataFrame.groupBy "Region"
        |> DataFrame.sum "Revenue"

output:
    totals
```

---

## SQL Node

```fnl
sql:

input:
    sales <- load-data.sales

query:
    SELECT
        Region,
        SUM(Revenue)
    FROM sales
    GROUP BY Region

output:
    totals
```

---

# Inputs

Nodes may consume one or more named inputs.

```fnl
input:
    customers <- customer-query.customers
    orders <- order-query.orders
```

Inputs establish graph edges.

```text
customers ──┐
            ▼
         analyze
            ▲
orders ─────┘
```

---

# Outputs

Nodes publish named outputs.

```fnl
output:
    totals
    revenue
    customers
```

Consumers reference outputs using fully-qualified references.

```fnl
input:
    totals <- aggregate.totals
```

---

# Type Contracts

Outputs may declare optional types.

```fnl
output:
    totals : Table
```

Custom types may also be used.

```fnl
output:
    customer : Customer
```

Future implementations may perform:

- Validation
- Schema checking
- Compatibility analysis
- Compile-time diagnostics

---

# Markdown Nodes

Markdown is represented as a first-class node.

```toml
[[cell]]
id = "report"

kind = "markdown"

content = """
# Revenue Report
"""
```

Markdown nodes do not participate in computation.

---

# Plot Nodes

Plots are represented as semantic nodes.

```fnl
plot:

input:
    data <- aggregate.totals

x:
    region

y:
    revenue
```

Implementations may render charts using any compatible visualization backend.

---

# Table Nodes

```fnl
table:

input:
    data <- aggregate.totals
```

---

# Query Nodes

```fnl
query:

source:
    sqlserver

sql:
    SELECT *
    FROM Sales
```

Query nodes act as data providers.

---

# Graph Construction

The notebook execution graph is derived from input/output relationships.

Example:

```text
load-data
    │
    ▼
aggregate
  ┌─┴─┐
  ▼   ▼
plot report
```

Implementations SHOULD derive execution order automatically.

---

# Execution Model

Execution is dependency-driven.

Nodes SHALL execute according to data dependencies rather than document order.

Example:

```text
load-data
    │
    ▼
aggregate
    │
    ▼
plot
```

---

# Incremental Recalculation

Whenever a node changes:

1. The node becomes invalid.
2. Downstream dependents become invalid.
3. Unaffected nodes remain valid.

Example:

```text
change
  │
  ▼
load-data
  │
  ▼
aggregate
  │
  ▼
plot
```

This enables efficient notebook execution.

---

# Semantic Editing

Editors SHOULD parse FNL into a structured AST.

Example:

```fnl
input:
    sales <- load-data.sales
```

may be represented as:

```fsharp
InputBinding(
    Name = "sales",
    Source = OutputRef(
        Node = "load-data",
        Output = "sales"
    )
)
```

This enables:

- Rename support
- Navigation
- Dependency visualization
- Static validation
- Graph introspection

without understanding runtime-specific code.

---

# Future Extensions

## Remote Execution

```fnl
runtime:
    python

executor:
    remote.cluster
```

---

## LLM Nodes

```fnl
llm:

model:
    phi-4

prompt:
    Summarize sales performance.

input:
    data <- aggregate.totals

output:
    summary
```

---

## Agent Nodes

```fnl
agent:

endpoint:
    crm-agent

input:
    customer <- lookup.customer

output:
    recommendation
```

---

## MCP Integration

```fnl
mcp:

server:
    github

tool:
    search_repositories
```

---

## A2A Integration

```fnl
agent:

protocol:
    a2a

target:
    analytics-agent
```

---

# Security Considerations

Implementations SHOULD:

- Sandbox runtime execution
- Validate graph integrity
- Enforce runtime permissions
- Isolate notebook state
- Audit external interactions

---

# Summary

Functor adopts a layered notebook architecture:

```text
TOML
  ↓
Notebook Structure

FNL
  ↓
Execution Graph

Runtime
  ↓
Computation
```

Responsibilities remain clearly separated:

| Layer | Responsibility |
|---------|---------|
| TOML | Storage and metadata |
| FNL | Orchestration and data flow |
| Runtimes | Computation |
| Functor Editor | Semantic editing and visualization |

This architecture positions FNL as a typed orchestration layer for graph-native notebooks, enabling rich editor capabilities, language interoperability, and long-term evolution without coupling notebook structure to execution technology.
