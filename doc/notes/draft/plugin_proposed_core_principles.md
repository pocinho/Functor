# Plugin Proposed Core Principles

## Purpose

The existing versioned, capability-based boundary is the correct foundation. The additions below make that boundary operable as plugins are discovered, enabled, upgraded, and eventually supplied by third parties.

## Principles To Add

### 1. A Capability Has Its Own Contract

An API version selects a coherent public surface, but individual capabilities must also have a named contract and lifecycle status. A plugin should declare both its API version and the capability contracts it uses.

For example, a plugin targeting API v2 might require `Diagnostics@1` and optionally use `Tokenization@1`. A later `Diagnostics@2` can introduce a breaking semantic change without forcing unrelated capabilities to change.

This avoids treating every new feature as a new host API version while retaining explicit upgrades when behavior changes.

### 2. Compatibility Is Negotiated Before Loading Code

The host must decide whether a plugin is compatible before it loads the plugin assembly or starts its process. Compatibility requires all of the following:

- a supported API version;
- supported required capability contracts;
- granted permissions;
- a compatible runtime and platform;
- no manifest or contribution conflicts.

The result must be a structured reason visible to the user and logs. A plugin that cannot activate is `Incompatible`, not `Failed`; `Failed` is reserved for a compatible plugin that throws or violates its runtime contract.

### 3. Manifests Are Declarative And Complete

The manifest is the source of truth for identity, compatibility, permissions, activation events, and declared contributions. Loading plugin code must not be required to discover what it requests.

At minimum, it should declare a stable publisher-qualified id, package version, API version or supported range, required and optional capabilities, permissions, activation events, and supported platforms. Language addons should also declare language ids and file associations as data.

This keeps discovery safe, lets Functor present an accurate enablement UI, and enables validation without executing third-party code.

### 4. Activation Must Be Deterministic And Idempotent

Each activation event is evaluated against the manifest, and a plugin is activated at most once for its current enabled session. Repeating an event cannot register duplicate providers, commands, or subscriptions.

Activation must have a defined timeout and cancellation path. The host records the activation cause and makes activation state observable. A plugin cannot activate merely by being installed, except when the user explicitly requests it or its manifest declares an approved startup activation event.

### 5. Registrations Are Leases Owned By The Host

Every command, provider, event subscription, process, view descriptor, and background task created for a plugin must have a host-owned registration handle. Disablement, failure, or replacement revokes all handles as one operation.

Plugins must not retain access to capability services after their lifecycle cancellation token is cancelled. Results submitted after revocation are rejected. This makes disablement reliable even when a plugin is slow, faulty, or restarted.

### 6. Async Work Is Correlated, Bounded, And Cancellable

All asynchronous capability operations need a request id, plugin id, document identity and revision when document-scoped, a cancellation token, and bounded payload sizes. The host rejects late, stale, or unowned results.

The host also owns concurrency limits and timeouts. A plugin must not be able to consume unbounded threads, memory, document snapshots, or diagnostic output. Exact limits are host policy, not part of the cross-platform plugin semantics.

### 7. Provider Selection Is Explicit And Explainable

For capabilities that can have multiple providers, such as tokenization or diagnostics, selection must be deterministic. The host should define precedence, provider scope, token layer merge rules, and fallback behavior.

The selected provider and the reason it won should be observable in diagnostics. Plugins must not depend on registration order or silently merge competing results unless that capability contract explicitly allows it.

### 8. Permissions Are Narrow, User-Visible, And Revocable

Capabilities describe what a plugin can contribute; permissions describe what resources it may access. Filesystem scope, process launch, network access, workspace data, and secret access require separate permission grants.

Permissions are granted per plugin, shown to the user before activation, and can be revoked. An in-process first-party plugin does not remove the need for the same API-level checks. The initial implementation may support a minimal permission set, but the model should exist from the first manifest format.

### 9. Observability Is Part Of The Plugin Contract

The host must publish structured plugin status, activation/deactivation events, compatibility reasons, and plugin-scoped logs. Diagnostics should always identify the plugin id, capability, operation, and failure category without exposing protected document content by default.

This is necessary for supportability and for a plugin manager UI. It also provides the evidence needed to distinguish host failures from plugin failures.

### 10. Configuration And Persistence Are Namespaced

Plugin configuration must be namespaced by stable plugin id and versioned independently from the host settings schema. Plugins receive only their own configuration and should supply explicit migration logic when their configuration contract changes.

Enablement state is host-owned and should survive restart independently of whether a plugin is currently compatible. An incompatible plugin remains installed and disabled with a clear remediation path rather than losing its configuration.

### 11. Public Contracts Must Be Frontend-Neutral

The API should use platform-neutral data transfer types and declarative contributions. It must not expose Avalonia controls, renderer objects, or UI-thread assumptions. A future UI contribution capability should describe panels, commands, state, and events; the host frontend renders them.

This preserves the existing goal of supporting desktop, browser, mobile, and headless use without exposing distinct plugin APIs for each frontend.

### 12. Trust Boundaries Must Be Honest

Separate API isolation, lifecycle isolation, failure isolation, and process isolation in both documentation and implementation. Assembly boundaries do not sandbox code. In-process plugins are trusted code; untrusted or executable-backed extensions should eventually run out of process.

The public API must not promise security guarantees that the chosen execution model cannot enforce.

## Principles To Retain As Written

The following existing principles should remain central and should not be weakened:

- plugins use only host-mediated, versioned capabilities;
- Functor retains authority over internal state transitions and rendering;
- document-scoped results are revision-aware and stale results are rejected;
- enable, disable, cancellation, disposal, and failure isolation are mandatory lifecycle behavior;
- language support is the first proof of the architecture, with token providers as the initial integration seam;
- LSP is a language-addon capability, not the plugin architecture itself.

## Do Not Add Yet

The following would add complexity without validating the core contract and should remain explicit non-goals for the first implementation:

- a general plugin dependency graph or plugin-to-plugin service imports;
- marketplace installation, signatures, remote updates, or package resolution;
- arbitrary UI control injection or plugin-defined shell layouts;
- an unrestricted document-write capability;
- a shared global event stream or internal MVU message dispatch;
- mandated process isolation for the first trusted, first-party plugin slice;
- a universal permission prompt system before any capability requires the relevant resource access;
- hot reload guarantees for arbitrary plugin implementations.

Plugin dependencies in particular should be avoided until real addons demonstrate a need. Depend on stable host capabilities rather than another plugin's implementation or activation order.

## Recommended Contract Shape

The public manifest and context can evolve toward this shape:

```text
PluginManifest
  id, publisher, packageVersion
  supportedApiVersions
  requiredCapabilities, optionalCapabilities
  permissions, activationEvents, platforms
  declarative contributions

PluginContext
  selectedApiVersion
  granted capabilities and permissions
  plugin-scoped logger
  lifecycle cancellation token
  registration factory

Host lifecycle
  discover -> validate -> enable -> activate -> revoke -> dispose
```

`supportedApiVersions` is preferable to a single version once parallel support is implemented. The host selects one version and exposes only that version's contract for the session. This allows a plugin to support v1 during migration and v2 after it has been updated, without receiving a blended surface.

## Prioritized Next Steps

1. Add a manifest model and compatibility validator for API and capability contracts.
2. Implement an in-memory lifecycle registry with host-owned registration handles and cancellation.
3. Add tests for compatibility classification, idempotent activation, revocation, stale-result rejection, and isolated failures.
4. Convert one built-in tokenizer into a registered first-party provider, retaining the built-in fallback.
5. Add permissions and configuration namespaces when the first resource-accessing capability is introduced.

These steps retain the narrow language-provider proof while ensuring it will not become an accidental public dependency on Functor internals.