# Functor Roadmap

This roadmap outlines the major milestones for Functor’s development.

---

## ✅ Phase 1 — Core Architecture (Completed)
- Pure domain model
- Editing, Syntax, Navigation, Diagnostics subdomains
- CoreModel, CoreEvent, CoreLogic
- Position-based coordinate system
- Vertical scrolling and viewport slicing
- Domain builds cleanly
- Domain and rendering test projects

---

## ✅ Phase 2 — Pure Rendering & Text Metrics
- Text layout engine (line measurement, glyph runs)
- Syntax token slicing based on viewport + offsets
- Cursor rendering
- Selection rendering
- Diagnostic geometry
- Line numbers
- Backend-neutral `TextMetrics` and grapheme-aware `TextMeasurer`
- Rune-aware measurement with UTF-16 offsets preserved
- Rendering pipeline is implemented and validated in the pure model layer
- Current concrete backend is the Avalonia adapter, with final drawing calls routed through Avalonia
- SkiaSharp backend integration remains a separate follow-up milestone rather than a blocker for Phase 2 completion

---

## ✅ Phase 3 — Application Session Foundation (Completed)
- `Functor.Application` project
- `EditorSession` owns editor state and dispatches domain events
- Avalonia controls consume the application session instead of owning `CoreModel`
- Async application commands and effects
- Platform-neutral file, clipboard, and dialog services
- Session state with status and error reporting
- Host-side effect interpreter with injected platform services
- Focused application tests for command/effect flow

---

## ✅ Phase 4 — Desktop File I/O & Document Lifecycle (Completed)
- `EditingModel.createFromText`
- UTF-8 file loading and saving
- Newline normalization and explicit encoding policy
- Dirty-state and saved-snapshot tracking
- Desktop file dialogs
- New/open/save/save-as/close command flow
- Unsaved-change confirmation state
- File errors surfaced through application state

Desktop file loading, saving, saved-state tracking, and lifecycle command orchestration are implemented. Browser-specific file flows remain part of future cross-platform hosting work.

---

## ✅ Phase 5 — Avalonia Hosting & Essential Editor (Completed)
- ✅ `Functor.Avalonia` shared frontend
- ✅ Desktop editor host
- ✅ Pointer selection and vertical/horizontal scrolling
- ✅ Clipboard integration
- ✅ Selection replacement and deletion
- ✅ Vertical cursor movement
- ✅ Basic overwrite mode
- ✅ Tab display and hit-testing consistency
- ✅ Command routing and focus handling
- ✅ File menu and keyboard shortcuts
- ✅ Fixed line-number gutter
- ✅ Editor status bar with cursor, file type, and dirty state
- ✅ Desktop theme configuration
- ✅ Command palette
- ✅ Settings UI

Phase 5 is complete for the desktop-hosted essential editor. Cross-target hosting and runtime validation have been moved after the rendering backend milestone.

---

## 🚧 Phase 6 — Syntax & Language Services
- Tokenizer service
- SyntaxModel integration
- Incremental tokenization
- Basic language definitions (F#, C#, JSON, Markdown)
- Highlighting pipeline

---

## 🚧 Phase 7 — Navigation
- Search engine
- Jump list
- Symbol navigation
- Go-to-definition (LSP)
- Document outline

---

## 🚧 Phase 8 — Diagnostics
- LSP diagnostics bridge
- Severity rendering
- Inline diagnostics
- Gutter markers

---

## 🚧 Phase 9 — Advanced Editor Features
- Multi-cursor support
- Block selections
- Undo/Redo improvements
- Document tabs after per-document session state is stable
- External file change detection
- Search/replace editing

---

## 🚧 Phase 10 — Skia Rendering Backend
- SkiaSharp surface integration
- Shared renderer contract for Avalonia hosts
- Backend-specific glyph shaping and measurement
- Rendering performance and resource caching
- Visual regression coverage
- Current concrete backend is Avalonia-first; Skia can be added behind the same abstraction when it is needed

---

## 🚧 Phase 10.5 — Cross-Target Avalonia Hosting
- Browser WASM target
- Android and iOS targets
- Browser file picker and download/save flow
- Cross-target runtime validation

---

## 🚧 Phase 11 — Packaging & Distribution
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
