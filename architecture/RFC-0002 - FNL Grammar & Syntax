# RFC-0002: FNL Grammar & Syntax

- **RFC:** 0002
- **Title:** FNL Grammar & Syntax
- **Status:** Draft
- **Version:** 0.1
- **Depends On:** RFC-0001 Functor Notebook Architecture & Design

---

# Abstract

This RFC defines the grammar, syntax, and parsing model of the Functor Notebook Language (FNL).

FNL is a declarative orchestration language used within Functor notebooks to describe:

- Runtime execution
- Data flow
- Dependencies
- Outputs
- Contracts
- Semantic nodes

FNL is not intended to be a general-purpose programming language.

Instead, FNL acts as a graph description language that coordinates computation performed by external runtimes.

---

# Goals

The syntax must be:

- Human-readable
- Git-friendly
- Structured
- Easily parsed
- Incrementally parsable
- Runtime agnostic

The grammar should favor clarity over compactness.

---

# Design Philosophy

FNL should read like a notebook graph.

Example:

```fnl
rust:

input:
    sales <- load.sales

code:
    let totals =
        aggregate(sales);

output:
    totals
```

A reader should immediately see:

- What runtime executes.
- What data enters.
- What data leaves.
- How the node participates in the graph.

---

# Language Structure

An FNL document describes exactly one node.

General form:

```fnl
<node-type>:

<section>:
    ...

<section>:
    ...
```

Example:

```fnl
python:

input:
    sales <- load.sales

code:
    ...

output:
    totals
```

---

# Grammar Overview

```ebnf
Node
    = NodeHeader
      Section* ;

NodeHeader
    = Identifier ":" ;

Section
    = Identifier ":"
      SectionBody ;

SectionBody
    = IndentedBlock ;

Identifier
    = Letter
      { Letter | Digit | "-" | "_" } ;
```

---

# Node Header

Every FNL document begins with a node header.

Examples:

```fnl
python:
```

```fnl
rust:
```

```fnl
plot:
```

```fnl
llm:
```

The node header determines semantic behavior.

---

# Runtime Nodes

Runtime nodes execute code.

Built-in runtime names:

```text
python
rust
fsharp
sql
javascript
bash
powershell
```

Example:

```fnl
fsharp:

code:
    let answer = 42

output:
    answer
```

---

# Semantic Nodes

Some nodes describe behavior instead of executing source code.

Examples:

```fnl
plot:
```

```fnl
table:
```

```fnl
markdown:
```

```fnl
llm:
```

```fnl
agent:
```

---

# Sections

Sections define node properties.

Example:

```fnl
python:

input:
    sales <- load.sales

code:
    ...

output:
    result
```

The grammar is structured as:

```text
Node
 ├─ input
 ├─ code
 ├─ output
 └─ metadata
```

---

# Input Section

The `input` section defines graph dependencies.

Syntax:

```fnl
input:
    local-name <- source-node.output-name
```

Example:

```fnl
input:
    sales <- load-data.sales
```

---

## Multiple Inputs

```fnl
input:
    customers <- customer-query.customers
    orders <- order-query.orders
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

# Output Section

Outputs publish values.

Syntax:

```fnl
output:
    result
```

Multiple outputs:

```fnl
output:
    customers
    orders
    revenue
```

---

# Typed Outputs

Outputs may declare contracts.

Syntax:

```fnl
output:
    sales : Table
```

Example:

```fnl
output:
    customer : Customer
```

---

# Code Section

The `code` section contains runtime-specific source code.

Example:

```fnl
python:

code:
    import pandas as pd

    sales = pd.read_csv(
        "sales.csv"
    )

output:
    sales
```

The FNL parser does not interpret runtime code.

It treats the contents as an opaque block.

---

# Query Section

Used by query-oriented nodes.

Example:

```fnl
sql:

query:
    SELECT *
    FROM sales

output:
    result
```

---

# Configuration Sections

Simple key-value sections may be defined.

Example:

```fnl
plot:

x:
    region

y:
    revenue
```

---

# Metadata Section

Optional notebook metadata.

Example:

```fnl
metadata:
    title = "Revenue Summary"
    owner = "Finance Team"
```

---

# References

Nodes reference outputs using fully-qualified paths.

Syntax:

```text
node.output
```

Example:

```fnl
input:
    totals <- aggregate.totals
```

---

# Qualified References

Future versions may support deeper references.

Example:

```fnl
input:
    city <- customer.address.city
```

Reference grammar:

```ebnf
Reference =
    Identifier
    { "." Identifier } ;
```

---

# Literals

FNL supports a minimal literal system.

---

## String

```fnl
title:
    "Revenue Report"
```

---

## Integer

```fnl
rows:
    1000
```

---

## Decimal

```fnl
threshold:
    0.85
```

---

## Boolean

```fnl
enabled:
    true
```

```fnl
enabled:
    false
```

---

# Lists

Lists are newline-oriented.

Example:

```fnl
tags:
    finance
    sales
    monthly
```

Equivalent AST:

```json
[
  "finance",
  "sales",
  "monthly"
]
```

---

# Objects

Nested objects are represented through indentation.

Example:

```fnl
runtime:
    executor:
        cluster
```

Equivalent AST:

```json
{
  "runtime": {
    "executor": "cluster"
  }
}
```

---

# Plot Node Grammar

Example:

```fnl
plot:

input:
    data <- aggregate.totals

x:
    region

y:
    revenue
```

AST:

```text
PlotNode
 ├─ Input
 ├─ X
 └─ Y
```

---

# Table Node Grammar

Example:

```fnl
table:

input:
    data <- aggregate.totals
```

---

# Markdown Node Grammar

Example:

```fnl
markdown:

content:
    # Revenue
```

---

# LLM Node Grammar

Example:

```fnl
llm:

model:
    phi-4

prompt:
    Summarize sales data.

input:
    data <- aggregate.totals

output:
    summary
```

---

# Agent Node Grammar

Example:

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

# Comments

Single-line comments begin with `#`.

Example:

```fnl
# Load revenue CSV
```

Inline comments:

```fnl
rows:
    1000 # sample size
```

Comments do not affect semantics.

---

# Whitespace Rules

FNL is indentation-sensitive.

Implementations SHOULD standardize on:

```text
4 spaces
```

Tabs SHOULD be rejected.

---

# Parsing Model

FNL is designed for incremental parsing.

Editor workflows should support:

- Structural parsing
- Partial parsing
- Live validation
- AST generation

without executing runtime code.

---

# Abstract Syntax Tree

Example:

```fnl
rust:

input:
    sales <- load.sales

output:
    totals
```

AST:

```text
Node
 ├─ Runtime(Rust)
 ├─ Inputs
 │    └─ Binding
 └─ Outputs
      └─ totals
```

---

# Validation Rules

A parser SHOULD validate:

- duplicate outputs
- duplicate inputs
- invalid references
- unresolved dependencies
- malformed sections
- cyclic references

A parser MUST NOT validate runtime-specific syntax.

That responsibility belongs to runtime engines.

---

# Reserved Keywords

The following keywords are reserved:

```text
input
output
code
query
metadata
runtime
executor
model
prompt
endpoint
content
```

Future RFCs may expand this list.

---

# Future Extensions

Potential future additions:

- Generic types
- Schema definitions
- Pattern matching
- Node templates
- Imports
- Reusable graph fragments
- Multi-output contracts
- Versioned types
- Visual node metadata

These extensions are intentionally excluded from the initial grammar.

---

# Example

Complete example:

```fnl
python:

input:
    customers <- load.customers

code:
    active =
        customers[
            customers.active == True
        ]

output:
    active : Table
```

Graph:

```text
load
 │
 ▼
python-node
 │
 ▼
active
```

---

# Conclusion

FNL provides a minimal, declarative grammar focused on graph orchestration rather than computation.

Core principles:

1. One FNL document represents one graph node.
2. Inputs create graph edges.
3. Outputs publish graph values.
4. Runtime code remains opaque to FNL.
5. Execution semantics remain independent from persistence.
6. Semantic tooling is enabled through a stable, structured AST.

Together with RFC-0001, this establishes the foundation for a graph-native, multi-runtime notebook platform where TOML handles persistence and FNL defines executable notebook topology.
