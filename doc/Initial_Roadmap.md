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

## 🚧 Phase 2 — Pure Rendering & Text Metrics
- Text layout engine (line measurement, glyph runs)
- Syntax token slicing based on viewport + offsets
- Cursor rendering
- Selection rendering
- Diagnostic geometry
- Line numbers
- Backend-neutral `TextMetrics` and grapheme-aware `TextMeasurer`
- Rune-aware measurement with UTF-16 offsets preserved
- SkiaSharp backend integration pending

---

## ✅ Phase 3 — Application Session Foundation (Started)
- `Functor.Application` project
- `EditorSession` owns editor state and dispatches domain events
- Avalonia controls consume the application session instead of owning `CoreModel`

### Remaining
- Async application commands and effects
- Platform-neutral file, clipboard, and dialog services
- Session status and error reporting

---

## 🚧 Phase 4 — File I/O & Document Lifecycle
- `EditingModel.createFromText`
- UTF-8 file loading and saving
- Newline and encoding policy
- Dirty-state and saved-snapshot tracking
- Desktop file dialogs
- Browser file picker and download/save flow
- File errors surfaced through application state

---

## 🚧 Phase 5 — Avalonia Hosting & Essential Editor
- `Functor.Avalonia` shared frontend
- Desktop and browser WASM targets
- Android and iOS targets
- Pointer selection and scrolling
- Clipboard integration
- Selection replacement and deletion
- Vertical cursor movement
- Basic overwrite mode
- Tab display and hit-testing consistency
- Command routing and focus handling

### Remaining
- Command palette
- Settings UI
- Cross-target runtime validation

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
