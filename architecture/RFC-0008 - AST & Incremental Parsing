# RFC-0008: AST & Incremental Parsing

- **RFC:** 0008
- **Title:** AST & Incremental Parsing
- **Status:** Draft
- **Version:** 0.1
- **Depends On:** RFC-0000, RFC-0001, RFC-0002, RFC-0003, RFC-0004, RFC-0005
- **Category:** Foundational
- **Audience:** Language Services, Editor, Tooling, Runtime Authors

---

# Abstract

This RFC defines the Abstract Syntax Tree (AST), parsing architecture, diagnostics model, and incremental parsing strategy for the Functor Notebook Language (FNL).

The primary goals are:

- Fast editor responsiveness
- Stable semantic analysis
- Structural navigation
- Refactoring support
- Graph generation
- Agent understanding

Functor treats syntax as a projection of semantic structures rather than the primary representation.

As a consequence:

```text
Text
    ↓
Parser
    ↓
AST
    ↓
Semantic Graph
```

The AST serves as the bridge between user-authored text and the Semantic Graph Model.

---

# Motivation

Traditional notebook editors often rely on:

```text
Text
    ↓
Execution
```

This model provides poor support for:

- Refactoring
- Navigation
- Dependency discovery
- Semantic understanding
- AI assistance

Functor instead establishes:

```text
Text
    ↓
AST
    ↓
Graph
    ↓
Execution
```

allowing tooling to reason about notebook structure without executing code.

---

# Design Goals

The parser MUST:

- Be deterministic
- Be lossless
- Preserve formatting
- Support diagnostics
- Support partial re-parsing
- Support semantic analysis

The parser SHOULD:

- Tolerate incomplete documents
- Recover from syntax errors
- Preserve user intent
- Operate incrementally

---

# Core Principle

The AST is the canonical representation of user-authored FNL.

Execution engines MUST NOT execute raw text.

Execution engines operate on graph structures derived from the AST.

---

# Parsing Pipeline

Functor adopts a multi-stage pipeline.

```text
Source Text
      │
      ▼
Lexer
      │
      ▼
Tokens
      │
      ▼
Parser
      │
      ▼
AST
      │
      ▼
Semantic Binder
      │
      ▼
Semantic Graph
```

Each stage has distinct responsibilities.

---

# Lexical Model

The lexer converts text into tokens.

Example:

```fnl
input:
    sales <- load.sales
```

Produces:

```text
Keyword(input)
Colon
Identifier(sales)
Arrow(<-)
Identifier(load)
Dot
Identifier(sales)
```

---

# Token Categories

---

## Keywords

Built-in language constructs.

Examples:

```text
input
output
intent
requires
code
query
runtime
metadata
```

---

## Identifiers

Named symbols.

Examples:

```text
sales
customer
monthlyRevenue
```

---

## Literals

Examples:

```text
42
0.97
"Revenue"
true
false
```

---

## Operators

Examples:

```text
<-
:
=
.
+
```

---

## Trivia

Non-semantic tokens.

Examples:

```text
Whitespace
Newline
Comment
```

Trivia MUST be preserved.

---

# Lossless Parsing

Functor adopts lossless parsing.

All source information is preserved.

Including:

```text
Whitespace
Comments
Formatting
```

Example:

```fnl
# Revenue data

sales <- load.sales
```

can be reconstructed exactly after parsing.

---

# Abstract Syntax Tree

The AST represents structure.

Example:

```fnl
input:
    sales <- load.sales
```

AST:

```text
InputSection
 └── Binding
      ├── LocalName
      └── Reference
```

---

# AST Principles

The AST MUST:

- Be immutable
- Be deterministic
- Preserve source ranges
- Support diagnostics

The AST SHOULD:

- Support incremental replacement
- Preserve formatting metadata

---

# Source Ranges

Every AST node records location information.

Example:

```text
TextSpan
 ├── Start
 ├── End
 └── Length
```

This enables:

- Syntax highlighting
- Refactoring
- Diagnostics
- Navigation

---

# Root Node

Every FNL document produces a root node.

```text
Document
 ├── Header
 ├── Sections
 └── Diagnostics
```

---

# Node Header AST

Example:

```fnl
python:
```

AST:

```text
RuntimeHeader
 └── RuntimeName
```

---

# Section AST

Example:

```fnl
output:
    totals
```

AST:

```text
OutputSection
 └── OutputDeclaration
```

---

# Input Binding AST

Example:

```fnl
input:
    sales <- load.sales
```

AST:

```text
InputBinding
 ├── LocalName("sales")
 └── Reference
      ├── Node("load")
      └── Output("sales")
```

---

# Output AST

Example:

```fnl
output:
    totals : Table
```

AST:

```text
OutputDeclaration
 ├── Name("totals")
 └── TypeReference("Table")
```

---

# Capability AST

Example:

```fnl
output:
    sales :
        Table
        + Queryable
        + Visualizable
```

AST:

```text
OutputDeclaration
 ├── Shape(Table)
 └── Capabilities
      ├── Queryable
      └── Visualizable
```

---

# Intent AST

Example:

```fnl
intent:
    Generate customer forecast
```

AST:

```text
IntentNode
 └── StringLiteral
```

---

# Semantic Binding

The parser produces syntax.

The binder produces meaning.

Parser:

```text
sales <- load.sales
```

Binder:

```text
InputBinding
      │
      ▼
NodeReference
      │
      ▼
Resolved Graph Edge
```

---

# Syntax Tree vs Semantic Graph

Syntax:

```fnl
sales <- load.sales
```

AST:

```text
InputBinding
```

Semantic Graph:

```text
Load.sales
      │
      ▼
CurrentNode.sales
```

These layers remain independent.

---

# Incremental Parsing

A core requirement of Functor.

Only modified regions SHOULD be reparsed.

Example:

```text
Document
```

User edits line:

```text
Revenue
```

Result:

```text
Reparse affected syntax subtree only
```

Not entire notebook.

---

# Green Tree Architecture

Functor SHOULD adopt immutable Green Trees.

Inspired by:

```text
Roslyn
Swift
Rowan
rust-analyzer
```

---

## Green Tree

Contains:

```text
Structure Only
```

No parent references.

No mutable state.

---

## Red Tree

Contains:

```text
Navigation
Parent Links
Editor Services
```

Layered over Green Tree.

---

# Benefits

Incremental updates become extremely efficient.

Example:

```text
Change 1 Node

Reuse 95% of Tree
```

instead of rebuilding the document.

---

# Error Recovery

Parser MUST recover from invalid input.

Example:

```fnl
output
```

instead of:

```fnl
output:
```

Produces:

```text
Diagnostic
```

while preserving an AST.

---

# Diagnostics

Diagnostics are first-class objects.

```text
Diagnostic
 ├── Severity
 ├── Message
 ├── Span
 └── Code
```

---

# Severity Levels

```text
Error
Warning
Info
Hint
```

---

# Example

```text
FNL001

Missing ':'
```

---

# Semantic Diagnostics

Some errors occur after parsing.

Example:

```fnl
sales <- missing.sales
```

Parser:

```text
Valid Syntax
```

Binder:

```text
Unknown Node Reference
```

Diagnostic:

```text
FNL100
```

---

# Syntax Highlighting

Syntax tokens SHOULD power semantic coloring.

Examples:

```text
Node References
Capabilities
Roles
Types
Keywords
```

---

# Refactoring Support

The AST enables:

```text
Rename Node
Rename Output
Extract Subgraph
Move Node
```

without text searches.

---

# Navigation

Editor navigation SHOULD operate on syntax trees.

Examples:

```text
Go To Definition
Find References
Show Dependencies
```

---

# Graph Projection

The AST supports graph construction.

Example:

```fnl
sales <- load.sales
```

Produces:

```text
Graph Edge
```

without executing code.

---

# Agent Readability

Agents SHOULD consume semantic structures derived from the AST.

Preferred:

```text
Node
Shape
Role
Capability
Intent
```

Not:

```text
Raw Text
```

This improves reliability and understanding.

---

# Parsing Services

Future implementations SHOULD expose:

```text
Parse()
Bind()
Validate()
ProjectGraph()
```

as standalone APIs.

---

# Workspace Model

A notebook belongs to a workspace.

```text
Workspace
 ├── Notebook
 ├── Trees
 ├── Graph
 └── Diagnostics
```

Incremental updates occur at workspace scope.

---

# Language Services

The AST serves as the foundation for:

```text
Autocomplete
Hover
Diagnostics
Formatting
Navigation
Refactoring
Graph Visualization
Agent Assistance
```

---

# Performance Goals

Target characteristics:

```text
Subtree Reuse > 90%
Incremental Parse < Full Parse
Graph Projection Independent of Execution
```

Exact implementation metrics are intentionally unspecified.

---

# Relationship To Other RFCs

---

## RFC-0002

Defines:

```text
Grammar
```

---

## RFC-0003

Defines:

```text
Types
```

---

## RFC-0004

Defines:

```text
Graph Structure
```

---

## RFC-0005

Defines:

```text
Graph Meaning
```

---

## RFC-0008

Defines:

```text
Syntax Representation
```

and:

```text
Incremental Analysis
```

---

# Foundational Principle

Functor treats source text as an editable projection of structured meaning.

The AST becomes the authoritative representation of syntax.

The graph becomes the authoritative representation of computation.

Execution engines operate on graphs.

Editors operate on syntax trees.

Agents operate on semantic models.

All three remain connected while staying independently evolvable.

---

# Conclusion

Functor adopts a lossless, immutable, incrementally parsed AST architecture inspired by modern compiler and IDE systems.

The architecture provides:

```text
Text
  ↓
Tokens
  ↓
AST
  ↓
Semantic Binding
  ↓
Graph
```

This model enables:

- High-performance editing
- Structural navigation
- Semantic tooling
- Graph generation
- AI-assisted understanding
- Runtime-independent execution

while preserving a clear separation between syntax, semantics, and execution.
