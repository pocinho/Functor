# Current Project Structure

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
    ├── build.fsx                # build script
    └── packaging/               # installers, icons, manifests

```

The rendering architecture is intentionally split into layers:

- `Functor.Domain`: editor state and behavior
- `Functor.Rendering`: pure slicing, layout, and geometry generation
- `Functor.Rendering.RenderBackend`: backend abstraction for platform draw adapters
- `Functor.Avalonia`: current concrete backend and frontend integration

This means the project is not tied to a single renderer yet, even though Avalonia is the active concrete backend today.
---