# RFC-0004: Semantic Graph Model

- **RFC:** 0004
- **Title:** Semantic Graph Model
- **Status:** Draft
- **Version:** 0.1
- **Depends On:** RFC-0000, RFC-0001, RFC-0002, RFC-0003
- **Referenced By:** RFC-0005 Behavioral Roles & Semantic Capability System
- **Category:** Foundational

---

# Abstract

This RFC defines the canonical graph model used internally and externally by Functor.

Functor notebooks are modeled as semantic directed graphs rather than ordered documents.

The graph model serves as the foundation for:

- Execution planning
- Dependency tracking
- Semantic analysis
- AI-assisted workflows
- Graph visualization
- Runtime orchestration
- Agent reasoning

This RFC intentionally defines structure rather than execution behavior.

Execution semantics are defined separately in RFC-0006.

---

# Motivation

Traditional notebook systems treat documents as sequential collections of cells.

```text
Cell 1
Cell 2
Cell 3
Cell 4
```

Execution dependencies become implicit.

As notebooks grow, users and tools struggle to answer:

- What depends on what?
- What breaks if this changes?
- What is safe to run?
- What is the purpose of a cell?
- How can an AI understand the workflow?

Functor addresses this by treating notebooks as explicit semantic graphs.

---

# Goals

The graph model MUST:

- Support semantic analysis
- Be runtime-agnostic
- Be statically analyzable
- Support incremental recomputation
- Support agent reasoning
- Be serializable
- Support graph visualization

The graph model SHOULD:

- Allow parallel execution
- Support partial evaluation
- Enable extensibility
- Remain implementation independent

---

# Core Principle

A notebook is a graph.

```text
Notebook
    │
    ▼
 Graph
    │
 ┌──┴───┐
 ▼      ▼
Nodes  Edges
```

A notebook is no longer defined by document order.

It is defined by relationships.

---

# Graph Model

Functor adopts a directed graph model.

```text
Graph
├── Nodes
├── Edges
├── Metadata
└── Intent
```

---

# Fundamental Concepts

## Notebook

The notebook is the highest-level container.

```text
Notebook
 ├── Metadata
 ├── Graph
 └── Intent
```

A notebook owns exactly one graph.

---

## Graph

A graph contains nodes and edges.

```text
Graph
 ├── Nodes
 └── Edges
```

Graphs define topology.

Graphs do not perform work.

---

## Node

A node represents a semantic unit of work.

Examples:

```text
Python Script
Rust Transform
Database Query
Chart
LLM Prompt
Agent
CSV Import
```

Every node possesses identity.

---

## Edge

An edge describes a dependency.

Edges are directional.

```text
Source
  │
  ▼
Consumer
```

The producer precedes the consumer.

---

# Node Definition

Every node contains:

```text
Node
├── Id
├── Name
├── Inputs
├── Outputs
├── Metadata
├── Runtime
├── Contracts
└── Intent
```

---

# Node Identity

A node MUST possess a globally unique graph identifier.

Example:

```text
load-sales
```

or

```text
aggregate-monthly-revenue
```

Identifiers are stable references.

---

# Node Name

Human-readable label.

Example:

```text
Load Sales CSV
```

A name is not required to be unique.

---

# Node Metadata

Optional descriptive information.

Example:

```text
Author
Description
Category
Tags
Created Date
```

Metadata MUST NOT influence graph semantics.

---

# Inputs

Inputs consume outputs from other nodes.

Example:

```text
aggregate.sales
```

Input definition:

```text
Input
├── Local Name
├── Source Node
└── Source Output
```

---

# Outputs

Outputs publish values.

Example:

```text
sales
```

Output definition:

```text
Output
├── Name
├── Type
└── Capabilities
```

---

# Ports

Inputs and outputs form node ports.

```text
Node

Input Ports
    │
    ▼

[ Runtime ]

    │
    ▼

Output Ports
```

Ports become attachment points for graph edges.

---

# Port Model

Input Port:

```text
InputPort
├── Name
├── Type
└── Requirements
```

Output Port:

```text
OutputPort
├── Name
├── Type
└── Capabilities
```

---

# Edge Definition

An edge connects:

```text
Output Port
     │
     ▼
Input Port
```

Formal model:

```text
Edge
├── Source Node
├── Source Port
├── Target Node
└── Target Port
```

---

# Example

```text
Load
 └─ sales

Aggregate
 └─ sales
```

Creates:

```text
Load.sales
    │
    ▼
Aggregate.sales
```

---

# Graph Topology

Functor graphs are directed.

```text
A
 │
 ▼
B
 │
 ▼
C
```

Direction expresses dependency.

---

# Directed Acyclic Graphs

Default Functor notebooks SHOULD be DAGs.

```text
A → B → C
```

Allowed.

---

```text
A → B → A
```

Rejected.

---

# Cycles

Cycles SHOULD be disallowed by default.

Example:

```text
A
▲
│
▼
B
```

Reason:

- Infinite invalidation
- Ambiguous execution
- Non-deterministic planning

Future RFCs may introduce controlled feedback loops.

---

# Semantic Relationships

Edges carry meaning.

They are not merely execution dependencies.

Example:

```text
Customer Data
    │
    ▼
Segmentation Model
```

An edge communicates:

```text
Consumes
Depends On
Acts Upon
```

---

# Graph Layers

Graphs may be analyzed at multiple layers.

---

## Structural Layer

```text
Nodes
Edges
Ports
```

---

## Type Layer

Defined in RFC-0003.

```text
Shapes
Contracts
```

---

## Semantic Layer

Defined in RFC-0005.

```text
Capabilities
Roles
Intent
```

---

# Graph Context

Graphs may contain shared context.

Example:

```text
Graph
 ├── Variables
 ├── Secrets
 ├── Configuration
 └── Environment
```

Context is available to participating nodes.

---

# Graph Intent

A notebook may expose intent.

Example:

```text
Analyze monthly sales performance.
```

Intent exists at graph scope.

---

```text
Customer churn analysis workflow.
```

Intent assists:

- Documentation
- Queries
- Agents
- Discovery

---

# Graph Boundaries

A graph forms an execution boundary.

Inputs may enter.

Outputs may leave.

---

## Graph Inputs

```text
Notebook Input
```

Example:

```text
sales.csv
```

---

## Graph Outputs

```text
Monthly Revenue Report
```

Graph outputs allow notebooks to behave as reusable components.

---

# Subgraphs

Future Functor versions MAY support nested graphs.

Example:

```text
Notebook
 │
 ▼
Graph
 ├─ Node
 ├─ Node
 └─ Subgraph
      ├─ Node
      └─ Node
```

Subgraphs enable:

- Composition
- Reuse
- Encapsulation

---

# Graph Versioning

Every graph possesses a revision identity.

Example:

```text
Version 1
Version 2
Version 3
```

Versioning applies to structure rather than runtime state.

---

# Node Lifecycle

Nodes move through states.

---

## Defined

Node exists.

```text
Defined
```

---

## Connected

Dependencies resolved.

```text
Connected
```

---

## Validated

Contracts satisfied.

```text
Validated
```

---

## Executable

Ready for execution.

```text
Executable
```

---

## Materialized

Outputs available.

```text
Materialized
```

---

# Materialization

A node becomes materialized when outputs exist.

Example:

```text
Query Executed
```

Result:

```text
Customer Table Available
```

Materialization state is separate from execution state.

---

# Dependency Resolution

Dependencies are derived from edges.

Example:

```text
Load
 │
 ▼
Aggregate
 │
 ▼
Report
```

Dependency ordering:

```text
1 Load
2 Aggregate
3 Report
```

---

# Reachability

The graph model MUST support reachability analysis.

Example:

```text
A → B → C
```

From:

```text
A
```

Reachable:

```text
B
C
```

Applications:

- Impact analysis
- Invalidation
- Refactoring
- Agent planning

---

# Graph Queries

Implementations SHOULD support:

```text
Upstream()
Downstream()
Neighbors()
Ancestors()
Descendants()
```

Example:

```text
What depends on Revenue?
```

---

# Semantic Navigation

The graph model must support navigation independent of source code.

Example:

```text
Show all sources.

Show all transforms.

Show all visualizations.

Show all agents.
```

This becomes especially important for AI tooling.

---

# Canonical Representation

A graph SHOULD be representable independently of storage format.

Example:

```json
{
  "nodes": [],
  "edges": []
}
```

or

```text
Graph Object Model
```

TOML remains a persistence format.

The graph model remains the canonical representation.

---

# AI Interpretation

The graph model is intentionally designed for machine understanding.

An agent should be able to answer:

```text
What does this notebook do?
```

without reading implementation code.

Instead it should inspect:

- Nodes
- Types
- Relationships
- Roles
- Capabilities
- Intent

---

# Example

Customer Analytics Notebook:

```text
CSV Import
      │
      ▼
Customer Transform
      │
      ▼
Segmentation
      │
      ▼
Recommendation Agent
      │
      ▼
Customer Report
```

An agent should infer:

```text
Purpose:
Customer analytics workflow

Inputs:
CSV Data

Processing:
Segmentation

Reasoning:
Recommendation Agent

Output:
Report
```

without executing a single node.

---

# Relationship To RFC-0005

This RFC defines:

```text
Structure
```

RFC-0005 defines:

```text
Meaning
```

Together they form:

```text
Semantic Graph

Graph Structure
+
Semantic Identity
```

---

# Foundational Principle

Functor does not model notebooks as collections of cells.

Functor models notebooks as semantic graphs.

Graphs become the primary representation for:

- Execution
- Navigation
- Analysis
- Planning
- Collaboration
- Agent reasoning

Source code becomes an implementation detail attached to graph nodes rather than the primary representation of computational intent.

---

# Conclusion

Functor adopts a directed semantic graph model as the foundational representation of notebooks.

The graph consists of:

```text
Graph
├── Nodes
├── Ports
├── Edges
├── Intent
└── Metadata
```

This structure provides a runtime-neutral, semantically rich foundation upon which execution engines, editors, AI agents, and future distributed workflows can operate.

By elevating graphs above source code, Functor establishes a model where computational intent is explicit, inspectable, analyzable, and understandable by both humans and machines.
