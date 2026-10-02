# Notebook Model

A notebook is a sequence of cells.

## Cell Types
- Markdown
- Code (Rust, Python, JS, etc.)
- Visualization (plots, charts)
- Agent (LLM-assisted)

## Execution Model
Cells execute in isolated runtimes:
- WASM
- Native processes
- Embedded interpreters

Outputs are rendered inline.
