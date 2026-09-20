# Plugin Implementation Startup

This roadmap defines the smallest useful first plugin system for Functor. It is intentionally narrower than a general plugin platform.

The initial system supports only curated, first-party language token providers written in compatible .NET languages. Providers are trusted application code and are manually composed into Functor. There is no user plugin installation, dynamic assembly loading, sandbox, marketplace, WASM runtime, or general extension API in this phase.

## Startup Policy

- [ ] Treat the initial plugins as Functor-owned, reviewed, and released with Functor.
- [ ] Do not load plugin assemblies from user-controlled directories.
- [ ] Do not provide a plugin installation or marketplace workflow.
- [ ] Register approved providers explicitly in the application composition root.
- [ ] Keep the first capability limited to language tokenization.
- [ ] Keep built-in tokenizers available as fallback providers.
- [ ] Clearly document that this phase accepts trusted code only and does not sandbox plugins.
- [ ] Notify users that arbitrary third-party .NET plugins must not be used with this phase.
- [ ] Revisit external or untrusted plugin support only as a separate security project.

This policy makes the initial user experience simple: users receive the recommended Functor plugins as part of the application or an official release, and do not need to evaluate arbitrary plugin code themselves.

## Scope of the First System

A language token provider may:

- [ ] receive an immutable tokenization request;
- [ ] receive document identity, revision, language, scope, lines, and lexer state;
- [ ] observe the host cancellation token;
- [ ] return tokens, snapshots, token layer, and final lexer state;
- [ ] return a defined failure result.

A language token provider may not initially:

- [ ] mutate documents or editor state;
- [ ] dispatch internal MVU messages or application commands;
- [ ] receive `AppSessionState`, `WorkspaceModel`, `CoreModel`, or domain models;
- [ ] register commands, views, panels, settings, or key bindings;
- [ ] access Functor filesystem, network, process, secret, or workspace services;
- [ ] communicate with other plugins;
- [ ] install or discover other plugins.

## Step 1: Freeze the Minimal Contract

- [ ] Define one provider contract for language tokenization.
- [ ] Keep the contract frontend-neutral and independent of Avalonia.
- [ ] Use immutable request and result data.
- [ ] Define the provider identity used for diagnostics and test assertions.
- [ ] Preserve document id and revision through the provider call.
- [ ] Preserve language, scope, lines, initial lexer state, token layer, snapshots, and final state.
- [ ] Preserve cancellation propagation.
- [ ] Keep the public contract usable from F#, C#, Visual Basic, and other compatible .NET languages.
- [ ] Avoid exposing Functor application types, internal domain events, or F#-specific implementation details as the plugin contract.
- [ ] Avoid adding a general manifest, permissions model, or lifecycle interface before the one capability is proven.

## Step 2: Keep Composition Explicit

- [ ] Create the provider registry or adapter in the application-owned integration boundary or `Functor.PluginHost`.
- [ ] Register only providers compiled and reviewed as part of the Functor solution.
- [ ] Give each provider a stable internal id and declared language ids.
- [ ] Reject duplicate provider identities and invalid language declarations.
- [ ] Normalize language ids at the registry boundary.
- [ ] Make registration disposal possible for tests and future disablement, even though startup plugins are not user-toggleable yet.
- [ ] Do not add assembly scanning, reflection-based discovery, package probing, or plugin directories.
- [ ] Do not make plugin availability depend on arbitrary runtime files.

## Step 3: Add Deterministic Provider Selection

- [ ] Route language tokenization through the provider registry or adapter.
- [ ] Select at most one provider for each language and token layer.
- [ ] Define selection precedence explicitly.
- [ ] Reject ambiguous provider registrations rather than relying on registration order.
- [ ] Keep built-in providers as the fallback for languages not supplied by a first-party provider.
- [ ] Return an explicit unsupported-language result when no provider exists.
- [ ] Record the selected provider in host diagnostics or test-visible results.
- [ ] Do not merge competing token providers in the startup phase.

Recommended startup precedence:

1. Curated first-party provider registered for the requested language and layer.
2. Built-in provider.
3. Unsupported language or layer.

## Step 4: Preserve Existing Editor Behavior

- [ ] Keep `TokenizationCoordinator` as the path that dispatches accepted results to the application.
- [ ] Preserve existing document and revision validation in `EditorSession`.
- [ ] Ensure provider output is translated to the existing internal tokenization result only at the application boundary.
- [ ] Preserve the current behavior of F#, C#, JSON, and Markdown tokenization.
- [ ] Migrate one small provider first, preferably JSON or Markdown.
- [ ] Keep the remaining built-in tokenizers unchanged until the first migration is stable.
- [ ] Verify that the editor works normally when no additional provider is selected.

## Step 5: Handle Cancellation and Failures Simply

- [ ] Pass the existing cancellation token to every provider request.
- [ ] Treat cancellation as normal control flow, not as an editor error.
- [ ] Cancel or invalidate work when a newer tokenization request supersedes it.
- [ ] Reject completion for a closed document or changed revision.
- [ ] Reject completion after a provider registration is disposed.
- [ ] Catch provider exceptions at the host boundary.
- [ ] Convert provider exceptions into an attributed tokenization failure.
- [ ] Ensure one provider failure does not terminate the editor or prevent later fallback requests.
- [ ] Avoid adding retries, background scheduling, or complex supervision until a provider requires them.

## Step 6: Add Focused Tests

- [ ] Test registration of a curated provider.
- [ ] Test duplicate provider identity rejection.
- [ ] Test language-id normalization.
- [ ] Test provider selection for supported language and token layer.
- [ ] Test built-in fallback.
- [ ] Test unsupported language behavior.
- [ ] Test propagation of document id, revision, language, scope, lines, lexer state, and cancellation.
- [ ] Test provider unregistration or disposal.
- [ ] Test stale completion rejection after a revision changes.
- [ ] Test cancellation behavior.
- [ ] Test provider exception containment and attribution.
- [ ] Test that the existing tokenization completion path remains unchanged for built-in providers.
- [ ] Run these tests without Avalonia, dynamic loading, external processes, or a real language server.

## Step 7: Curate and Ship Providers

- [ ] Define a small review checklist for first-party providers.
- [ ] Require provider code to remain within the tokenization contract.
- [ ] Review allocations, cancellation behavior, failure behavior, and language declarations.
- [ ] Add provider tests before including a provider in an official build.
- [ ] Prefer providers that are deterministic and local, with no filesystem, network, or process access.
- [ ] Ship approved providers as part of the Functor application or an official, version-matched package.
- [ ] Document the provider list and supported languages for each Functor release.
- [ ] Do not promise compatibility for independently compiled third-party plugins yet.

## Startup Completion Criteria

The startup system is complete when:

- [ ] Functor can explicitly compose at least one curated first-party token provider.
- [ ] The provider can be selected for its declared language and layer.
- [ ] Built-in tokenizers remain reliable fallbacks.
- [ ] Requests preserve document identity, revision, scope, state, and cancellation.
- [ ] Stale and disposed-provider results are rejected.
- [ ] Provider failures are contained and attributed.
- [ ] No provider receives unrestricted editor state or internal message access.
- [ ] Users are clearly warned that this trusted in-process model is not suitable for arbitrary untrusted plugins.

## Deliberately Deferred

The following are outside this startup roadmap:

- [ ] external plugin packages and user installation;
- [ ] dynamic assembly loading and discovery;
- [ ] plugin manifests and dependency resolution;
- [ ] persisted enablement and a plugin manager;
- [ ] diagnostics, commands, views, settings, and document editing capabilities;
- [ ] LSP process management;
- [ ] arbitrary Avalonia control injection;
- [ ] plugin-to-plugin communication;
- [ ] third-party trust or security enforcement;
- [ ] marketplace, updates, signatures, and package management;
- [ ] hot reload;
- [ ] WASM support.

These features may become separate projects later. They should not enlarge the startup contract before the narrow token-provider path has proved useful, understandable, and maintainable.

## Practical Recommendation

Keep the startup system closer to a curated provider registry than to a conventional user-installed plugin platform. This gives Functor the benefit of a real extension seam while keeping the security story honest and the user experience uncomplicated: official providers are reviewed and shipped with the editor, while arbitrary plugins remain unsupported and explicitly untrusted until a separate isolation design exists.
