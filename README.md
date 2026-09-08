# Functor

*A cross‑platform text editor built with F#.*

Functor is a modern, experimental text editor designed around **clarity**, **predictability**, and **modularity**. It uses a pure **Domain Core** (Model–View-Update) and one or more **frontends**, starting with Avalonia.

The architecture is inspired by how real editors evolved: VSCode/Monaco, Lapce, Zed, Xi, Helix, and Neovim. Functor is built to scale beyond a single UI, supporting multiple frontends, agentic workflows, LSP, plugins, and workspace‑level features.

---

## ✨ Goals

- A clean, predictable MVU architecture
- High‑performance Skia rendering
- Cross‑platform support (Desktop, Browser, Mobile)
- Modular domain subcomponents
- Workspace‑aware architecture
- Syntax highlighting + LSP integration
- Agentic workflows (A2A, MCP)
- F#‑first ergonomics and composability
- Long-term maintainability

---

## [DRAFT] Project Structure for Functor (F# + MVU + Skia)

```
Functor/
├── src/
|   ├── Functor.Domain/          # Pure editor engine (MVU core)
|   ├── Functor.Workspace/       # Project + file tree + settings
|   ├── Functor.Rendering/       # Skia/GPU rendering engine
|   ├── Functor.Input/           # Keymaps + commands + input system
|   ├── Functor.Serialization/   # Undo/redo + snapshots + persistence
|   ├── Functor.Rpc/             # JSON RPC + transports
|   ├── Functor.Lsp/             # Language Server Protocol
|   ├── Functor.Agent/           # MCP + AI integration
|   ├── Functor.Platform/        # OS-specific services
|   ├── Functor.PluginHost/      # Plugin system
|   ├── Functor.Shared/          # Common utilities
|   ├── Functor.Application/     # Application/session orchestration
|   ├── Functor.Avalonia/        # Avalonia frontend
|   ├── Functor.Avalonia.Desktop/
|   ├── Functor.Avalonia.Browser/
|   ├── Functor.Avalonia.Android/
|   ├── Functor.Avalonia.iOS/
|   └── Functor.Tests/
└── build/
    ├── build.fsx                        # FAKE build script
    └── packaging/                       # installers, icons, manifests

```
---

## Building Functor

Build-script **help** requires the F# Interactive delimiter:

```
dotnet fsi .\build\build.fsx -- --help
```

Use **custom** parameters, for example:


```
dotnet fsi .\build\build.fsx --configuration Debug --runtimeIdentifier win-x64 --outputRoot out --outputDirectory out/win-x64
```

Or you can run **build-release-win-x64.fsx**, which invokes the reusable script with the Release win-x64 settings:

```
dotnet fsi .\build\build-release-win-x64.fsx
```

---