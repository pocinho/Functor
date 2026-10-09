# RFC-0003: FNL Type System & Data Contracts

- **RFC:** 0003
- **Title:** FNL Type System & Data Contracts
- **Status:** Draft
- **Version:** 0.1
- **Depends On:** RFC-0001, RFC-0002

---

# Abstract

This RFC defines the Functor type system.

Functor distinguishes between:

1. Data Types
2. Behavioral Capability Types

Data Types define the shape of values flowing through the graph.

Behavioral Capability Types define the operations a node can perform.

This separation enables:

- Runtime interoperability
- Static validation
- Semantic tooling
- Capability discovery
- Cross-language graph composition

---

# Design Philosophy

Traditional systems often model only data.

Example:

```text
Table
String
Image
Json
```

Functor additionally models behavior.

Example:

```text
Queryable
Summarizable
Visualizable
Embeddable
Searchable
AgentCallable
```

A value is defined by:

```text
Data Shape
+
Behavioral Capabilities
```

---

# Type Categories

Functor defines two orthogonal type domains.

```text
Type
├── Data Type
└── Capability Type
```

---

# Data Types

Data Types define structure.

---

## Primitive Types

```text
String
Boolean
Integer
Float
Decimal
Date
DateTime
Duration
Guid
Binary
```

Example:

```fnl
output:
    customer_id : Guid
```

---

## Collection Types

```text
List<T>
Set<T>
Map<TKey,TValue>
```

Example:

```fnl
output:
    customers : List<Customer>
```

---

## Structured Types

```fnl
type Customer
```

Example:

```fnl
output:
    customer : Customer
```

---

## Table Type

Tabular data is a first-class concept.

```fnl
output:
    sales : Table
```

Optional schema:

```fnl
output:
    sales : Table<
        Region : String,
        Revenue : Decimal
    >
```

---

## Dataset Type

Large distributed datasets.

```fnl
output:
    telemetry : Dataset
```

---

## Stream Type

Continuous data flow.

```fnl
output:
    events : Stream<Event>
```

---

# Capability Types

Capability Types describe behavior.

Capabilities are independent of runtime.

---

# Why Capabilities Exist

A Python dataframe and a SQL result set may have different implementations.

However both may support:

```text
Queryable
Filterable
Groupable
Sortable
```

Functor reasons about capabilities instead of implementation details.

---

# Capability Declaration

Syntax:

```fnl
output:
    sales :
        Table
        + Queryable
        + Visualizable
```

---

Equivalent form:

```fnl
capabilities:
    Queryable
    Visualizable
```

---

# Standard Capability Types

---

## Queryable

May participate in query operations.

Example consumers:

```text
SQL Node
Filter Node
Join Node
```

---

## Filterable

Supports filtering operations.

```text
input
  │
  ▼
Filter Node
```

---

## Sortable

Supports ordering operations.

---

## Groupable

Supports aggregation.

---

## Joinable

Supports joins.

---

## Aggregatable

Supports aggregate operations.

Example:

```text
SUM
AVG
COUNT
MIN
MAX
```

---

# Data Science Capabilities

---

## Visualizable

Can be consumed by visualization nodes.

Example:

```fnl
plot:
```

requires:

```text
Visualizable
```

---

## Statistical

Supports statistical analysis.

Example:

```fnl
statistics:
```

---

## Embeddable

Can generate vector embeddings.

Example:

```fnl
embeddings:
```

---

## Searchable

Supports semantic retrieval.

---

# AI Capabilities

---

## Summarizable

Can be summarized by LLM nodes.

Example:

```fnl
llm:
```

---

## Classifiable

Can be classified.

---

## Extractable

Supports information extraction.

---

## Translatable

Supports language transformation.

---

## Annotatable

Supports tagging and enrichment.

---

# Agent Capabilities

---

## AgentCallable

Can be invoked by agents.

---

## ToolCallable

Can be exposed as an MCP tool.

---

## WorkflowCallable

Can participate in workflow graphs.

---

## EventCallable

Can subscribe to events.

---

# Infrastructure Capabilities

---

## Persistable

Can be stored.

---

## Cacheable

Can be cached.

---

## Versionable

Supports version tracking.

---

## Replicable

Supports distributed copies.

---

# Capability Inference

Capabilities may be explicit.

```fnl
output:
    sales :
        Table
        + Queryable
        + Visualizable
```

or inferred.

Example:

```text
Table
```

automatically implies:

```text
Queryable
Filterable
Sortable
Groupable
```

---

# Capability Hierarchy

Capabilities may inherit.

Example:

```text
Queryable
 ├─ Filterable
 ├─ Sortable
 └─ Joinable
```

Another example:

```text
Visualizable
 ├─ Chartable
 ├─ Mappable
 └─ Dashboardable
```

---

# Runtime Contracts

Nodes may declare requirements.

Example:

```fnl
plot:

requires:
    Visualizable
```

---

Example:

```fnl
vector-search:

requires:
    Embeddable
    Searchable
```

---

# Validation

The graph validator verifies compatibility.

Example:

```text
sales
  └─ Visualizable
```

connected to:

```text
plot
  └─ requires Visualizable
```

Result:

```text
Valid
```

---

Example:

```text
String
```

connected to:

```text
plot
```

Result:

```text
Invalid
```

---

# Behavioral Composition

Capabilities compose naturally.

Example:

```fnl
output:
    documents :
        Dataset
        + Searchable
        + Embeddable
        + Summarizable
```

This output may participate in:

- Vector search
- RAG
- LLM summarization
- Semantic indexing

without the graph caring whether the implementation is Rust, Python, SQL, or remote.

---

# Capability-Based Execution

Future execution engines may optimize scheduling.

Example:

```text
Embeddable
```

might route work to:

```text
GPU Cluster
```

while

```text
Queryable
```

may execute near a database.

---

# AI & Agent Extensions

Future capability categories include:

```text
Retrievable
Reasonable
ToolDiscoverable
AgentAddressable
McpExposable
A2ACompatible
```

allowing Functor graphs to become agent-native.

---

# Example

```fnl
python:

output:
    sales :
        Table
        + Queryable
        + Visualizable
        + Summarizable
```

Consumer:

```fnl
plot:

requires:
    Visualizable
```

Consumer:

```fnl
llm:

requires:
    Summarizable
```

Both nodes are compatible without understanding Python internals.

---

# Conclusion

Functor adopts a dual type model:

```text
Value
├── Data Type
└── Capability Types
```

Data Types describe what a value is.

Capability Types describe what a value can do.

This capability-centered approach enables semantic graph analysis, runtime independence, AI integration, and advanced editor tooling while preserving a simple and declarative user experience.
