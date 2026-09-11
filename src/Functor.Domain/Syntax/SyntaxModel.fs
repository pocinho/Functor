namespace Functor.Domain.Syntax

open Functor.Domain.Document

/// Represents a single lexical token produced by the tokenizer.
type Token =
    { Kind: string // e.g., "identifier", "keyword", "string", "comment"
      Line: int
      Column: int
      Length: int }

/// Represents the result of tokenizing a single line.
type LineTokens = { Line: int; Tokens: Token list }

/// Represents the syntax state of a document:
/// - language identity
/// - cached tokens
/// - incremental tokenization metadata
/// - last tokenization timestamp
type SyntaxModel =
    {
        /// Document whose contents the syntax state describes.
        DocumentId: DocumentId option

        /// Current buffer revision for the bound document.
        DocumentRevision: int64

        /// Language ID (e.g., "fsharp", "csharp", "json")
        Language: string option

        /// Cached tokens for each line.
        /// Rendering and semantic layers consume this.
        Tokens: LineTokens list

        /// Buffer revision represented by the current token cache.
        TokenizedRevision: int64 option

        /// Whether the syntax model is currently out of date.
        /// (e.g., editing occurred since last tokenization)
        IsDirty: bool

        /// Inclusive line ranges whose tokens are not current.
        DirtyRanges: (int * int) list

    }

module SyntaxModel =

    /// Empty syntax model (no language, no tokens).
    let create () =
        { DocumentId = None
          DocumentRevision = 0L
          Language = None
          Tokens = []
          TokenizedRevision = None
          IsDirty = false
          DirtyRanges = [] }

    let createForDocument documentId revision =
        let model = create ()

        { model with
            DocumentId = Some documentId
            DocumentRevision = revision }

    /// Marks the syntax model as needing re-tokenization.
    let markDirty revision (model: SyntaxModel) =
        { model with
            DocumentRevision = revision
            IsDirty = true
            DirtyRanges = [ (0, System.Int32.MaxValue) ] }

    let private normalizeRanges ranges =
        ranges
        |> List.map (fun (startLine, endLine) -> min startLine endLine, max startLine endLine)
        |> List.sortBy fst
        |> List.fold (fun merged range ->
            match merged with
            | (startLine, endLine) :: rest when fst range <= endLine + 1 ->
                (startLine, max endLine (snd range)) :: rest
            | _ -> range :: merged) []
        |> List.rev

    let markDirtyRange revision startLine endLine (model: SyntaxModel) =
        let ranges = normalizeRanges ((startLine, endLine) :: model.DirtyRanges)

        { model with
            DocumentRevision = revision
            IsDirty = true
            DirtyRanges = ranges }

    /// Updates the language ID.
    let setLanguage (model: SyntaxModel) (lang: string) =
        { model with
            Language = Some lang
            IsDirty = true
            DirtyRanges = [ (0, System.Int32.MaxValue) ] }

    let private removeRange startLine endLine ranges =
        ranges
        |> List.collect (fun (rangeStart, rangeEnd) ->
            if rangeEnd < startLine || rangeStart > endLine then
                [ rangeStart, rangeEnd ]
            else
                [ if rangeStart < startLine then
                      yield rangeStart, startLine - 1
                  if rangeEnd > endLine then
                      yield endLine + 1, rangeEnd ])
        |> normalizeRanges

    let markCleanFrom line (model: SyntaxModel) =
        let remainingRanges = removeRange line System.Int32.MaxValue model.DirtyRanges

        { model with
            IsDirty = remainingRanges <> []
            DirtyRanges = remainingRanges }

    /// Updates the token cache only when the result still matches the active buffer.
    let setTokens documentId revision tokens (model: SyntaxModel) =
        if model.DocumentId = Some documentId && model.DocumentRevision = revision then
            { model with
                Tokens = tokens
                TokenizedRevision = Some revision
                IsDirty = false
                DirtyRanges = [] }
        else
            model

    /// Replaces cached tokens for a line range only when the result still matches the active buffer.
    let setTokenRange documentId revision startLine endLine tokens (model: SyntaxModel) =
        if model.DocumentId = Some documentId && model.DocumentRevision = revision then
            let firstLine = min startLine endLine
            let lastLine = max startLine endLine
            let outsideRange =
                model.Tokens
                |> List.filter (fun lineTokens -> lineTokens.Line < firstLine || lineTokens.Line > lastLine)

            let replacement =
                tokens
                |> List.filter (fun lineTokens -> lineTokens.Line >= firstLine && lineTokens.Line <= lastLine)

            let remainingRanges = removeRange firstLine lastLine model.DirtyRanges

            { model with
                Tokens = List.sortBy (fun lineTokens -> lineTokens.Line) (outsideRange @ replacement)
                TokenizedRevision = Some revision
                IsDirty = remainingRanges <> []
                DirtyRanges = remainingRanges }
        else
            model

    /// Immutable empty model for initialization
    let empty = create ()
