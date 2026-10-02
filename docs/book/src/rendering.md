# Rendering Engine

The rendering engine is responsible for:

- text shaping
- syntax highlighting
- cell layout
- GPU drawing
- animations
- viewport management

The renderer consumes a virtual layout tree produced by MVU and resolves it into
GPU primitives.

Potential backends:
- SkiaSafe
- WGPU
- Vello
