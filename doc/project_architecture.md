# Functor Project Architecture

The project follows a layered architecture pattern.

## Layer 1: Domain Layer (`Functor.Domain`)
- Core domain-driven design with subdomains:
  - `Core`: Core model, events, logic
  - `Document`: Document-specific operations (Model, Event, Logic)
  - `Editing`: Editing operations and events
  - `Navigation`: Navigation operations and events
  - `Diagnostics`: Diagnostics and error handling
- Each subdomain defines its own types, events, and logic in F# files

## Layer 2: Application Layer (`Functor.Application`)
- High-level commands and effects that remain framework-agnostic
- `AppCommand`: Defines UI-emitted commands (ExecuteCoreEvent, NewDocumentRequested, etc.)
- `AppEffect`: Defines platform-specific side effects (WriteFile, OpenFile, etc.)

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

## Key Characteristics
- Written in F# with functional programming principles
- Clean separation of concerns through layered architecture
- Domain events (SearchEvent, Navigation events, etc.) drive state changes
- UI layer adapts platform-specific behavior while preserving domain logic
- All layers expose commands/effects that are decoupled from implementation details

---