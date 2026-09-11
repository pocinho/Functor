namespace Functor.Workspace

open Functor.Domain.Document

type WorkspaceReplacement =
    | WorkspaceReady of WorkspaceModel
    | RequiresDiscardConfirmation of documentIds: DocumentId list

module WorkspaceLifecycle =
    let requestReplacement rootPath workspace =
        let unsaved = WorkspaceQuery.unsavedDocumentIds workspace

        if unsaved.IsEmpty then
            WorkspaceReady(WorkspaceModel.create rootPath)
        else
            RequiresDiscardConfirmation unsaved

    let replaceAfterDiscard rootPath (_workspace: WorkspaceModel) =
        WorkspaceModel.create rootPath

    let clearDocuments workspace =
        WorkspaceLogic.update ClearDocuments workspace
