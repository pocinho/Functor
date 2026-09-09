# Functor

*A cross‑platform text editor built with F#.*

Functor is a modern, experimental text editor designed around **clarity**, **predictability**, and **modularity**. It uses a pure **Domain Core** and a rendering pipeline that stays backend-neutral, with Avalonia as the current concrete frontend and drawing backend.

The architecture is inspired by how real editors evolved: VSCode/Monaco, Lapce, Zed, Xi, Helix, and Neovim. Functor is built to scale beyond a single UI, supporting multiple frontends, agentic workflows, LSP, plugins, and workspace‑level features.

The current implementation already includes a clear render pipeline:

- pure domain state and editing logic
- viewport-aware slicing and layout computation
- a geometry-first `RenderingModel`
- a backend boundary for rendering adapters
- current concrete drawing through Avalonia

This keeps the editor core portable while leaving room for a future Skia-specific backend behind the same abstraction.

---

## ✨ Goals

- A clean, predictable MVU-style architecture
- Pure rendering pipeline with backend abstraction
- Avalonia-first concrete backend today
- Cross‑platform support (Desktop, Browser, Mobile)
- Modular domain subcomponents
- Workspace‑aware architecture
- Syntax highlighting + LSP integration
- Agentic workflows (A2A, MCP)
- F#‑first ergonomics and composability
- Long-term maintainability
- A future Skia backend behind the same abstraction model

---

## Current Project Structure

```
Functor/
├── src/
|   ├── Functor.Domain/          # Pure editor engine and domain state
|   ├── Functor.Application/     # Application/session orchestration + app settings
|   ├── Functor.Rendering/       # Geometry-first rendering pipeline
|   ├── Functor.Avalonia/        # Avalonia frontend and concrete backend adapter
|   ├── Functor.Avalonia.Desktop/
|   ├── Functor.Avalonia.Browser/
|   ├── Functor.Avalonia.Android/
|   ├── Functor.Avalonia.iOS/
|   ├── Functor.Input/           # Keymaps + commands + input system
|   ├── Functor.Platform/        # OS-specific services
|   ├── Functor.Workspace/       # Project + file tree + settings
|   ├── Functor.Lsp/             # Language Server Protocol
|   ├── Functor.Agent/           # MCP + AI integration
|   ├── Functor.PluginHost/      # Plugin system
|   ├── Functor.Tests/           # Domain and rendering tests
|   └── Functor.WIP/             # Future or transitional modules as needed
└── build/
    ├── build.fsx                        # FAKE build script
    └── packaging/                       # installers, icons, manifests

```

The rendering architecture is intentionally split into layers:

- `Functor.Domain`: editor state and behavior
- `Functor.Rendering`: pure slicing, layout, and geometry generation
- `Functor.Rendering.RenderBackend`: backend abstraction for platform draw adapters
- `Functor.Avalonia`: current concrete backend and frontend integration

This means the project is not tied to a single renderer yet, even though Avalonia is the active concrete backend today.
---

## Desktop Theme Configuration

The desktop application optionally loads its theme from:

```text
%APPDATA%\Functor\settings.json
```

The built-in theme is used when the file is missing or invalid. Theme colors accept `#RRGGBB`, `#AARRGGBB`, or the same hexadecimal values without the leading `#`.

```json
{
    "theme": {
        "background": "#202020",
        "foreground": "#F0F0F0",
        "selection": "#405A82D9",
        "cursor": "#FFFFFF",
        "lineNumber": "#888888",
        "gutterBackground": "#181818",
        "editorBorder": "#555555",
        "editorBorderWidth": 2.0,
        "diagnosticError": "#FF5C5C",
        "diagnosticWarning": "#FFC857",
        "diagnosticInfo": "#5CC8FF"
    }
}
```

Only the values you need to change are required; unspecified palette values retain their defaults.
The `gutterBackground` value controls the fixed vertical line-number gutter, which remains stationary while the editor content scrolls horizontally.

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