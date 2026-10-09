# RFC-0006: Graph Execution Engine

- **RFC:** 0006
- **Title:** Graph Execution Engine
- **Status:** Draft
- **Version:** 0.1
- **Depends On:** RFC-0000, RFC-0001, RFC-0002, RFC-0003, RFC-0004, RFC-0005
- **Category:** Foundational

---

# Abstract

This RFC defines the Graph Execution Engine responsible for evaluating Functor notebook graphs.

The execution engine consumes the Semantic Graph Model and transforms it into executable work.

The engine is responsible for:

- Dependency resolution
- Scheduling
- Materialization
- Invalidation
- Incremental recomputation
- Caching
- Parallel execution
- Runtime coordination

The engine is intentionally independent of:

- Storage formats
- UI implementations
- Runtime technologies

---

# Motivation

Traditional notebook systems execute cells in document order.

```text
Cell 1
Cell 2
Cell 3
```

This creates problems:

- Hidden dependencies
- Out-of-order execution
- Stale state
- Difficult reproducibility
- Poor parallelism

Functor instead executes the graph.

```text
Load Data
     │
     ▼
Aggregate
  ┌──┴──┐
  ▼     ▼
Chart Report
```

Execution becomes dependency-driven rather than position-driven.

---

# Design Goals

The execution engine MUST:

- Respect graph dependencies
- Produce deterministic results
- Support incremental execution
- Support runtime independence
- Support graph validation
- Support partial execution

The execution engine SHOULD:

- Support parallel scheduling
- Support distributed execution
- Support execution caching
- Support semantic planning

---

# Execution Model

Execution occurs over a Semantic Graph.

Input:

```text
Graph
```

Output:

```text
Materialized Outputs
```

Process:

```text
Graph
  │
  ▼
Validation
  │
  ▼
Planning
  │
  ▼
Scheduling
  │
  ▼
Execution
  │
  ▼
Materialization
```

---

# Fundamental Concept

Functor executes dependencies.

It does not execute cells.

Execution is a graph operation.

---

# Execution Units

The smallest schedulable unit is a Node.

```text
Node
├── Inputs
├── Outputs
├── Runtime
└── State
```

Nodes execute independently.

---

# Node State Machine

A node progresses through states.

```text
Defined
   │
   ▼
Resolved
   │
   ▼
Validated
   │
   ▼
Ready
   │
   ▼
Running
   │
   ▼
Materialized
```

---

# Additional States

```text
Failed
Cancelled
Invalidated
Cached
```

---

# Defined

Node exists within graph.

```text
Defined
```

No validation performed.

---

# Resolved

All dependencies discovered.

```text
Resolved
```

Inputs are connected.

---

# Validated

Contracts and requirements satisfied.

```text
Validated
```

Examples:

- Type checks
- Capability checks
- Runtime availability

---

# Ready

All upstream dependencies are materialized.

```text
Ready
```

Node may execute.

---

# Running

Execution has begun.

```text
Running
```

Runtime owns execution.

---

# Materialized

Outputs are available.

```text
Materialized
```

Dependents may proceed.

---

# Dependency Resolution

Dependencies originate from graph edges.

Example:

```text
Load
 │
 ▼
Aggregate
 │
 ▼
Chart
```

Execution order becomes:

```text
1. Load
2. Aggregate
3. Chart
```

Derived automatically.

---

# Topological Scheduling

Execution plans MUST be generated through topological ordering.

Example:

```text
A
│
▼
B
│
▼
C
```

Plan:

```text
A → B → C
```

---

# Parallel Scheduling

Independent branches MAY execute concurrently.

Example:

```text
         Load
           │
     ┌─────┴─────┐
     ▼           ▼
Aggregate     Analyze
     ▼           ▼
 Chart      Summary
```

Possible schedule:

```text
Load

Aggregate || Analyze

Chart || Summary
```

---

# Materialization

Execution produces materialized outputs.

Example:

```text
Python Node
```

Produces:

```text
DataFrame
```

Materialized output becomes available to downstream nodes.

---

# Output Store

Materialized outputs are placed into an execution context.

```text
Execution Context
 ├── Sales
 ├── Revenue
 ├── Summary
 └── Forecast
```

Consumers retrieve outputs from this store.

---

# Execution Context

The execution context represents graph state.

```text
ExecutionContext
├── Variables
├── Outputs
├── Runtime Resources
├── Diagnostics
└── Metadata
```

---

# Invalidation

Graph changes create invalidation.

Example:

```text
Source Modified
```

Results:

```text
Invalidate Source
Invalidate Descendants
```

---

## Example

```text
Load Data
     │
     ▼
Aggregate
     │
     ▼
Chart
```

Aggregate changes.

Result:

```text
Aggregate → Invalid
Chart     → Invalid
```

Load Data remains valid.

---

# Incremental Recomputation

Only affected nodes SHOULD execute.

Example:

```text
A
│
▼
B
│
▼
C
```

B changes.

New schedule:

```text
B
▼
C
```

A is reused.

---

# Caching

Nodes MAY cache materialized outputs.

Cache key:

```text
Node Definition
+
Inputs
+
Configuration
+
Runtime Version
```

---

# Cache Hit

```text
Inputs unchanged
```

Result:

```text
Reuse Materialization
```

Node execution is skipped.

---

# Cache Miss

```text
Inputs changed
```

Result:

```text
Execute Node
```

---

# Runtime Isolation

Execution MUST remain runtime-neutral.

The execution engine does not execute:

```text
Rust
Python
SQL
F#
```

directly.

Instead:

```text
Engine
  │
  ▼
Runtime Provider
  │
  ▼
Execution
```

---

# Runtime Delegation

Example:

```text
Rust Node
```

Delegates to:

```text
Rust Runtime
```

Example:

```text
Python Node
```

Delegates to:

```text
Python Runtime
```

---

# Runtime Independence

The engine reasons about:

```text
Nodes
Types
Capabilities
Roles
```

rather than implementation languages.

---

# Capability-Based Scheduling

Execution planning MAY leverage capabilities.

Example:

```text
Embeddable
```

could route to:

```text
GPU Executor
```

Example:

```text
Queryable
```

could route to:

```text
Database Executor
```

---

# Role-Aware Scheduling

Behavioral roles influence execution strategy.

---

## Source

Prioritize acquisition.

```text
CSV
Database
API
```

---

## Transform

Prioritize throughput.

```text
Aggregation
Join
Normalization
```

---

## Sink

Prioritize presentation.

```text
Charts
Reports
Tables
```

---

## Agent

Prioritize reasoning resources.

```text
LLM Agent
Planner Agent
Reviewer Agent
```

---

# Execution Plans

The engine produces a plan before execution.

Example:

```text
Plan
├── Nodes
├── Dependencies
├── Parallel Groups
└── Resource Requirements
```

---

# Partial Execution

Users MAY request execution from a specific node.

Example:

```text
Execute:
Revenue Report
```

Engine computes:

```text
Required Upstream Nodes
```

Only necessary work executes.

---

# Reachability Execution

Given:

```text
A
│
▼
B
│
▼
C
```

Request:

```text
Execute C
```

Engine resolves:

```text
A
B
C
```

Automatically.

---

# Failure Handling

Node failures do not automatically invalidate entire graphs.

---

## Failure State

```text
Failed
```

Contains:

```text
Error
Stack Trace
Diagnostics
```

---

# Failure Propagation

Failed outputs cannot materialize.

Downstream nodes become:

```text
Blocked
```

rather than failed.

---

## Example

```text
A → B → C
```

B fails.

Result:

```text
A = Success
B = Failed
C = Blocked
```

---

# Retry Behavior

Nodes MAY support retries.

Example:

```text
API Request
```

Policy:

```text
Retry 3 Times
```

---

# Cancellation

Execution MAY be cancelled.

States:

```text
Running
```

becomes:

```text
Cancelled
```

All descendants become:

```text
Pending
```

---

# Determinism

The same graph with:

- identical inputs
- identical runtime versions
- identical configuration

SHOULD produce identical outputs.

This is a core execution guarantee.

---

# Distributed Execution

Future engines MAY distribute workloads.

Example:

```text
Notebook
```

Execution:

```text
Local Machine
GPU Cluster
Remote Agent
Cloud Runtime
```

The graph remains unchanged.

---

# Event Model

Nodes generate execution events.

Examples:

```text
Started
Completed
Failed
Invalidated
Materialized
```

These events support:

- Dashboards
- Logs
- Diagnostics
- Agents

---

# Agent-Aware Execution

Agent nodes are first-class execution units.

Example:

```text
Research Agent
```

The execution engine schedules the node exactly like:

```text
Transform Node
```

The planner does not special-case intelligence.

---

# Semantic Awareness

The execution engine SHOULD understand:

```text
Shape
Capability
Role
Intent
```

when optimizing plans.

This enables future semantic execution strategies.

---

# Example Workflow

```text
Customer CSV
      │
      ▼
Import
      │
      ▼
Normalize
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

Execution order:

```text
Import
↓
Normalize
↓
Segmentation
↓
Recommendation Agent
↓
Report
```

Derived automatically from graph topology.

---

# Relationship To Other RFCs

---

## RFC-0004

Provides:

```text
Graph Structure
```

---

## RFC-0005

Provides:

```text
Graph Meaning
```

---

## RFC-0006

Provides:

```text
Graph Execution
```

Together:

```text
Structure
+
Meaning
+
Execution
```

form the core computational model of Functor.

---

# Foundational Principle

Functor executes graphs, not documents.

Execution is driven by:

- Dependencies
- Contracts
- Capabilities
- Roles
- Intent

rather than notebook position or runtime implementation.

This allows execution engines, planners, editors, and agents to operate against a shared semantic model.

---

# Conclusion

Functor adopts a graph-native execution engine built upon the Semantic Graph Model.

Execution proceeds through:

```text
Validation
    ↓
Planning
    ↓
Scheduling
    ↓
Execution
    ↓
Materialization
```

The engine supports:

- Deterministic execution
- Incremental recomputation
- Parallel scheduling
- Runtime independence
- Semantic optimization
- Future distributed execution

By treating graph execution as a first-class concern, Functor establishes a foundation capable of supporting everything from local exploratory notebooks to large-scale semantic and agentic workflows.
