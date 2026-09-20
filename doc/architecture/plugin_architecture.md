# Functor Plugin Architecture

## Purpose

Functor should support plugins as optional, capability-based extensions to the editor. The architecture must be broad enough for future language services, commands, diagnostics, views, notebooks, agents, and other editor features, while keeping the first implementation small and trustworthy.

The plugin API is a host boundary, not a second editor core. Functor remains responsible for its document, workspace, application, and rendering state. Plugins contribute through capabilities that the host defines, validates, and owns.

## Architectural Principles

### 1. Plugins Extend Capabilities, Not the Internal Model

Plugins must not receive or mutate `AppSessionState`, `WorkspaceModel`, `CoreModel`, internal domain events, or the internal MVU message stream. They must not receive an unrestricted dispatcher or a live editor model subscription.

The MVU and application flows remain Functor implementation details. The host translates valid plugin requests into internal commands or effects and decides whether those requests are accepted.

This preserves the draft's goals of deterministic state transitions and a single source of truth without making the current internal model part of the plugin contract.

### 2. Capabilities Are the Extension Surface

A capability is a bounded contribution or service with its own request types, result types, validation rules, lifecycle, and version. Examples include:

- language tokenization;
- diagnostics;
- commands;
- read-only document access;
- language-server integration;
- declarative views or panels;
- notebook or agent services.

These are possible future capabilities, not a promise that they are currently available. Each capability must be justified and implemented independently.

Plugins receive only the capabilities granted to them. A plugin must not gain access to unrelated Functor services merely because it implements one capability.

### 3. The API Is Versioned

The Plugin API is a versioned public contract. A plugin declares the API and capability versions it supports, and the host selects a compatible contract before activation.

Versioning rules:

- a version defines stable observable behavior, not the host's internal build number;
- capability versions may evolve independently where practical;
- additive changes are preferred;
- changes to required meaning or safety guarantees require an incompatible version;
- deprecated versions remain explicit until support is intentionally removed;
- internal Functor refactoring does not require a new API version unless a public invariant changes.

The first implementation may use an internal or first-party contract namespace while these semantics are proven. It should not claim external compatibility before the contract is deliberately stabilized.

### 4. Public Types Are Frontend-Neutral

The contract must not expose Avalonia controls, renderer objects, UI-thread assumptions, or frontend-specific models. Views and panels, if added later, should be declarative contributions rendered by the active frontend.

Public data should be immutable, ordinary, and usable from F#, C#, Visual Basic, and other compatible .NET languages. It should be serializable in principle so another execution adapter can be added later without changing capability meaning.

### 5. The Host Owns Lifecycle and Resources

The host owns plugin activation, capability registration, cancellation, disposal, and failure state. Every provider, command, subscription, view descriptor, process, or background operation must be associated with its plugin activation and revocable by the host.

A future lifecycle may be represented as:

```text
discover -> validate -> enable -> activate -> operate -> revoke -> dispose
```

The initial implementation may omit discovery and persistence, but it should still establish ownership and safe disposal for manually composed providers.

Activation should be deterministic and idempotent. Repeating activation must not create duplicate registrations. Revocation must stop new work, cancel owned work, and prevent late results from being accepted.

### 6. Requests and Results Are Validated

Capability operations must carry enough context for the host to validate them. Document-scoped work should include document identity and revision. Asynchronous work should also have cancellation and, where needed, an operation identity.

The host must reject results that are stale, revoked, invalid, unauthorized, or outside declared limits. It owns payload, range, concurrency, and timeout policy as appropriate for each capability.

### 7. Failures Are Isolated and Observable

Plugin exceptions, invalid results, timeouts, and process failures must become attributed host diagnostics. They must not corrupt editor state or terminate unrelated capabilities where containment is possible.

Status and logs should identify the plugin, capability, operation, and failure category without exposing document content by default. Compatibility failure must be distinguishable from runtime failure.

### 8. Trust Boundaries Must Be Explicit

An in-process .NET plugin is trusted code. Interfaces, assemblies, and capability restrictions provide API isolation for well-behaved plugins, but they do not sandbox malicious or compromised code from filesystem, network, process, native, CPU, or memory access available to the Functor process.

The initial supported plugins should therefore be Functor-owned or explicitly trusted. Users should be warned not to run arbitrary third-party plugins. Supporting untrusted plugins requires a separate execution and security design.

## Plugin Shape

The eventual contract may provide a small plugin lifecycle and context rather than a broad editor interface:

```text
PluginManifest
  id, publisher, package version
  supported API and capability versions
  declared capabilities and contributions
  activation conditions and permissions

IPlugin
  manifest
  Activate(context)
  Dispose()

PluginContext
  selected API version
  granted capability services
  lifecycle cancellation
  host-owned registration factory
```

This is a direction for the versioned contract, not a requirement to implement every field immediately. The context must not expose unrestricted model access or arbitrary internal message dispatch.

## Initial Capability: Language Tokenization

Tokenization is the first implementation seam because Functor already has a request, provider output, cancellation path, and revision-aware completion flow.

A token provider receives an immutable request containing, at minimum:

- document identity;
- document revision;
- language identity;
- tokenization scope;
- document lines or an approved snapshot;
- lexer state;
- cancellation.

It returns tokens, token layer, snapshots, final lexer state, or a defined failure. The host passes the request through, validates the result, and routes accepted output through the existing application completion path.

Built-in tokenizers remain providers and fallbacks. The first registry should support explicit, curated provider selection and one provider per language and token layer. It should reject ambiguous registrations rather than merge competing results.

Users may select a provider for the language they are working with, such as F#, while other providers remain inactive. If providers are compiled into the application, inactive means they are not invoked or initialized for work; it does not necessarily mean their assemblies are absent from process memory.

The first implementation should be manually composed and should not require dynamic assembly discovery, user plugin directories, manifests, marketplace support, or a plugin manager.

## Future Capability Families

The architecture may later support the following through separate versioned capabilities:

- **Language services:** diagnostics, completion, hover, symbols, formatting, and language-server integration.
- **Commands:** namespaced commands with host-owned registration and invocation.
- **Documents:** approved read-only snapshots and, only when justified, validated host-mediated edits.
- **Configuration:** namespaced plugin settings owned and persisted by the host.
- **Views:** declarative panels, inspectors, logs, and visualizations rendered by each frontend.
- **Notebook and agent services:** execution and workflow capabilities that do not require access to the global editor model.

No future capability should silently expand an existing contract. It must define its own authority, validation, lifecycle, cancellation, disposal, and failure behavior.

## Multi-Frontend Support

The host contract should remain independent of a particular frontend. Desktop, browser, mobile, CLI, notebook, and agent integrations may use different drivers or adapters, but plugins should communicate through the same capability semantics.

Drivers render or operate on host-owned projections. They do not give plugins direct control over frontend state or widget trees.

## Initial Trust and Composition Model

The first plugins are reviewed, Functor-owned .NET code. They may be written in F#, C#, Visual Basic, or another compatible .NET language. They are explicitly registered in the composition root or an application-owned plugin-host module.

The initial system must not load assemblies from user-controlled locations or claim security isolation. Official providers can be shipped with Functor or in an official version-matched package, giving users recommended features without asking them to evaluate arbitrary code.

## Initial Validation Criteria

Before adding a second capability, tests should demonstrate:

- contract and provider identity validation;
- deterministic provider selection and built-in fallback;
- propagation of document identity, revision, scope, lexer state, and cancellation;
- provider registration and disposal;
- cancellation and rejection of revoked or stale results;
- exception containment and provider attribution;
- preservation of the existing editor tokenization path;
- no access to internal state or arbitrary MVU messages.

## Non-Goals of the First Implementation

The first implementation does not include:

- a general shared-MVU plugin runtime;
- arbitrary model observation or mutation;
- unrestricted internal message dispatch;
- arbitrary Avalonia control injection;
- user-installed or dynamically discovered plugins;
- a marketplace, updates, or dependency resolver;
- untrusted plugin execution or sandboxing;
- plugin-to-plugin services;
- full LSP support;
- notebook or agent execution as the first proof;
- alternate plugin runtimes.

These are possible future projects, not assumptions required by the initial token-provider implementation.

## Design Rule

Keep the public API broad in concept but narrow in authority: versioned capabilities may grow over time, while every capability remains host-mediated, validated, cancellable, disposable, and isolated from Functor's internal state.
