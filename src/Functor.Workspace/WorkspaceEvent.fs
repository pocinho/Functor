namespace Functor.Workspace

open Functor.Domain.Diagnostics
open Functor.Domain.Document
open Functor.Domain.Editing
open Functor.Domain.Navigation
open Functor.Domain.Syntax

type WorkspaceEvent =
    | AddDocument of DocumentModel
    | ActivateDocument of DocumentId
    | RemoveDocument of DocumentId
    | SetRootPath of string option
    | ReplaceWorkspace of rootPath: string option
    | ClearDocuments
    | ClearRecentlyClosed
    | ConsumeRecentlyClosed of path: string
    | ReplaceDocumentState of PerDocumentSessionState
    | ApplyEditingEvent of documentId: DocumentId * event: EditingEvent
    | ApplySyntaxEvent of documentId: DocumentId * revision: int64 * event: SyntaxEvent
    | ApplyNavigationEvent of documentId: DocumentId * revision: int64 * event: NavigationEvent
    | ApplyDiagnosticsEvent of documentId: DocumentId * revision: int64 * event: DiagnosticsEvent
