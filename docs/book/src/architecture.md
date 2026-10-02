# High-Level Architecture

Functor’s architecture is intentionally modular and layered:

## 1. Core Layer (MVU)
The MVU engine defines:
- `Model` — immutable state tree.
- `Message` — events that mutate the model.
- `Update` — pure function transforming state.
- `View` — declarative rendering description.

This layer is platform-agnostic and test-friendly.

## 2. Rendering Layer
A GPU-backed renderer (Skia, WGPU, or Vello) draws:
- text buffers
- notebook cells
- UI chrome
- agent panels
- diagnostics

The renderer consumes a virtual layout tree produced by MVU.

## 3. Notebook Layer
Cells are typed units:
- `CodeCell`
- `MarkdownCell`
- `VisualizationCell`
- `AgentCell`

Each cell has:
- execution context
- output surface
- metadata (tags, dependencies)

## 4. Agent Layer
Agents are pluggable:
- Copilot
- Local LLMs (llama.cpp, ONNX, QNN)
- Remote inference endpoints

Agents operate through a unified protocol:
- `AgentRequest`
- `AgentResponse`
- `AgentTask`

## 5. Workspace Layer
A workspace is a folder containing:
- project files
- notebooks
- metadata
- agent history

## 6. Plugin Layer
Plugins extend:
- language support
- cell types
- agent capabilities
- renderers
- commands
