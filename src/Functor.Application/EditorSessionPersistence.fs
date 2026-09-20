namespace Functor.Application

open Functor.Domain.Document

type PendingDocumentSave =
    { Revision: int64
      ExpectedPath: string option
      Buffer: string list }

type EditorSessionPersistence() =
    let mutable pendingSaves: Map<DocumentId, PendingDocumentSave list> = Map.empty

    member _.Reset() = pendingSaves <- Map.empty

    member _.Queue(documentId, revision, expectedPath, buffer) =
        let pending = pendingSaves |> Map.tryFind documentId |> Option.defaultValue []

        pendingSaves <-
            pendingSaves.Add(
                documentId,
                { Revision = revision
                  ExpectedPath = expectedPath
                  Buffer = buffer }
                :: pending
            )

    member _.Remove(documentId, revision, expectedPath) =
        pendingSaves <-
            pendingSaves
            |> Map.change documentId (function
                | None -> None
                | Some pending ->
                    let remaining =
                        pending
                        |> List.filter (fun save -> save.Revision <> revision || save.ExpectedPath <> expectedPath)

                    if remaining.IsEmpty then None else Some remaining)

    member _.RemoveDocument(documentId) =
        pendingSaves <- pendingSaves |> Map.remove documentId

    member _.TryFindFileCompletion(path, activeDocumentId, activeRevision) =
        let canonicalPath = DocumentModel.canonicalizePath path

        pendingSaves
        |> Map.tryPick (fun documentId pending ->
            pending
            |> List.tryFind (fun save ->
                let matchesPath = save.ExpectedPath = Some canonicalPath

                let matchesUntitledSave =
                    save.ExpectedPath.IsNone
                    && activeDocumentId = Some documentId
                    && activeRevision = save.Revision

                matchesPath || matchesUntitledSave)
            |> Option.map (fun save -> documentId, save.Revision, save.ExpectedPath))

    member _.TryFindDocumentCompletion(documentId, revision, path) =
        let canonicalPath = DocumentModel.canonicalizePath path

        pendingSaves
        |> Map.tryFind documentId
        |> Option.bind (
            List.tryFind (fun save ->
                save.Revision = revision
                && (save.ExpectedPath.IsNone || save.ExpectedPath = Some canonicalPath))
        )
        |> Option.map (fun save -> save.Revision, save.ExpectedPath, save.Buffer)
