namespace Functor.Domain.Core

open Functor.Domain.Document
open Functor.Domain.Editing
open Functor.Domain.Syntax
open Functor.Domain.Navigation
open Functor.Domain.Diagnostics

/// Root event union for the entire editor.
/// All subdomain events are composed here.
type CoreEvent =
    // ────────────────────────────────────────────────
    // Document Lifecycle
    // ────────────────────────────────────────────────
    | OpenDocument of path: string
    | CloseDocument of id: DocumentId
    | SwitchDocument of id: DocumentId
    | ApplyDocumentEvent of DocumentEvent

    // ────────────────────────────────────────────────
    // Editing (buffer, cursor, selection)
    // ────────────────────────────────────────────────
    | ApplyEditingEvent of EditingEvent

    // ────────────────────────────────────────────────
    // Syntax Highlighting + Tokenization
    // ────────────────────────────────────────────────
    | ApplySyntaxEvent of SyntaxEvent

    // ────────────────────────────────────────────────
    // Navigation (jump list, search, symbols)
    // ────────────────────────────────────────────────
    | ApplyNavigationEvent of NavigationEvent

    // ────────────────────────────────────────────────
    // Diagnostics (LSP errors, warnings, hints)
    // ────────────────────────────────────────────────
    | ApplyDiagnosticsEvent of DiagnosticsEvent

    // ────────────────────────────────────────────────
    // Editor Mode (normal, insert, visual)
    // ────────────────────────────────────────────────
    | ChangeMode of EditorMode

    // ────────────────────────────────────────────────
    // Rendering + Layout
    // ────────────────────────────────────────────────
    | ResizeViewport of width: int * height: int
    | ScrollTo of offset: int
    | ScrollBy of delta: int

    // ────────────────────────────────────────────────
    // Workspace-level events (future extension)
    // ────────────────────────────────────────────────
    | WorkspaceEvent of obj

    // ────────────────────────────────────────────────
    // Agentic events (OpFlow, MCP, AI integration)
    // ────────────────────────────────────────────────
    | AgentEvent of obj

    // ────────────────────────────────────────────────
    // No-op (safe default)
    // ────────────────────────────────────────────────
    | NoOp
