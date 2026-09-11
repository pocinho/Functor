namespace Functor.Application

open System.Threading
open Functor.Domain.Document
open Functor.Domain.Syntax

type TokenizationScope =
    | FullDocument
    | Line of line:int
    | LineRange of startLine:int * endLine:int

type LexerState =
    | Initial
    | FSharpState of blockCommentDepth:int
    | CSharpState of inBlockComment:bool * inVerbatimString:bool * expectsTypeDeclarationName:bool
    | MarkdownState of inHtmlComment:bool * fencedCodeMarker:string option

type LexerSnapshot = { Line: int; State: LexerState }

type TokenProviderKind =
    | LocalLexical
    | SemanticLsp

type TokenLayer =
    | Lexical
    | Semantic

type TokenizationRequest = { DocumentId: DocumentId; Revision: int64; Language: string; Scope: TokenizationScope; Lines: string list; InitialState: LexerState }

type TokenProviderOutput =
    { Provider: TokenProviderKind
      Layer: TokenLayer
      Tokens: LineTokens list
      Snapshots: LexerSnapshot list
      FinalState: LexerState }

type TokenizationOutput = TokenProviderOutput

type TokenizationResult =
    { DocumentId: DocumentId
      Revision: int64
      Scope: TokenizationScope
      Provider: TokenProviderKind
      Layer: TokenLayer
      Tokens: LineTokens list
      Snapshots: LexerSnapshot list
      FinalState: LexerState }

type ITokenProvider =
    abstract Tokenize: request: TokenizationRequest * cancellationToken: CancellationToken -> Async<Result<TokenProviderOutput, string>>

type ITokenizerService = ITokenProvider