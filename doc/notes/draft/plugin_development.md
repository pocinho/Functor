# Functor Plugin Development

## Purpose

Functor plugins may initially be developed as .NET components and, at a later stage, may be extended to support WebAssembly (WASM) plugins. This note records the development implications of those execution models.

The two models should share the same capability contracts and lifecycle semantics. They should differ only in how plugin code is hosted and how calls cross the execution boundary.

## Initial Runtime: .NET

The first plugin implementation may support any language that targets the .NET runtime and can consume the Functor plugin contract. This includes, in principle:

- F#;
- C#;
- Visual Basic;
- other .NET languages that produce compatible managed assemblies.

Language choice must not create a separate plugin API. A plugin is compatible because its assembly implements the versioned Functor contract, not because it was written in a particular .NET language.

The initial .NET model is appropriate for trusted first-party or development plugins. It is an in-process execution model, so it provides API separation and lifecycle management but does not provide a security sandbox. A loaded assembly may still access process resources, block threads, consume excessive memory, or retain references after disablement.

The host must therefore continue to enforce:

- capability restrictions;
- registration ownership;
- cancellation and disposal;
- request and result validation;
- failure attribution;
- explicit enablement and disablement.

A separate assembly is not a trust boundary. Third-party or untrusted plugins should not be described as sandboxed merely because they target .NET or are loaded into a separate assembly.

## Cross-Language Contract Rules

Supporting several .NET languages is practical, but the public contract should be designed for language interoperability rather than for F# convenience.

Public plugin contracts should prefer:

- stable identifiers and version values;
- records or classes with clear serialized meanings;
- enums or documented string values for extensible categories;
- immutable request and result data;
- explicit asynchronous operations;
- cancellation tokens or equivalent host cancellation signals;
- ordinary collections with documented ordering and nullability rules;
- exceptions translated into host-defined failure records.

The contract should avoid exposing internal Functor types, domain events, mutable editor state, Avalonia types, or implementation-specific F# representations. F# discriminated unions may be useful inside Functor, but public unions require deliberate cross-language and versioning treatment. They should not cross the plugin boundary by accident.

The same caution applies to .NET-specific object graphs, delegates retained by the host, reflection-based conventions, and direct references to application services. A plugin should communicate through documented capability interfaces and host-owned registrations.

## Recommended .NET Packaging Direction

The exact packaging format is not part of this note, but a .NET plugin should eventually declare enough metadata for the host to validate it before activation. That metadata should identify:

- stable plugin id and publisher;
- package and plugin contract versions;
- supported host and capability contract versions;
- required and optional capabilities;
- supported target frameworks and platforms;
- declared permissions and contributions;
- activation conditions.

The host should validate compatibility and contribution conflicts before executing plugin code. Assembly loading, discovery directories, package installation, and dependency resolution are later implementation concerns, not requirements of the first provider proof.

## Future Runtime: WebAssembly

There is no fundamental architectural impediment to supporting WASM plugins later, provided the plugin API remains capability-based, host-mediated, and independent of CLR object identity.

A WASM plugin cannot be treated as a normal .NET assembly. It will require a WASM host or runtime adapter, such as a WASI- or Component Model-compatible host, and a defined protocol for calls between Functor and the plugin.

The WASM stage should preserve the same observable plugin behavior:

- manifest and compatibility validation before activation;
- explicit capability grants;
- host-owned registration handles;
- correlated requests and results;
- document and revision validation;
- cancellation and timeouts;
- bounded payloads and resource use;
- failure reporting and disposal;
- no direct access to Functor's internal model or messages.

The implementation may use an embedded runtime, a separate process, or another host arrangement. That choice should be made when WASM support is designed. It must not change the capability semantics presented to the rest of Functor.

## Protocol Boundary for WASM

To keep a future WASM path viable, capability operations should have a serializable representation even when the first implementation uses direct .NET calls.

A suitable future protocol would define:

- operation and request identifiers;
- plugin and capability identifiers;
- document identity and revision for document-scoped work;
- request and result schemas;
- explicit error categories;
- cancellation and shutdown messages;
- bounded message and data sizes;
- version negotiation for the host and each capability.

The protocol should carry data, not shared references. A WASM plugin must receive approved snapshots or request results, not pointers to `AppSessionState`, workspace objects, renderer objects, or live .NET services.

This does not require implementing a wire protocol for the first .NET plugin. It means new public contracts should avoid assumptions that would make serialization or an out-of-process adapter impossible later.

## Execution Models

The two plugin runtimes should be understood as separate host adapters over one logical plugin contract:

```text
Functor capability host
        |
        +-- .NET adapter -> trusted managed plugin
        |
        +-- WASM adapter -> WASM module or WASM process
```

The adapter is responsible for translating calls, enforcing lifecycle state, propagating cancellation, and converting failures into host diagnostics. Capability selection, validation, provider precedence, and stale-result handling remain host responsibilities.

A WASM adapter may provide stronger process or memory boundaries than an in-process .NET adapter, but WASM should not automatically be treated as a complete security sandbox. The runtime, host configuration, imported capabilities, filesystem access, networking, and process arrangement determine the actual security properties.

## Capability Compatibility

A plugin manifest should identify both its execution model and its API requirements. For example, a future plugin could declare that it supports a tokenization capability while being available as a .NET assembly, a WASM module, or both.

Execution model must not determine capability meaning. A token provider implemented in C#, F#, Visual Basic, or WASM should receive the same logical request and be subject to the same validation and revision rules.

Where runtime limitations differ, the host should report a structured incompatibility rather than silently weakening the contract. Examples include:

- a capability requiring a .NET-only service;
- a requested permission unavailable to the WASM host;
- an unsupported platform or WASM feature;
- an incompatible capability contract version.

## Development Stages

### Stage 1: Trusted .NET Proof

Start with manually composed, trusted first-party .NET providers. Prove the narrow language-provider seam, including registration, selection, fallback, cancellation, unregistration, stale-result rejection, and failure isolation.

The proof should not require dynamic assembly discovery or a plugin marketplace.

### Stage 2: Stable Serialized Contracts

Before supporting WASM, define capability request and result types with explicit versioning and serialization rules. Keep internal Functor models and F# implementation types behind the host adapter.

Add conformance tests that can exercise the same provider behavior through direct .NET calls and a serialized representation where practical.

### Stage 3: WASM Adapter

Introduce one WASM runtime arrangement and one narrow capability, preferably a language provider. Validate:

- manifest and capability compatibility;
- activation and disposal;
- request correlation;
- cancellation and timeout behavior;
- payload limits;
- stale-result rejection;
- failure and runtime termination handling;
- fallback to built-in providers.

Do not introduce broad UI or model access as part of the first WASM proof.

## Design Constraints

The following constraints should remain explicit:

- .NET language support does not imply arbitrary access to Functor internals.
- In-process .NET plugins are trusted code, not sandboxed code.
- WASM support is a future execution option, not a reason to expose a global shared model.
- The public contract must remain frontend-neutral and serializable in principle.
- Runtime adapters must not change capability validation or lifecycle guarantees.
- A plugin that cannot meet a capability or runtime requirement must be rejected with a structured reason.
- Resource limits, cancellation, and disposal apply to both execution models.

## Conclusion

Functor can reasonably begin with plugins written in F#, C#, Visual Basic, and other compatible .NET languages. No fundamental architectural conflict prevents adding WASM plugins later.

The critical prerequisite is to keep the plugin boundary expressed as versioned, host-mediated capabilities carrying validated data, rather than as shared CLR objects or direct access to Functor's internal MVU state. With that boundary, .NET and WASM can be treated as different execution adapters over the same plugin architecture.
