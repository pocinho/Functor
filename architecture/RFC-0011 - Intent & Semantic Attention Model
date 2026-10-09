# RFC-0011: Intent & Semantic Attention Model

- **RFC:** 0011
- **Title:** Intent & Semantic Attention Model
- **Status:** Draft
- **Version:** 0.1
- **Depends On:** RFC-0004, RFC-0005, RFC-0006, RFC-0010
- **Category:** Foundational / Strategic

---

# Abstract

This RFC defines how meaning is represented, discovered, propagated, and consumed within Functor.

Functor introduces Intent as a first-class semantic concept and Semantic Attention as the mechanism through which humans, editors, execution planners, and agents understand computational systems.

The objective of this RFC is to allow participants to answer:

```text
What is this graph trying to accomplish?
```

without requiring inspection of implementation details.

---

# Motivation

Most software systems expose implementation before meaning.

Example:

```python
sales.groupby(...)
```

A reader must infer:

```text
Purpose
Intent
Business meaning
Expected outcome
```

from implementation.

Functor reverses this relationship.

Meaning becomes explicit.

Implementation becomes secondary.

---

# Core Principle

Meaning should be declared.

Execution should be inferred.

Traditional systems:

```text
Code
    ↓
Infer Meaning
```

Functor:

```text
Meaning
    ↓
Infer Execution
```

---

# Intent

Intent represents the purpose of an entity.

Intent answers:

```text
Why does this exist?
```

Intent is independent from:

- Runtime
- Language
- Infrastructure
- Storage
- Deployment

---

# Intent Hierarchy

Intent may exist at multiple scopes.

```text
Notebook
├── Graph Intent
├── Node Intent
└── Output Intent
```

---

# Notebook Intent

Describes the purpose of the notebook.

Example:

```text
Analyze customer churn.
```

Example:

```text
Forecast quarterly revenue.
```

Example:

```text
Produce operational risk report.
```

Notebook intent acts as the highest semantic layer.

---

# Graph Intent

Graphs may expose specialized goals.

Example:

```text
Generate customer segmentation insights.
```

Example:

```text
Detect abnormal transaction patterns.
```

---

# Node Intent

Every node MAY expose intent.

Example:

```text
Normalize customer records.
```

Example:

```text
Calculate monthly revenue.
```

Example:

```text
Rank sales opportunities.
```

---

# Output Intent

Outputs may describe intended usage.

Example:

```text
Executive reporting dataset.
```

Example:

```text
Downstream forecasting input.
```

---

# Intent Is Not Documentation

Intent is not a comment.

Intent participates in graph semantics.

Intent is machine-readable.

Intent is queryable.

Intent is discoverable.

Intent is composable.

---

# Semantic Identity

RFC-0005 introduced:

```text
Shape
Capability
Role
```

This RFC extends the model.

```text
Semantic Identity

├── Shape
├── Capability
├── Role
└── Intent
```

Intent becomes a first-class semantic dimension.

---

# Semantic Attention

Semantic Attention is the prioritization of meaning over implementation.

Participants SHOULD attempt reasoning using semantic metadata before examining implementation details.

---

# Attention Hierarchy

Functor defines a preferred attention model.

```text
Intent
    ↓
Role
    ↓
Capability
    ↓
Shape
    ↓
Implementation
```

Meaning is preferred over mechanics.

---

# Human Attention

Humans naturally operate this way.

Example:

A user opening a graph asks:

```text
What does this system do?
```

before asking:

```text
What language is this node written in?
```

Functor formalizes this process.

---

# Agent Attention

Agents SHOULD reason from semantic metadata first.

Preferred:

```text
Intent
Role
Capability
```

Fallback:

```text
Source Code
```

Code becomes the exception.

Not the default.

---

# Semantic Neighborhoods

Meaning emerges through relationships.

Nodes inherit context from their surroundings.

---

## Example

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

An observer may infer:

```text
Customer Analytics Workflow
```

without execution.

---

# Intent Propagation

Intent may propagate through graph relationships.

---

## Example

Notebook Intent:

```text
Forecast revenue.
```

Node Intent:

```text
Calculate monthly revenue.
```

Downstream Node:

```text
Predict next quarter revenue.
```

The system recognizes alignment.

---

# Intent Coherence

Graphs possess semantic coherence.

Example:

```text
Revenue Analysis
```

and:

```text
Sales Forecast
```

demonstrate alignment.

---

Example:

```text
Revenue Forecast
```

connected to:

```text
Image Classification
```

may indicate inconsistency.

---

# Intent Validation

Future implementations MAY detect:

```text
Contradictory Intent
Unrelated Intent
Missing Intent
```

within a graph.

---

# Semantic Context Windows

Agents require bounded context.

Functor introduces semantic context generation.

Instead of exposing:

```text
Entire Notebook
```

Functor may expose:

```text
Notebook Intent

Key Sources
Key Agents
Key Outputs

Primary Workflow
```

---

# Graph Summarization

Every graph SHOULD be summarizable.

Example:

```text
Purpose:
    Customer Retention Analysis

Sources:
    CRM
    Billing Data

Transforms:
    Segmentation

Agents:
    Recommendation Agent

Outputs:
    Retention Report
```

Generated from semantics.

Not implementation.

---

# Semantic Compression

Intent enables aggressive context compression.

1000 nodes may become:

```text
Revenue Forecast Workflow
```

from an agent's perspective.

This significantly improves reasoning efficiency.

---

# Discoverability

Users SHOULD be able to ask:

```text
Find graphs about churn.

Find forecasting workflows.

Find recommendation systems.
```

using semantic metadata.

---

# Goal Alignment

Intent becomes a planning surface.

Example:

```text
Goal:
    Improve customer retention.
```

Planner searches for graph components with matching semantic intent.

---

# Agent Planning

Agents SHOULD construct plans by aligning:

```text
Intent
Capability
Role
```

rather than implementation details.

---

## Example

Goal:

```text
Generate customer recommendations.
```

Planner identifies:

```text
Customer Sources
Recommendation Agents
Report Outputs
```

without inspecting runtimes.

---

# Intent-Based Composition

Future systems MAY compose graphs from goals.

Example:

```text
Create revenue forecasting workflow.
```

Planner discovers:

```text
Revenue Source
Forecast Engine
Visualization
Reporting
```

and composes a graph automatically.

---

# Semantic Search

Search operations SHOULD leverage semantic metadata.

Examples:

```text
Find all forecasting agents.

Find all recommendation workflows.

Find all customer intelligence notebooks.
```

---

# Shared Understanding

Humans and agents should consume the same semantic surface.

Functor adopts:

```text
Single Meaning Model
```

rather than separate human-centric and machine-centric representations.

---

# Cognitive Symmetry

Functor aims to establish Cognitive Symmetry.

Definition:

```text
The degree to which humans and agents perceive and reason about a graph through the same semantic structures.
```

Higher symmetry improves:

- Collaboration
- Explainability
- Discoverability
- Trust
- Reuse

---

# Semantic Contracts

Intent MAY participate in validation.

Example:

```text
Intent:
    Forecast Revenue
```

Expected capabilities:

```text
Forecastable
Statistical
```

Incompatible nodes may be flagged.

---

# Explainability

Every graph SHOULD be explainable.

Questions:

```text
What does this graph do?

Why does this node exist?

Why was this tool selected?

Why was this workflow generated?
```

should be answerable through semantics.

---

# Relationship To Agents

RFC-0010 defines:

```text
How agents participate.
```

This RFC defines:

```text
How agents understand.
```

The two RFCs are complementary.

---

# Relationship To Editors

Future editors SHOULD surface:

```text
Intent Maps
Semantic Neighborhoods
Goal Flows
Attention Paths
```

rather than merely syntax.

---

# Foundational Principle

Functor treats meaning as a first-class computational artifact.

Semantic metadata is not auxiliary documentation.

It is part of the operational model.

Humans, agents, planners, and execution engines operate against a shared semantic surface.

---

# Conclusion

Functor introduces Intent and Semantic Attention as foundational concepts.

Together with Shape, Capability, and Role they form:

```text
Functor Semantic Model

├── Shape
├── Capability
├── Role
└── Intent
```

This model enables humans and intelligent systems to develop a shared understanding of computational workflows.

The long-term objective is not merely executable notebooks.

The objective is semantic systems that are understandable, composable, explainable, and collaboratively navigable by both humans and agents.
