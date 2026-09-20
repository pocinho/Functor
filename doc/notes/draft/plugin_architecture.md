# Functor Plugin Architecture

## Purpose

This document is a practical design baseline for discussing plugin support in Functor. It describes principles and boundaries that should guide implementation without claiming that the full plugin system already exists or that every detail is settled.

Functor should remain useful as an editor without plugins enabled. Plugins should add optional capabilities without becoming owners of Functor's document, workspace, rendering, or application state.

The first implementation should validate the boundary with a small language-provider scenario. Broader capabilities should be added only after the host can reliably manage registration, cancellation, disposal, validation, and failure isolation.

## Architectural Position

Functor's domain and application layers remain authoritative for editor behavior and state transitions. MVU-style commands, effects, and state management are internal implementation mechanisms, not the plugin contract.

A plugin must not receive or mutate `AppSessionState`, `WorkspaceModel`, `CoreModel`, internal domain events, or an unrestricted dispatcher. The host translates approved plugin requests into the internal application flow and validates them before they can affect editor state.

The plugin boundary should be:

- capability-based rather than a general shared-model runtime;
- host-mediated rather than direct access to internal state;
- versioned before external compatibility is promised;
- independent of Avalonia and other frontend-specific types;
- narrow enough to test without starting a frontend.

## Core Principles

### 1. Plugins Are Optional Addons

Language support, language servers, diagnostics, debuggers, notebooks, agents, and specialized workflows should be independently optional where practical. The editor core must retain its basic document and workspace behavior when no addon is active.

The first plugin family should be language support. It directly exercises the most useful existing seam while keeping the contribution narrow and observable.

### 2. Capabilities Are the Only Extension Surface

A plugin receives only the capability contracts granted to it. A capability describes a specific contribution or service, such as tokenization, diagnostics, commands, or read-only document access.

Capabilities should have their own names, versions, lifecycle rules, validation rules, and status. Adding a capability must not silently expand the contract of an existing capability.

A plugin manifest or equivalent host metadata should declare, before activation:

- stable plugin identity and package version;
- supported host and capability contracts;
- required and optional capabilities;
- declared contributions;
- activation conditions and supported platforms;
- permissions or resource requirements.

The host should reject incompatible or conflicting declarations before loading plugin code or starting a plugin process.

### 3. The Host Owns State Transitions

Plugins may request supported operations through capability services. They may not dispatch arbitrary internal messages, install domain events, or mutate Functor models directly.

The host owns:

- document and workspace identity;
- validation of ranges, revisions, and payloads;
- translation into internal commands or effects;
- persistence and rendering;
- provider selection and fallback;
- lifecycle, cancellation, and disposal.

This allows Functor to refactor its internal MVU implementation without making every internal model change a plugin API breaking change.

### 4. Public Contracts Are Frontend-Neutral

Public plugin contracts should use platform-neutral data and declarative contributions. They must not expose Avalonia controls, renderer objects, UI-thread assumptions, or frontend-specific model types.

Future views or panels should be described by host-owned descriptors and rendered by the active frontend. Arbitrary control injection is outside the initial design.

### 5. Every Registration Has Host-Owned Lifetime

Commands, providers, subscriptions, views, processes, and background work created for a plugin must be owned by the host and associated with the plugin activation.

Registration returns a revocable handle where appropriate. Disablement, activation failure, replacement, or shutdown must be able to revoke the plugin's registrations as one operation. A plugin must not retain usable capability services after its lifecycle has ended.

### 6. Asynchronous Work Is Correlated and Revision-Aware

Document-scoped requests must carry document identity and the document revision that was analyzed. Requests should also have an operation identity, cancellation token, and bounded input and output sizes where applicable.

The host must reject results that are:

- stale for the current document revision;
- submitted after registration or lifecycle revocation;
- associated with an unknown document or request;
- outside the plugin's granted capability;
- invalid or larger than the host policy allows.

Cancellation is normal lifecycle behavior. Disabling a plugin, closing a document, superseding a request, or shutting down the host must stop accepting new work and cancel relevant in-flight work.

### 7. Provider Selection Is Deterministic

Capabilities with multiple possible providers must define selection, precedence, scope, and fallback behavior. Registration order must not decide behavior accidentally.

For language tokenization, a practical initial order is:

1. an enabled, explicitly selected provider;
2. another enabled addon provider;
3. a built-in local provider;
4. no provider, represented as an unsupported capability.

Competing providers must not be silently merged unless the capability contract defines merge semantics. The selected provider and the reason for selection should be available to diagnostics or logging.

### 8. Failures Are Isolated and Observable

A plugin exception, timeout, invalid result, or process failure must become a plugin-scoped host diagnostic. It must not corrupt editor state, terminate unrelated plugins, or take down the editor where containment is possible.

The host should record structured status for discovery, compatibility, activation, runtime failure, deactivation, and disposal. Logs should identify the plugin, capability, operation, and failure category without exposing protected document content by default.

Compatibility failure and runtime failure are different states:

- `Incompatible` means the host rejected the plugin before activation.
- `Failed` means a compatible plugin failed during activation or use.

### 9. Permissions and Trust Must Be Honest

Capabilities and permissions are separate concerns. A plugin may contribute a provider without automatically receiving filesystem, network, process, workspace, or secret access.

In-process F#/.NET plugins may be suitable for trusted first-party development, but an assembly boundary is not a security sandbox. The initial host must not promise third-party isolation that it cannot enforce.

Process isolation can be introduced for untrusted or executable-backed extensions, especially language servers, when the product and implementation require it. API isolation, lifecycle isolation, and failure isolation are required regardless of execution model.

### 10. Configuration and Enablement Belong to the Host

Plugin enablement, disablement, status, and configuration ownership belong to the host. Configuration keys must be namespaced by stable plugin identity and must not silently collide with Functor settings.

Disabling a plugin must stop new activation and requests, cancel its work, revoke its registrations, dispose owned resources, and allow affected features to fall back where possible. Re-enabling must create a fresh activation rather than reuse stale registrations.

## Initial Capability: Language Providers

The first useful proof should be a language provider, preferably one small first-party provider such as JSON or Markdown.

A tokenization provider should receive a host-defined request containing, at minimum:

- document identity;
- document revision;
- normalized language identity;
- tokenization scope;
- document lines or an approved snapshot;
- lexer state;
- cancellation.

Its result should identify the provider and token layer and contain tokens plus any state needed for incremental tokenization. The host must pass request identity and cancellation through unchanged where the implementation supports the existing contract, then route accepted results through the normal application completion path.

Built-in tokenizers should remain valid fallback providers. The first provider registry should make selection and unregistration testable without requiring Avalonia, assembly discovery, a marketplace, or a real language server.

Diagnostics, completion, hover, formatting, symbols, and other language features may become separate capabilities later. They should not be smuggled into the tokenization contract.

## Lifecycle Baseline

The host lifecycle should have explicit states and ownership:

```text
discover -> validate -> enable -> activate -> operate -> revoke -> dispose
```

The exact implementation may begin in memory and with manually composed first-party code. It should still establish these invariants:

- compatibility is checked before activation;
- activation is deterministic and idempotent for an enabled session;
- every resource and registration has an owner;
- lifecycle cancellation reaches plugin work;
- revocation prevents late results from being accepted;
- disposal is safe to repeat;
- failures are attributed to the owning plugin and capability.

Discovery, dynamic assembly loading, persisted enablement, and package installation should be added only when the host contract and lifecycle behavior have been demonstrated locally.

## Future Contributions

Commands, diagnostics, document access, language-server integration, declarative panels, configuration, and process-backed providers can be added as separate capabilities. Each addition should define its own request and result types, validation, permissions, cancellation, disposal, and compatibility status.

Notebook and agent features should consume the same host-mediated capability model if they become plugins. They should not require a global shared model or unrestricted access to internal messages.

## Explicit Non-Goals for the First Implementation

The first implementation should not attempt to provide:

- a general shared-MVU plugin runtime;
- arbitrary model observation or mutation;
- unrestricted internal message dispatch;
- arbitrary Avalonia control injection;
- plugin-defined domain events;
- plugin-to-plugin service dependencies;
- marketplace installation, signatures, or automatic updates;
- hot reload guarantees for arbitrary implementations;
- a universal permission system before a capability needs protected resources;
- full LSP coverage;
- notebook or agent execution as the first proof;
- security sandboxing for trusted in-process assemblies.

## Validation Before Expansion

Before adding broader capabilities, tests should demonstrate:

- compatibility and contribution validation;
- deterministic provider selection and built-in fallback;
- propagation of document identity, revision, scope, lexer state, and cancellation;
- registration disposal and disablement;
- cancellation of owned work;
- rejection of stale and revoked results;
- idempotent activation and disposal;
- namespaced command collision handling when commands are introduced;
- attribution and isolation of provider or plugin failures.

These tests are the evidence for evolving the design. They are not a promise that every capability listed in this document is already implemented.

## Decision Rule

When a proposed plugin feature is discussed, first ask whether it can be expressed as a small, host-mediated capability without exposing Functor internals. If it cannot, define the missing boundary and its lifecycle and validation rules before adding the feature.

The plugin architecture should grow from proven capabilities rather than from a single broad interface that grants access to the editor as a whole.
