# Functor Practical Architecture Guidelines

This document complements the foundational architecture documents. It does not replace or modify them. Its purpose is to clarify how Functor's principles apply to the practical constraints of a cross-platform Avalonia editor.

The foundational values remain unchanged:

- Keep domain logic pure, deterministic, and framework-independent.
- Keep rendering UI-agnostic.
- Keep application state explicit and testable.
- Preserve clear boundaries between domain, application, rendering, platform, and UI layers.
- Prefer immutable models, discriminated unions, composition, and small focused modules.

## Practical MVU Boundary

Functor follows MVU for authoritative application state, while allowing Avalonia controls to manage framework-local lifecycle concerns.

### Application And Domain State

Application and domain state must be:

- immutable where possible;
- owned by the appropriate Model or application coordinator;
- updated through explicit commands, events, or application operations;
- available to every consumer that needs it;
- testable without constructing an Avalonia control.

State that affects editor behavior, persistence, commands, document workflows, or other views must not exist only inside a view.

### View Lifecycle State

Avalonia controls may contain local mutable state when it is required to operate the framework or efficiently project the model. Examples include:

- event subscriptions and disposal handles;
- references to named controls;
- transient pointer, drag, focus, or gesture state;
- refresh and re-entrancy guards;
- cached projections and rendered control identities;
- generation counters for rejecting stale asynchronous results;
- rendering and measurement caches;
- animation or layout coordination state.

This state is legitimate when it is local, bounded, and derived from or subordinate to authoritative application state.

View-local state must not become a second source of truth. When state must survive control recreation, affect persistence, drive commands, or coordinate multiple views, move it into the application model or an application-layer coordinator.

## Views As Projections And Adapters

Avalonia views should primarily:

1. receive or observe application state;
2. project state into controls;
3. translate user input into application commands or domain events;
4. coordinate framework lifecycle and platform interaction;
5. report failures through the established application flow.

Views should not contain domain algorithms, persistence rules, syntax logic, or independent business workflows. Pure helper logic extracted from a view should live in a focused application or UI-support module and be unit tested independently.

## Application Coordinators

Application coordinators such as `EditorSession` may be mutable when they bridge synchronous model updates, asynchronous effects, cancellation, tokenization, persistence, and event publication.

The practical requirement is cohesion and explicit ownership, not the total absence of mutation. Coordinators should:

- keep authoritative editor state explicit;
- isolate pure transitions from effect orchestration;
- isolate cancellation and asynchronous scheduling;
- isolate persistence and pending-operation tracking;
- publish state changes consistently;
- reject stale asynchronous results;
- remain testable through focused seams.

A mutable coordinator is acceptable; a coordinator that silently becomes a second domain model is not.

## Rendering Boundary

`Functor.Rendering` remains UI-agnostic. It may maintain internal caches or use mutable implementation details for performance, provided that:

- its public behavior is deterministic for the same inputs;
- it does not reference Avalonia or SkiaSharp;
- it does not own window, DPI, or control state;
- cache invalidation is explicit when relevant theme, font, or measurement inputs change.

Avalonia rendering controls may own the framework-facing cache and redraw lifecycle that surrounds the pure rendering engine.

## Composition And Services

Platform services should be created at a composition boundary rather than deep inside reusable controls when practical. Small service records or interfaces are preferred for clipboard, file, dialog, tokenizer, and session dependencies.

This is an incremental design goal. Existing construction inside a control is acceptable while the behavior is covered and the replacement boundary is not yet needed, but new code should avoid hard-wiring platform implementations into reusable logic.

## Aggregates And Subdomains

Aggregate models such as the workspace may contain state from several subdomains when their responsibility is to preserve identity, membership, ordering, and lifecycle across workflows.

This composition is not itself a boundary violation. The following rules still apply:

- domain algorithms remain in their owning subdomain;
- aggregate modules coordinate results rather than reimplementing algorithms;
- cross-subdomain communication uses explicit data and events;
- revision and identity checks protect asynchronous updates;
- coupling should be reduced only when it improves ownership or testability.

Do not introduce opaque state blobs or extra abstraction layers solely to reduce legitimate aggregate composition.

## Module Size And Cohesion

File length is a signal, not a rule. Split a file when it contains independently testable responsibilities, changes for unrelated reasons, or makes ownership difficult to discover.

Prioritize extraction from orchestration-heavy files before splitting cohesive pure modules:

1. shell coordination and workspace-tree refresh;
2. editor-session tokenization and persistence coordination;
3. settings draft parsing, validation, and projection;
4. theme-schema parsing and overlay application;
5. service composition boundaries.

A large pure module such as a layout engine may remain intact when its functions share one clear responsibility and its behavior is well tested.

## Testing Expectations

Tests should reflect the real architectural boundaries:

- pure domain and rendering transformations should have direct unit tests;
- application coordinators should have focused tests for state transitions, cancellation, stale results, and effect ordering;
- Avalonia tests should cover important projection and interaction workflows;
- platform smoke tests should cover framework behavior that headless tests cannot reliably resolve;
- extracted pure helpers should be testable without a full UI host.

Testability is a reason to introduce a boundary, not a reason to remove all state from framework adapters.

## Review Questions

When reviewing new code, ask:

1. Where is the authoritative state?
2. Is this mutation domain/application state or framework-local lifecycle state?
3. Can the pure decision be tested without Avalonia?
4. Does the view translate input into commands, or does it perform domain logic directly?
5. Does an asynchronous operation reject stale results?
6. Does this module have one coherent reason to change?
7. Does a new abstraction clarify ownership, or only hide legitimate composition?
8. Does the change preserve portability, determinism, and the separation between rendering and UI?

The goal is practical purity: strict boundaries around core behavior, with enough local state at framework edges to keep the editor responsive, testable, and maintainable.
