# RFC-0013: Semantic Search & Discovery

- **RFC:** 0013
- **Title:** Semantic Search & Discovery
- **Status:** Draft
- **Version:** 0.1
- **Depends On:**
  - RFC-0001 Functor Notebook Architecture & Design
  - RFC-0002 FNL Grammar & Syntax
  - RFC-0003 FNL Type System & Data Contracts
  - RFC-0005 Semantic Node Model (Proposed)

---

# Abstract

This RFC defines the Semantic Search & Discovery subsystem of Functor.

Unlike traditional notebook search systems that operate primarily on text,
Functor performs search over:

- notebook structure
- graph topology
- semantic contracts
- runtime metadata
- relationships
- documentation
- embeddings
- execution history

The goal is to allow users to discover knowledge, capabilities, and relationships rather than merely matching strings.

---

# Motivation

Traditional notebook search answers:

> Where did this text appear?

Functor should answer:

> What produces customer data?

> Which notebooks use this model?

> What nodes generate embeddings?

> Where is revenue calculated?

> Which agents consume this output?

These are graph questions rather than text questions.

---

# Design Goals

The discovery system MUST support:

- Structural search
- Semantic search
- Contract search
- Graph navigation
- Symbol discovery
- Capability discovery

The system SHOULD support:

- Embedding search
- Natural language search
- AI-assisted exploration

---

# Discovery Architecture

```text
Notebook
    │
    ▼
Graph Model
    │
    ├──── Structure Index
    ├──── Symbol Index
    ├──── Contract Index
    ├──── Relationship Index
    └──── Embedding Index
```

Search queries may target any combination of indexes.

---

# Search Domains

Functor recognizes six primary search domains.

---

## 1. Text Search

Traditional content search.

Examples:

```text
invoice
customer
revenue
```

Queries match:

- Markdown
- Comments
- Documentation
- Runtime source code

---

## 2. Symbol Search

Searches graph-level symbols.

Examples:

```text
Customer
RevenueReport
calculateRevenue
```

Targets:

- Outputs
- Contracts
- Functions
- Type definitions
- Node identifiers

---

## 3. Contract Search

Searches based on data contracts.

Example:

```text
Dataset<Customer>
```

Result:

```text
load-customers
sync-customers
active-customers
```

---

# 4. Capability Search

Searches for behaviors rather than data.

Examples:

```text
embedding generation

vector search

customer classification
```

Result:

```text
Node A
Node B
Agent C
Workflow D
```

Capability search is derived from:

- Node types
- Runtime metadata
- Semantic annotations
- Behavioral contracts

---

# 5. Relationship Search

Finds graph relationships.

Queries:

```text
What uses Customer?

What depends on revenue?

Where is customer data produced?
```

The system traverses graph edges.

---

# 6. Semantic Search

Uses embeddings.

Queries:

```text
Find notebooks similar to revenue forecasting.

Find examples of churn prediction.

Customer retention calculations.
```

Similarity is based on meaning rather than exact terms.

---

# Searchable Resources

The following artifacts are searchable.

```text
Notebook
Node
Contract
Runtime
Graph
Function
Query
Prompt
Agent
MCP Tool
Workflow
Documentation
```

Future node types automatically participate.

---

# Search Targets

## Notebook

Example:

```text
Find notebooks about forecasting.
```

Result:

```text
Revenue Forecasting
Quarterly Trends
Market Analysis
```

---

## Node

Example:

```text
Find nodes that generate embeddings.
```

Result:

```text
Customer Embeddings
Product Vectorizer
Document Encoder
```

---

## Contract

Example:

```text
Find producers of Customer.
```

Result:

```text
Import Customers
Sync CRM
Load Snapshot
```

---

## Runtime

Example:

```text
Find Rust revenue calculations.
```

Result:

```text
Revenue Aggregator
Tax Engine
Forecast Calculator
```

---

# Structural Search

Structural search matches graph patterns.

Query:

```text
Dataset<Customer>
    ↓
Embedding
```

Result:

```text
Customer Embedding Workflow
```

---

# Dependency Search

Queries the graph itself.

Examples:

```text
Show dependents of Revenue.
```

```text
Show producers of SalesReport.
```

```text
Trace lineage of Customer.
```

---

# Data Lineage

Functor maintains lineage information.

Example:

```text
Customer Import
        │
        ▼
Customer Clean
        │
        ▼
Customer Scoring
        │
        ▼
Customer Embedding
```

Users can inspect:

- Origins
- Transformations
- Consumers

---

# Symbol Index

The Symbol Index contains:

```text
Types
Outputs
Inputs
Functions
Queries
Agents
Workflows
```

Example:

```text
Customer
```

Returns:

```text
Type Definition
Output Producers
Output Consumers
Related Workflows
```

---

# Contract Index

The Contract Index enables type-centric discovery.

Example:

```text
Find Dataset<Customer>.
```

Result:

```text
Producers
Consumers
Transformations
```

---

# Documentation Search

Documentation participates as a first-class resource.

Sources:

```text
Markdown Nodes
Comments
Notebook Metadata
Generated Docs
```

Example:

```text
How is revenue calculated?
```

The system may return:

```text
Revenue Aggregation Node
Finance Notebook
Revenue Contract
```

---

# Semantic Embeddings

Functor MAY maintain semantic embeddings.

Searchable artifacts include:

- Nodes
- Notebooks
- Documentation
- Prompts
- Contracts

---

## Embedding Generation

Embeddings may be generated from:

```text
Node names
Descriptions
Contracts
Documentation
FNL definitions
Graph topology
```

Implementation remains provider-dependent.

---

# Natural Language Queries

Users may issue questions directly.

Examples:

```text
Which node generates customer embeddings?
```

```text
Where is invoice data loaded?
```

```text
What depends on quarterly revenue?
```

The search engine should translate these into graph queries.

---

# Search Ranking

Ranking factors MAY include:

## Structural Relevance

```text
Graph distance
Dependency proximity
Contract matches
```

## Semantic Relevance

```text
Embedding similarity
```

## Usage Relevance

```text
Execution frequency
References
```

## Recency

```text
Recently modified
Recently executed
```

---

# Discovery API Model

Conceptually discovery operates on a graph.

```text
Search Query
      │
      ▼
 Discovery Engine
      │
 ┌────┼─────┐
 ▼    ▼     ▼
Text Symbol Semantic
```

Implementations remain free to optimize.

---

# Semantic Metadata

Nodes may provide discovery hints.

Example:

```fnl
metadata:
    description:
        Calculates monthly recurring revenue

    tags:
        finance
        revenue
        forecasting
```

These hints enrich search quality.

---

# AI-Assisted Discovery

Future versions may support:

```text
Explain this graph.

Find similar workflows.

Recommend reusable nodes.

Suggest existing contracts.

Detect duplicate implementations.
```

AI assistance SHALL remain optional.

---

# Visual Discovery

Functor editors MAY expose discovery visually.

Examples:

```text
Contract Explorer
Type Explorer
Dependency Explorer
Lineage Explorer
Capability Explorer
```

---

# Security Considerations

Search MUST respect:

- Notebook boundaries
- Workspace permissions
- Runtime permissions
- Secret visibility rules

Sensitive data SHALL NOT be indexed automatically.

---

# Future Directions

Potential future capabilities:

- Cross-workspace discovery
- Federated search
- Knowledge graph generation
- Agent-assisted discovery
- Graph recommendations
- Automated lineage analysis
- Pattern mining
- Similarity clustering

These capabilities are intentionally excluded from this RFC.

---

# Example Queries

## Contract Discovery

```text
Find Dataset<Customer>.
```

Returns:

```text
Import Customers
CRM Sync
Customer Snapshot
```

---

## Capability Discovery

```text
Find embedding generators.
```

Returns:

```text
Customer Encoder
Document Encoder
Product Encoder
```

---

## Dependency Discovery

```text
What uses RevenueForecast?
```

Returns:

```text
Forecast Dashboard
Budget Planner
Executive Report
```

---

## Semantic Discovery

```text
Find notebooks similar to churn prediction.
```

Returns semantically related workflows regardless of implementation language.

---

# Conclusion

Functor treats notebooks as knowledge graphs rather than text documents.

Semantic Search & Discovery enables users to locate:

- Data
- Capabilities
- Relationships
- Contracts
- Workflows
- Documentation

through a combination of:

- Structural indexes
- Symbol indexes
- Contract indexes
- Relationship analysis
- Embedding-based semantic retrieval

This transforms notebooks from isolated documents into a discoverable and navigable computational knowledge system.
