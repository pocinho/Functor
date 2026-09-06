# Functor Roadmap

This roadmap outlines the major milestones for Functor’s development.

---

## ✅ Phase 1 — Core Architecture (Completed)
- Pure domain model
- Editing, Syntax, Navigation, Diagnostics subdomains
- CoreModel, CoreEvent, CoreLogic
- Position-based coordinate system
- Vertical + horizontal scrolling
- Domain builds cleanly

---

## 🚧 Phase 2 — Rendering Engine
- Text layout engine (line measurement, glyph runs)
- Syntax token slicing based on viewport + offsets
- Cursor rendering
- Selection rendering
- Diagnostics gutter
- Line numbers
- SkiaSharp surface integration

---

## 🚧 Phase 3 — Syntax & Language Services
- Tokenizer service
- SyntaxModel integration
- Incremental tokenization
- Basic language definitions (F#, C#, JSON, Markdown)
- Highlighting pipeline

---

## 🚧 Phase 4 — Navigation
- Search engine
- Jump list
- Symbol navigation
- Go-to-definition (LSP)
- Document outline

---

## 🚧 Phase 5 — Diagnostics
- LSP diagnostics bridge
- Severity rendering
- Inline diagnostics
- Gutter markers

---

## 🚧 Phase 6 — Editor Features
- Multi-cursor support
- Block selections
- Undo/Redo improvements
- Clipboard integration
- File I/O layer

---

## 🚧 Phase 7 — Functor.App (Avalonia)
- MVU wiring
- EditorSurface control
- Input handling
- Command palette
- Settings UI

---

## 🚧 Phase 8 — Packaging & Distribution
- Windows installer
- macOS bundle
- Linux AppImage
- NuGet packages for reusable components

---

## 🚀 Long-Term Vision
- Agentic integration (MCP)
- Plugin system
- Workspace model
- Debugger integration
- Terminal panel
