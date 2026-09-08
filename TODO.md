1. **Add `Functor.Rendering.Skia` project** and implement a Skia renderer that consumes only `RenderingModel` (no changes to domain/application code). Keep Avalonia responsible *only* for hosting the drawing surface, handling key/pointer events, focus management, and scrollbar rendering.

2. **Introduce `Functor.Application` layer** with an `EditorSession` type:
   ```fsharp
   type EditorSession =
       { Core: CoreModel
         Status: AppStatus      // e.g., Loading, Loaded, Saving, Clean, Error
         PendingOperation: Operation option }
   ```
   `EditorControl` will *only*:
   - Dispatch UI commands (`OpenRequested`, `LoadCompleted`, `SaveRequested`, `SaveCompleted`, `CopyRequested`, `PasteRequested`)
   - Observe `EditorSession` state changes
   - *Not* own or modify `CoreModel` directly

3. **Define platform-neutral service interfaces** (in `Functor.Platform`):
   ```fsharp
   type FileReference = LocalPath of string | BrowserFile of obj
   type FileText = { Text: string; Encoding: Encoding; HasBom: bool; Newline: NewlineStyle }
   type FileService =
       { PickOpenFile: unit -> Async<Result<FileDocument, FileError>>
         ReadFile: FileReference -> Async<Result<FileText, FileError>>
         WriteFile: FileReference -> string -> Async<Result<unit, FileError>>
         PickSaveFile: string -> Async<Result<FileReference, FileError>> }
   type ClipboardService = { SetText: string -> Async<Result<unit, ClipboardError>>; GetText: unit -> Async<Result<string, ClipboardError>> }
   type DialogService = { ShowOpenDialog: unit -> Async<Result<FileReference, DialogError>>; ShowSaveDialog: string -> Async<Result<FileReference, DialogError>> }
   ```
   > *All services are async-only. Domain/Rendering never calls them directly.*

4. **Add `EditingModel.createFromText`** (domain-only, no platform dependency):
   - Normalize all input to LF internally (e.g., `"a\nb\n"` → `["a"; "b"; ""]`)
   - Clear undo/redo stacks
   - Set cursor to `(line 0, column 0)`
   - Set `IsDirty = false`
   - Detect/preserve BOM (internal editor uses LF; external save uses detected newline style if known)
   - Return `EditingModel` with lines and cursor

5. **Implement async `FileSystem` in `Functor.Platform`** (first version: UTF-8 only):
   - `ReadText`: `string (path) -> Async<Result<FileText, string>>` (detects UTF-8 BOM, normalizes internal to LF)
   - `WriteText`: `string (path) -> FileText -> Async<Result<unit, string>>` (saves using original newline style if file was loaded with it; otherwise uses default LF)
   - Errors are returned as strings (e.g., "Permission denied", "Invalid encoding"); no exceptions in UI path

6. **Add application commands** (UI dispatches commands; `EditorSession` updates state; platform services execute async IO):
   - `LoadFile(path)` → `EditorSession` sets `Status = Loading`; `FileService.ReadFile` async; on success: create document metadata, initialize `Editing` via `createFromText`, set active doc, clear dirty/cursor/selection/undo/redo
   - `Save()` → `EditorSession` sets `Status = Saving`; `FileService.WriteFile` async; on success: serialize buffer, mark doc clean, update metadata
   - `SaveAs(path)` → same as `SaveFile` but uses `PickSaveFile`; UI shows save dialog via Avalonia adapter only (domain receives `FileReference`, not dialog object)
   - `CopyRequested`/`PasteRequested` → handled by `EditorSession`; clipboard service async call

7. **Separate document identity from file path**:
   - `DocumentIdentity = { Id: DocumentId; Name: string; Location: FileReference option }` (e.g., `BrowserFile` for loaded docs; `LocalPath` for saved docs)
   - `Name` uses user-visible display name (not `Path.GetFileName`), since browser docs may lack stable paths
   - `CoreModel` stores `Documents: DocumentIdentity list` (not raw paths); `EditorControl` observes `DocumentIdentity`

8. **Define renderer-host boundary** (no domain/rendering changes needed for Skia):
   - `type IEditorRenderer = abstract Render: RenderingModel -> unit`
   - Avalonia initially implements this (hosts surface, handles input)
   - Skia renderer will implement `IEditorRenderer` later by consuming only `RenderingModel`; no coupling to Avalonia or platform
   - `RenderingConfig` injects `Measurer` (text measurement) and `Theme`; Avalonia/Browser provide their own measurer implementations if needed

9. **Browser-specific handling (explicit, no assumptions)**:
   - Open: use `FileService.PickOpenFile` → browser file picker; document gets `BrowserFile` `FileReference`
   - Save existing: `FileService.WriteFile` → browser "download" (no writable path claim unless browser grants it)
   - Save as: `FileService.PickSaveFile` → browser save dialog
   - Autosave: optional IndexedDB metadata only (no persistent file path); actual file write only on explicit "Save"
   - Recent files: IndexedDB metadata only (no path assumptions)
   - *Never* expose `path` to domain/rendering; domain uses `DocumentIdentity.Name` and `Location: FileReference option`

10. **Implementation order (strictly follow this sequence)**:
    1. Define async `FileService`, `ClipboardService`, `DialogService` interfaces + minimal desktop implementations (local FS, native dialogs). *No browser yet.*
    2. Implement `EditingModel.createFromText` (normalize LF internally).
    3. Refactor `CoreEvent.OpenDocument` to accept loaded `FileText` (via `FileService`) and create `Editing` via `createFromText`; `EditorSession` owns `Documents` and `ActiveDocumentId`.
    4. Add `EditorSession` and application commands (`LoadFile`, `Save`, `SaveAs`) in `Functor.Application`; `EditorControl` only dispatches commands and observes `EditorSession`.
    5. Implement desktop `FileService` (local FS + native dialogs); test open/load/save via commands.
    6. Implement browser `FileService` (browser file picker + download); test open/load/save with browser docs (use `BrowserFile` `FileReference`).
    7. Add `Functor.Rendering.Skia` project; implement `IEditorRenderer` for Skia using only `RenderingModel` (no Avalonia-specific code in rendering).
    8. Add browser persistence (IndexedDB for autosave metadata only) *after* basic open/save works in browser; no path assumptions.
    9. Add document tabs *only after* `EditorSession` correctly maintains per-document state (buffers, dirty flags, cursor, undo/redo per doc).

> ✅ **Why this works**:  
> - All platform-specific logic (file dialogs, browser file handling, persistence) lives in `Functor.Platform` and `Functor.Avalonia`/`Functor.Avalonia.Browser`. Domain (`Functor.Domain`) and Rendering (`Functor.Rendering`) remain pure, with no `#if`, browser APIs, or path assumptions.  
> - Async capabilities (`async` services) are used everywhere file/clipboard operations occur; desktop can complete async locally without blocking.  
> - `EditorControl` only dispatches commands and observes state—no session ownership, making UI testing easier and safe for browser recreation.  
> - `DocumentIdentity` separates name (user-visible) from location (`FileReference`), handling browser docs without stable paths.  
> - Skia rendering is added via `IEditorRenderer` boundary after `RenderingModel` is stable; no domain changes needed.  
> - Implementation order ensures foundational async/session layer is built first, then platform services (desktop → browser), then renderer swap (Avalonia → Skia), then tabs. No premature browser persistence.

