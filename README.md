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
|   ├── Functor.App/             # Avalonia frontend
|   ├── Functor.App.Desktop/
|   ├── Functor.App.Browser/
|   ├── Functor.App.Android/
|   ├── Functor.App.iOS/
|   └── Functor.Tests/
└── build/
    ├── build.fsx                        # FAKE build script
    └── packaging/                       # installers, icons, manifests

```
---