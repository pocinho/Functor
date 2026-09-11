namespace Functor.Application

open Functor.Domain.Document
open Functor.Domain.Editing

type IncrementalTokenizationState =
    { Snapshots: Map<DocumentId * int64 * int, LexerState>
      Range: (int * int) option }

module IncrementalTokenizationState =
    let empty = { Snapshots = Map.empty; Range = None }

    let reset = empty

    let beginEdit documentId oldRevision newRevision (change: EditingChange) state =
        let snapshots =
            state.Snapshots
            |> Map.toList
            |> List.choose (fun ((snapshotDocumentId, revision, line), snapshot) ->
                if snapshotDocumentId <> documentId || revision <> oldRevision then
                    None
                elif line > change.OldEndLine then
                    Some((documentId, newRevision, line + change.LineDelta), snapshot)
                else
                    Some((documentId, newRevision, line), snapshot))
            |> Map.ofList

        { Snapshots = snapshots
          Range = Some(change.StartLine, change.EndLine) }

    let snapshotAt documentId revision line state =
        state.Snapshots |> Map.tryFind (documentId, revision, line)

    let requestRange state = state.Range

    let initialState documentId revision scope state =
        match scope with
        | FullDocument -> Initial
        | Line line -> snapshotAt documentId revision line state |> Option.defaultValue Initial
        | LineRange(startLine, _) -> snapshotAt documentId revision startLine state |> Option.defaultValue Initial

    let recordCompletion documentId revision scope snapshots finalState state =
        let updatedSnapshots =
            snapshots
            |> List.fold (fun values (snapshot: LexerSnapshot) -> Map.add (documentId, revision, snapshot.Line) snapshot.State values) state.Snapshots

        let finalLine =
            match scope with
            | FullDocument -> snapshots |> List.tryLast |> Option.map (fun snapshot -> snapshot.Line + 1)
            | Line line -> Some(line + 1)
            | LineRange(_, endLine) -> Some(endLine + 1)

        let updatedSnapshots =
            match finalLine with
            | Some line -> Map.add (documentId, revision, line) finalState updatedSnapshots
            | None -> updatedSnapshots

        { state with Snapshots = updatedSnapshots }

    let isStable documentId revision bufferLength endLine finalState state =
        let nextLine = endLine + 1

        nextLine >= bufferLength
        || snapshotAt documentId revision nextLine state = Some finalState

    let extend startLine endLine state =
        { state with Range = Some(startLine, endLine + 1) }

    let clearRange state = { state with Range = None }