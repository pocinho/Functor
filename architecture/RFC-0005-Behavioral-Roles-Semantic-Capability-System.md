# RFC-0005: Behavioral Roles & Semantic Capability System

- **RFC:** 0005
- **Title:** Behavioral Roles & Semantic Capability System
- **Status:** Draft
- **Version:** 0.1
- **Depends On:** RFC-0001, RFC-0002, RFC-0003
- **Category:** Foundational

---

# Abstract

This RFC introduces the Semantic Capability System for Functor.

Functor models notebook graphs using three orthogonal dimensions:

```text
Node Meaning
├── Shape
├── Capability
└── Role
```

Where:

| Dimension | Answers |
|------------|------------|
| Shape | What is this? |
| Capability | What can it do? |
| Role | What purpose does it serve? |

This model allows both humans and AI agents to reason about notebook graphs semantically rather than syntactically.

The ultimate goal is to enable execution planners, assistants, agents, workflows, MCP tools, and future AI systems to understand the meaning of a graph without requiring knowledge of implementation details.

---

# Motivation

Traditional notebook systems expose only implementation detail.

Example:

```python
df = pd.read_csv("sales.csv")
```

An observer sees:

```text
Python
DataFrame
```

but learns little about its purpose.

Functor instead aims to expose intent.

Example:

```text
Node
 ├─ Shape
 │   Table
 │
 ├─ Capabilities
 │   Queryable
 │   Visualizable
 │   Summarizable
 │
 └─ Role
     Source
```

This description is useful to:

- Users
- Editors
- Execution planners
- Agents
- LLMs
- MCP clients
- Workflow engines

regardless of whether the implementation is:

```text
Rust
Python
DuckDB
Spark
F#
SQL
Remote Service
```

---

# Design Goals

The system MUST allow:

- Semantic graph understanding
- Agent discoverability
- Automatic tool selection
- Capability matching
- Runtime independence
- Graph reasoning
- Intelligent execution planning

The system SHOULD:

- Remain language-neutral
- Be serializable
- Be statically analyzable
- Be machine-readable

---

# Core Model

Functor adopts the following semantic model:

```text
Node
├── Shape
├── Capability
└── Role
```

These dimensions are independent.

---

# Shape

Shape describes the structure of a value.

A Shape answers:

> What is this thing?

---

## Examples

```text
String
Integer
Decimal
Boolean
DateTime
```

---

### Structured Shapes

```text
Customer
Invoice
Order
Employee
```

---

### Composite Shapes

```text
Record
Table
Dataset
Stream
Vector
Graph
Image
Audio
Video
Document
```

---

# Capability

Capabilities describe possible interactions.

A Capability answers:

> What can be done with this thing?

---

## Examples

```text
Queryable
Visualizable
Searchable
Sortable
Filterable
Summarizable
Embeddable
Versionable
```

---

# Role

Role describes graph purpose.

A Role answers:

> Why does this node exist?

---

## Examples

```text
Source
Transform
Sink
Agent
Tool
Workflow
Control
Memory
Gateway
```

---

# Semantic Identity

Together:

```text
Shape
+
Capability
+
Role
```

form a node's Semantic Identity.

Example:

```text
Customer Dataset

Shape:
    Table

Capabilities:
    Queryable
    Filterable
    Searchable

Role:
    Source
```

---

# Why Role Matters

Most orchestration systems understand:

```text
Input
Output
```

Functor additionally understands:

```text
Purpose
```

Purpose becomes essential for:

- Agent planning
- Workflow generation
- Tool discovery
- Graph visualization

---

# Standard Roles

---

## Source

Produces information.

Consumes nothing or little.

Example:

```text
CSV Import
Database Query
API Request
Sensor Feed
```

---

### Diagram

```text
Source
  │
  ▼
Transform
```

---

## Transform

Consumes information and produces information.

Example:

```text
Aggregate
Filter
Normalize
Join
Enrich
```

---

### Diagram

```text
Data
 │
 ▼
Transform
 │
 ▼
Data
```

---

## Sink

Consumes information for presentation.

Example:

```text
Report
Chart
Dashboard
Markdown Output
```

---

### Diagram

```text
Data
 │
 ▼
Visualization
```

---

## Control

Affects execution behavior.

Example:

```text
Branch
Loop
Retry
Condition
```

Control nodes may not produce business data.

---

## Agent

Performs autonomous reasoning.

Example:

```text
Research Agent
Review Agent
Planning Agent
```

---

## Tool

Represents callable functionality.

Example:

```text
Send Email
Execute Query
Generate Report
```

---

## Workflow

Coordinates multiple nodes.

Example:

```text
Sales Analysis Pipeline
Document Review Process
Customer Onboarding
```

---

## Memory

Maintains state.

Example:

```text
Vector Store
Cache
Knowledge Base
Conversation Store
```

---

## Gateway

Represents external systems.

Example:

```text
REST API
MCP Server
A2A Endpoint
Database Connection
```

---

# Capability Taxonomy

Capabilities describe behavior.

Roles describe purpose.

These concepts are intentionally separate.

---

## Example

A node can have:

```text
Role:
    Source
```

and:

```text
Capabilities:
    Queryable
    Searchable
```

or:

```text
Role:
    Agent
```

and:

```text
Capabilities:
    Summarizable
    Reasonable
    ToolCallable
```

---

# Capability Categories

---

## Data Capabilities

```text
Queryable
Filterable
Sortable
Joinable
Groupable
Aggregatable
```

---

## Analytics Capabilities

```text
Statistical
Forecastable
Clusterable
Visualizable
```

---

## Knowledge Capabilities

```text
Searchable
Embeddable
Summarizable
Annotatable
Extractable
```

---

## Agentic Capabilities

```text
Reasonable
GoalDriven
ToolCallable
Planable
Delegatable
```

---

## Workflow Capabilities

```text
Composable
Retryable
Cacheable
Parallelizable
```

---

## Infrastructure Capabilities

```text
Persistable
Versionable
Replicable
Observable
```

---

# Semantic Compatibility

Graphs SHOULD validate semantically.

---

## Example

Producer:

```text
Table
+
Visualizable
```

Consumer:

```text
Requires:
    Visualizable
```

Result:

```text
Compatible
```

---

Producer:

```text
String
```

Consumer:

```text
Requires:
    Visualizable
```

Result:

```text
Invalid
```

---

# Semantic Planning

Capabilities allow automatic graph planning.

Example:

```text
Input:
    Customer Dataset

Capabilities:
    Searchable
    Summarizable
```

An agent may infer:

```text
Possible Actions
 ├─ Search
 ├─ Retrieve
 ├─ Summarize
 └─ Analyze
```

without implementation knowledge.

---

# Agent Graph Comprehension

This RFC formally introduces:

## Semantic Attention

Semantic Attention is the process by which agents understand graph meaning through node metadata.

Agents SHOULD avoid reasoning from source code whenever semantic metadata exists.

Instead:

```text
Shape
Capability
Role
```

becomes the primary planning surface.

---

# Example

Consider:

```text
Python Node
```

Traditional interpretation:

```text
Unknown Code
```

Functor interpretation:

```text
Role:
    Source

Shape:
    Table

Capabilities:
    Queryable
    Visualizable
```

An agent immediately understands its purpose.

---

# Semantic Neighborhoods

Nodes create meaning through local relationships.

Example:

```text
Customer Source
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

An agent may infer:

```text
Purpose:
    Customer segmentation workflow
```

without analyzing implementation code.

---

# Intent Surfaces

Nodes MAY expose explicit intent.

Example:

```fnl
intent:
    Analyze customer purchasing behavior
```

---

## Execution Planning Intent

```fnl
intent:
    Produce monthly sales summary
```

---

## Agent Planning Intent

```fnl
intent:
    Recommend next actions for account managers
```

This allows AI systems to reason from declared objectives.

---

# Semantic Graph Index

Future Functor implementations SHOULD generate a semantic graph index.

Example:

```text
Notebook

Sources:
    5

Transforms:
    12

Agents:
    3

Sinks:
    7
```

Capabilities:

```text
Visualizable:
    8

Queryable:
    12

Searchable:
    4
```

This index becomes a high-value context surface for agents.

---

# MCP Integration

Capabilities map naturally to MCP tools.

Example:

```text
Tool Node

Role:
    Tool

Capabilities:
    ToolCallable
```

An agent can discover tools from graph metadata rather than runtime inspection.

---

# A2A Integration

Agents become discoverable through semantic metadata.

Example:

```text
Role:
    Agent

Capabilities:
    Searchable
    Summarizable
    Delegatable
```

Peer agents can determine suitability automatically.

---

# Future Extensions

Potential future additions include:

```text
Authority
Trust Level
Security Scope
Privacy Classification
Data Ownership
Execution Cost
Latency Classes
```

These concerns are intentionally deferred to future RFCs.

---

# Foundational Principle

Functor adopts a semantic-first architecture.

Nodes are not defined primarily by:

```text
Programming Language
```

or:

```text
Implementation Technology
```

but by:

```text
Shape
Capability
Role
```

This enables notebook graphs to become understandable by:

- Humans
- Editors
- Planners
- Agents
- MCP clients
- A2A systems
- Future orchestration engines

without requiring implementation-specific knowledge.

---

# Conclusion

Functor formally defines graph semantics using three orthogonal dimensions:

```text
Node Meaning

├── Shape
│     What is it?
│
├── Capability
│     What can it do?
│
└── Role
      Why does it exist?
```

These dimensions collectively form a node's Semantic Identity.

Semantic Identity becomes the primary surface through which editors, execution engines, and agents understand notebook graphs.

By elevating meaning above implementation, Functor establishes a foundation for semantic computing, agent-native orchestration, and graph-based reasoning that can evolve independently of any runtime technology.
