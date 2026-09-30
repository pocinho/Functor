# Functor Project Architecture

This document is the source of truth for project ownership and dependency direction.
The architecture is layered, with application policy and contracts inside concrete
platform, UI, protocol, and serialization adapters.

## Layer 1: Domain Layer (`Functor.Domain`)
- Core domain-driven design with subdomains:
  - `Core`: Core model, events, logic
  - `Document`: Document-specific operations (Model, Event, Logic)
  - `Editing`: Editing operations and events
  - `Navigation`: Navigation operations and events
  - `Diagnostics`: Diagnostics and error handling
- Each subdomain defines its own types, events, and logic in F# files

## Layer 2: Application Layer (`Functor.Application`)
- Use cases, application state, commands, effects, and port contracts
- `AppCommand`: Defines UI-emitted commands (ExecuteCoreEvent, NewDocumentRequested, etc.)
- `AppEffect`: Describes side effects without executing file, UI, protocol, or framework operations
- Application may reference `Functor.Domain`, `Functor.Workspace`, and `Functor.Rendering`
- Application must not reference Avalonia, Platform, LSP, PluginHost, Agent, or concrete serialization/I/O implementations

## Layer 3: UI Layer (`Functor.Avalonia`)
- Cross-platform UI implementation using Avalonia.fsx
- Platform-specific sub-layers:
  - `Functor.Avalonia.Desktop` - Windows desktop UI
  - `Functor.Avalonia.Android` - Android UI
  - `Functor.Avalonia.iOS` - iOS UI
  - `Functor.Avalonia.Browser` - Browser/web UI
- Contains views (axaml/fs), models, services, and controls specific to each platform

## Layer 4: Integration Layers
- `Functor.Agent`: AI agent integration (FS, MCP)
- `Functor.Lsp`: Language Server Protocol for syntax highlighting and code editing
- `Functor.Platform`: Cross-platform utilities (Clipboard, Filesystem, etc.)
- Integration projects are outer adapters. They may reference application contracts, but inner projects must not reference them.

## Dependency Rules

- `Functor.Domain` references no Functor project and no UI, I/O, protocol, or serialization library.
- `Functor.Workspace` and `Functor.Rendering` may reference `Functor.Domain`, but not `Functor.Application` or an outer adapter.
- `Functor.Application` may reference `Functor.Domain`, `Functor.Workspace`, and `Functor.Rendering` only.
- `Functor.Platform`, `Functor.Lsp`, `Functor.PluginHost`, `Functor.Agent`, and `Functor.Avalonia` are outer adapters and may reference inner contracts.
- The executable host is the composition root. Concrete adapters are constructed there and passed into application use cases.
- A project reference must express ownership. It must not be added only to reuse a convenience type.

These rules are checked by `Functor.Tests.Architecture`. Keep that test in sync when a
new project or an intentional architectural exception is introduced.

## Key Characteristics
- Written in F# with functional programming principles
- Clean separation of concerns through layered architecture
- Domain events (SearchEvent, Navigation events, etc.) drive state changes
- UI layer adapts platform-specific behavior while preserving domain logic
- All layers expose commands/effects that are decoupled from implementation details

---