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

## ✅ Phase 5.5 — Syntax & Rendering Contract Hardening (Completed)
- Buffer revisions advance only when document text changes
- Syntax state is invalidated by buffer revisions rather than cursor or selection movement
- Syntax and diagnostics reset across active-document lifecycle transitions
- Token caches are bound to document identity and buffer revision
- Stale asynchronous tokenization results are rejected
- Platform-neutral asynchronous tokenizer service and cancellation boundary
- Backend-neutral styled text runs with semantic color roles, weight, and slant
- Avalonia rendering consumes styled runs without interpreting token kinds
- UTF-16 token offsets and tab-aligned run geometry are covered by focused tests
- Syntax state transitions remain deterministic without domain-generated timestamps

This phase establishes the contracts required for incremental tokenization and highlighting without coupling language services to Avalonia or a future Skia backend.

---

## ✅ Phase 6 — Syntax & Language Services
- ✅ Platform-neutral basic tokenizer service boundary
- ✅ F# lexical tokenizer baseline targeting the repository's F# 10 / .NET 10 environment
- ✅ F# token classification for keywords, identifiers, type-like names, numbers, strings, characters, comments, directives, operators, and punctuation
- ✅ UTF-16 token offsets, cancellation, unsupported-language errors, and nested block-comment handling
- ✅ Revision-bound tokenizer request/completion flow integrated with `SyntaxModel`
- ✅ Hybrid scheduling: immediate tokenization on F# document load/switch and cancellable 150 ms debounce after edits
- ✅ Basic F# highlighting flowing through the existing backend-neutral rendering pipeline
- ✅ Application and tokenizer tests for lexical spans, Unicode offsets, cancellation, stale results, and debounced requests
- ✅ Language definitions and lexical tokenizers for C#, JSON, and Markdown
- ✅ Language selection and automatic tokenization for those additional file types
- ✅ Tokenization request scopes for full-document, line, and line-range tokenization
- ✅ Partial token-cache updates that replace only tokenized line ranges
- ✅ Dirty-range tracking in `SyntaxModel` instead of dirty/clean state only
- ✅ Editing change metadata so syntax invalidation can identify affected lines
- ✅ Stateful lexer snapshots for multiline constructs such as block comments, verbatim strings, fenced code blocks, and HTML comments
- ✅ Edit-time incremental tokenization that retokenizes from the changed range until lexer state stabilizes
- ✅ Provider abstraction for local lexical tokens versus future semantic/LSP tokens

Compiler-backed accuracy, FSharp.Compiler.Service integration, and LSP semantic tokens remain future work. The current tokenizer is intentionally lexical and does not validate or compile F# source.

---

## ✅ Phase 6.5 — Tokenization Architecture Cleanup
- ✅ Incremental snapshot and range bookkeeping extracted from `EditorSession`
- ✅ Lexer snapshots scoped by document identity and buffer revision
- ✅ Line insertion/deletion snapshot remapping covered by focused tests
- ✅ Provider-neutral token envelopes separated from local lexer state
- ✅ Lexical and semantic token layers defined before LSP integration

## ✅ Phase 6.6 — Multi-Document Workspace and Tabs (Completed)
### 6.6.1 — Workspace and Document Model
- ✅ Workspace model with identity, canonical root path, and active document
- ✅ Workspace-scoped document collection and tab membership
- ✅ Per-document session state for editing, syntax, diagnostics, navigation, and view
- ✅ Independent cursor, selection, undo/redo, and dirty state per tab

### 6.6.2 — Document Routing
- ✅ Document ID routing for editing, syntax, diagnostics, navigation, and language-service events
- ✅ Document and buffer revision validation for asynchronous results
- ✅ Document-scoped tokenization, lexer snapshots, and stale-result rejection

### 6.6.3 — Workspace Lifecycle
- ✅ Open-folder workflow with unsaved-change handling when replacing the active workspace
- ✅ Workspace switching and active-document fallback behavior
- ✅ Untitled and unsaved documents associated with the active workspace
- ✅ Workspace-scoped settings and persistence boundary

### 6.6.4 — Tab State and Behavior
- ✅ Canonical file-path identity, duplicate-open prevention, and explicit out-of-root file policy
- ✅ Tab activation and deterministic close behavior
- ✅ Workspace/document-aware save, close, reopening of already-open files, and revision-safe save-completion handling
- ✅ Recent-closed-tab history and an explicit reopen-closed-tab command

### 6.6.5 — File and Language-Service Integration
- ✅ Document-aware save requests and revision-safe save completion
- ✅ Workspace-aware language-service context
- ✅ Preservation of per-document syntax and lexer state across tab switches

### 6.6.6 — Workspace Projections
- ✅ Backend-neutral tab and workspace projections
- ✅ Workspace file-tree projection with stable document identity
- ✅ Active-document projection for frontend hosts

### 6.6.7 — Host UI (Completed)
- ✅ Avalonia tab bar and active-document projection
- ✅ Host-specific tab-bar rendering without coupling workspace behavior to Avalonia or Skia
- ✅ Multi-document, workspace, and persistence tests
- ✅ Automated Avalonia UI tests

## ✅ Phase 6.7 — Unicode and Emoji Text Integrity
- ✅ Preserve UTF-16 document offsets as the editor and tokenizer coordinate contract
- ✅ Keep cursor movement on grapheme boundaries, including surrogate-pair emoji such as `🚧`
- ✅ Make backspace and delete remove complete grapheme clusters
- ✅ Normalize externally supplied and vertically projected cursor positions to grapheme boundaries
- ✅ Add domain coverage for emoji surrogate pairs and combining-mark graphemes
- ✅ Add backend-neutral grapheme advance measurement instead of assuming every grapheme has one fixed advance
- ✅ Configure Avalonia to use Segoe UI Emoji as the Windows emoji fallback family
- ✅ Keep token, selection, diagnostic, cursor, and rendered text boundaries aligned to complete grapheme clusters
- ✅ Add rendering and Avalonia UI coverage for cursor spacing and hit testing after emoji

## 🚧 Phase 7 — Navigation

### Phase 7.0 — Host UI Foundation (Pending)
- Create Avalonia side panel widget abstraction (`SidePanelView`)
- Build document list widget (open tabs, file icons, close controls)
- Implement tab bar host component (active indicator, minimize/close)
- Wire into `Functor.Avalonia.Desktop` host

### Phase 7.1 — Search Implementation
- Basic regex-free text search engine (linear scan, defer indexing optimization)
- Case-insensitive by default, toggleable
- Highlight all matches in editor (`StyledText` runs)
- Preview snippet generation (context around match positions)
- Floating search widget UI (Avalonia control)
- Search in files across workspace (requires phase 7.0 side panel)
- Replace functionality
- Search history

### Phase 7.2 — Advanced Navigation
- Jump list implementation
- Symbol navigation index
- Go-to-definition (LSP bridge - future)
- Document outline tree

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
- Crash-recovery snapshots for dirty documents
- Startup detection and recovery-session prompt
- Recovery snapshot cleanup after save, discard, or explicit rejection

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
