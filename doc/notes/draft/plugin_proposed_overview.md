# Plugin Proposed Overview

## Purpose

Plugins extend Functor through a small public API. The API is versioned and capability-based: a plugin targets one Functor Plugin API version and can use only the capabilities that version exposes.

Functor's domain, application state, MVU messages, and Avalonia controls are implementation details. Plugins do not receive a mutable model, an internal dispatcher, or arbitrary view injection. The host validates every contribution and translates accepted requests into Functor's internal state flow.

## API Versions

A plugin declares an API version in its manifest. Functor selects and validates that contract before activation.

Example:

| API version | Status | Capabilities |
| --- | --- | --- |
| v1 | Deprecated; supported for migration | Diagnostics |
| v2 | Current | Diagnostics, Commands, Tokenization, DocumentRead |

An API version is a complete contract, not a label for the host build. A new version can add capabilities or make a breaking contract change. Plugins stay on their declared version until they explicitly upgrade. The host may support several versions in parallel while an older version is deprecated.

Deprecation means the host continues to activate compatible plugins, but reports the replacement version. Removing a deprecated version requires a later breaking host release; it must never silently change the meaning of the old contract.

## Capabilities

Capabilities are the sole extension surface. A manifest requests the capabilities it needs, and the host grants only those available in the selected API version.

- `Diagnostics`: report revision-aware diagnostics for an open document.
- `Commands`: contribute namespaced commands.
- `Tokenization`: provide lexical or semantic token results.
- `DocumentRead`: read immutable document snapshots supplied by the host.

Capability methods must carry document identity and revision where applicable. The host rejects stale, invalid, oversized, or unauthorized requests. The host owns rendering, persistence, cancellation, and disposal.

## Lifecycle

1. Discover a plugin and validate its manifest, version, and requested capabilities.
2. Activate it with a `PluginContext` containing only its permitted capabilities and a cancellation token.
3. Record registrations and resources as owned by that plugin.
4. On disablement, stop new work, cancel active work, remove registrations, dispose the plugin, and fall back to other providers.

An exception during activation or use transitions only that plugin to `Failed`; it must not stop Functor or unrelated plugins.

## Minimal Public Contract

The public surface is intentionally small:

```text
PluginManifest: id, display name, plugin package version,
				API version, required capabilities

IPlugin:       Manifest, ActivateAsync(context), Dispose()

PluginContext: selected API version, granted capability services,
				lifecycle cancellation token
```

The current `Functor.PluginHost` scaffold implements these concepts as an API boundary and validates that a plugin requests capabilities available in its declared version. It deliberately does not yet load assemblies, expose views, or permit state mutation.

## Example: v1 Diagnostics

A v1 language plugin requests `Diagnostics`. For an analysis result it reports a document id, the revision it analyzed, and a message. Functor validates the capability and revision, then updates its own diagnostics state and rendering. The plugin does not know how squiggles, panels, or accessibility output are produced.

## Evolution Rules

- Add a capability in a new API version rather than expanding an old contract silently.
- Keep each version's capability semantics stable for its support lifetime.
- Mark old versions deprecated with an explicit replacement version.
- Require plugins to update their manifest and implementation to opt into a newer contract.
- Keep internal refactors behind the host boundary.

Future work can add provider registration, commands, declarative UI contributions, file permissions, and process-isolated language servers, each as an explicit versioned capability rather than access to Functor internals.