# Plugin Implementation

This checklist turns the plugin architecture into an incremental implementation plan. It covers the initial trusted .NET plugin system only. The first proof is a manually composed language token provider; it is not a marketplace, a general shared-MVU runtime, or a security sandbox.

## Definition of Done for the Initial System

- [ ] A trusted first-party .NET provider can be registered, selected, invoked, disabled, and disposed.
- [ ] Built-in tokenizers remain available as deterministic fallbacks.
- [ ] Plugin requests and results preserve document identity, revision, scope, state, and cancellation.
- [ ] Stale, revoked, invalid, and failed provider results cannot corrupt editor state.
- [ ] The host attributes failures and owns all registrations and in-flight work.
- [ ] The behavior is covered by focused tests without requiring Avalonia, assembly discovery, or a real language server.
- [ ] The public contract does not expose internal Functor state, MVU messages, Avalonia types, or F# implementation details accidentally.

## Step 1: Confirm the Boundary

- [ ] Record the initial execution model as trusted, in-process .NET code.
- [ ] Keep the contract usable from F#, C#, Visual Basic, and other compatible .NET languages.
- [ ] Define the first capability as language tokenization only.
- [ ] Define tokenization request and result data independently from `AppSessionState`, `WorkspaceModel`, `CoreModel`, and internal domain events.
- [ ] Decide which request and result fields are required: plugin/provider identity, document identity, revision, language, scope, lines or snapshot, lexer state, token layer, tokens, snapshots, final state, and cancellation.
- [ ] Define capability and contract identifiers with an explicit versioning policy.
- [ ] Define host failure categories for invalid input, cancellation, stale results, provider failure, incompatibility, and disposal.
- [ ] Document that in-process .NET plugins are not sandboxed.
- [ ] Record the future requirement that public capability data must be serializable in principle for a later WASM adapter.

## Step 2: Establish the Contract Project Boundary

- [ ] Place stable plugin contract types in a frontend-neutral project or namespace with minimal dependencies.
- [ ] Keep Avalonia, renderer objects, application state, domain events, and platform services out of the contract project.
- [ ] Prefer ordinary interoperable .NET types and documented nullability, ordering, and async behavior.
- [ ] Avoid exposing F#-specific representations, internal discriminated unions, delegates with unclear lifetime, or shared mutable object graphs.
- [ ] Define immutable or host-owned request and result data.
- [ ] Define registration handles and lifecycle cancellation as host-owned concepts.
- [ ] Add contract tests that can be consumed from at least one non-F# .NET test fixture when practical.

## Step 3: Build the In-Memory Host Registry

- [ ] Create an in-memory registry for manually composed providers.
- [ ] Validate plugin identity, provider identity, language declarations, token layers, and duplicate registrations.
- [ ] Normalize language identifiers consistently at the host boundary.
- [ ] Associate every registration with its owning plugin and capability.
- [ ] Return a disposable or revocable registration handle.
- [ ] Make disposal idempotent.
- [ ] Ensure disposed registrations cannot be resolved for new requests.
- [ ] Define deterministic provider selection and reject ambiguous matches unless precedence explicitly resolves them.
- [ ] Keep built-in providers available as fallback providers.
- [ ] Expose enough selection information for diagnostics or logging without exposing internal state.

## Step 4: Integrate the Tokenization Path

- [ ] Adapt the existing built-in tokenizer dispatcher to the provider registry without changing the editor completion path unnecessarily.
- [ ] Route tokenization requests through the registry while preserving document id and revision.
- [ ] Preserve language, scope, lines or snapshot, initial lexer state, token layer, snapshots, and final state.
- [ ] Pass cancellation through to the selected provider.
- [ ] Translate provider output into the existing internal tokenization result only inside the host/application integration.
- [ ] Preserve the existing editor-session validation that rejects stale document or revision results.
- [ ] Keep unsupported language behavior explicit and observable.
- [ ] Confirm that a provider failure does not remove built-in fallback behavior for later requests.
- [ ] Register one small first-party provider, preferably JSON or Markdown, through the new path.
- [ ] Keep the remaining built-in providers working while the first provider is migrated.

## Step 5: Implement Lifecycle and Cancellation Ownership

- [ ] Define host lifecycle states sufficient for the first system: registered, active, revoked, failed, and disposed.
- [ ] Associate every in-flight request with its plugin, provider, capability, document, revision, and operation identity where needed.
- [ ] Cancel requests when superseded by a newer tokenization request.
- [ ] Cancel or invalidate requests when a document is closed.
- [ ] Cancel and invalidate requests when a provider is unregistered or disabled.
- [ ] Stop accepting new requests after revocation begins.
- [ ] Reject results that arrive after revocation, cancellation, disposal, or ownership loss.
- [ ] Ensure lifecycle disposal is safe to repeat and does not leave registrations or callbacks behind.
- [ ] Define timeouts or bounded execution policy before introducing providers that can block or perform external work.

## Step 6: Contain Failures and Add Observability

- [ ] Catch provider exceptions at the host boundary.
- [ ] Convert exceptions into host-defined failure results rather than allowing them to escape into the editor loop.
- [ ] Attribute every failure to plugin id, provider id, capability, and operation where available.
- [ ] Distinguish cancellation from provider failure in diagnostics.
- [ ] Record structured status for registration, selection, activation, revocation, failure, and disposal.
- [ ] Avoid logging document content or other protected data by default.
- [ ] Verify that one failing provider does not terminate unrelated providers or the editor.
- [ ] Verify that a later request can still use an available fallback provider.

## Step 7: Add Focused Contract and Integration Tests

- [ ] Test valid registration and normalized language resolution.
- [ ] Test invalid registration and duplicate plugin/provider identity rejection.
- [ ] Test token layer declarations and unsupported language/layer behavior.
- [ ] Test deterministic provider selection and built-in fallback.
- [ ] Test propagation of document id, revision, language, scope, lines, lexer state, and cancellation.
- [ ] Test registration disposal and provider unregistration.
- [ ] Test cancellation and rejection of results after revocation.
- [ ] Test stale completion rejection after a document revision changes.
- [ ] Test provider exceptions and plugin/provider attribution.
- [ ] Test invalid provider output and payload or range validation where applicable.
- [ ] Test idempotent disposal and absence of retained registrations or callbacks.
- [ ] Test the first-party provider without Avalonia or a real language server.
- [ ] Run the application/session tests that cover the existing tokenization completion path.

## Step 8: Compose the First-Party Provider Explicitly

- [ ] Create the registry in the appropriate composition root or application-owned integration module.
- [ ] Register only approved first-party providers explicitly.
- [ ] Do not add assembly scanning, dynamic loading, package installation, or a user plugin directory yet.
- [ ] Keep provider registration independent of frontend-specific composition where possible.
- [ ] Make enablement and disablement behavior explicit even if the first implementation is in-memory.
- [ ] Document the current trusted-code limitation next to the composition boundary.

## Step 9: Review Before Expanding Capabilities

- [ ] Confirm that the first provider proof satisfies registration, selection, fallback, cancellation, unregistration, stale-result rejection, and failure isolation.
- [ ] Review whether the contract can be implemented by C# and another .NET language without Functor-specific knowledge.
- [ ] Review whether request and result types can be serialized without changing their meaning.
- [ ] Confirm that no plugin receives unrestricted model access or internal message dispatch.
- [ ] Confirm that no public type accidentally depends on Avalonia or a frontend runtime.
- [ ] Decide whether the registry belongs in `Functor.PluginHost` or an application-owned integration module based on the proven boundary.
- [ ] Record unresolved design questions before adding a second capability.

## Deferred Work: Plugin Packaging and Discovery

Do not start this work until the in-memory provider proof is complete.

- [ ] Define a declarative manifest for stable identity, package version, host/API compatibility, capabilities, contributions, permissions, activation conditions, and supported platforms.
- [ ] Validate manifests before loading plugin code.
- [ ] Classify incompatible plugins separately from plugins that fail during execution.
- [ ] Add discovery and dependency resolution only after the contract is stable.
- [ ] Add persisted enablement state owned by the host.
- [ ] Add deterministic activation and idempotent deactivation.
- [ ] Add package installation, updates, or marketplace support only as separately approved work.

## Deferred Work: Additional Capabilities

Each capability must have its own contract, validation, permissions, lifecycle, and tests.

- [ ] Add diagnostics as a host-mediated, revision-aware capability.
- [ ] Add commands with namespaced identifiers and host-owned registrations.
- [ ] Add read-only document access using approved snapshots.
- [ ] Add file associations and language configuration as declarative language contributions.
- [ ] Add LSP integration behind a language capability rather than making LSP the plugin boundary.
- [ ] Add declarative views or panels only after frontend-neutral contribution types are defined.
- [ ] Defer arbitrary Avalonia control injection and unrestricted document writes.
- [ ] Defer notebook and agent execution until the capability model has been proven by language support.

## Deferred Work: WASM Runtime

WASM is a later execution model over the same logical capability contracts, not a second incompatible plugin API.

- [ ] Define a serialized protocol for operation ids, plugin and capability ids, requests, results, errors, cancellation, shutdown, and version negotiation.
- [ ] Define message and resource limits for the WASM host.
- [ ] Select a WASM hosting arrangement and document its actual security properties.
- [ ] Implement a WASM adapter without exposing CLR object references or internal Functor state.
- [ ] Reuse provider selection, revision validation, lifecycle, disposal, and failure semantics from the .NET host.
- [ ] Prove one narrow WASM language provider before adding broader capabilities.
- [ ] Document that WASM is not automatically a complete security sandbox.

## Completion Rule

The initial plugin implementation is complete only when the first-party .NET language provider can be enabled and disabled without changing editor invariants, leaving stale registrations, accepting stale results, or taking down the editor. Further plugin capabilities and runtime models should be added as separate, explicitly validated increments.
